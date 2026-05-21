# Solidity

## Critical rules

**MUST:**

- Run `forge fmt` before committing
- Emit events for all state changes external systems track
- After ABI changes: `just gen-bindings`

**NEVER:**

- Modify V1 contract storage layout (V2 inherits V1; breaks upgrades). Storage is append-only across versions.

## Commands

```bash
forge test                            # unit tests
just sol-test                         # full suite (unit, fuzz, invariant)
just gen-bindings                     # regenerate Rust bindings after ABI changes
```

## Project conventions

- Use `vm.expectEmit()` without arguments where possible in tests.

## Key contracts (`contracts/src/`)

- `LightClient.sol`: verifies HotShot state proofs, stores block commitments, exposes `authRoot()`
- `StakeTable.sol` / `StakeTableV2.sol` / `StakeTableV3.sol`: validator staking, delegations, withdrawals, x25519/p2p
  registration (V3, fast finality)
- `FeeContract.sol`: builder fee deposits, read by Espresso node from finalized L1
- `EspToken.sol`: ESP token (ERC20)
- `RewardClaim.sol`: validator reward distribution; verifies merkle proofs against `lightClient.authRoot()`

## Version compatibility

ABIs are supersets across versions: V3 includes V2 includes V1.

- Runtime code (contract calls, event decoding from Rust): use the latest bindings (`StakeTableV3`).
- V1/V2 bindings: only for deploy/upgrade code.

## Upgrades

See `doc/smart-contract-upgrades.md`.
