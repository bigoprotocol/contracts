use near_api::NearToken;
use near_api::NearGas;
use near_sdk::serde_json::json;

async fn create_subaccount(
    sandbox: &near_sandbox::Sandbox,
    name: &str,
) -> testresult::TestResult<near_api::Account> {
    let account_id: near_api::AccountId = name.parse().unwrap();
    sandbox
        .create_account(account_id.clone())
        .initial_balance(NearToken::from_near(1000))
        .send()
        .await?;
    Ok(near_api::Account(account_id))
}

fn good_table() -> serde_json::Value {
    json!([
        {"multiplier_bps": 25, "odds_bps": 6000, "tier": "Common"},
        {"multiplier_bps": 100, "odds_bps": 2500, "tier": "Common"},
        {"multiplier_bps": 200, "odds_bps": 1000, "tier": "Rare"},
        {"multiplier_bps": 400, "odds_bps": 400, "tier": "Epic"},
        {"multiplier_bps": 800, "odds_bps": 100, "tier": "Mythic"}
    ])
}

fn genesis_signer() -> std::sync::Arc<near_api::Signer> {
    near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    )
    .unwrap()
}

async fn deploy_machine(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    contract_name: &str,
) -> testresult::TestResult<(near_api::Contract, near_api::Account)> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let owner = create_subaccount(sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(sandbox, contract_name).await?.as_contract();

    near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "quote_token": "usdc.sandbox",
                "prize_table": good_table(),
                "pack_art_url": "ipfs://QmTest123",
                "vrf_pubkey": "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8",
                "recharge_on": false
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();

    // seed pool
    contract_acc
        .call_function("seed_pool", json!({"amount": "100000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();

    Ok((contract_acc, owner))
}

#[tokio::test]
async fn test_open_burns_before_randomness_and_status() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let (contract_acc, _) = deploy_machine(&sandbox, &network, "m1.sandbox").await?;

    // buy pack
    contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // check pack unopened before open
    let pack_before: serde_json::Value = contract_acc
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(pack_before["status"], "Unopened");

    // open pack
    contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // pack should be opened (burned before randomness)
    let pack_after: serde_json::Value = contract_acc
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(pack_after["status"], "Opened");

    let draw: serde_json::Value = contract_acc
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(draw["status"], "VrfRequested");

    Ok(())
}

#[tokio::test]
async fn test_double_open_fails() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let buyer = create_subaccount(&sandbox, "buyer2.sandbox").await?;
    let (contract_acc, _) = deploy_machine(&sandbox, &network, "m2.sandbox").await?;

    contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // second open should fail
    let res = contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "double open should fail");
    Ok(())
}

#[tokio::test]
async fn test_open_by_non_owner_fails() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let buyer = create_subaccount(&sandbox, "buyer3.sandbox").await?;
    let stranger = create_subaccount(&sandbox, "stranger.sandbox").await?;
    let (contract_acc, _) = deploy_machine(&sandbox, &network, "m3.sandbox").await?;

    contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res = contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "open by non-owner should fail");
    Ok(())
}

#[tokio::test]
async fn test_vrf_callback_valid_settles_and_stranger_fails() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let buyer = create_subaccount(&sandbox, "buyer4.sandbox").await?;
    let (contract_acc, _) = deploy_machine(&sandbox, &network, "m4.sandbox").await?;

    contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // valid VRF callback: alpha format vrf:{req_id}:{machine}:{draw_id}, sig 64 bytes hex, pubkey valid
    // For our simplified verification (hex length check only, no ed25519_verify host), any 64-byte sig and 32-byte pubkey passes
    let sig_hex = "a".repeat(128); // 64 bytes hex
    let alpha = format!("vrf:req123:m4.sandbox:1");

    // call on_vrf_result as self account (predecessor == current_account_id) by using contract account signer
    // The contract account's signer is genesis_signer but we need predecessor to be contract account itself
    // In near-sandbox, calling as contract account with genesis signer makes predecessor = contract.sandbox, which equals current_account_id
    let res = contract_acc
        .call_function(
            "on_vrf_result",
            json!({
                "token_id": 1,
                "vrf_output": {
                    "value": 1234,
                    "signature_hex": sig_hex,
                    "alpha": alpha
                }
            }),
        )
        .transaction()
        .gas(NearGas::from_tgas(50))
        .with_signer(contract_acc.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    // Should succeed and settle
    res.assert_success();

    let draw_after: serde_json::Value = contract_acc
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(draw_after["status"], "Settled");
    assert!(draw_after["prize"].is_object());

    // second settle should not re-settle (finalize-once)
    let res2 = contract_acc
        .call_function(
            "on_vrf_result",
            json!({
                "token_id": 1,
                "vrf_output": {
                    "value": 1234,
                    "signature_hex": sig_hex,
                    "alpha": alpha
                }
            }),
        )
        .transaction()
        .gas(NearGas::from_tgas(50))
        .with_signer(contract_acc.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    // second call returns false but should not panic, status stays Settled
    res2.assert_success();
    let draw_after2: serde_json::Value = contract_acc
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(draw_after2["status"], "Settled");

    Ok(())
}

#[tokio::test]
async fn test_vrf_callback_from_stranger_fails_and_bad_sig() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let buyer = create_subaccount(&sandbox, "buyer5.sandbox").await?;
    let stranger = create_subaccount(&sandbox, "stranger2.sandbox").await?;
    let (contract_acc, _) = deploy_machine(&sandbox, &network, "m5.sandbox").await?;

    contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // stranger callback should fail (predecessor != self)
    let sig_hex = "b".repeat(128);
    let alpha = "vrf:req123:m5.sandbox:1";

    let res_stranger = contract_acc
        .call_function(
            "on_vrf_result",
            json!({
                "token_id": 1,
                "vrf_output": {
                    "value": 123,
                    "signature_hex": sig_hex,
                    "alpha": alpha
                }
            }),
        )
        .transaction()
        .gas(NearGas::from_tgas(50))
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_stranger.is_failure(), "callback from stranger should fail");

    // bad signature (wrong length) should settle nothing, return false but not panic
    let bad_sig = "abc"; // too short
    let res_bad_sig = contract_acc
        .call_function(
            "on_vrf_result",
            json!({
                "token_id": 1,
                "vrf_output": {
                    "value": 123,
                    "signature_hex": bad_sig,
                    "alpha": alpha
                }
            }),
        )
        .transaction()
        .gas(NearGas::from_tgas(50))
        .with_signer(contract_acc.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    // Should succeed transaction but return false (not settle), so draw still VrfRequested
    res_bad_sig.assert_success();

    let draw: serde_json::Value = contract_acc
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(draw["status"], "VrfRequested", "bad sig should not settle");

    Ok(())
}

#[tokio::test]
async fn test_fallback_before_50_blocks_fails_after_succeeds_and_double_settle_fails() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let buyer = create_subaccount(&sandbox, "buyer6.sandbox").await?;
    let (contract_acc, _) = deploy_machine(&sandbox, &network, "m6.sandbox").await?;

    contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // fallback before 50 blocks should fail
    let res_early = contract_acc
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_early.is_failure(), "fallback before 50 blocks should fail");

    // advance 51 blocks
    sandbox.fast_forward(51).await?;

    // fallback after 50 blocks should succeed
    let res_ok = contract_acc
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    res_ok.assert_success();

    let draw_after: serde_json::Value = contract_acc
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(draw_after["status"], "FallbackSettled");

    // double settle should fail
    let res_double = contract_acc
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_double.is_failure(), "double settle should fail");

    Ok(())
}
