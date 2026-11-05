mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

async fn setup(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    name: &str,
) -> testresult::TestResult<(
    near_api::Account,
    near_api::Account,
    near_api::Contract,
    near_api::Contract,
    near_api::Contract,
)> {
    let owner = create_subaccount(sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(sandbox, network, "usdc.sandbox", &owner).await?;
    let (machine, coin) = setup_machine(
        sandbox,
        network,
        name,
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    Ok((owner, buyer, usdc, machine, coin))
}

async fn settle_via_fallback(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    buyer: &near_api::Account,
    token_id: u64,
) -> testresult::TestResult<()> {
    machine
        .call_function("open_pack", json!({"token_id": token_id}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(VRF_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    sandbox.fast_forward(51).await?;
    machine
        .call_function("settle_draw_fallback", json!({"draw_id": token_id}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_claim_pays_real_coins() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (_owner, buyer, usdc, machine, coin) = setup(&sandbox, &network, "m1.sandbox").await?;

    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    settle_via_fallback(&sandbox, &network, &machine, &buyer, 1).await?;

    let draw: serde_json::Value = machine
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let prize = u128_str(&draw["prize"]["prize_amount"]);
    assert!(prize > 0);

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
        prize
    );
    let claimable: String = machine
        .call_function("get_claimable", json!({"account": buyer.account_id()}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(claimable, "0");

    Ok(())
}

#[tokio::test]
async fn test_claim_with_nothing_fails() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (_owner, buyer, _usdc, machine, _coin) = setup(&sandbox, &network, "m2.sandbox").await?;

    let res = machine
        .call_function("claim", json!({}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(1))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "claim with no balance must fail");

    Ok(())
}

#[tokio::test]
async fn test_claim_works_while_paused() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, buyer, usdc, machine, coin) = setup(&sandbox, &network, "m3.sandbox").await?;

    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    settle_via_fallback(&sandbox, &network, &machine, &buyer, 1).await?;

    machine
        .call_function("pause", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

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

    Ok(())
}

#[tokio::test]
async fn test_listing_buy_pays_seller_and_locks_pack() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, seller, usdc, machine, _coin) = setup(&sandbox, &network, "m4.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer2.sandbox").await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;

    ft_buy(&network, &usdc, &seller, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    machine
        .call_function("list_pack", json!({"token_id": 1, "price": "1000"}))
        .transaction()
        .with_signer(seller.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let pack: serde_json::Value = machine
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(pack["status"], "Listed");

    register(&network, &machine, &buyer).await?;
    let seller_before = ft_balance(&network, &usdc, seller.account_id()).await?;
    usdc.call_function(
        "ft_transfer_call",
        json!({
            "receiver_id": machine.account_id(),
            "amount": "1000",
            "msg": "{\"action\":\"buy_listing\",\"token_id\":1}"
        }),
    )
    .transaction()
    .deposit(near_api::NearToken::from_yoctonear(1))
    .gas(near_api::NearGas::from_tgas(150))
    .with_signer(buyer.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    sandbox.fast_forward(8).await?;

    let pack: serde_json::Value = machine
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(pack["owner"], buyer.account_id().to_string());
    assert_eq!(pack["status"], "Unopened");
    let seller_after = ft_balance(&network, &usdc, seller.account_id()).await?;
    assert_eq!(
        seller_after,
        seller_before + 999,
        "0.15 percent fee is 1 unit at 1000"
    );

    Ok(())
}

#[tokio::test]
async fn test_paused_blocks_listing_buy() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, seller, usdc, machine, _coin) = setup(&sandbox, &network, "m5.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer2.sandbox").await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;

    ft_buy(&network, &usdc, &seller, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;
    machine
        .call_function("list_pack", json!({"token_id": 1, "price": "1000"}))
        .transaction()
        .with_signer(seller.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    register(&network, &machine, &buyer).await?;
    machine
        .call_function("pause", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let buyer_before = ft_balance(&network, &usdc, buyer.account_id()).await?;
    usdc.call_function(
        "ft_transfer_call",
        json!({
            "receiver_id": machine.account_id(),
            "amount": "1000",
            "msg": "{\"action\":\"buy_listing\",\"token_id\":1}"
        }),
    )
    .transaction()
    .deposit(near_api::NearToken::from_yoctonear(1))
    .gas(near_api::NearGas::from_tgas(150))
    .with_signer(buyer.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    sandbox.fast_forward(8).await?;

    assert_eq!(
        ft_balance(&network, &usdc, buyer.account_id()).await?,
        buyer_before
    );
    let pack: serde_json::Value = machine
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(pack["owner"], seller.account_id().to_string());

    Ok(())
}
