module

public import StakeTableImpl.Node
public import StakeTableImpl.Node.Refines
public import StakeTableImpl.Apply
public import StakeTableImpl.Conformance
public import StakeTableImpl.Indexed.Apply
public import StakeTableImpl.Indexed.Lists
public import StakeTableImpl.Indexed.Refines
public import StakeTableImpl.Select
public import StakeTableImpl.Reward
public import StakeTableImpl.Events
public import StakeTableImpl.Schedule
public import StakeTableImpl.Committee
public import StakeTableImpl.Payout
import StakeTableImpl.Checks
meta import Lean.Elab.Command

/-!
# The stake table implementation

The node's stake table logic, proved to meet the specification, at each level from the
node's own to the one to run:

* `StakeTableImpl.Node`: the node's state and what each L1 event does to it, outcome
  for outcome (`StakeTable.step`). `StakeTableImpl.Node.Refines` proves it implements
  the specification: each event the node applies comes to a change the
  specification's table takes, each it skips or rejects does not, and the node's
  validators are the members of the table of the history the events make
  (`StakeTable.step_refines`, `StakeTable.replay_refines`).
* `StakeTable.apply`, the node's `apply_event` check for check and error for error,
  with the proof that it gives the outcomes of `step` (`StakeTable.apply_outcome`).
* `StakeTable.Indexed.apply`, on hash maps and sets, the one to run, with the proof
  that it refines `apply` (`StakeTable.Indexed.applyAll_replay`), and whose members
  are the table of the history the events make (`StakeTable.Indexed.summaries_table`).

And the rest of the logic, each proved to meet the specification:
`StakeTable.select` (`StakeTable.select_spec`), which the node's own selection from its
state agrees with (`StakeTable.State.select_eq`), `StakeTable.blockReward`
(`StakeTable.blockReward_eq_some_iff`), `StakeTable.rewardAdmitted`
(`StakeTable.rewardAdmitted_iff`), `StakeTable.orderLogs` and
`StakeTable.chunks` (`StakeTable.orderLogs_inL1Order`, `StakeTable.chunks_chunked`),
`StakeTable.scheduleOrder` and `StakeTable.leader`, together `StakeTable.committeeLeader`
(`StakeTable.scheduleOrder_spec`, `StakeTable.leader_eq_some_iff`,
`StakeTable.committeeLeader_eq_some_iff`). `StakeTable.committeeAt` puts ordering,
replay and selection together and reads an epoch's committee from the logs on L1
(`StakeTable.committeeAt_spec`). `StakeTable.payout`, the node's `compute_rewards`,
credits each account what the specification says (`StakeTable.payout_credit`).

The differential checks against the node are in `lean/stake-table-diff`, which runs
these on cases the node's tests generate and compares the results.

Building this module fails if anything the specification or the implementation
declares rests on an axiom beyond Lean's own, `sorry` among them
(`StakeTable.checkAxioms`). The check runs here because this module imports every
other one.
-/

namespace StakeTable

/--
Nothing the specification or the implementation declares rests on an axiom beyond
Lean's own.

Covers every module this one imports. Fails also if it finds no module of one of the
two packages, so that it cannot pass by checking nothing.
-/
meta def checkAxioms : Lean.MetaM Unit := do
  let env ← Lean.getEnv
  let allowed : List Lean.Name := [`propext, `Classical.choice, `Quot.sound]
  let packages : List Lean.Name := [`StakeTableSpec, `StakeTableImpl]
  for p in packages do
    unless env.allImportedModuleNames.any (p.isPrefixOf ·) do
      throwError m!"the axiom check found no module of {p}"
  let mut bad : Array (Lean.Name × Lean.Name) := #[]
  for (name, idx) in env.const2ModIdx.toList do
    let some m := env.allImportedModuleNames[idx.toNat]? | continue
    unless packages.any (·.isPrefixOf m) do continue
    for a in ← Lean.collectAxioms name do
      unless allowed.contains a do bad := bad.push (name, a)
  unless bad.isEmpty do
    throwError m!"the stake table rests on axioms beyond Lean's own:\n{bad.toList}"

run_meta checkAxioms

end StakeTable
