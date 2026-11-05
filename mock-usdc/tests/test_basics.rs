use near_api::NearToken;
use near_sdk::serde_json::json;

async fn create_subaccount(
    sandbox: &near_sandbox::Sandbox,
    name: &str,
) -> testresult::TestResult<near_api::Account> {
    let account_id: near_api::AccountId = name.parse().unwrap();
    sandbox
        .create_account(account_id.clone())
        .initial_balance(NearToken::from_near(10))
        .send()
        .await?;
    Ok(near_api::Account(account_id))
}

fn build_table() -> serde_json::Value {
    json!([
        {"multiplier_bps": 25, "odds_bps": 6000, "tier": "Common"},
        {"multiplier_bps": 100, "odds_bps": 2500, "tier": "Common"},
        {"multiplier_bps": 200, "odds_bps": 1000, "tier": "Rare"},
        {"multiplier_bps": 400, "odds_bps": 400, "tier": "Epic"},
        {"multiplier_bps": 800, "odds_bps": 100, "tier": "Mythic"}
    ])
}

async fn test_basics_on(contract_wasm: Vec<u8>, crate_name: &str) -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let contract_acc = create_subaccount(&sandbox, "contract.sandbox")
        .await?
        .as_contract();
    let signer = near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )?;
    let init_args = if crate_name == "machine" {
        let coin: near_api::AccountId = "coin.sandbox".parse().unwrap();
        json!({
            "owner": owner.account_id(),
            "coin": coin,
            "prize_table": build_table(),
            "pack_art_url": "ipfs://QmTest",
            "vrf_pubkey": "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8"
        })
    } else {
        json!({"owner": owner.account_id()})
    };
    near_api::Contract::deploy(contract_acc.account_id().clone())
        .use_code(contract_wasm)
        .with_init_call("new", init_args)?
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();
    let method = if crate_name == "machine" {
        "get_table_hash"
    } else {
        "get_owner"
    };
    let _res: serde_json::Value = contract_acc
        .call_function(method, ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    Ok(())
}

#[tokio::test]
async fn test_contract_is_operational() -> testresult::TestResult<()> {
    let wasm_path = cargo_near_build::build_with_cli(Default::default())?;
    let wasm = std::fs::read(wasm_path)?;
    let crate_name = env!("CARGO_PKG_NAME");
    test_basics_on(wasm, crate_name).await
}
