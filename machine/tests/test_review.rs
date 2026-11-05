mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

#[tokio::test]
async fn test_leaderboard_capped_under_buyer_growth() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "board.sandbox",
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(&network, &usdc, &owner, owner.account_id(), 200_000).await?;

    for i in 0..30u32 {
        let buyer = create_subaccount(&sandbox, &format!("buyer{}.sandbox", i)).await?;
        mint(&network, &usdc, &owner, buyer.account_id(), 10_000).await?;
        ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    }
    sandbox.fast_forward(5).await?;

    let top: serde_json::Value = machine
        .call_function("get_top_pack_buyers", json!({"limit": 100}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        top.as_array().unwrap().len(),
        10,
        "top buyers must stay capped"
    );

    let summary: serde_json::Value = machine
        .call_function("get_machine_summary", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(summary["total_packs_sold"], 30);

    Ok(())
}

#[tokio::test]
async fn test_bricking_table_rejected_at_init() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let machine_acc = create_subaccount(&sandbox, "brick.sandbox").await?;

    // the coin is minted to the machine account
    let coin_wasm = build_workspace_crate("coin", "coin");
    let coin_acc = create_subaccount(&sandbox, "brickcoin.sandbox").await?;
    let coin = coin_acc.as_contract();
    near_api::Contract::deploy(coin.account_id().clone())
        .use_code(coin_wasm)
        .with_init_call(
            "new",
            json!({
                "owner": machine_acc.account_id(),
                "total_supply": COIN_TOTAL_SUPPLY.to_string(),
                "metadata": coin_metadata(),
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    // total 851000 passes at init, the 1000 bps upgrade pushes it to 855300
    let table = json!([
        {"multiplier_bps": 50, "odds_bps": 9000, "tier": "Common"},
        {"multiplier_bps": 800, "odds_bps": 430, "tier": "Mythic"},
        {"multiplier_bps": 100, "odds_bps": 570, "tier": "Rare"}
    ]);
    let res = near_api::Contract::deploy(machine_acc.account_id().clone())
        .use_code(build_machine_wasm())
        .with_init_call(
            "new",
            json!({
                "owner": owner.account_id(),
                "coin": coin.account_id(),
                "coin_token": coin.account_id(),
                "quote_token": "usdc.sandbox",
                "prize_table": table,
                "pack_art_url": "ipfs://QmTest",
                "vrf_pubkey": VRF_PUBKEY,
                "outlayer_account": "outlayer.testnet",
                "vrf_project_id": "bigodeployer.testnet/bigo-vrf",
                "milestone_threshold": 1,
                "milestone_new_top_mult_bps": 1000,
                "ft_gas": 30,
                "curve_coin_allocation": null,
                "prize_coin_allocation": null,
                "max_pack_price": null,
                "scrap_discount_amount": null
            }),
        )?
        .max_gas()
        .with_signer(genesis_signer())
        .send_to(&network)
        .await?;
    assert!(
        res.is_failure(),
        "a table that bricks after the milestone must fail at init"
    );

    Ok(())
}

#[tokio::test]
async fn test_fallback_emits_event() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let owner = create_subaccount(&sandbox, "owner.sandbox").await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = deploy_mock_usdc(&sandbox, &network, "usdc.sandbox", &owner).await?;
    let (machine, _coin) = setup_machine(
        &sandbox,
        &network,
        "event.sandbox",
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
    )
    .await?;
    mint(&network, &usdc, &owner, buyer.account_id(), 100_000).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 1000).await?;
    wait_for_pack(&sandbox, &network, &machine, 1).await?;

    machine
        .call_function("open_pack", json!({"token_id": 1}))
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(VRF_DEPOSIT))
        .gas(near_api::NearGas::from_tgas(250))
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(51).await?;

    let res = machine
        .call_function("settle_draw_fallback", json!({"draw_id": 1}))
        .transaction()
        .with_signer(buyer.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    let logs: Vec<String> = res.logs().iter().map(|s| s.to_string()).collect();
    res.assert_success();
    let joined = logs.join("\n");
    assert!(
        joined.contains("EVENT_JSON"),
        "fallback must emit a NEP-297 event"
    );
    assert!(
        joined.contains("draw_fallback"),
        "event name must be draw_fallback"
    );

    Ok(())
}
