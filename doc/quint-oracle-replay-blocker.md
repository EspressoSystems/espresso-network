# Quint oracle replay blocker — `new-consensus-protocol`

Status: **blocked**. Instrumentation works and traces are captured correctly, but
the oracle cannot replay them past the first few steps. Seven full oracle runs
across four different hypotheses; none resolved it.

- Spec: `quint-specs/new-consensus-protocol.qnt`
- Replay tests: `quint-specs/new-consensus-protocol_test.qnt`
- Config: `quint-oracle.new-consensus-protocol.json`
- Instrumentation: `crates/hotshot/new-protocol/src/quint_oracle.rs`, call sites in
  `crates/hotshot/new-protocol/src/consensus.rs`
- Reports:
  `~/.local/share/co.quint.studio/projects/EspressoSystems_espresso-network/reports/new-consensus-protocol/report.json`

## What works

- The test boundary fires: every run reports `3 tests, 3 with logged traces`
  (2 in later runs, because fail-fast stops the suite earlier).
- The logged traces are correct. Decoded by hand they match what the code does,
  in order, with the right views and leaves.
- **The spec accepts the failing traces.** `new-consensus-protocol_test.qnt`
  replays both of them step by step, calling the actions directly, and passes.
  So no guard is false at the step replay stops on.

## The failure

Always `failureCode: argument-not-pinned`, always with `pinnedArgs: []`:

> Logged arguments [...] for action '...' pinned no replay choice at step N:
> either none of the logged names matches a nondet/parameter the spec picks for
> this action, or no transition for '...' is enabled from the replayed state.

Both halves of that disjunction are contradicted by the evidence: the logged
argument names match the nondet names *and* the action parameter names, and the
replay tests prove the transition is enabled.

## What was tried

| # | Change | Result |
|---|---|---|
| 1 | `parent` pick domain derived from `leaf` → static `PARENT_LEAVES` | no change; fails at `receive_proposal`, step 5 |
| 2 | `stepReplayBudget` 50→600, `unguidedStepBudget` 10→20 | no change |
| 3 | Renamed every local binding shadowing a pick name (`leaf`/`parent`/`view`) | no change |
| 4 | Dropped `parent` from the log (single-argument action) | **improved**: 2 of 3 tests matched |
| 5 | Packed `(leaf, parent)` into one pinned argument | worse: broke `vote_1`, a previously matching test regressed |
| 6 | One `nondet event` packing `(view, leaf, parent)`, always logged, nothing unpinned | still fails, now at `vote_1` step 2 |

Run 6 is the decisive one. There is exactly **one** pick, it is logged on every
event, its value is in the domain, and the action is enabled — and the oracle
still reports `pinnedArgs: []`. That rules out unpinned-pick search, budget
exhaustion, name shadowing, and argument arity as the cause.

## Current state of the repo

Reverted to the configuration from run 4, the best observed (2 of 3 tests
matched):

- three picks in `step`: `leaf`, `parent`, `view`, with named action parameters;
- the instrumentation logs `leaf` only for `receive_proposal` /
  `reject_unsafe_proposal`, leaving `parent` unpinned.

Verified by a final run after the revert: `3 tests, 3 with logged traces,
1 unmatched` — `test_cert2_broadcast_once` still fails at `receive_proposal`,
step 5. The remaining failure is the longest of the three traces, which is
consistent with the parent being resolved by search rather than pinned.

That last point is a real correctness caveat, not a clean state: the parent is
resolved by search rather than pinned. It happens to be discriminated by later
steps (`vote_1` at the child's view requires the recorded parent to match the
stored proposal), but it is not exact.

## What to ask the Quint Studio authors

1. What exactly does `pinnedArgs: []` mean when the logged argument name equals
   both the `nondet` name in `step` and the action's parameter name?
2. Does replay require every `nondet` in `step` to be resolvable, including ones
   the fired action does not read? If so, what is the supported way to model a
   step whose actions need different arguments?
3. Is there a size limit on a `nondet` pick domain for pinning? `EVENTS` had
   1092 elements in run 6; the working single-argument case had 12.
4. Is `any { ... }` over actions with differing arities supported for MBT replay,
   and is the fired disjunct recovered from MBT metadata or by search?

## Reproducing

```
cargo nextest run -p hotshot-new-protocol --test-threads=1   # suite passes normally
quint test --main new_consensus_protocol_test quint-specs/new-consensus-protocol_test.qnt
```

Then run the oracle from Quint Studio against
`quint-oracle.new-consensus-protocol.json`.
