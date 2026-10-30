use serde_json::json;

#[tokio::test]
async fn smoke_init() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let wasm = near_workspaces::compile_project("./").await?;
    let contract = worker.dev_deploy(&wasm).await?;
    let owner = worker.dev_create_account().await?;
    // branch by crate name
    #[allow(clippy::if_same_then_else)]
    let args = if env!("CARGO_PKG_NAME") == "machine" {
        let coin = worker.dev_create_account().await?;
        let prize_table = vec![
            json!({"multiplier_bps": 25, "odds_bps": 6000, "tier": "Common"}),
            json!({"multiplier_bps": 100, "odds_bps": 2500, "tier": "Common"}),
            json!({"multiplier_bps": 200, "odds_bps": 1000, "tier": "Rare"}),
            json!({"multiplier_bps": 400, "odds_bps": 400, "tier": "Epic"}),
            json!({"multiplier_bps": 800, "odds_bps": 100, "tier": "Mythic"}),
        ];
        json!({
            "owner": owner.id(),
            "coin": coin.id(),
            "prize_table": prize_table,
            "pack_art_url": "ipfs://QmTest",
            "vrf_pubkey": "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8"
        })
    } else {
        json!({"owner": owner.id()})
    };
    let res = contract.call("new").args_json(args).transact().await?;
    assert!(res.is_success(), "init failed: {:?}", res);
    Ok(())
}
