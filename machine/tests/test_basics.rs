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
async fn test_init_happy_and_views() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    println!("wasm size {}", wasm.len());

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "contract.sandbox").await?.as_contract();

    let res = near_api::Contract::deploy(contract_acc.account_id().clone())
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
        .send_to(&network)
        .await?;

    // Inspect full
    println!("Full debug: {:#?}", res);

    res.assert_success();

    let machine: serde_json::Value = contract_acc
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;

    println!("machine: {:?}", machine);
    assert_eq!(machine["pack_art_url"], "ipfs://QmTest123");
    Ok(())
}

#[tokio::test]
async fn test_init_rejects_bad_table() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "contract2.sandbox").await?.as_contract();
    let res = near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "prize_table": [{"multiplier_bps": 10000, "odds_bps": 10000, "tier": "Mythic"}],
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": "pubkey"
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure());
    Ok(())
}

#[tokio::test]
async fn test_init_rejects_empty_art() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts::builder().features("unit-testing").build())?;
    let wasm = std::fs::read(wasm_path)?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "contract3.sandbox").await?.as_contract();
    let res = near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": "coin.sandbox",
                "prize_table": good_table(),
                "pack_art_url": "",
                "vrf_pubkey": "pubkey"
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure());
    Ok(())
}
