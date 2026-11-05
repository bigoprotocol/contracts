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
    let trader = create_subaccount(sandbox, "trader.sandbox").await?;
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
    mint(network, &usdc, &owner, trader.account_id(), 100_000).await?;
    Ok((owner, trader, usdc, machine, coin))
}

async fn machine_view(
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
) -> testresult::TestResult<serde_json::Value> {
    Ok(machine
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(network)
        .await?
        .data)
}

#[tokio::test]
async fn test_curve_buy_sell_round_trip_real_balances() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (_owner, trader, usdc, machine, coin) = setup(&sandbox, &network, "curve1.sandbox").await?;

    let before = machine_view(&network, &machine).await?;
    let curve_before = u128_str(&before["curve_coin_reserve"]);
    let quote_before = ft_balance(&network, &usdc, trader.account_id()).await?;

    register_coin(&network, &coin, &trader).await?;
    curve_buy(&network, &usdc, &trader, &machine, 1_000, None).await?;
    sandbox.fast_forward(8).await?;

    let coins = ft_balance(&network, &coin, trader.account_id()).await?;
    assert!(coins > 0, "curve buy must transfer real coins");
    let after_buy = machine_view(&network, &machine).await?;
    assert_eq!(
        u128_str(&after_buy["curve_coin_reserve"]),
        curve_before - coins
    );

    let creator_fees: String = machine
        .call_function("get_creator_fees", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let protocol_fees: String = machine
        .call_function("get_protocol_fees", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(creator_fees, "5");
    assert_eq!(protocol_fees, "5");

    curve_sell(&network, &coin, &trader, &machine, coins, None).await?;
    sandbox.fast_forward(10).await?;

    assert_eq!(ft_balance(&network, &coin, trader.account_id()).await?, 0);
    assert!(ft_balance(&network, &usdc, trader.account_id()).await? > quote_before - 1_000);
    let coin_fees: serde_json::Value = machine
        .call_function("get_protocol_fees_coin", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(u128_str(&coin_fees) > 0);

    Ok(())
}

#[tokio::test]
async fn test_curve_buy_slippage_refunds_quote() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (_owner, trader, usdc, machine, coin) = setup(&sandbox, &network, "curve2.sandbox").await?;

    let before = ft_balance(&network, &usdc, trader.account_id()).await?;
    register_coin(&network, &coin, &trader).await?;
    curve_buy(&network, &usdc, &trader, &machine, 1_000, Some(u128::MAX)).await?;
    sandbox.fast_forward(8).await?;

    assert_eq!(ft_balance(&network, &coin, trader.account_id()).await?, 0);
    assert_eq!(
        ft_balance(&network, &usdc, trader.account_id()).await?,
        before
    );
    let view = machine_view(&network, &machine).await?;
    assert_eq!(u128_str(&view["curve_quote_reserve"]), 100_000_000);

    Ok(())
}

#[tokio::test]
async fn test_curve_sell_more_than_held_reverts() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (_owner, trader, _usdc, machine, coin) =
        setup(&sandbox, &network, "curve3.sandbox").await?;

    let res = coin
        .call_function(
            "ft_transfer_call",
            json!({
                "receiver_id": machine.account_id(),
                "amount": "1",
                "msg": "{\"action\":\"curve_sell\"}"
            }),
        )
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(150))
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "selling more than held must revert");

    Ok(())
}

#[tokio::test]
async fn test_curve_sell_failed_payout_restores_coins() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, trader, usdc, machine, coin) = setup(&sandbox, &network, "curve4.sandbox").await?;

    register_coin(&network, &coin, &trader).await?;
    curve_buy(&network, &usdc, &trader, &machine, 1_000, None).await?;
    sandbox.fast_forward(8).await?;
    let coins = ft_balance(&network, &coin, trader.account_id()).await?;
    let before = machine_view(&network, &machine).await?;

    set_fail_transfers(&network, &usdc, &owner, true).await?;
    curve_sell(&network, &coin, &trader, &machine, coins, None).await?;
    sandbox.fast_forward(12).await?;
    set_fail_transfers(&network, &usdc, &owner, false).await?;

    let pending: String = machine
        .call_function(
            "get_pending_coin_refund",
            json!({"account": trader.account_id()}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        pending.parse::<u128>().unwrap(),
        coins,
        "failed payout is credited to the refund ledger"
    );
    machine
        .call_function("claim_coin_refund", json!({}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(1))
        .gas(near_api::NearGas::from_tgas(150))
        .with_signer(trader.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(8).await?;
    let seller_coins = ft_balance(&network, &coin, trader.account_id()).await?;
    let after = machine_view(&network, &machine).await?;
    assert_eq!(
        seller_coins, coins,
        "failed quote payout must restore the coins"
    );
    assert_eq!(after["curve_coin_reserve"], before["curve_coin_reserve"]);
    assert_eq!(after["curve_quote_reserve"], before["curve_quote_reserve"]);

    Ok(())
}

#[tokio::test]
async fn test_curve_price_rises_across_buys() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (_owner, trader, usdc, machine, coin) = setup(&sandbox, &network, "curve5.sandbox").await?;

    register_coin(&network, &coin, &trader).await?;
    curve_buy(&network, &usdc, &trader, &machine, 1_000, None).await?;
    sandbox.fast_forward(8).await?;
    let first = u128_str(&machine_view(&network, &machine).await?["curve_quote_reserve"]);
    register_coin(&network, &coin, &trader).await?;
    curve_buy(&network, &usdc, &trader, &machine, 1_000, None).await?;
    sandbox.fast_forward(8).await?;
    let second = u128_str(&machine_view(&network, &machine).await?["curve_quote_reserve"]);
    assert!(second > first, "price must rise across consecutive buys");

    Ok(())
}

#[tokio::test]
async fn test_interleaved_buy_failure_keeps_reserves() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, trader_a, usdc, machine, coin) =
        setup(&sandbox, &network, "curve6.sandbox").await?;
    let trader_b = create_subaccount(&sandbox, "trader_b.sandbox").await?;
    mint(&network, &usdc, &owner, trader_a.account_id(), 10_000).await?;
    mint(&network, &usdc, &owner, trader_b.account_id(), 10_000).await?;

    // A is registered for the quote but not for the coin, so its payout fails
    register(&network, &machine, &trader_a).await?;
    register_coin(&network, &coin, &trader_b).await?;
    register(&network, &machine, &trader_b).await?;

    let before = machine_view(&network, &machine).await?;
    let before_quote = u128_str(&before["curve_quote_reserve"]);
    let before_coin = u128_str(&before["curve_coin_reserve"]);

    curve_buy(&network, &usdc, &trader_a, &machine, 1_000, None).await?;
    curve_buy(&network, &usdc, &trader_b, &machine, 1_000, None).await?;
    sandbox.fast_forward(12).await?;

    let b_coins = ft_balance(&network, &coin, trader_b.account_id()).await?;
    let after = machine_view(&network, &machine).await?;
    assert_eq!(
        u128_str(&after["curve_quote_reserve"]),
        before_quote + 990,
        "only B's net quote is committed"
    );
    assert_eq!(
        u128_str(&after["curve_coin_reserve"]),
        before_coin - b_coins,
        "only B's coins leave the curve"
    );

    let refund: String = machine
        .call_function(
            "get_pending_refund",
            json!({"account": trader_a.account_id()}),
        )
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(refund, "1000", "the failed buyer is refunded in full");
    assert_eq!(ft_balance(&network, &coin, trader_a.account_id()).await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_ten_parallel_buys_keep_accounting() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, first, usdc, machine, coin) = setup(&sandbox, &network, "curve7.sandbox").await?;
    let mut traders = vec![first];
    for i in 0..9 {
        let trader = create_subaccount(&sandbox, &format!("p{}.sandbox", i)).await?;
        traders.push(trader);
    }
    for trader in &traders {
        mint(&network, &usdc, &owner, trader.account_id(), 10_000).await?;
        register_coin(&network, &coin, trader).await?;
        register(&network, &machine, trader).await?;
    }

    let before = machine_view(&network, &machine).await?;
    let before_quote = u128_str(&before["curve_quote_reserve"]);
    let before_coin = u128_str(&before["curve_coin_reserve"]);

    for trader in &traders {
        curve_buy(&network, &usdc, trader, &machine, 1_000, None).await?;
    }
    sandbox.fast_forward(20).await?;

    let mut total_coins = 0u128;
    for trader in &traders {
        total_coins += ft_balance(&network, &coin, trader.account_id()).await?;
    }
    let after = machine_view(&network, &machine).await?;
    assert_eq!(
        u128_str(&after["curve_quote_reserve"]),
        before_quote + 9_900,
        "each buy commits its net quote"
    );
    assert_eq!(
        u128_str(&after["curve_coin_reserve"]),
        before_coin - total_coins,
        "the coin reserve matches the actual payouts"
    );

    Ok(())
}
