mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

#[tokio::test]
async fn test_coin_full_supply_and_transfers() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let holder = create_subaccount(&sandbox, "holder.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, coin) = setup_machine(
        &sandbox,
        &network,
        "machine.sandbox",
        "coin.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;

    let supply: String = coin
        .call_function("ft_total_supply", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(supply, COIN_TOTAL_SUPPLY.to_string());
    assert_eq!(
        ft_balance(&network, &coin, machine.account_id()).await?,
        COIN_TOTAL_SUPPLY
    );

    // no mint path exists
    let mint = coin
        .call_function(
            "mint",
            json!({"account_id": holder.account_id(), "amount": "1"}),
        )
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(mint.is_failure(), "the coin must have no mint method");

    Ok(())
}

#[tokio::test]
async fn test_sync_exposes_insolvency() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let holder = create_subaccount(&sandbox, "holder.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, coin) = setup_machine(
        &sandbox,
        &network,
        "machine.sandbox",
        "coin.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    register_coin(&network, &coin, &holder).await?;

    // drain the machine without updating its cache
    coin.call_function(
        "ft_transfer",
        json!({"receiver_id": holder.account_id(), "amount": "100000000000000"}),
    )
    .transaction()
    .deposit(near_api::NearToken::from_yoctonear(1))
    .with_signer(machine.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();

    machine
        .call_function("sync_coin_balance", json!({}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(150))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(5).await?;

    let before = ft_balance(&network, &usdc, buyer.account_id()).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    sandbox.fast_forward(5).await?;
    assert_eq!(
        ft_balance(&network, &usdc, buyer.account_id()).await?,
        before,
        "the insolvent buy is refunded"
    );
    let pack: Option<serde_json::Value> = machine
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(pack.is_none(), "no pack is minted while insolvent");

    Ok(())
}
