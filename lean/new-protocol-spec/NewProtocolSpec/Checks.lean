module

import NewProtocolSpec.Safety
import NewProtocolSpec.DecideStream
import NewProtocolSpec.Invariants
import NewProtocolSpec.Deadlock
import NewProtocolSpec.Round
import NewProtocolSpec.Implements
import NewProtocolSpec.Checks.Examples
meta import Lean.Elab.Command

/-!
# Checks on the claims this specification makes about itself

`NewProtocolSpec.Assumptions` names what is believed and asserts that nothing
else is; `NewProtocolSpec` names the results and the way in. Prose cannot check
either, so what can be checked is checked here at build time:

* the results depend on no axiom beyond Lean's own, and on no `sorry`;
* `SafetySpec` has exactly the clauses the no-fork proof is entitled to, so a
  clause added to it is a deliberate widening of what safety rests on rather
  than something that happens while a proof is being repaired;
* the premises are exactly the fields of `Committee` and `Network`, so a premise
  cannot be added without the trust surface being restated;
* the windows of `NewProtocolSpec.Progress` have exactly the fields they claim.
  Those are hypotheses rather than premises, so a field added there does not widen
  what is believed — it weakens what is concluded, silently, which is the same rot
  wearing the other hat;
* the obligations can be owed at all, which is `NewProtocolSpec.Checks.Examples`:
  a guard no state satisfies would leave every result about it vacuous, and prose
  cannot tell you which;
* the delivery bundles of `NewProtocolSpec.Round` have exactly the fields they
  claim, for the same reason the windows do: each is a hypothesis, and one grows
  by being weakened.

These exist because that rot has happened here: two premises that became theorems
and stayed listed as premises, and two clause counts that drifted after a clause
was removed.

What is checked here is the specification only. `new-protocol-impl` claims an
axiom footprint of its own, for the conformance proof, and checks it in
`NewProtocolImpl.Checks` — it has to, since this package cannot see that one.

What is *not* checked here is that the declarations named in prose still exist.
Two things cover that between them: `../Lint.lean` walks every docstring in the
specification and fails on a backticked name that resolves to nothing, and
`new-protocol-docs` fails to build when a declaration it splices is renamed.

The docstring check cannot run here — docstrings of imported declarations are not
visible to `run_meta` in a later module — so it is a program that imports the
specification itself.
-/

open Lean

namespace NewProtocol
namespace Checks

/-! ## Axioms

`propext`, `Classical.choice` and `Quot.sound` are Lean's own. `sorryAx` would
appear here if any proof were incomplete. Two of the results below use fewer:
turning a quorum's votes into a certificate is bookkeeping, and reaches for no
classical principle.

The list names results one at a time, which is what records each footprint, and
it is not the guard. It cannot be: a result added without a line beside it would
simply not be checked, and several were — `cert2_unique` and `timeoutCert_reached`
sat unlisted and unreached by anything listed, so a `sorry` in either would have
built green. `checkAxioms` below is the guard, made over every declaration the
specification has rather than over a list someone has to remember to extend.
-/

/-- info: 'NewProtocol.decideSafety' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decideSafety

/-- info: 'NewProtocol.decideInv_reachable' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decideInv_reachable

/-- info: 'NewProtocol.cert1_forms_of_owed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms cert1_forms_of_owed

/-- info: 'NewProtocol.cert2_forms_of_owed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms cert2_forms_of_owed

/-- info: 'NewProtocol.admitted_held' depends on axioms: [propext] -/
#guard_msgs in #print axioms admitted_held

/-- info: 'NewProtocol.cert1_forms' depends on axioms: [propext] -/
#guard_msgs in #print axioms cert1_forms

/-- info: 'NewProtocol.cert2_forms' depends on axioms: [propext] -/
#guard_msgs in #print axioms cert2_forms

/-- info: 'NewProtocol.quorum_on_chain' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms quorum_on_chain

/-- info: 'NewProtocol.vote1_owed_of_validated' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms vote1_owed_of_validated

/-- info: 'NewProtocol.vote2_owed_of_reconstructed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms vote2_owed_of_reconstructed

/-- info: 'NewProtocol.vote1_unstalled' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms vote1_unstalled

/-- info: 'NewProtocol.vote2_unstalled' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms vote2_unstalled

/-- info: 'NewProtocol.decide_unstalled' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decide_unstalled

/-- info: 'NewProtocol.propose_unstalled' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms propose_unstalled

/-- info: 'NewProtocol.vote1_forced' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms vote1_forced

/-- info: 'NewProtocol.vote2_forced' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms vote2_forced

/-- info: 'NewProtocol.decide_forced' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decide_forced

/-- info: 'NewProtocol.propose_forced' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms propose_forced

/-- info: 'NewProtocol.round_cert1' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms round_cert1

/-- info: 'NewProtocol.round_cert2' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms round_cert2

/-- info: 'NewProtocol.round_completes' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms round_completes

/-- info: 'NewProtocol.round_decided' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms round_decided

/--
Nothing the specification declares rests on an axiom beyond Lean's own.

The same claim as every line above, made once over everything rather than over
the results someone thought to name — so a `sorry` fails the build wherever it
is, in a result, in a working lemma, or in an auxiliary declaration no one reads.
Auxiliary declarations are included deliberately: a proof term can carry a
`sorry` in a `match` it elaborated to, and that is not a place a hand list ever
reaches.
-/
meta def checkAxioms : MetaM Unit := do
  let env ← getEnv
  let allowed : List Name := [`propext, `Classical.choice, `Quot.sound]
  let mut bad : Array (Name × Name) := #[]
  for (name, idx) in env.const2ModIdx.toList do
    let some m := env.allImportedModuleNames[idx.toNat]? | continue
    unless m == `NewProtocolSpec || (`NewProtocolSpec).isPrefixOf m do continue
    for a in ← Lean.collectAxioms name do
      unless allowed.contains a do bad := bad.push (name, a)
  unless bad.isEmpty do
    throwError m!"the specification rests on axioms beyond Lean's own:\n{bad.toList}"

run_meta checkAxioms

/-! ## What is stated where

The lists below are the specification's own claims about its shape: which clauses
safety rests on, which premises there are, and what a progress result may assume.
Each is compared with the declaration it describes, and a mismatch fails the build
with the name of what changed and where else it is written down.
-/

/-- Fail with an actionable message when a structure's fields are not what is expected. -/
meta def checkFields (parent : Name) (expected : List Name) (alsoUpdate : String) :
    MetaM Unit := do
  let actual := (getStructureFields (← getEnv) parent).toList
  if actual == expected then return
  let gained := actual.filter (!expected.contains ·)
  let lost := expected.filter (!actual.contains ·)
  let mut msg := m!"{parent} is not what `NewProtocolSpec.Checks` expects."
  unless gained.isEmpty do msg := msg ++ m!"\n  gained: {gained}"
  unless lost.isEmpty do msg := msg ++ m!"\n  lost: {lost}"
  if gained.isEmpty && lost.isEmpty then msg := msg ++ m!"\n  reordered: {actual}"
  throwError msg ++ m!"\n\nIf that is intended, update this check and {alsoUpdate}."

/-!
The twenty-one clauses no-fork rests on, by name.

A count would miss the change most worth catching: swapping one clause for
another, or renaming one, leaves the count alone.
-/
run_meta (checkFields `NewProtocol.SafetySpec
  [`proposalProvenance, `admissionJustified, `cert1Provenance,
   `barredViewUnchanged, `vote1NotBarred, `vote2NotBarred,
   `lockMono, `decidedRetained, `voted1Retained, `vote1BranchesRetained, `voted2Retained,
   `vote1Once, `vote1Justified, `vote1Records,
   `vote2Once, `vote2Justified, `vote2LockOrdered, `vote2NotInSkippedView, `vote2AboveFloor,
   `lockJustified, `timeoutVoteSound]
  "the clause count in `NewProtocolSpec.Safety` and the one in `new-protocol-docs`")

/-!
And that `StepSpec` adds the rest, rather than moving one of them across.

`toSafetySpec` is the parent projection, so it stands for all twenty-one above.
-/
run_meta (checkFields `NewProtocol.StepSpec
  [`toSafetySpec,
   `vidShareProvenance, `validatedProvenance, `reconstructedProvenance, `headerProvenance,
   `cert2Provenance, `timeoutCertProvenance,
   `proposalIngested, `cert1Ingested, `cert2Ingested, `timeoutCertIngested,
   `blockValidatedIngested, `reconstructedIngested, `headerIngested,
   `currentViewMono, `currentViewJustified, `timeoutViewMono, `timeoutViewJustified,
   `proposeNotBarred, `contentRetained, `proposedRetained,
   `vote1Bar, `vote1CarriesShare, `vote1Marked, `vote1BranchesSound,
   `vote2NotAfterCert2, `vote2Marked,
   `proposeOnce, `proposeBar, `proposeJustified, `proposedMarked,
   `decideJustified, `decidedMarked, `cert2RelayOwed,
   `advanceOwed, `timeoutCertSound, `timeoutCertAdvanceOwed, `timeoutVoteOwed]
  "the clause count in `new-protocol-docs`")

/-!
## What a node owes over a whole run

Four actions, and no more: a field added here is a new obligation on every
implementation, and one removed is an action a node may sit on for ever.
-/
run_meta (checkFields `NewProtocol.WeaklyFair
  [`vote1, `vote2, `decide, `propose]
  "`NewProtocolSpec.Liveness`, and the results of `NewProtocolSpec.Progress` that consume it")

/-!
## What progress is conditional on

A window is a hypothesis, and a hypothesis grows by being weakened. These lists
are what each progress result may assume of a node beyond the action being owed.
-/
run_meta (checkFields `NewProtocol.LockAllows
  [`safe, `below]
  "the field list in `NewProtocolSpec.Progress.Defs`")

run_meta (checkFields `NewProtocol.AnchorKept
  [`proposal, `cert]
  "the field list in `NewProtocolSpec.Progress.Defs`")

run_meta (checkFields `NewProtocol.Vote1Window
  [`bar, `timedOut, `lock, `floor, `parentFloor]
  "the field list in `NewProtocolSpec.Progress.Defs`")

run_meta (checkFields `NewProtocol.Vote2Window
  [`bar, `floor, `noSkip, `noCert2, `notDecided]
  "the field list in `NewProtocolSpec.Progress.Defs`")

run_meta (checkFields `NewProtocol.DecideWindow
  [`floor]
  "the field list in `NewProtocolSpec.Progress.Defs`")

run_meta (checkFields `NewProtocol.ProposeWindow
  [`bar, `timedOut, `floor, `parentFloor, `anchorKept, `lock]
  "the field list in `NewProtocolSpec.Progress.Defs`")

/-!
And what `NewProtocolSpec.Deadlock` may assume of the state an input arrives at,
and of the room the node has left itself. Mutation-sensitive for the same reason
as the windows: a field added here weakens the theorem that takes the bundle.
-/
run_meta (checkFields `NewProtocol.ProposalAdmissible
  [`bar, `admitted, `proposals, `vidShares, `safe, `wellFormed, `share]
  "the field list in `NewProtocolSpec.Deadlock.Defs`")

run_meta (checkFields `NewProtocol.Vote1Room
  [`bar, `timedOut, `lock, `floor, `parentFloor]
  "the field list in `NewProtocolSpec.Deadlock.Defs`")

run_meta (checkFields `NewProtocol.Vote2Room
  [`bar, `floor, `noSkip, `noCert2, `notDecided]
  "the field list in `NewProtocolSpec.Deadlock.Defs`")

run_meta (checkFields `NewProtocol.ProposeRoom
  [`bar, `timedOut, `floor, `parentFloor, `lock]
  "the field list in `NewProtocolSpec.Deadlock.Defs`")

run_meta (checkFields `NewProtocol.ProposeReady
  [`leads, `wellFormed, `justified, `parentHeld]
  "the field list in `NewProtocolSpec.Deadlock.Defs`")

/-!
And what a hop of a round may assume arrives. A field added here is a delivery the
environment must make before the round is claimed to complete, which is how a
result about delivery weakens without the trust surface changing.
-/
run_meta (checkFields `NewProtocol.Vote1Delivery
  [`arrival, `validated, `order, `parentHeld, `valid, `writable, `fresh, `window]
  "the field list in `NewProtocolSpec.Round.Defs`")

run_meta (checkFields `NewProtocol.Vote2Delivery
  [`certArrival, `payloadArrival, `order, `admitted, `writable, `fresh, `window]
  "the field list in `NewProtocolSpec.Round.Defs`")

run_meta (checkFields `NewProtocol.DecideDelivery
  [`arrival, `cert1Held, `blockHeld, `writable, `fresh, `window]
  "the field list in `NewProtocolSpec.Round.Defs`")

/-!
And that a `LiveNetwork` is a `Network` with fairness, rather than a second set of
premises alongside it.
-/
run_meta (checkFields `NewProtocol.LiveNetwork
  [`run, `fair, `net, `netRun]
  "`NewProtocolSpec.Progress.Defs`, and the premise list in `NewProtocolSpec.Assumptions`")

/-! What the argument takes from stake, and nothing more. -/
run_meta (checkFields `NewProtocol.Committee
  [`honest, `Quorum, `intersect]
  "`NewProtocolSpec.Assumptions`, which lists the premises")

/-!
The premises about a network.

`run` and `start` say what a network *is*; the rest are taken on trust.
-/
run_meta (checkFields `NewProtocol.Network
  [`run, `start, `Before, `beforeNext, `beforeTrans, `beforeWF,
   `evidenceValid, `timeoutOneHonestBacked, `cert1Delivered, `parentCertValid]
  "`NewProtocolSpec.Assumptions`, which lists the premises")

end Checks
end NewProtocol
