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
async fn test_claim_happy_path() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine_claim.sandbox").await?.as_contract();

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
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("credit_claimable", json!({"account": buyer.account_id(), "amount": "200"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res = contract_acc
        .call_function("claim", json!({}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    res.assert_success();

    let claimable: String = contract_acc
        .call_function("get_claimable", json!({"account": buyer.account_id()}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(claimable, "0");
    Ok(())
}

#[tokio::test]
async fn test_claim_fails_nothing() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer2.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine_claim2.sandbox").await?.as_contract();

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

    let res = contract_acc
        .call_function("claim", json!({}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    assert!(res.is_failure());
    Ok(())
}

#[tokio::test]
async fn test_claim_works_while_paused() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer3.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine_claim3.sandbox").await?.as_contract();

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
        .call_function("credit_claimable", json!({"account": buyer.account_id(), "amount": "150"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("pause", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res = contract_acc
        .call_function("claim", json!({}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;

    res.assert_success();
    Ok(())
}

#[tokio::test]
async fn test_buy_fails_while_paused_and_escrow_atomic() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let seller = create_subaccount(&sandbox, "seller.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer4.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "machine_claim4.sandbox").await?.as_contract();

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
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(seller.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("list_pack", json!({"token_id": 1, "price": "500"}))
        .transaction()
        .with_signer(seller.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    contract_acc
        .call_function("pause", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res_paused = contract_acc
        .call_function("buy_pack", json!({"pack_coins": "100"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_paused.is_failure());

    let res_list_paused = contract_acc
        .call_function("buy_listed_pack", json!({"token_id": 1}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res_list_paused.is_failure());

    contract_acc
        .call_function("unpause", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let res_buy = contract_acc
        .call_function("buy_listed_pack", json!({"token_id": 1}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    res_buy.assert_success();

    let nft: serde_json::Value = contract_acc
        .call_function("nft_token", json!({"token_id": "1"}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(nft["owner_id"], "buyer4.sandbox");

    let seller_claim: String = contract_acc
        .call_function("get_claimable", json!({"account": seller.account_id()}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(seller_claim != "0");

    Ok(())
}
