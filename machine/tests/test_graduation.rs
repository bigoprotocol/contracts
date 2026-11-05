mod common;

use common::*;
use near_sdk::serde_json::json;

const VRF_PUBKEY: &str = "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8";

async fn setup(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    name: &str,
    target: Option<u128>,
    with_rhea: bool,
) -> testresult::TestResult<(
    near_api::Account,
    near_api::Contract,
    near_api::Contract,
    Option<near_api::Contract>,
)> {
    setup_inner(sandbox, network, name, target, with_rhea, true).await
}

async fn setup_unfunded(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    name: &str,
    target: Option<u128>,
    with_rhea: bool,
) -> testresult::TestResult<(
    near_api::Account,
    near_api::Contract,
    near_api::Contract,
    Option<near_api::Contract>,
)> {
    setup_inner(sandbox, network, name, target, with_rhea, false).await
}

async fn setup_inner(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    name: &str,
    target: Option<u128>,
    with_rhea: bool,
    fund_quote: bool,
) -> testresult::TestResult<(
    near_api::Account,
    near_api::Contract,
    near_api::Contract,
    Option<near_api::Contract>,
)> {
    let owner = create_subaccount(sandbox, "owner.sandbox").await?;
    let usdc = deploy_mock_usdc(sandbox, network, "usdc.sandbox", &owner).await?;
    let rhea = if with_rhea {
        Some(deploy_mock_rhea(sandbox, network, "rhea.sandbox", &owner).await?)
    } else {
        None
    };
    let (machine, coin) = setup_machine_full(
        sandbox,
        network,
        name,
        "c.sandbox",
        &owner,
        usdc.account_id(),
        VRF_PUBKEY,
        None,
        None,
        target,
        rhea.as_ref().map(|r| r.account_id()),
    )
    .await?;
    // back the virtual quote reserve with real tokens so graduation can seed it
    if fund_quote {
        mint(network, &usdc, &owner, owner.account_id(), 1_000_000_000).await?;
        register(network, &machine, &owner).await?;
        usdc.call_function(
            "ft_transfer",
            json!({
                "receiver_id": machine.account_id(),
                "amount": "100000000",
            }),
        )
        .transaction()
        .deposit(near_api::NearToken::from_yoctonear(1))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    }
    Ok((owner, machine, coin, rhea))
}

async fn graduate_step(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    signer: &near_api::Account,
    method: &str,
) -> testresult::TestResult<()> {
    machine
        .call_function(method, json!({}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(500))
        .with_signer(signer.account_id().clone(), genesis_signer())
        .send_to(network)
        .await?
        .assert_success();
    sandbox.fast_forward(2).await?;
    Ok(())
}

async fn wait_for(
    sandbox: &near_sandbox::Sandbox,
    network: &near_api::NetworkConfig,
    machine: &near_api::Contract,
    pred: impl Fn(&serde_json::Value) -> bool,
) -> testresult::TestResult<serde_json::Value> {
    for _ in 0..40 {
        sandbox.fast_forward(2).await?;
        let grad: serde_json::Value = machine
            .call_function("get_graduation", ())
            .read_only()
            .fetch_from(network)
            .await?
            .data;
        if pred(&grad) {
            return Ok(grad);
        }
    }
    panic!("graduation step did not complete");
}

#[tokio::test]
async fn test_graduate_below_target_is_rejected() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, _coin, _rhea) = setup(
        &sandbox,
        &network,
        "grad1.sandbox",
        Some(10u128.pow(30)),
        true,
    )
    .await?;

    let res = machine
        .call_function("graduate", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(res.is_failure(), "graduate must revert below the target");

    let status: String = machine
        .call_function("get_graduation_status", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(status, "CurveActive");

    Ok(())
}

#[tokio::test]
async fn test_graduation_creates_seeds_and_locks_pool() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, coin, rhea) =
        setup(&sandbox, &network, "grad2.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");

    let quote_reserve = {
        let state: serde_json::Value = machine
            .call_function("get_curve_state", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data;
        u128_str(&state["quote_reserve"])
    };
    let coin_reserve = {
        let state: serde_json::Value = machine
            .call_function("get_curve_state", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data;
        u128_str(&state["coin_reserve"])
    };

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;
    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    let grad = wait_for(&sandbox, &network, &machine, |g| g["status"] == "Graduated").await?;
    assert_eq!(grad["status"], "Graduated");
    assert_eq!(grad["tradable_on_rhea"], true);
    let pool_id = grad["pool_id"].as_u64().expect("pool id");
    assert_eq!(pool_id, 0);

    let pool: serde_json::Value = rhea
        .call_function("get_pool", json!({"pool_id": pool_id}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let tokens = pool["token_account_ids"].as_array().unwrap();
    let amounts = pool["amounts"].as_array().unwrap();
    assert_eq!(tokens[0], coin.account_id().to_string());
    assert_eq!(u128_str(&amounts[0]), coin_reserve, "coin side seeded");
    assert_eq!(u128_str(&amounts[1]), quote_reserve, "quote side seeded");

    let state: serde_json::Value = machine
        .call_function("get_curve_state", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(u128_str(&state["coin_reserve"]), 0);
    assert_eq!(u128_str(&state["quote_reserve"]), 0);
    assert_eq!(state["tradable_on_rhea"], true);
    assert_eq!(state["graduation_pool_id"].as_u64(), Some(pool_id));

    Ok(())
}

#[tokio::test]
async fn test_post_graduation_buy_pack_and_curve_are_retired() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, _coin, _rhea) =
        setup(&sandbox, &network, "grad3.sandbox", Some(1), true).await?;
    let buyer = create_subaccount(&sandbox, "buyer.sandbox").await?;
    let usdc = near_api::Account("usdc.sandbox".parse().unwrap()).as_contract();
    mint(&network, &usdc, &owner, buyer.account_id(), 1_000).await?;

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;
    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    wait_for(&sandbox, &network, &machine, |g| g["status"] == "Graduated").await?;

    let before = ft_balance(&network, &usdc, buyer.account_id()).await?;
    ft_buy(&network, &usdc, &buyer, &machine, 100).await?;
    sandbox.fast_forward(5).await?;
    let after = ft_balance(&network, &usdc, buyer.account_id()).await?;
    assert_eq!(after, before, "post-graduation pack buys are refunded");

    let pack: Option<serde_json::Value> = machine
        .call_function("get_pack", json!({"token_id": 1}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(pack.is_none(), "no pack is minted after graduation");

    Ok(())
}

#[tokio::test]
async fn test_skewed_existing_pool_stays_pending() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, coin, rhea) =
        setup(&sandbox, &network, "grad4.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");
    let stranger = create_subaccount(&sandbox, "stranger.sandbox").await?;
    let usdc = near_api::Account("usdc.sandbox".parse().unwrap()).as_contract();
    mint(&network, &usdc, &owner, stranger.account_id(), 1_000).await?;
    register_coin(&network, &coin, &stranger).await?;

    // the stranger front-runs with a 1:1 pool, far from the curve price
    rhea.call_function(
        "add_simple_pool",
        json!({"tokens": [coin.account_id(), usdc.account_id()], "fee": 25}),
    )
    .transaction()
    .deposit(near_api::NearToken::from_millinear(100))
    .with_signer(owner.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    curve_buy(&network, &usdc, &stranger, &machine, 10, None).await?;
    let rhea_acc = near_api::Account(rhea.account_id().clone());
    register_coin(&network, &coin, &rhea_acc).await?;
    for (token, amount) in [(&coin, "1"), (&usdc, "1")] {
        token
            .call_function(
                "ft_transfer_call",
                json!({
                    "receiver_id": rhea.account_id(),
                    "amount": amount,
                    "msg": "",
                }),
            )
            .transaction()
            .deposit(near_api::NearToken::from_yoctonear(1))
            .gas(near_api::NearGas::from_tgas(150))
            .with_signer(stranger.account_id().clone(), genesis_signer())
            .send_to(&network)
            .await?
            .assert_success();
    }
    rhea.call_function(
        "add_liquidity",
        json!({"pool_id": 0, "amounts": ["1", "1"]}),
    )
    .transaction()
    .deposit(near_api::NearToken::from_yoctonear(1))
    .gas(near_api::NearGas::from_tgas(150))
    .with_signer(stranger.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    sandbox.fast_forward(10).await?;

    let status: String = machine
        .call_function("get_graduation_status", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        status, "GraduationPending",
        "a skewed pool must not graduate"
    );

    Ok(())
}

#[tokio::test]
async fn test_double_seed_graduation_panics() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, _coin, _rhea) =
        setup(&sandbox, &network, "grad5.sandbox", Some(1), true).await?;

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;

    let second = machine
        .call_function("seed_graduation", json!({}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(500))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?;
    assert!(second.is_failure(), "a second seed_graduation must panic");

    Ok(())
}

#[tokio::test]
async fn test_fee_coins_excluded_from_seed() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, coin, rhea) =
        setup(&sandbox, &network, "grad6.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");
    let usdc = near_api::Account("usdc.sandbox".parse().unwrap()).as_contract();
    let trader = create_subaccount(&sandbox, "trader.sandbox").await?;
    mint(&network, &usdc, &owner, trader.account_id(), 10_000).await?;
    register_coin(&network, &coin, &trader).await?;
    register(&network, &machine, &trader).await?;

    curve_buy(&network, &usdc, &trader, &machine, 1_000, None).await?;
    sandbox.fast_forward(8).await?;
    let coins = ft_balance(&network, &coin, trader.account_id()).await?;
    curve_sell(&network, &coin, &trader, &machine, coins / 2, None).await?;
    sandbox.fast_forward(8).await?;

    let fees_before: (String, String) = (
        machine
            .call_function("get_protocol_fees_coin", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data,
        machine
            .call_function("get_creator_fees_coin", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data,
    );
    let state: serde_json::Value = machine
        .call_function("get_curve_state", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let curve_coin = u128_str(&state["coin_reserve"]);

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;
    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    wait_for(&sandbox, &network, &machine, |g| g["status"] == "Graduated").await?;

    let pool: serde_json::Value = rhea
        .call_function("get_pool", json!({"pool_id": 0}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let amounts = pool["amounts"].as_array().unwrap();
    assert_eq!(
        u128_str(&amounts[0]),
        curve_coin,
        "seed equals the curve reserve, not the fee coins"
    );
    let fees_after: (String, String) = (
        machine
            .call_function("get_protocol_fees_coin", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data,
        machine
            .call_function("get_creator_fees_coin", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data,
    );
    assert_eq!(fees_after, fees_before, "fee ledgers survive graduation");
    let cache: String = machine
        .call_function("get_coin_balance_cached", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert!(
        cache.parse::<u128>().unwrap() >= fees_after.0.parse::<u128>().unwrap(),
        "fee coins stay onchain"
    );

    Ok(())
}

#[tokio::test]
async fn test_quote_deposit_mismatch_fails_graduation() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, _coin, rhea) =
        setup(&sandbox, &network, "grad7.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;

    rhea.call_function("set_fail_deposit", json!({"fail": true}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();

    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    for _ in 0..15 {
        sandbox.fast_forward(2).await?;
        let grad: serde_json::Value = machine
            .call_function("get_graduation", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data;
        if grad["error"]
            .as_str()
            .map(|e| !e.is_empty())
            .unwrap_or(false)
        {
            break;
        }
    }
    let grad: serde_json::Value = machine
        .call_function("get_graduation", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(grad["status"], "GraduationPending");
    assert!(
        grad["error"].as_str().unwrap().contains("quote deposit"),
        "mismatch reason is recorded"
    );

    Ok(())
}

#[tokio::test]
async fn test_token_order_quote_first_pool() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, coin, rhea) =
        setup(&sandbox, &network, "grad8.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");
    let usdc = near_api::Account("usdc.sandbox".parse().unwrap()).as_contract();

    let state: serde_json::Value = machine
        .call_function("get_curve_state", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let quote_reserve = u128_str(&state["quote_reserve"]);
    let coin_reserve = u128_str(&state["coin_reserve"]);

    rhea.call_function(
        "add_simple_pool",
        json!({
            "tokens": [usdc.account_id(), coin.account_id()],
            "fee": 25,
        }),
    )
    .transaction()
    .deposit(near_api::NearToken::from_millinear(100))
    .with_signer(owner.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    rhea.call_function(
        "set_pool_amounts",
        json!({
            "pool_id": 0,
            "amounts": [quote_reserve.to_string(), coin_reserve.to_string()],
        }),
    )
    .transaction()
    .with_signer(owner.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;
    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    wait_for(&sandbox, &network, &machine, |g| g["status"] == "Graduated").await?;

    let pool: serde_json::Value = rhea
        .call_function("get_pool", json!({"pool_id": 0}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let tokens = pool["token_account_ids"].as_array().unwrap();
    let amounts = pool["amounts"].as_array().unwrap();
    assert_eq!(tokens[0], usdc.account_id().to_string());
    assert_eq!(
        u128_str(&amounts[0]),
        quote_reserve * 2,
        "quote side added in pool order"
    );
    assert_eq!(
        u128_str(&amounts[1]),
        coin_reserve * 2,
        "coin side added in pool order"
    );

    Ok(())
}

#[tokio::test]
async fn test_pending_coin_refund_excluded_from_seed() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, coin, rhea) =
        setup(&sandbox, &network, "grad9.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");
    let usdc = near_api::Account("usdc.sandbox".parse().unwrap()).as_contract();
    let trader = create_subaccount(&sandbox, "trader.sandbox").await?;
    mint(&network, &usdc, &owner, trader.account_id(), 10_000).await?;
    register_coin(&network, &coin, &trader).await?;
    register(&network, &machine, &trader).await?;

    curve_buy(&network, &usdc, &trader, &machine, 1_000, None).await?;
    sandbox.fast_forward(8).await?;
    let coins = ft_balance(&network, &coin, trader.account_id()).await?;
    set_fail_transfers(&network, &usdc, &owner, true).await?;
    curve_sell(&network, &coin, &trader, &machine, coins / 2, None).await?;
    sandbox.fast_forward(8).await?;
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
    let pending_amount = pending.parse::<u128>().unwrap();
    assert!(pending_amount > 0, "the failed sell is a pending refund");
    let state: serde_json::Value = machine
        .call_function("get_curve_state", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let curve_coin = u128_str(&state["coin_reserve"]);

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;
    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    wait_for(&sandbox, &network, &machine, |g| g["status"] == "Graduated").await?;

    let pool: serde_json::Value = rhea
        .call_function("get_pool", json!({"pool_id": 0}))
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    let amounts = pool["amounts"].as_array().unwrap();
    assert_eq!(
        u128_str(&amounts[0]),
        curve_coin,
        "the seed excludes the pending refund"
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
    assert_eq!(
        ft_balance(&network, &coin, trader.account_id()).await?,
        coins,
        "the pending refund is still claimable after graduation"
    );

    Ok(())
}

#[tokio::test]
async fn test_quote_shortfall_and_stranded_recovery() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, _coin, _rhea) =
        setup_unfunded(&sandbox, &network, "grad10.sandbox", Some(1), true).await?;

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    wait_for(&sandbox, &network, &machine, |g| g["pool_ready"] == true).await?;
    graduate_step(&sandbox, &network, &machine, &owner, "seed_graduation").await?;
    wait_for(&sandbox, &network, &machine, |g| g["coin_seeded"] == true).await?;
    graduate_step(
        &sandbox,
        &network,
        &machine,
        &owner,
        "seed_quote_graduation",
    )
    .await?;
    for _ in 0..15 {
        sandbox.fast_forward(2).await?;
        let grad: serde_json::Value = machine
            .call_function("get_graduation", ())
            .read_only()
            .fetch_from(&network)
            .await?
            .data;
        if grad["error"]
            .as_str()
            .map(|e| !e.is_empty())
            .unwrap_or(false)
        {
            break;
        }
    }
    let grad: serde_json::Value = machine
        .call_function("get_graduation", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(grad["status"], "GraduationPending");
    assert!(
        grad["error"]
            .as_str()
            .unwrap()
            .contains("insufficient quote backing"),
        "shortfall is reported before the deposit"
    );
    assert!(
        u128_str(&grad["stranded_coin"]) > 0,
        "the coin seed is stranded"
    );

    machine
        .call_function("recover_stranded_graduation_funds", json!({}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(300))
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    sandbox.fast_forward(10).await?;
    let grad: serde_json::Value = machine
        .call_function("get_graduation", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(
        u128_str(&grad["stranded_coin"]),
        0,
        "the stranded coin is recovered"
    );

    machine
        .call_function("recover_from_pending", json!({}))
        .transaction()
        .with_signer(owner.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    let status: String = machine
        .call_function("get_graduation_status", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(status, "CurveActive");

    Ok(())
}

#[tokio::test]
async fn test_abort_graduation_after_timeout() -> testresult::TestResult<()> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let (owner, machine, coin, rhea) =
        setup(&sandbox, &network, "grad11.sandbox", Some(1), true).await?;
    let rhea = rhea.expect("rhea deployed");
    let usdc = near_api::Account("usdc.sandbox".parse().unwrap()).as_contract();

    rhea.call_function(
        "add_simple_pool",
        json!({
            "tokens": [usdc.account_id(), coin.account_id()],
            "fee": 25,
        }),
    )
    .transaction()
    .deposit(near_api::NearToken::from_millinear(100))
    .with_signer(owner.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();
    rhea.call_function(
        "set_pool_amounts",
        json!({
            "pool_id": 0,
            "amounts": ["1", "1"],
        }),
    )
    .transaction()
    .with_signer(owner.account_id().clone(), genesis_signer())
    .send_to(&network)
    .await?
    .assert_success();

    graduate_step(&sandbox, &network, &machine, &owner, "graduate").await?;
    sandbox.fast_forward(10).await?;
    let grad: serde_json::Value = machine
        .call_function("get_graduation", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(grad["status"], "GraduationPending");

    sandbox.fast_forward(10_001).await?;
    let stranger = create_subaccount(&sandbox, "stranger.sandbox").await?;
    machine
        .call_function("abort_graduation", json!({}))
        .transaction()
        .gas(near_api::NearGas::from_tgas(50))
        .with_signer(stranger.account_id().clone(), genesis_signer())
        .send_to(&network)
        .await?
        .assert_success();
    let status: String = machine
        .call_function("get_graduation_status", ())
        .read_only()
        .fetch_from(&network)
        .await?
        .data;
    assert_eq!(status, "CurveActive");

    Ok(())
}
