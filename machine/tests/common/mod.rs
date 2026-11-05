#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use near_api::{NearToken, Signer};
use near_sdk::serde_json::json;
use std::sync::Arc;

pub const STORAGE_DEPOSIT: u128 = 10_000_000_000_000_000_000_000;
pub const COIN_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;
pub const VRF_DEPOSIT: u128 = 1_000_000_000_000_000_000_000;

pub fn genesis_signer() -> Arc<Signer> {
    Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )
    .unwrap()
}

pub async fn create_subaccount(
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

pub fn good_table() -> serde_json::Value {
    json!([
        {"multiplier_bps": 25, "odds_bps": 6000, "tier": "Common"},
        {"multiplier_bps": 100, "odds_bps": 2500, "tier": "Common"},
        {"multiplier_bps": 200, "odds_bps": 1000, "tier": "Rare"},
        {"multiplier_bps": 400, "odds_bps": 400, "tier": "Epic"},
        {"multiplier_bps": 800, "odds_bps": 100, "tier": "Mythic"}
    ])
}

pub fn coin_metadata() -> serde_json::Value {
    json!({
        "spec": "ft-1.0.0",
        "name": "Test Coin",
        "symbol": "TEST",
        "icon": null,
        "reference": null,
        "reference_hash": null,
        "decimals": 6
    })
}

pub fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

pub fn build_workspace_crate(crate_dir: &str, wasm_name: &str) -> Vec<u8> {
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

pub fn build_machine_wasm() -> Vec<u8> {
    let path = cargo_near_build::build_with_cli(Default::default()).expect("build machine");
    std::fs::read(path).expect("read machine wasm")
}

pub fn build_mock_outlayer_wasm() -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mock-outlayer");
    let status = std::process::Command::new("cargo")
        .args(["near", "build", "non-reproducible-wasm"])
        .current_dir(&dir)
        .status()
        .expect("cargo near build mock outlayer");
    assert!(
        status.success(),
        "cargo near build failed for mock-outlayer"
    );
    let wasm = dir.join("target/near/mock_outlayer.wasm");
    std::fs::read(wasm).expect("read mock outlayer wasm")
}

pub fn build_mock_rhea_wasm() -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mock-rhea");
    let status = std::process::Command::new("cargo")
        .args(["near", "build", "non-reproducible-wasm"])
        .current_dir(&dir)
        .status()
        .expect("cargo near build mock rhea");
    assert!(status.success(), "cargo near build failed for mock-rhea");
    let wasm = dir.join("target/near/mock_rhea.wasm");
    std::fs::read(wasm).expect("read mock rhea wasm")
}

pub async fn deploy_mock_rhea(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    name: &str,
    owner: &near_api::Account,
) -> testresult::TestResult<near_api::Contract> {
    let wasm = build_mock_rhea_wasm();
    let acc = create_subaccount(sandbox, name).await?;
    let contract = acc.as_contract();
    near_api::Contract::deploy(contract.account_id().clone())
        .use_code(wasm)
        .with_init_call("new", json!({"owner": owner.account_id()}))?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(contract)
}

pub async fn deploy_mock_usdc(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    name: &str,
    owner: &near_api::Account,
) -> testresult::TestResult<near_api::Contract> {
    let wasm = build_workspace_crate("mock-usdc", "mock_usdc");
    let acc = create_subaccount(sandbox, name).await?;
    let contract = acc.as_contract();
    near_api::Contract::deploy(contract.account_id().clone())
        .use_code(wasm)
        .with_init_call("new", json!({"owner": owner.account_id()}))?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(contract)
}

/// Creates the machine account, deploys the coin with the machine as owner,
/// then deploys and initializes the machine.
pub async fn setup_machine(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine_name: &str,
    coin_name: &str,
    owner: &near_api::Account,
    quote_token: &near_api::AccountId,
    vrf_pubkey: &str,
) -> testresult::TestResult<(near_api::Contract, near_api::Contract)> {
    setup_machine_with(
        sandbox,
        network,
        machine_name,
        coin_name,
        owner,
        quote_token,
        vrf_pubkey,
        None,
        None,
    )
    .await
}

pub async fn setup_machine_with(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine_name: &str,
    coin_name: &str,
    owner: &near_api::Account,
    quote_token: &near_api::AccountId,
    vrf_pubkey: &str,
    curve_alloc: Option<u128>,
    prize_alloc: Option<u128>,
) -> testresult::TestResult<(near_api::Contract, near_api::Contract)> {
    setup_machine_full(
        sandbox,
        network,
        machine_name,
        coin_name,
        owner,
        quote_token,
        vrf_pubkey,
        curve_alloc,
        prize_alloc,
        None,
        None,
    )
    .await
}

pub async fn setup_machine_full(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine_name: &str,
    coin_name: &str,
    owner: &near_api::Account,
    quote_token: &near_api::AccountId,
    vrf_pubkey: &str,
    curve_alloc: Option<u128>,
    prize_alloc: Option<u128>,
    graduation_target: Option<u128>,
    rhea: Option<&near_api::AccountId>,
) -> testresult::TestResult<(near_api::Contract, near_api::Contract)> {
    let machine_acc = create_subaccount(sandbox, machine_name).await?;
    let machine_id = machine_acc.account_id().clone();

    let coin_wasm = build_workspace_crate("coin", "coin");
    let coin_acc = create_subaccount(sandbox, coin_name).await?;
    let coin = coin_acc.as_contract();
    near_api::Contract::deploy(coin.account_id().clone())
        .use_code(coin_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": machine_id,
                "total_supply": COIN_TOTAL_SUPPLY.to_string(),
                "metadata": coin_metadata(),
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();

    let machine_wasm = build_machine_wasm();
    let machine = machine_acc.as_contract();
    near_api::Contract::deploy(machine_id.clone())
        .use_code(machine_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": coin.account_id(),
                "coin_token": coin.account_id(),
                "quote_token": quote_token,
                "prize_table": good_table(),
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": vrf_pubkey,
                "outlayer_account": "outlayer.testnet",
                "vrf_project_id": "bigodeployer.testnet/bigo-vrf",
                "milestone_threshold": 50,
                "milestone_new_top_mult_bps": 1000,
                "ft_gas": 30,
                "curve_coin_allocation": curve_alloc.map(|v| v.to_string()),
                "prize_coin_allocation": prize_alloc.map(|v| v.to_string()),
                "max_pack_price": "1000",
                "scrap_discount_amount": null,
                "treasury": null,
                "fan_coin": null,
                "initial_coin_balance": null,
                "graduation_target": graduation_target.map(|v| v.to_string()),
                "rhea": rhea
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok((machine, coin))
}

pub async fn mint(
    network: &near_api::NetworkConfig,
    token: &near_api::Contract,
    owner: &near_api::Account,
    to: &near_api::AccountId,
    amount: u128,
) -> testresult::TestResult<()> {
    token
        .call_function(
            "mint",
            json!({"account_id": to, "amount": amount.to_string()}),
        )
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

pub async fn ft_balance(
    network: &near_api::NetworkConfig,
    token: &near_api::Contract,
    account: &near_api::AccountId,
) -> testresult::TestResult<u128> {
    let value: String = token
        .call_function("ft_balance_of", json!({"account_id": account}))
        .read_only()
        .fetch_from(network)
        .await?
        .data;
    Ok(value.parse().unwrap_or(0))
}

pub async fn register(
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    account: &near_api::Account,
) -> testresult::TestResult<()> {
    machine
        .call_function(
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

pub async fn register_coin(
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

pub async fn ft_buy(
    network: &near_api::NetworkConfig,
    token: &near_api::Contract,
    buyer: &near_api::Account,
    machine: &near_api::Contract,
    amount: u128,
) -> testresult::TestResult<()> {
    register(network, machine, buyer).await?;
    token
        .call_function(
            "ft_transfer_call",
            json!({
                "receiver_id": machine.account_id(),
                "amount": amount.to_string(),
                "msg": "{\"action\":\"buy_pack\"}"
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

pub async fn curve_buy(
    network: &near_api::NetworkConfig,
    token: &near_api::Contract,
    buyer: &near_api::Account,
    machine: &near_api::Contract,
    amount: u128,
    min_coins_out: Option<u128>,
) -> testresult::TestResult<()> {
    register(network, machine, buyer).await?;
    let msg = match min_coins_out {
        Some(min) => format!("{{\"action\":\"curve_buy\",\"min_coins_out\":\"{}\"}}", min),
        None => "{\"action\":\"curve_buy\"}".to_string(),
    };
    token
        .call_function(
            "ft_transfer_call",
            json!({
                "receiver_id": machine.account_id(),
                "amount": amount.to_string(),
                "msg": msg
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

/// Sends coins through ft_transfer_call on the coin contract.
pub async fn curve_sell(
    network: &near_api::NetworkConfig,
    coin: &near_api::Contract,
    seller: &near_api::Account,
    machine: &near_api::Contract,
    coin_amount: u128,
    min_quote_out: Option<u128>,
) -> testresult::TestResult<()> {
    let msg = match min_quote_out {
        Some(min) => format!(
            "{{\"action\":\"curve_sell\",\"min_quote_out\":\"{}\"}}",
            min
        ),
        None => "{\"action\":\"curve_sell\"}".to_string(),
    };
    coin.call_function(
        "ft_transfer_call",
        json!({
            "receiver_id": machine.account_id(),
            "amount": coin_amount.to_string(),
            "msg": msg
        }),
    )
    .transaction()
    .deposit(NearToken::from_yoctonear(1))
    .gas(near_api::NearGas::from_tgas(250))
    .with_signer(seller.account_id().clone(), genesis_signer())
    .send_to(network)
    .await?
    .assert_success();
    Ok(())
}

pub async fn set_fail_transfers(
    network: &near_api::NetworkConfig,
    token: &near_api::Contract,
    owner: &near_api::Account,
    fail: bool,
) -> testresult::TestResult<()> {
    token
        .call_function("set_fail_transfers", json!({"fail": fail}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

pub async fn seed_pool(
    network: &near_api::NetworkConfig,
    token: &near_api::Contract,
    owner: &near_api::Account,
    machine: &near_api::Contract,
    amount: u128,
) -> testresult::TestResult<()> {
    token
        .call_function(
            "ft_transfer_call",
            json!({
                "receiver_id": machine.account_id(),
                "amount": amount.to_string(),
                "msg": "{\"action\":\"seed_pool\"}"
            }),
        )
        .transaction()
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(100))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

pub async fn wait_for_pack(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    token_id: u64,
) -> testresult::TestResult<serde_json::Value> {
    for _ in 0..40 {
        sandbox.fast_forward(1).await?;
        let pack: Option<serde_json::Value> = machine
            .call_function("get_pack", json!({"token_id": token_id}))
            .read_only()
            .fetch_from(network)
            .await?
            .data;
        if let Some(p) = pack {
            return Ok(p);
        }
    }
    panic!("pack {} was not created", token_id);
}

pub async fn wait_for_draw(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    draw_id: u64,
    settled: bool,
) -> testresult::TestResult<serde_json::Value> {
    for _ in 0..40 {
        sandbox.fast_forward(1).await?;
        let draw: serde_json::Value = machine
            .call_function("get_draw", json!({"draw_id": draw_id}))
            .read_only()
            .fetch_from(network)
            .await?
            .data;
        let status = draw["status"].as_str().unwrap_or("");
        if settled && (status == "Settled" || status == "FallbackSettled") {
            return Ok(draw);
        }
        if !settled && status != "Committed" && !status.is_empty() {
            return Ok(draw);
        }
    }
    panic!("draw {} did not reach the expected state", draw_id);
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn u128_str(value: &serde_json::Value) -> u128 {
    value.as_str().unwrap_or("0").parse().unwrap_or(0)
}
