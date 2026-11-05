mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

#[tokio::test]
async fn test_milestone_triggers_on_51_not_50() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine_with(
        &sandbox,
        &network,
        "milestone.sandbox",
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
        Some(100_000_000_000_000),
        Some(900_000_000_000_000),
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;

    for token_id in 1..=51u64 {
        ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
        wait_for_pack(&sandbox, &network, &machine, token_id).await?;
        machine
            .call_function("open_pack", json!({"token_id": token_id}))
            .transaction()
            .deposit(near_api::NearToken::from_yoctonear(VRF_DEPOSIT))
            .gas(near_api::NearGas::from_tgas(250))
            .with_signer(buyer.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();

        if token_id == 50 {
            let info: serde_json::Value = machine
                .call_function("get_milestone_info", ())
                .read_only()
                .fetch_from(&network)
                .await?
                .data;
            assert_eq!(info["triggered"], false, "must not trigger on pack 50");
        }
    }

    let info: serde_json::Value = machine
        .call_function("get_milestone_info", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(info["triggered"], true, "must trigger on pack 51");
    let table = info["current_table"].as_array().unwrap();
    let mythic = table.iter().find(|t| t["tier"] == "Mythic").unwrap();
    assert_eq!(mythic["multiplier_bps"], 1000);

    Ok(())
}

#[tokio::test]
async fn test_scrap_redeem_requires_credits() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "scrap.sandbox",
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;

    let res = machine
        .call_function("redeem_scrap", json!({}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "redeem without credits must fail");

    Ok(())
}

#[tokio::test]
async fn test_megapack_claim_requires_graduation() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "mega.sandbox",
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;

    let res = machine
        .call_function("megapack_claim", json!({}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(
        res.is_failure(),
        "megapack claim before graduation must fail"
    );

    Ok(())
}
