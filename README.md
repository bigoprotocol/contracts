# bigo/contracts

NEAR contracts for Bigo protocol.

Four crates in a Cargo workspace:

- `factory` - registry of machines, publishes the machine global contract hash
- `machine` - core gacha machine, one global contract implementation referenced by hash
- `squad` - group pack buys with escrowed deposits and pro-rata prize split
- `mock-usdc` - NEP-141 mock token with 6 decimals and faucet mint for testnet demo

## Install

Rust stable with `wasm32-unknown-unknown` target and `cargo-near`:

```bash
rustup target add wasm32-unknown-unknown
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/near/cargo-near/releases/latest/download/cargo-near-installer.sh | sh
```

## Build

```bash
cd bigo/contracts
cargo near build non-reproducible-wasm --manifest-path factory/Cargo.toml
cargo near build non-reproducible-wasm --manifest-path machine/Cargo.toml
cargo near build non-reproducible-wasm --manifest-path squad/Cargo.toml
cargo near build non-reproducible-wasm --manifest-path mock-usdc/Cargo.toml
# or build all:
for C in factory machine squad mock-usdc; do cargo near build non-reproducible-wasm --manifest-path $C/Cargo.toml; done
```

Artifacts: `target/near/<crate>/<crate>.wasm`

Reproducible WASM (requires `[reproducible_build]` in Cargo.toml and clean git):

```bash
cargo near build reproducible-wasm --manifest-path factory/Cargo.toml --no-locked
```

## Test

```bash
cargo test
```

Integration tests in each crate `tests/` use `near-workspaces` sandbox.

## Structure

Each crate:

```
src/lib.rs       contract entry
src/error.rs     error enum, no bare string panics for user-facing errors
tests/           sandbox integration tests
scripts/build.sh thin wrapper around cargo near build
```


## Security

- State before cross-call, private callbacks (`#[private]`), pull prize `claim()` with `on_claim_result` recovery
- VRF envelope verified: `ed25519_verify(alpha, sig, pubkey)`, alpha format `vrf:{id}:{machine}:{draw}`, roll from `sha256(sig) % 10000`
- Worst liability snapshot per draw `worst_liability`, `pool_reserved` accounting, checked math
- Listing locks: `PackStatus::Listed`, `nft_transfer` blocked when listed
- Squad escrow via `ft_on_transfer`, pro-rata prize split, refund after deadline race guarded
