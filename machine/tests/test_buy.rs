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

#[tokio::test]
async fn test_successful_buy() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine.sandbox").await?.as_contract();

    near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "quote_token": "usdc.sandbox",
                "prize_table": good_table(),
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8",
                "recharge_on": false
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("seed_pool", json!({"amount": "100000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function(
            "buy_pack",
            json!({
                "max_quote_budget": "500",
                "min_net_tokens": "50",
                "min_maximum": "100",
                "pack_coins": "100"
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let nft: serde_json::Value = contract_acc
        .call_function("nft_token", json!({"token_id": "1"}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(nft["owner_id"], "buyer.sandbox");

    let draw: serde_json::Value = contract_acc
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(draw["status"], "Committed");

    let summary: serde_json::Value = contract_acc
        .call_function("get_machine_summary", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(summary["total_packs_sold"], 1);

    Ok(())
}

#[tokio::test]
async fn test_buy_fails_past_max_budget() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer2.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine2.sandbox").await?.as_contract();

    near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "quote_token": "usdc.sandbox",
                "prize_table": good_table(),
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": "pubkey",
                "recharge_on": false
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("seed_pool", json!({"amount": "100000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res = contract_acc
        .call_function("buy_pack", json!({"max_quote_budget": "10", "pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    assert!(res.is_failure());
    Ok(())
}

#[tokio::test]
async fn test_buy_fails_reservation() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer3.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine3.sandbox").await?.as_contract();

    near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "quote_token": "usdc.sandbox",
                "prize_table": good_table(),
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": "pubkey",
                "recharge_on": false
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let _ = owner;

    let res = contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    assert!(res.is_failure());
    Ok(())
}

#[tokio::test]
async fn test_buy_fails_min_net_tokens() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer4.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine4.sandbox").await?.as_contract();

    near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "quote_token": "usdc.sandbox",
                "prize_table": good_table(),
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": "pubkey",
                "recharge_on": false
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("seed_pool", json!({"amount": "100000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res = contract_acc
        .call_function("buy_pack", json!({"min_net_tokens": "200", "pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    assert!(res.is_failure());
    Ok(())
}
