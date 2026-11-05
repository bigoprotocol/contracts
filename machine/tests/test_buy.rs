mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

#[tokio::test]
async fn test_buy_moves_coins_into_prize_pool() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, coin) = setup_machine(
        &sandbox,
        &network,
        "m1.sandbox",
        "c1.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;

    let before: serde_json::Value = machine
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let curve_before = u128_str(&before["curve_coin_reserve"]);
    let prize_before = u128_str(&before["prize_pool"]);

    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    let pack = wait_for_pack(&sandbox, &network, &machine, 1).await?;
    assert_eq!(pack["owner"], buyer.account_id().to_string());
    assert_eq!(pack["status"], "Unopened");

    let draw: serde_json::Value = machine
        .call_function("get_draw", json!({"draw_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let pack_coins = u128_str(&draw["pack_coins"]);
    assert!(pack_coins > 0, "the pack must buy coins");

    let after: serde_json::Value = machine
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        u128_str(&after["curve_coin_reserve"]),
        curve_before - pack_coins
    );
    assert_eq!(u128_str(&after["prize_pool"]), prize_before + pack_coins);

    // the buyer does not receive the coins, they are the pack prize backing
    assert_eq!(ft_balance(&network, &coin, buyer.account_id()).await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_unregistered_buy_is_refunded() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "m2.sandbox",
        "c2.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 1_000).await?;

    usdc.call_function(
        "ft_transfer_call",
        json!({
            "receiver_id": machine.account_id(),
            "amount": "100",
            "msg": "{\"action\":\"buy_pack\"}"
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
        1_000
    );

    let summary: serde_json::Value = machine
        .call_function("get_machine_summary", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(summary["total_packs_sold"], 0);

    Ok(())
}

#[tokio::test]
async fn test_pack_buy_requires_storage_balance() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "m2.sandbox",
        "c2.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    register(&network, &machine, &buyer).await?;

    for _ in 0..2 {
        usdc.call_function(
            "ft_transfer_call",
            json!({
                "receiver_id": machine.account_id(),
                "amount": "1000",
                "msg": "{\"action\":\"buy_pack\"}"
            }),
        )
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    }
    sandbox.fast_forward(8).await?;
    assert!(wait_for_pack(&sandbox, &network, &machine, 2).await.is_ok());

    let before = ft_balance(&network, &usdc, buyer.account_id()).await?;
    usdc.call_function(
        "ft_transfer_call",
        json!({
            "receiver_id": machine.account_id(),
            "amount": "1000",
            "msg": "{\"action\":\"buy_pack\"}"
        }),
    )
    .transaction()
    .deposit(near_api::NearToken::from_yoctonear(1))
    .gas(near_api::NearGas::from_tgas(250))
    .with_signer(buyer.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    sandbox.fast_forward(8).await?;

    assert_eq!(
        ft_balance(&network, &usdc, buyer.account_id()).await?,
        before,
        "the third buy is refunded when the storage balance is spent"
    );
    let pack: Option<serde_json::Value> = machine
        .call_function("get_pack", json!({"token_id": 3}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(pack.is_none(), "no third pack is minted");

    Ok(())
}
