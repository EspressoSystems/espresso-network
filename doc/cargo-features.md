# Cargo features for zkVM builds

Four crates have a default-on feature whose absence yields a build for constrained targets (SP1 zkVM, riscv32/riscv64).
Default builds are unaffected.

| Crate                       | Feature  | Gates                                                                                       |
| --------------------------- | -------- | ------------------------------------------------------------------------------------------- |
| espresso-types              | `node`   | L1 client, persistence traits, Fetcher L1 methods, block proposal, full alloy               |
| hotshot-query-service-types | `web`    | tide-disco/surf-disco/events-service error types                                            |
| espresso-utils              | `full`   | node and tooling helpers (clap, tokio, surf, ...); the pure `ser` module stays              |
| light-client                | `client` | host query client, sqlite storage, query-service provider; `state.rs` and `consensus/` stay |

- `espresso-types/testing` implies `node`.
- Types, serde, `Committable` impls, the `SeqTypes: NodeType` impl and pure validation compile without `node`.
- Cargo silently ignores `default-features = false` on workspace-inherited deps; alloy, espresso-utils and
  hotshot-query-service-types are declared directly (non-inherited) in the affected crates for this reason.
- CI coverage: `just check-features-ci` (host feature powerset) and `just check-sp1-target` (SP1 target build of the
  `sp1/target-check` probe crate, which also carries the getrandom zkVM workarounds).

## Footguns: panics without `node`

`SeqTypes: NodeType` must stay implemented, so proposer and L1 code paths keep their signatures and panic with
`unimplemented!` when called without the feature:

- `ValidatedState::validate_and_apply_header` (`crates/espresso/types/src/v0/impls/state.rs`): always panics.
- `BlockHeader::new` for `Header` (`crates/espresso/types/src/v0/impls/header.rs`): always panics.
- `EpochCommittees::add_epoch_root` (`crates/espresso/types/src/v0/impls/committee.rs`): always panics.
- `get_l1_deposits` (`crates/espresso/types/src/v0/impls/state.rs`): panics only if the chain has a fee contract and the
  header references a finalized L1 block; otherwise returns `[]` like the node build.
- `Fetcher::initial_supply_or_fetch` (`crates/espresso/types/src/v0/impls/stake_table.rs`): panics unless
  `initial_supply` was pre-populated; reachable through `fetch_and_calculate_block_reward`.

Functions with no non-node callers are compiled out instead (e.g. `EpochCommittees::reload_stake`, the `L1Client`
cluster): calling them without `node` is a compile error, not a panic.
