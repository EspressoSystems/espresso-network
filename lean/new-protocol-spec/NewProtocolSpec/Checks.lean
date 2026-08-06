module

import NewProtocolSpec.Safety
import NewProtocolSpec.DecideStream
import NewProtocolSpec.Implements
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
  cannot be added without the trust surface being restated.

These exist because that rot has happened here: two premises that became theorems
and stayed listed as premises, and two clause counts that drifted after a clause
was removed.

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
appear here if any proof were incomplete.
-/

/-- info: 'NewProtocol.decideSafety' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decideSafety

/-- info: 'NewProtocol.decideInv_reachable' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decideInv_reachable

/-! ## What is stated where

The three lists below are the specification's own claims about its shape: which
clauses safety rests on, and which premises there are. Each is compared with the
declaration it describes, and a mismatch fails the build with the name of what
changed and where else it is written down.
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
