mod common;

use common::*;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

#[tokio::test]
async fn test_machine_and_coin_operational() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
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

    let machine_coin = ft_balance(&network, &coin, machine.account_id()).await?;
    assert_eq!(
        machine_coin, COIN_TOTAL_SUPPLY,
        "machine holds the full supply"
    );

    let hash: String = machine
        .call_function("get_table_hash", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(!hash.is_empty());

    let pool: serde_json::Value = machine
        .call_function("get_prize_pool", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(u128_str(&pool["prize_pool"]), 200_000_000_000_000);
    assert_eq!(pool["coin_token"], "coin.sandbox");

    let view: serde_json::Value = machine
        .call_function("get_machine", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(view["coin_token"], "coin.sandbox");
    assert_eq!(view["quote_token"], "usdc.sandbox");

    Ok(())
}
