use near_api::NearToken;
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
async fn test_graduation_happy_path() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "grad1.sandbox").await?.as_contract();

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

    // seed pool for resolution
    contract_acc
        .call_function("seed_pool", json!({"amount": "100000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // buy many via curve to reach graduation target 100_000_000_000? Our initial target is 100B but we use small reserves 100k/1M
    // For test, we can manually trigger graduation by setting graduation_target low via direct call? Our contract's target is fixed 100B, but we can buy enough to reach? Actually curve_quote_reserve starts 100k, need 100B => need huge buys
    // Simpler: we will directly call graduate after manually increasing quote_reserve via many buys
    // Let's do 1000 buys of 100M each? That's 100B

    // Instead, we will test graduate logic by first calling a helper that simulates reaching target
    // Our graduate checks quote_reserve >= target, so we need to reach it
    // Let's do curve_buy with large amount

    // For test, we'll set target artificially low by using a contract method that allows owner to set? Not exists
    // Instead we test the failure path: graduate before target should fail

    let res_before = contract_acc
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_before.is_failure(), "graduate before target should fail");

    // Now buy enough to reach target: we need 100B quote reserve, start 100k, need 100B
    // Buy 100B in one go
    let res_buy = contract_acc
        .call_function("curve_buy", json!({"quote_amount": "100000000000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    // This should succeed (price logic)
    res_buy.assert_success();

    let curve_state: String = contract_acc
        .call_function("get_curve_state", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    println!("curve after big buy: {}", curve_state);

    // Now graduate should succeed
    let res_grad = contract_acc
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    res_grad.assert_success();

    // Check status Graduated, largest pack unlocked, LP locked, prize pool untouched
    let status: String = contract_acc
        .call_function("get_graduation_status", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(status, "Graduated");

    let machine: serde_json::Value = contract_acc
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let pool_balance: String = machine["pool_balance"].as_str().unwrap().to_string();
    assert_eq!(pool_balance, "100000", "prize pool never touched during graduation");

    let lp_locked: bool = contract_acc
        .call_function("is_lp_locked", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(lp_locked, "LP should be locked permanently");

    let largest_unlocked: bool = contract_acc
        .call_function("is_largest_pack_unlocked", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(largest_unlocked, "largest pack should unlock on GRADUATED");

    Ok(())
}

#[tokio::test]
async fn test_failed_step_stays_pending_purchases_paused() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let trader = create_subaccount(&sandbox, "trader.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "grad2.sandbox").await?.as_contract();

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

    contract_acc
        .call_function("curve_buy", json!({"quote_amount": "100000000000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // Simulate failure at step 2
    let res_fail = contract_acc
        .call_function("graduate_with_failure_simulation", json!({"fail_at_step": 2}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    res_fail.assert_success(); // returns false but tx success

    let status: String = contract_acc
        .call_function("get_graduation_status", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(status, "GraduationPending", "should stay pending after failure");

    // purchases should be paused while pending
    let res_buy_pending = contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_buy_pending.is_failure(), "buy_pack should be paused while graduation pending");

    let res_curve_pending = contract_acc
        .call_function("curve_buy", json!({"quote_amount": "1000"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_curve_pending.is_failure(), "curve_buy should be paused while pending");

    Ok(())
}

#[tokio::test]
async fn test_bad_ratio_fails_closed_and_lp_lock_permanent() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "grad3.sandbox").await?.as_contract();

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
        .call_function("curve_buy", json!({"quote_amount": "100000000000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // set pool exists with bad ratio
    contract_acc
        .call_function("set_rhea_pool_state_for_test", json!({"exists": true, "bad_ratio": true}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res = contract_acc
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "bad ratio should fail closed");

    // now set bad_ratio false but exists true -> should also fail closed (pool already exists)
    contract_acc
        .call_function("set_rhea_pool_state_for_test", json!({"exists": true, "bad_ratio": false}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res2 = contract_acc
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res2.is_failure(), "existing pool should fail closed");

    // reset to no pool, graduate successfully, then check LP lock permanent (can't unlock)
    contract_acc
        .call_function("set_rhea_pool_state_for_test", json!({"exists": false, "bad_ratio": false}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res_ok = contract_acc
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    res_ok.assert_success();

    let lp_locked: bool = contract_acc
        .call_function("is_lp_locked", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(lp_locked);

    // graduation again should fail (already graduated)
    let res_again = contract_acc
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_again.is_failure(), "already graduated should fail");

    Ok(())
}


#[tokio::test]
async fn test_candle_snapshots_roll_correctly() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "grad4.sandbox").await?.as_contract();

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

    // snapshot first candle
    contract_acc
        .call_function("snapshot_candle", json!({"quote_reserve": "100000", "coin_reserve": "1000000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // snapshot second candle after fast_forward
    sandbox.fast_forward(10).await?;

    contract_acc
        .call_function("snapshot_candle", json!({"quote_reserve": "110000", "coin_reserve": "900000"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let candles: serde_json::Value = contract_acc
        .call_function("get_candles", json!({"limit": 10}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let len = candles.as_array().unwrap().len();
    assert!(len >= 1, "should have at least 1 candle, got {}", len);

    // Test cap 288 - create 300 candles via fast_forward each time
    for _ in 0..300 {
        sandbox.fast_forward(6).await?;
        contract_acc
            .call_function("snapshot_candle", json!({"quote_reserve": "100000", "coin_reserve": "1000000"}))
            .transaction()
            .with_signer(owner.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }

    let candles_final: serde_json::Value = contract_acc
        .call_function("get_candles", json!({"limit": 500}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let len_final = candles_final.as_array().unwrap().len();
    assert_eq!(len_final, 288, "candles should cap at 288, got {}", len_final);

    Ok(())
}

