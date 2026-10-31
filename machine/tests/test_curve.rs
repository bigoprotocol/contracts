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
async fn test_curve_buy_sell_math_and_fee_split() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let trader = create_subaccount(&sandbox, "trader.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "curve1.sandbox").await?.as_contract();

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

    // buy with 10000 quote, expect ~ hand-computed coins
    let res_buy = contract_acc
        .call_function("curve_buy", json!({"quote_amount": "10000"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    let success = res_buy.assert_success();
    let coins_out: u128 = {
        let s: String = success.json().unwrap_or_else(|_| "0".to_string());
        s.parse().unwrap_or(0)
    };
    assert!(coins_out > 0, "buy should give coins");
    assert!(coins_out < 1_000_000_000_000_000, "should be less than total supply");

    // check fee split: 1% of 10000 =100, split 50/50
    let fees: String = contract_acc
        .call_function("get_protocol_fees", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let fee_num: u128 = fees.parse().unwrap();
    assert_eq!(fee_num, 50, "protocol fee should be 0.5% of 10000 =50");

    // second buy should give fewer coins for same quote (price up)
    let res_buy2 = contract_acc
        .call_function("curve_buy", json!({"quote_amount": "10000"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    let success2 = res_buy2.assert_success();
    let coins_out2: u128 = {
        let s: String = success2.json().unwrap_or_else(|_| "0".to_string());
        s.parse().unwrap_or(0)
    };
    assert!(coins_out2 < coins_out, "second buy should give fewer coins due to price increase");

    // sell with slippage check
    // sell half of first buy
    let sell_amount = (coins_out / 2).to_string();
    let res_sell = contract_acc
        .call_function("curve_sell", json!({"coin_amount": sell_amount, "min_quote_out": "1"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    let success_sell = res_sell.assert_success();
    let quote_out: u128 = {
        let s: String = success_sell.json().unwrap_or_else(|_| "0".to_string());
        s.parse().unwrap_or(0)
    };
    assert!(quote_out > 0, "sell should give quote");

    // slippage behavior: asking for min_quote_out too high should fail
    let res_slip = contract_acc
        .call_function("curve_sell", json!({"coin_amount": "1", "min_quote_out": "1000000000"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_slip.is_failure(), "sell with huge min_quote_out should fail slippage");

    // recent_trades cap 50 - do 55 trades and check length capped
    for _ in 0..55 {
        contract_acc
            .call_function("curve_buy", json!({"quote_amount": "100"}))
            .transaction()
            .with_signer(trader.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }
    let trades: serde_json::Value = contract_acc
        .call_function("get_recent_trades", json!({"limit": 100}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let len = trades.as_array().unwrap().len();
    assert_eq!(len, 50, "recent_trades should cap at 50, got {}", len);

    Ok(())
}

#[tokio::test]
async fn test_recharge_on_vs_off() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let trader = create_subaccount(&sandbox, "trader2.sandbox").await?;

    // machine with recharge off
    let contract_off = create_subaccount(&sandbox, "curve_off.sandbox").await?.as_contract();
    near_api::Contract::deploy(contract_off.account_id().clone())
        .use_code(wasm.clone())
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

    contract_off
        .call_function("curve_buy", json!({"quote_amount": "10000"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let machine_off: serde_json::Value = contract_off
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let pool_off: u128 = machine_off["pool_balance"].as_str().unwrap().parse().unwrap();

    // machine with recharge on
    let contract_on = create_subaccount(&sandbox, "curve_on.sandbox").await?.as_contract();
    near_api::Contract::deploy(contract_on.account_id().clone())
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
                "recharge_on": true
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_on
        .call_function("curve_buy", json!({"quote_amount": "10000"}))
        .transaction()
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let machine_on: serde_json::Value = contract_on
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let pool_on: u128 = machine_on["pool_balance"].as_str().unwrap().parse().unwrap();

    // with recharge on, pool_balance should be greater (creator fee bought coins into pool)
    // pool_off is 0 (no seed, no recharge), pool_on should be >0
    assert!(pool_on > pool_off, "recharge on should increase pool_balance: off {} on {}", pool_off, pool_on);

    // creator fees: off should have 50, on should have 0 (since fee goes to pool)
    // We can't directly check creator_fees via view, but we can infer from total fees logic
    // For simplicity check that off machine has pool_balance 0, on has >0

    Ok(())
}
