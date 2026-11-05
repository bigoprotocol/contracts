use near_api::NearToken;
use near_sdk::base64::{engine::general_purpose::STANDARD, Engine};
use near_sdk::serde_json::json;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn build_crate(crate_dir: &str, wasm_name: &str) -> Vec<u8> {
    let root = workspace_root();
    let dir = root.join(crate_dir);
    let status = std::process::Command::new("cargo")
        .args(["near", "build", "non-reproducible-wasm"])
        .current_dir(&dir)
        .status()
        .expect("cargo near build");
    assert!(
        status.success(),
        "cargo near build failed for {}",
        crate_dir
    );
    let wasm = root
        .join("target/near")
        .join(wasm_name)
        .join(format!("{}.wasm", wasm_name));
    std::fs::read(wasm).expect("read wasm")
}

fn genesis_signer() -> std::sync::Arc<near_api::Signer> {
    near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )
    .unwrap()
}

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

fn machine_config() -> serde_json::Value {
    json!({
        "prize_table": [
            {"multiplier_bps": 25, "odds_bps": 6000, "tier": "Common"},
            {"multiplier_bps": 100, "odds_bps": 2500, "tier": "Common"},
            {"multiplier_bps": 200, "odds_bps": 1000, "tier": "Rare"},
            {"multiplier_bps": 400, "odds_bps": 400, "tier": "Epic"},
            {"multiplier_bps": 800, "odds_bps": 100, "tier": "Mythic"}
        ],
        "pack_art_url": "ipfs://QmTest",
        "vrf_pubkey": "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8",
        "outlayer_account": "outlayer.testnet",
        "vrf_project_id": "bigodeployer.testnet/bigo-vrf",
        "milestone_threshold": 50,
        "milestone_new_top_mult_bps": 1000,
        "ft_gas": 30,
        "curve_coin_allocation": 800000000000000u64,
        "prize_coin_allocation": 200000000000000u64,
        "max_pack_price": 1000,
        "scrap_discount_amount": 1
    })
}

#[tokio::test]
async fn test_launch_deploys_coin_and_machine() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let factory_acc = create_subaccount(&sandbox, "bigo.sandbox").await?;
    let factory = factory_acc.as_contract();

    let coin_wasm = build_crate("coin", "coin");
    let machine_wasm = build_crate("machine", "machine");
    let factory_wasm = cargo_near_build::build_with_cli(Default::default())?;
    let factory_wasm = std::fs::read(factory_wasm)?;

    near_api::Contract::deploy(factory.account_id().clone())
        .use_code(factory_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin_wasm": STANDARD.encode(&coin_wasm),
                "machine_wasm": STANDARD.encode(&machine_wasm),
                "machine_config": machine_config(),
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    factory
        .call_function("set_global_contract_hash", json!({"hash": "testhash"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    factory
        .call_function(
            "launch_coin",
            json!({
                "coin_name": "duck",
                "quote_token": "usdc.bigo.testnet",
                "is_fan_coin": false,
                "existing_coin": null,
                "treasury": null,
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(
            20_000_000_000_000_000_000_000_000,
        ))
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(5).await?;
    factory
        .call_function(
            "deploy_machine",
            json!({"machine_id": "duck-machine.bigo.sandbox"}),
        )
        .transaction()
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(10).await?;

    let count: serde_json::Value = factory
        .call_function("get_machine_count", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(count.as_u64(), Some(1));

    let machines: Vec<String> = factory
        .call_function("get_machines", json!({}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(machines.len(), 1);
    assert_eq!(machines[0], "duck-machine.bigo.sandbox");

    let info: serde_json::Value = factory
        .call_function(
            "get_machine_info",
            json!({"machine": "duck-machine.bigo.sandbox"}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(info["coin"], "duck.bigo.sandbox");
    assert_eq!(info["coin_token"], "duck.bigo.sandbox");

    // the coin minted the entire supply to the machine
    let supply: serde_json::Value = factory
        .call_function("get_machine_count", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(supply.as_u64(), Some(1));

    let coin_acc = near_api::Account("duck.bigo.sandbox".parse().unwrap());
    let coin = coin_acc.as_contract();
    let total: String = coin
        .call_function("ft_total_supply", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(total, "1000000000000000");

    let balance: String = coin
        .call_function(
            "ft_balance_of",
            json!({"account_id": "duck-machine.bigo.sandbox"}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(balance, "1000000000000000");

    // the machine initialized with the coin token
    let machine_acc = near_api::Account("duck-machine.bigo.sandbox".parse().unwrap());
    let machine = machine_acc.as_contract();
    let view: serde_json::Value = machine
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(view["coin_token"], "duck.bigo.sandbox");

    // duplicate launch is rejected
    let dup = factory
        .call_function(
            "launch_coin",
            json!({"coin_name": "duck", "quote_token": null, "is_fan_coin": false, "existing_coin": null, "treasury": null}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(20_000_000_000_000_000_000_000_000))
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(dup.is_failure(), "duplicate coin must be rejected");

    Ok(())
}

#[tokio::test]
async fn test_coin_deploy_failure_refunds_and_skips_registry() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let factory_acc = create_subaccount(&sandbox, "bigo.sandbox").await?;
    let factory = factory_acc.as_contract();

    let coin_wasm = build_crate("coin", "coin");
    let machine_wasm = build_crate("machine", "machine");
    let factory_wasm = cargo_near_build::build_with_cli(Default::default())?;
    let factory_wasm = std::fs::read(factory_wasm)?;
    near_api::Contract::deploy(factory.account_id().clone())
        .use_code(factory_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin_wasm": STANDARD.encode(&coin_wasm),
                "machine_wasm": STANDARD.encode(&machine_wasm),
                "machine_config": machine_config(),
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    factory
        .call_function("set_global_contract_hash", json!({"hash": "testhash"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    // invalid code makes the coin deploy fail
    factory
        .call_function(
            "set_coin_code",
            json!({"wasm": STANDARD.encode([1u8, 2, 3])}),
        )
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    factory
        .call_function(
            "launch_coin",
            json!({
                "coin_name": "duck",
                "quote_token": "usdc.bigo.testnet",
                "is_fan_coin": false,
                "existing_coin": null,
                "treasury": null,
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(
            20_000_000_000_000_000_000_000_000,
        ))
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(12).await?;

    let count: serde_json::Value = factory
        .call_function("get_machine_count", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        count.as_u64(),
        Some(0),
        "the machine must not be registered"
    );
    let info: serde_json::Value = factory
        .call_function(
            "get_machine_info",
            json!({"machine": "duck-machine.bigo.sandbox"}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(info.is_null(), "no machine info on a failed launch");
    let refunded: String = factory
        .call_function("get_total_refunded", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        refunded.parse::<u128>().unwrap(),
        20_000_000_000_000_000_000_000_000 - 100_000_000_000_000_000_000_000,
        "the creator is refunded the deposit minus the gas reserve"
    );

    Ok(())
}

#[tokio::test]
async fn test_protocol_fees_go_to_the_factory_treasury() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let factory_acc = create_subaccount(&sandbox, "bigo.sandbox").await?;
    let factory = factory_acc.as_contract();
    let usdc_acc = create_subaccount(&sandbox, "usdc.sandbox").await?;
    let usdc = usdc_acc.as_contract();
    near_api::Contract::deploy(usdc.account_id().clone())
        .use_code(build_crate("mock-usdc", "mock_usdc"))
        .with_init_call("new", json!({"owner": owner.account_id()}))?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let coin_wasm = build_crate("coin", "coin");
    let machine_wasm = build_crate("machine", "machine");
    let factory_wasm = cargo_near_build::build_with_cli(Default::default())?;
    let factory_wasm = std::fs::read(factory_wasm)?;
    near_api::Contract::deploy(factory.account_id().clone())
        .use_code(factory_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin_wasm": STANDARD.encode(&coin_wasm),
                "machine_wasm": STANDARD.encode(&machine_wasm),
                "machine_config": machine_config(),
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    factory
        .call_function("set_global_contract_hash", json!({"hash": "testhash"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    factory
        .call_function(
            "launch_coin",
            json!({
                "coin_name": "duck",
                "quote_token": usdc.account_id(),
                "is_fan_coin": false,
                "existing_coin": null,
                "treasury": null,
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(
            20_000_000_000_000_000_000_000_000,
        ))
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(5).await?;
    factory
        .call_function(
            "deploy_machine",
            json!({"machine_id": "duck-machine.bigo.sandbox"}),
        )
        .transaction()
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(10).await?;

    let machine = near_api::Account("duck-machine.bigo.sandbox".parse().unwrap()).as_contract();
    let view: serde_json::Value = machine
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(view["treasury"], factory.account_id().to_string());

    usdc.call_function(
        "mint",
        json!({"account_id": buyer.account_id(), "amount": "100000"}),
    )
    .transaction()
    .with_signer(owner.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    machine
        .call_function(
            "storage_deposit",
            json!({"account_id": buyer.account_id(), "registration_only": true}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(10_000_000_000_000_000_000_000))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    usdc.call_function(
        "ft_transfer_call",
        json!({
            "receiver_id": machine.account_id(),
            "amount": "1000",
            "msg": "{\"action\":\"buy_pack\"}"
        }),
    )
    .transaction()
    .deposit(NearToken::from_yoctonear(1))
    .gas(near_api::NearGas::from_tgas(250))
    .with_signer(buyer.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    sandbox.fast_forward(8).await?;

    let fees: String = machine
        .call_function("get_protocol_fees", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(fees, "2", "the 0.2 percent pack fee is protocol revenue");

    let creator_withdraw = machine
        .call_function("withdraw_protocol_fees", json!({"amount": "2"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(150))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(
        creator_withdraw.is_failure(),
        "the creator is not the treasury"
    );

    machine
        .call_function("withdraw_protocol_fees", json!({"amount": "2"}))
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(150))
        .with_signer(factory.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(8).await?;
    let treasury_balance: String = usdc
        .call_function("ft_balance_of", json!({"account_id": factory.account_id()}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        treasury_balance, "2",
        "the treasury receives the protocol fees"
    );

    Ok(())
}
