use near_api::NearToken;
use near_sdk::base64::{engine::general_purpose::STANDARD, Engine};
use near_sdk::serde_json::json;

const LAUNCH_DEPOSIT: u128 = 20_000_000_000_000_000_000_000_000;
const VRF_DEPOSIT: u128 = 1_000_000_000_000_000_000_000;
const STORAGE_DEPOSIT: u128 = 10_000_000_000_000_000_000_000;

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

async fn register_coin(
    network: &near_api::NetworkConfig,
    coin: &near_api::Contract,
    account: &near_api::Account,
) -> testresult::TestResult<()> {
    coin.call_function(
        "storage_deposit",
        json!({"account_id": account.account_id(), "registration_only": true}),
    )
    .transaction()
    .deposit(NearToken::from_yoctonear(STORAGE_DEPOSIT))
    .with_signer(account.account_id().clone(), genesis_signer())
    .send_to(network)
    .await?
    .assert_success();
    Ok(())
}

fn machine_config(quote: &near_api::AccountId) -> serde_json::Value {
    let _ = quote;
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

async fn launch_machine(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    owner: &near_api::Account,
    quote: &near_api::AccountId,
) -> testresult::TestResult<(near_api::Contract, near_api::Contract, near_api::Contract)> {
    let factory_acc = create_subaccount(sandbox, "bigo.sandbox").await?;
    let factory = factory_acc.as_contract();
    let coin_wasm = build_crate("coin", "coin");
    let machine_wasm = build_crate("machine", "machine");
    let factory_wasm = build_crate("factory", "factory");
    near_api::Contract::deploy(factory.account_id().clone())
        .use_code(factory_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin_wasm": STANDARD.encode(&coin_wasm),
                "machine_wasm": STANDARD.encode(&machine_wasm),
                "machine_config": machine_config(quote),
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    factory
        .call_function("set_global_contract_hash", json!({"hash": "testhash"}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    factory
        .call_function(
            "launch_coin",
            json!({"coin_name": "duck", "quote_token": quote, "is_fan_coin": false, "existing_coin": null, "treasury": null}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(LAUNCH_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
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
        .send_to(network)
        .await?
        .assert_success();
    sandbox.fast_forward(12).await?;

    let machine_acc = near_api::Account("duck-machine.bigo.sandbox".parse().unwrap());
    let coin_acc = near_api::Account("duck.bigo.sandbox".parse().unwrap());
    Ok((factory, machine_acc.as_contract(), coin_acc.as_contract()))
}

async fn deploy_squad(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    owner: &near_api::Account,
    factory: &near_api::Contract,
) -> testresult::TestResult<near_api::Contract> {
    let squad_acc = create_subaccount(sandbox, "squad.sandbox").await?;
    let squad = squad_acc.as_contract();
    near_api::Contract::deploy(squad.account_id().clone())
        .use_code(build_crate("squad", "squad"))
        .with_init_call(
            "new",
            json!({"owner": owner.account_id(), "factory_account": factory.account_id()}),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(squad)
}

async fn squad_status(
    network: &near_api::NetworkConfig,
    squad: &near_api::Contract,
    id: u64,
) -> testresult::TestResult<String> {
    let view: serde_json::Value = squad
        .call_function("get_squad", json!({"squad_id": id}))
        .read_only()
        .fetch_from(network)
        .await?
        .data;
    Ok(view["status"].as_str().unwrap_or("").to_string())
}

#[tokio::test]
async fn test_winning_squad_distributes_real_coins() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let quote = create_subaccount(&sandbox, "usdc.sandbox").await?;
    let quote_contract = quote.as_contract();
    near_api::Contract::deploy(quote_contract.account_id().clone())
        .use_code(build_crate("mock-usdc", "mock_usdc"))
        .with_init_call("new", json!({"owner": owner.account_id()}))?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let (factory, machine, coin) =
        launch_machine(&sandbox, &network, &owner, quote_contract.account_id()).await?;
    let squad = deploy_squad(&sandbox, &network, &owner, &factory).await?;

    let member1 = create_subaccount(&sandbox, "m1.sandbox").await?;
    let member2 = create_subaccount(&sandbox, "m2.sandbox").await?;
    for member in [&member1, &member2] {
        quote_contract
            .call_function(
                "mint",
                json!({"account_id": member.account_id(), "amount": "100000"}),
            )
            .transaction()
            .with_signer(owner.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }

    squad
        .call_function(
            "create_squad",
            json!({"machine": machine.account_id(), "pack_price": "1000", "deadline_blocks": 100}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(50_000_000_000_000_000_000_000))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(5).await?;
    assert_eq!(squad_status(&network, &squad, 1).await?, "Created");

    squad
        .call_function(
            "register_with_machine",
            json!({"machine": machine.account_id()}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(STORAGE_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(100))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(3).await?;

    for member in [&member1, &member2] {
        quote_contract
            .call_function(
                "ft_transfer_call",
                json!({
                    "receiver_id": squad.account_id(),
                    "amount": "1000",
                    "msg": "{\"action\":\"join_squad\",\"squad_id\":1}"
                }),
            )
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .gas(near_api::NearGas::from_tgas(200))
            .with_signer(member.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }
    sandbox.fast_forward(5).await?;

    squad
        .call_function("buy_squad_pack", json!({"squad_id": 1}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(12).await?;
    assert_eq!(squad_status(&network, &squad, 1).await?, "Bought");

    squad
        .call_function("open_squad_pack", json!({"squad_id": 1}))
        .transaction()
        .deposit(NearToken::from_yoctonear(VRF_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(member1.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(51).await?;
    machine
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let draw: serde_json::Value = machine
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let prize: u128 = draw["prize"]["prize_amount"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let squad_acc = near_api::Account(squad.account_id().clone());
    register_coin(&network, &coin, &squad_acc).await?;

    squad
        .call_function("sync_squad_prize", json!({"squad_id": 1}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(12).await?;
    for member in [&member1, &member2] {
        register_coin(&network, &coin, member).await?;
        squad
            .call_function("claim_squad_share", json!({"squad_id": 1}))
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .gas(near_api::NearGas::from_tgas(250))
            .with_signer(member.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }
    sandbox.fast_forward(12).await?;

    let bal1: String = coin
        .call_function("ft_balance_of", json!({"account_id": member1.account_id()}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let bal2: String = coin
        .call_function("ft_balance_of", json!({"account_id": member2.account_id()}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let share = prize / 2;
    assert_eq!(
        bal1.parse::<u128>().unwrap(),
        share,
        "member 1 gets its pro rata share"
    );
    assert_eq!(
        bal2.parse::<u128>().unwrap(),
        share,
        "member 2 gets its pro rata share"
    );
    assert_eq!(squad_status(&network, &squad, 1).await?, "Opened");

    Ok(())
}

#[tokio::test]
async fn test_full_refund_marks_refunded_and_blocks_second_refund() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let quote = create_subaccount(&sandbox, "usdc.sandbox").await?;
    let quote_contract = quote.as_contract();
    near_api::Contract::deploy(quote_contract.account_id().clone())
        .use_code(build_crate("mock-usdc", "mock_usdc"))
        .with_init_call("new", json!({"owner": owner.account_id()}))?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let (factory, machine, _coin) =
        launch_machine(&sandbox, &network, &owner, quote_contract.account_id()).await?;
    let squad = deploy_squad(&sandbox, &network, &owner, &factory).await?;
    let member = create_subaccount(&sandbox, "m1.sandbox").await?;
    quote_contract
        .call_function(
            "mint",
            json!({"account_id": member.account_id(), "amount": "100000"}),
        )
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    squad
        .call_function(
            "create_squad",
            json!({"machine": machine.account_id(), "pack_price": "1000", "deadline_blocks": 100}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(50_000_000_000_000_000_000_000))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(5).await?;
    squad
        .call_function(
            "register_with_machine",
            json!({"machine": machine.account_id()}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(STORAGE_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(100))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(3).await?;
    quote_contract
        .call_function(
            "ft_transfer_call",
            json!({
                "receiver_id": squad.account_id(),
                "amount": "1000",
                "msg": "{\"action\":\"join_squad\",\"squad_id\":1}"
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(200))
        .with_signer(member.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(5).await?;

    machine
        .call_function("pause", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    squad
        .call_function("buy_squad_pack", json!({"squad_id": 1}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(12).await?;

    assert_eq!(squad_status(&network, &squad, 1).await?, "Refunded");
    let deposit: String = squad
        .call_function(
            "get_member_deposit",
            json!({"squad_id": 1, "account": member.account_id()}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        deposit.parse::<u128>().unwrap(),
        0,
        "deposits are zeroed on a full refund"
    );
    let refund: String = squad
        .call_function(
            "get_pending_refund",
            json!({"squad_id": 1, "account": member.account_id()}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(
        refund.parse::<u128>().unwrap() > 0,
        "the failed buy must be refunded to the payer"
    );

    // a second refund must not credit the deposit twice
    let second = squad
        .call_function("refund_squad", json!({"squad_id": 1}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(second.is_failure(), "second refund must panic");

    Ok(())
}

#[tokio::test]
async fn test_two_squads_get_distinct_draws() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let quote = create_subaccount(&sandbox, "usdc.sandbox").await?;
    let quote_contract = quote.as_contract();
    near_api::Contract::deploy(quote_contract.account_id().clone())
        .use_code(build_crate("mock-usdc", "mock_usdc"))
        .with_init_call("new", json!({"owner": owner.account_id()}))?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    let (factory, machine, _coin) =
        launch_machine(&sandbox, &network, &owner, quote_contract.account_id()).await?;
    let squad = deploy_squad(&sandbox, &network, &owner, &factory).await?;
    let m1 = create_subaccount(&sandbox, "m1.sandbox").await?;
    let m2 = create_subaccount(&sandbox, "m2.sandbox").await?;
    for member in [&m1, &m2] {
        quote_contract
            .call_function(
                "mint",
                json!({"account_id": member.account_id(), "amount": "100000"}),
            )
            .transaction()
            .with_signer(owner.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }

    for _ in 0..2 {
        squad
            .call_function("create_squad", json!({"machine": machine.account_id(), "pack_price": "1000", "deadline_blocks": 100}))
            .transaction()
            .deposit(NearToken::from_yoctonear(50_000_000_000_000_000_000_000))
            .with_signer(owner.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }
    sandbox.fast_forward(5).await?;
    squad
        .call_function(
            "register_with_machine",
            json!({"machine": machine.account_id()}),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(STORAGE_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(100))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(3).await?;

    for (member, id) in [(&m1, 1u64), (&m2, 2u64)] {
        quote_contract
            .call_function(
                "ft_transfer_call",
                json!({
                    "receiver_id": squad.account_id(),
                    "amount": "1000",
                    "msg": format!("{{\"action\":\"join_squad\",\"squad_id\":{}}}", id)
                }),
            )
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .gas(near_api::NearGas::from_tgas(200))
            .with_signer(member.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }
    sandbox.fast_forward(5).await?;

    for id in [1u64, 2u64] {
        squad
            .call_function("buy_squad_pack", json!({"squad_id": id}))
            .transaction()
            .gas(near_api::NearGas::from_tgas(300))
            .with_signer(owner.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
        sandbox.fast_forward(15).await?;
    }

    let s1: serde_json::Value = squad
        .call_function("get_squad", json!({"squad_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let s2: serde_json::Value = squad
        .call_function("get_squad", json!({"squad_id": 2}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let d1 = s1["draw_id"].as_u64().expect("squad 1 draw");
    let d2 = s2["draw_id"].as_u64().expect("squad 2 draw");
    assert_ne!(d1, d2, "two squads must not share a draw");

    Ok(())
}
