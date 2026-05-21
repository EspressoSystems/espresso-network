# Solidity guidance for agents

Read [`../../AGENTS.md`](../../AGENTS.md) first for overview and cross-cutting rules.

## Critical rules

**NEVER:**

- Modify V1 contract storage layout. V2 inherits V1; changing V1 storage breaks upgrades.

**MUST:**

- Emit events for all state changes external systems need to track.
- Run `forge fmt` before committing.

## Commands

```bash
forge fmt
forge test                            # unit tests
just sol-test                         # full Solidity suite (unit, fuzz, invariant)
just gen-bindings                     # regenerate Rust bindings after ABI changes
```

## Code style

- Use `forge fmt` (default rules) before committing.
- Use `vm.expectEmit()` without arguments where possible in tests.
- Upgradeable contracts: VN extends V(N-1). Never modify earlier storage; only append.
- Emit events for every externally-observable state change.

## Type and API design

- Make storage layout explicit and append-only across versions.
- Custom errors over `require(..., "string")` (cheaper and clearer at the bytecode level).
- Use enums for state machines, not magic numbers.
- Mark constants and immutables; don't waste storage slots on values that never change.
- Restrict access with modifiers built from typed roles, not raw addresses.

## Key contracts (`contracts/src/`)

- `LightClient.sol`: verifies HotShot state proofs, stores block commitments, exposes `authRoot()`
- `StakeTable.sol` / `StakeTableV2.sol` / `StakeTableV3.sol`: validator staking, delegations, withdrawals, x25519/p2p
  registration (V3, fast finality)
- `FeeContract.sol`: builder fee deposits, read by Espresso node from finalized L1
- `EspToken.sol`: ESP token (ERC20)
- `RewardClaim.sol`: validator reward distribution; verifies merkle proofs against `lightClient.authRoot()`

## Version compatibility

Contract ABIs are supersets across versions: V3 includes all V2 types, V2 includes all V1 types.

- **Runtime code** (contract calls, event decoding from Rust): always use the latest version's bindings (e.g.,
  `StakeTableV3`).
- **V1/V2 bindings**: only used in deploy/upgrade code.

After changing a contract:

1. `forge fmt`
2. `just sol-test`
3. `just gen-bindings` (regenerates Rust bindings)
4. If storage layout changed, confirm V(N-1) layout is unchanged before merging.

## Upgrades

See [`../smart-contract-upgrades.md`](../smart-contract-upgrades.md) for the upgrade process and proxy pattern details.
