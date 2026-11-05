mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

#[tokio::test]
async fn test_candles_roll_forward_and_are_permissionless() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let stranger = create_subaccount(&sandbox, "stranger.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "candles.sandbox",
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;

    machine
        .call_function("snapshot_candle", json!({}))
        .transaction()
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    sandbox.fast_forward(101).await?;
    machine
        .call_function("snapshot_candle", json!({}))
        .transaction()
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    let candles: serde_json::Value = machine
        .call_function("get_candles", json!({"limit": 10}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(candles.as_array().unwrap().len(), 2);

    Ok(())
}
