mod common;

use common::*;
use ed25519_dalek::{Signer as _, SigningKey};
use near_sdk::serde_json::json;

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

async fn deploy_mock_outlayer(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    alpha: &str,
    signature_hex: &str,
) -> testresult::TestResult<near_api::Contract> {
    let acc = create_subaccount(sandbox, "outlayer.testnet").await?;
    let contract = acc.as_contract();
    near_api::Contract::deploy(contract.account_id().clone())
        .use_code(build_mock_outlayer_wasm())
        .with_init_call(
            "new",
            json!({"alpha": alpha, "signature_hex": signature_hex}),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(contract)
}

async fn set_mock_result(
    network: &near_api::NetworkConfig,
    outlayer: &near_api::Contract,
    alpha: &str,
    signature_hex: &str,
) -> testresult::TestResult<()> {
    outlayer
        .call_function(
            "set_result",
            json!({"alpha": alpha, "signature_hex": signature_hex}),
        )
        .transaction()
        .with_signer("sandbox".parse().unwrap(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

async fn open_pack(
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    buyer: &near_api::Account,
    token_id: u64,
) -> testresult::TestResult<()> {
    let _ = machine
        .call_function("open_pack", json!({"token_id": token_id}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(VRF_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(network)
        .await;
    Ok(())
}

async fn draw(
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    draw_id: u64,
) -> testresult::TestResult<serde_json::Value> {
    Ok(machine
        .call_function("get_draw", json!({"draw_id": draw_id}))
        .read_only()
        .fetch_from(network)
        .await?
        .data)
}

#[tokio::test]
async fn test_vrf_settles_and_pays_real_coins() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;

    let key = signing_key();
    let pubkey_hex = hex(&key.verifying_key().to_bytes());
    let machine_id: near_api::AccountId = "m1.sandbox".parse().unwrap();
    let alpha = format!("vrf:mock:{}:1", machine_id);
    let sig_hex = hex(&key.sign(alpha.as_bytes()).to_bytes());
    let outlayer = deploy_mock_outlayer(&sandbox, &network, &alpha, &sig_hex).await?;

    let (machine, coin) = setup_machine(
        &sandbox,
        &network,
        "m1.sandbox",
        "c1.sandbox",
        &owner,
        usdc.account_id(),
        &pubkey_hex,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;

    open_pack(&network, &machine, &buyer, 1).await?;
    let settled = wait_for_draw(&sandbox, &network, &machine, 1, true).await?;
    assert_eq!(settled["status"], "Settled");
    let prize = u128_str(&settled["prize"]["prize_amount"]);
    assert!(prize > 0);

    let machine_before = ft_balance(&network, &coin, machine.account_id()).await?;
    register_coin(&network, &coin, &buyer).await?;
    machine
        .call_function("claim", json!({}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(150))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(8).await?;

    assert_eq!(
        ft_balance(&network, &coin, buyer.account_id()).await?,
        prize,
        "the winner receives the exact prize in real coins"
    );
    assert_eq!(
        ft_balance(&network, &coin, machine.account_id()).await?,
        machine_before - prize
    );

    // adversarial: wrong machine in alpha must not settle
    let wrong_machine_alpha = format!("vrf:mock:{}:2", "wrong.sandbox");
    let wrong_machine_sig = hex(&key.sign(wrong_machine_alpha.as_bytes()).to_bytes());
    set_mock_result(
        &network,
        &outlayer,
        &wrong_machine_alpha,
        &wrong_machine_sig,
    )
    .await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 2).await?;
    open_pack(&network, &machine, &buyer, 2).await?;
    sandbox.fast_forward(5).await?;
    assert_eq!(draw(&network, &machine, 2).await?["status"], "VrfRequested");

    Ok(())
}

#[tokio::test]
async fn test_vrf_bad_signature_and_wrong_draw_do_not_settle() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;

    let key = signing_key();
    let pubkey_hex = hex(&key.verifying_key().to_bytes());
    let machine_id: near_api::AccountId = "m2.sandbox".parse().unwrap();
    let good_alpha = format!("vrf:mock:{}:1", machine_id);
    let bad_sig = hex(&key.sign(b"not the alpha").to_bytes());
    let outlayer = deploy_mock_outlayer(&sandbox, &network, &good_alpha, &bad_sig).await?;

    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "m2.sandbox",
        "c2.sandbox",
        &owner,
        usdc.account_id(),
        &pubkey_hex,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    open_pack(&network, &machine, &buyer, 1).await?;
    sandbox.fast_forward(5).await?;
    assert_eq!(draw(&network, &machine, 1).await?["status"], "VrfRequested");

    let wrong_draw_alpha = format!("vrf:mock:{}:99", machine_id);
    let wrong_draw_sig = hex(&key.sign(wrong_draw_alpha.as_bytes()).to_bytes());
    set_mock_result(&network, &outlayer, &wrong_draw_alpha, &wrong_draw_sig).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 2).await?;
    open_pack(&network, &machine, &buyer, 2).await?;
    sandbox.fast_forward(5).await?;
    assert_eq!(draw(&network, &machine, 2).await?["status"], "VrfRequested");

    Ok(())
}

#[tokio::test]
async fn test_callback_from_stranger_fails() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let stranger = create_subaccount(&sandbox, "stranger.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "m3.sandbox",
        "c3.sandbox",
        &owner,
        usdc.account_id(),
        "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8",
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    open_pack(&network, &machine, &buyer, 1).await?;

    let res = machine
        .call_function(
            "on_vrf_result",
            json!({
                "token_id": 1,
                "result": {"results": [{"value": 0, "signature_hex": "00", "alpha": "vrf:x:m3.sandbox:1"}]}
            }),
        )
        .transaction()
        .gas(near_api::NearGas::from_tgas(50))
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "callback from a stranger must fail");
    assert_eq!(draw(&network, &machine, 1).await?["status"], "VrfRequested");

    Ok(())
}

#[tokio::test]
async fn test_double_open_and_non_owner_open_fail() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let stranger = create_subaccount(&sandbox, "stranger.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "m4.sandbox",
        "c4.sandbox",
        &owner,
        usdc.account_id(),
        "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8",
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;

    let res = machine
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(VRF_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "non-owner open must fail");

    open_pack(&network, &machine, &buyer, 1).await?;
    let res = machine
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(VRF_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "double open must fail");

    Ok(())
}

#[tokio::test]
async fn test_fallback_timing_and_double_settle() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "m5.sandbox",
        "c5.sandbox",
        &owner,
        usdc.account_id(),
        "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8",
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    open_pack(&network, &machine, &buyer, 1).await?;

    let early = machine
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(early.is_failure(), "fallback before 50 blocks must fail");

    sandbox.fast_forward(51).await?;
    machine
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    assert_eq!(
        draw(&network, &machine, 1).await?["status"],
        "FallbackSettled"
    );

    let double = machine
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(double.is_failure(), "double settle must fail");

    Ok(())
}
