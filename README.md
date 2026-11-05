# bigo/contracts

NEAR contracts for Bigo, an onchain gacha for NEAR coins.

Five crates in a Cargo workspace:

- `coin` - the launched coin. Standard NEP-141, fixed supply of 1,000,000,000 at
  6 decimals minted entirely to the machine at init.
- `factory` - machine registry and fan-coin allowlist. `launch_coin` deploys the
  coin token, then `deploy_machine` deploys the machine once the coin callback
  confirms it. The registry is written only after both succeed.
- `machine` - the gacha machine. Custodies the coin, runs the bonding curve,
  holds the coin prize pool, mints pack NFTs, requests randomness from Outlayer,
  settles draws, and pays prizes in real coins.
- `squad` - group pack buys. Members deposit the quote token, the squad buys and
  opens one pack, and the settled coin prize is credited to per-member ledgers
  that each member pulls with `claim_squad_share`.
- `mock-usdc` - NEP-141 mock quote token with 6 decimals and an owner mint for
  the demo.

## Install

Rust stable with the `wasm32-unknown-unknown` target and `cargo-near`:

```bash
rustup target add wasm32-unknown-unknown
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/near/cargo-near/releases/latest/download/cargo-near-installer.sh | sh
```

`near-sdk` is pinned to `=5.29.0`. Newer releases import host functions that the
bundled sandbox node does not provide.

## Build

```bash
cd bigo/contracts
for C in coin factory machine squad mock-usdc; do
  (cd "$C" && cargo near build non-reproducible-wasm)
done
```

Artifacts land in `target/near/<crate>/<crate>.wasm`. Each crate also has
`scripts/build.sh` with the same command.

Reproducible builds need Docker and a clean git tree:

```bash
cd machine && cargo near build reproducible-wasm
```

## Test

```bash
cargo test
```

Unit tests live in each `src/lib.rs`. Integration tests in each `tests/` folder
run against a local sandbox using `near-sandbox` and `near-api`, not
`near-workspaces`. The machine tests build the release WASM and deploy a mock
Outlayer contract to exercise the real VRF promise chain.

## Structure

```
src/lib.rs       contract entry and state
src/fees.rs      fee math (machine only)
tests/           sandbox integration tests
scripts/build.sh thin wrapper around cargo near build
```

There is no `error.rs`. User-facing failures panic with explicit string
messages, for example `"insufficient pool: free 0 + net 99 < worst 800"`.

## Token flow

The quote token is the pack currency. The coin token is the prize asset. The
machine is the sole initial holder of the coin supply.

A pack is bought by calling `ft_transfer_call` on the quote token:

```json
{"action":"buy_pack"}
```

The machine takes the 0.20 percent fee, swaps the net quote through the internal
curve, and moves the bought coins from the curve partition into the prize pool.
The pack records `pack_coins`, the coins the pack bought. A prize is
`pack_coins * multiplier / 100`, paid in real coins to the winner's wallet on
`claim()`.

Curve actions:

- buy: `ft_transfer_call` on the quote token with
  `{"action":"curve_buy","min_coins_out":N}`. The machine transfers real coins to
  the buyer.
- sell: `ft_transfer_call` on the coin token with
  `{"action":"curve_sell","min_quote_out":N}`. The machine transfers quote to the
  seller.

Other quote actions are `seed_pool` and
`buy_listing` (`{"action":"buy_listing","token_id":1}`). The unused part of a
transfer is returned and the token contract refunds it.

The machine's coin balance is cached at `coin_balance_cached` and refreshed with
the permissionless `sync_coin_balance`. `assert_coin_solvent` runs on every state
changing method and requires the cached balance to cover
`curve_coin_reserve + prize_pool`.

## Launch allocation

The machine receives the entire coin supply. At init the creator sets
`curve_coin_allocation` (default 80 percent) and `prize_coin_allocation`
(default 20 percent). Init rejects the launch when:

- the two allocations exceed the machine's coin balance
- swapping `max_pack_price` through the initial curve has more than 10 percent
  price impact
- `prize_coin_allocation` is below 100 times the largest pack's max prize, where
  max prize = coins bought by `max_pack_price` at the initial price times
  `max_mult_bps / 100`

Both bounds are enforced with integer cross multiplication, no division.

## Launch deposit

`launch_coin` requires 20 NEAR, a non refundable storage payment. The launch is
two calls because the full coin and machine deploy chain exceeds the 300 Tgas
transaction cap. `launch_coin` deploys the coin and stores the pending launch.
Once the coin callback confirms, anyone can call `deploy_machine(machine_id)` to
deploy the machine and write the registry. A failed coin deploy refunds the
creator `attached - 0.1 NEAR` and writes nothing. The 0.1 NEAR reserve covers the
factory gas.

The attached deposit breaks down as:

- 1 NEAR stays with the factory for the registry records
- 9.5 NEAR goes to the coin account, covering its code (about 1.5 NEAR for
  150 KB) and account state
- 9.5 NEAR goes to the machine account, covering its code (about 3.5 NEAR for
  344 KB) and account state

A fan coin has no coin account, so the full 19 NEAR goes to the machine.

The transfer action runs before the deploy so the new account can pay for its
code. The factory stores an index map of machines, never a linear vector scan.

## Bonding curve

The curve is a constant-product market on internal coin balances. The quote side
is always real token movement through the quote token.

Both sides commit only after the external transfer succeeds. The request
computes the output against the current reserves, then the callback applies the
reserve and fee updates. A failed transfer credits the buyer's quote refund or
the seller's coin refund and changes nothing else. The tradeoff is that the
price can move between computation and commit. The `min_coins_out` and
`min_quote_out` guards bound that movement, and the committed amounts are the
ones actually transferred.

Buy, through `ft_transfer_call` with `{"action":"curve_buy","min_coins_out":N}`:

```
fee = quote_in / 100            // 1 percent, half creator half protocol
net = quote_in - fee
coin_out = coin_reserve * net / (quote_reserve + net)
```

Sell, a direct call `curve_sell(coin_amount, min_quote_out)` with 1 yocto:

```
fee = coin_in / 100             // 1 percent, half creator half protocol
net = coin_in - fee
quote_out = quote_reserve * net / (coin_reserve + net)
```

The creator share of a sell fee is credited as coin on a successful payout. The
protocol share stays in the coin reserve. On a failed quote payout the machine
credits `pending_coin_refunds` for the seller and leaves the reserves untouched.
The seller pulls it with `claim_coin_refund`.

## Storage cost model

`ft_on_transfer` cannot carry a NEAR deposit, the token contract calls it with
zero. First-time buyers must call `storage_deposit` once with at least 0.01 NEAR
before their `ft_transfer_call` succeeds. The deposit covers the records the
machine writes:

- `DrawRecord` with a five tier snapshot: about 400 bytes
- `PackRecord`: about 120 bytes
- `buy_counts`, `buy_volume`, `storage_registered`: about 100 bytes each

At 1 NEAR per 100 KB, a buyer costs roughly 0.006 NEAR. The minimum deposit is
set above that. `storage_deposit` follows the NEP-145 shape and
`storage_balance_of` reports the deposited total.

## Security

- State is written before any cross-contract call. Callbacks are `#[private]`.
- Prizes use the pull pattern. `claim()` pays coin and restores
  the ledger in `on_claim_result` if the transfer fails.
- VRF output is verified onchain. `on_vrf_result` takes the Outlayer envelope as
  a `#[callback_result]`, checks `alpha` is `vrf:{request}:{machine}:{draw}` with
  the exact machine account and draw id, verifies the Ed25519 signature over
  `alpha`, and maps `sha256(signature)` into the frozen table snapshot.
- Each draw stores its own `worst_liability`. Settlement releases exactly that
  amount, so a later table upgrade cannot change an open draw's liability.
- `pool_reserved` is maintained on the single buy path and released on settle.
- The milestone transformation is a pure function and is proven against the
  initial table in `new`, so an upgrade can never brick `open_pack`.
- Leaderboards are updated incrementally with a bounded sort, never by scanning
  the full map on the buy or open path.
- The prize table compares the raw product sum: `total <= 950000` hard cap and
  `total <= 855000` so a fully discounted pack stays at or under 0.95 player EV.
- `vrf_pubkey` must decode to 32 bytes at init. Fallback settlement emits a
  NEP-297 event.
- Listed packs cannot be opened or transferred until delisted.
- NEP-297 events are emitted for pack buy, open, VRF settle, fallback settle,
  claim, list, sale, and curve trades.
- Graduation is four permissionless steps: `graduate` creates or reuses the
  Rhea pool, `seed_graduation` deposits the coin side, `seed_quote_graduation`
  deposits the quote side, and `finish_graduation` calls `add_liquidity`. Each
  step fits NEAR's per-transaction gas cap. A failed step leaves the machine in
  `GraduationPending` with purchases paused. `recover_from_pending` returns to
  the curve phase.
- `withdraw_protocol_fees` and `withdraw_protocol_fees_coin` are treasury only.
  `withdraw_creator_fees` and `withdraw_creator_fees_coin` are owner only. They
  can only move their own ledger, never `pool_balance` or reserves.
- `open_pack` requires exactly the VRF deposit, no excess NEAR is kept.
- Megapack funding is an explicit quarter of the pack fee after graduation, the
  rest goes to protocol fees. Each fee lands in exactly one ledger.
- `launch_coin` on the factory requires a storage deposit and refuses to
  overwrite a registered machine. The registry is written only in the
  `on_deploy_complete` callback, so a failed deploy leaves the name reusable.
  On mainnet the factory must additionally verify the machine account code hash
  against the published global hash.
- The coin refuses transfers to unregistered receivers. Register a receiver with
  the coin's `storage_deposit` before the machine pays it. `mock-usdc` stays
  lenient on purpose so demo buyers do not need a second registration.
- `nft_transfer` is payable and requires exactly 1 yocto. A pack in
  `PendingSale` cannot be opened, transferred, or relisted.
- A failed curve sell credits `pending_coin_refunds` instead of pushing a
  transfer from a callback. `claim_coin_refund` pulls it and the cache drops
  only when the coins actually leave.
- Resale fees sit in `pending_protocol_fees` until the seller is paid. A failed
  sale drops the pending fee and refunds the buyer. The treasury can only
  withdraw from `protocol_fees`.
- `sync_coin_balance` sets the cache to `max(accounted_minimum, min(balance,
  cache))` where `accounted_minimum = curve_coin_reserve + prize_pool +
  pending_coin_refunds_total`, so it never exceeds the onchain balance and never
  drops below the accounted minimum.
- `claim_draw(draw_id)` pays one settled draw. `claim()` pays every settled draw
  the caller owns.

## Graduation

The curve graduates when the implied market cap
`quote_reserve * COIN_TOTAL_SUPPLY / coin_reserve` reaches
`graduation_target`. Graduation is permissionless and runs in four calls so
each promise chain stays under the 300 Tgas transaction cap:

1. `graduate()` - checks the target, sets `GraduationPending`, registers the
   machine with Rhea, reads `get_pools`, and either reuses a pool for the exact
   `[coin, quote]` pair or calls `add_simple_pool` with fee 25. An existing pool
   is accepted only when its implied price is within 5 percent of the curve's
   final price, checked with integer cross multiplication. A skewed pool leaves
   the machine pending.
2. `seed_graduation()` - registers Rhea with the coin and deposits the coin
   side with an empty `ft_transfer_call` message.
3. `seed_quote_graduation()` - deposits the quote side.
4. `finish_graduation()` - calls `add_liquidity(pool_id, amounts)` in the pool's
   token order. The quote callback calls it automatically.

The seed excludes the coin fee ledgers: `coin_amount = coin_balance_cached -
prize_pool - protocol_fees_coin - creator_fees_coin`. Fee coins stay in the
machine for later withdrawal. If a reused pool lists `[quote, coin]` the amounts
are passed in that order. The `graduation_phase` guard is 0 idle, 1 coin
seeding, 2 quote ready, 3 liquidity pending. `seed_graduation` requires 0 and
sets 1, the coin callback sets 2, `seed_quote_graduation` requires 2 and sets 3,
and `finish_graduation` requires 3. The quote deposit is verified to equal the
expected amount within one unit.

On success the curve reserves are zeroed, `coin_balance_cached` becomes the
prize pool plus the fee coins, the status is `Graduated`, and the LP position is locked by
construction: the machine exposes no `remove_liquidity` path, so 100 percent of
the LP stays locked and the creator can withdraw 0 percent. After graduation the
curve panics with `curve retired, trade on Rhea pool <id>` and pack buys panic
with `packs paused after graduation`. Trading moves to the Rhea pool.

The graduation quote side is the curve's quote reserve. The machine must hold
that quote onchain. The initial virtual reserve is not backed by tokens, so the
creator must call `seed_pool` with at least the quote reserve before graduation.
`seed_quote_graduation` checks the machine's actual quote balance first and
fails with `insufficient quote backing` when it is short.

## Fan coins

A fan coin reuses an existing allowlisted coin instead of deploying a new one.
The creator must transfer `curve_coin_allocation + prize_coin_allocation` coins
to the machine before opening packs. `get_fan_coin_funding_status()` reports the
required amount, the cached balance, and the shortfall.
The factory allowlist is a `(coin, creator)` pair. `launch_coin` takes
`existing_coin` and `is_fan_coin`. A fan launch skips the coin deploy, points the
machine at the existing coin, and starts the machine with
`initial_coin_balance = 0`. The creator funds the machine with prize seed coins
by calling the coin's `ft_transfer` to the machine before opening packs. The
machine's solvency check enforces the real balance at the first state change.

## Squad

`create_squad` is payable and requires 0.05 NEAR. `MIN_SQUAD_DEPOSIT` is 1000
units and the creation deposit raises the sybil cost. `buy_squad_pack` calls
`get_my_last_draw(squad)` after the buy, stores that id as the squad's `draw_id`,
and moves to `BuyingFailed` if the view returns none. `open_squad_pack` takes
only `squad_id`. `sync_squad_prize` registers the squad with its coin (0.01 NEAR from the
squad's balance) and calls `claim_draw(draw_id)`, then credits each member's
pro-rata share to `member_claimable`. `claim_squad_share` pulls one
member's share with a single `ft_transfer`. `refund_squad` credits each deposit
once and zeroes the deposit ledger, so a second refund cannot double pay. A
failed buy does the same and moves the squad to `Refunded`.

Gas requirements: `buy_squad_pack` 300 Tgas, `open_squad_pack` 250,
`sync_squad_prize` 250, `claim_squad_share` and `withdraw_refund` 70. Views and
simple state changes (`get_squad`, `create_squad`, `refund_squad`,
`recover_to_funded`, `recover_stuck_squad`) have no minimum.

## Deviations

- The Rhea swap message shape is the verified contract shape
  `{"actions":[{pool_id, token_in, token_out, amount_in, min_amount_out}]}`,
  not the `{"action":"swap"}` shape in the work order.
- Graduation is split into four calls because the full promise chain exceeds
  the 300 Tgas transaction cap.
- The factory launch is two calls, `launch_coin` then `deploy_machine`, for the
  same gas reason. The machine cannot deploy before the coin callback confirms.
- Curve trades commit reserves in the callback and account for the actual
  transferred amounts. A price move above 2 percent emits a `price_moved` event.
  That 2 percent is the documented testnet MEV tolerance; a completed transfer
  cannot be clawed back.
- `sync_coin_balance` sets the cached balance to the exact onchain balance, so
  a shortfall surfaces on the next state-changing call.
- Megapack funding and holder registration were removed from `buy_pack` because
  post-graduation pack buys are paused. `megapack_claim` stays but no holder is
  registered.
- Post-graduation pack buys are paused. The work order allows this fallback.
  Users trade the coin on the Rhea pool instead.
- Tier seeding is deferred. The machine seeds one pool with the full curve
  allocation and the quote reserve.

## Deployment checklist

Every testnet default below must be replaced for mainnet:

- quote token default `usdc.bigo.testnet` in `machine` and `factory`
- Rhea default `ref-finance-101.testnet` in `machine`
- coin fallback `coin.bigo.testnet` in `squad`
- Outlayer account `outlayer.testnet` and project id
  `bigodeployer.testnet/bigo-vrf` in the machine config
- factory owner, treasury, and the deployed factory, machine, and squad
  accounts
- `global_contract_hash` must match the published machine code

Deploy and init every contract in one batched transaction. The factory does this
with create_account, deploy, and new in a single promise chain.
