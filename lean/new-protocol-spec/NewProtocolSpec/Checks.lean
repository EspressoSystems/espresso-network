module

import NewProtocolSpec.Proofs.Decide
import NewProtocolSpec.Proofs.Liveness.Epochs
import NewProtocolSpec.Witness
meta import Lean.Elab.Command

/-!
# Checks on the specification's own shape

Two things prose cannot check, checked at build time:

* nothing the specification declares rests on an axiom beyond Lean's own, and on
  no `sorry`;
* the rule structures, the premises and the synchrony assumptions have exactly the
  fields listed below. A field added to `SafeHistory` widens what safety rests on,
  and one added to `Network`, `Committee` or `Synchrony` widens what is assumed; so
  either should be a deliberate change here, not something that happens while a
  proof is being repaired.

That the safety premises can be met together is `NewProtocolSpec.Witness`: a network
meeting all of them, in which a block is committed and a new epoch opened behind
it. That the liveness premises can be met is the implementation package's
witness: a node running the machine meets every premise of `ChainGrows`.
-/

open Lean

namespace NewProtocol
namespace Checks

/-- Nothing the specification declares rests on an axiom beyond Lean's own. -/
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

/-! The results proved so far, and their footprints. -/

/-- info: 'NewProtocol.noFork' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms noFork

/-- info: 'NewProtocol.decideAgreement' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms decideAgreement

/-- info: 'NewProtocol.decidesValid' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms decidesValid

/--
info: 'NewProtocol.Liveness.chainGrows' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in #print axioms Liveness.chainGrows

/-- info: 'NewProtocol.Witness.premises_met' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms Witness.premises_met

/-- info: 'NewProtocol.Witness.per_epoch' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms Witness.per_epoch

/-- Fail when a structure's fields are not what is expected. -/
meta def checkFields (parent : Name) (expected : List Name) : MetaM Unit := do
  let actual := (getStructureFields (← getEnv) parent).toList
  if actual == expected then return
  let gained := actual.filter (!expected.contains ·)
  let lost := expected.filter (!actual.contains ·)
  let mut msg := m!"{parent} is not what `NewProtocolSpec.Checks` expects."
  unless gained.isEmpty do msg := msg ++ m!"\n  gained: {gained}"
  unless lost.isEmpty do msg := msg ++ m!"\n  lost: {lost}"
  if gained.isEmpty && lost.isEmpty then msg := msg ++ m!"\n  reordered: {actual}"
  throwError msg

/-! The signing rules: what safety rests on. -/
run_meta (checkFields `NewProtocol.SafeHistory
  [`vote1Justified, `vote1Once, `vote2Justified, `vote2Once, `vote2BeforeTimeout, `timeoutLock,
   `decideJustified, `decideOnce])

/-! The rest of the protocol: what liveness reads of a node beyond safety. -/
run_meta (checkFields `NewProtocol.ProtocolHistory
  [`toSafeHistory, `vote1Leader, `timeoutJustified, `timeoutAnswered, `proposeJustified,
   `revoteJustified, `proposeOnce])

/-! What a node owes. -/
run_meta (checkFields `NewProtocol.OwedVote1
  [`received, `wellFormed, `validated, `parent, `safe, `opens, `current, `notTimedOut, `notVoted,
   `inView])

run_meta (checkFields `NewProtocol.OwedVote1Again
  [`received, `wellFormed, `safe, `payload, `current, `notTimedOut, `notVoted, `inView])

run_meta (checkFields `NewProtocol.OwedVote2
  [`holds, `notVoted, `notCommitted, `notPast, `afterFloor])

run_meta (checkFields `NewProtocol.OwedDecide [`holds])

/-! What is assumed of stake. -/
run_meta (checkFields `NewProtocol.Committee
  [`honest, `members, `Quorum, `intersect, `honestFinite])

/-! What is assumed of certificates: the verification layer's contract. -/
run_meta (checkFields `NewProtocol.Network
  [`trace, `safe, `cert1Genuine, `cert2Genuine, `timeoutCertGenuine, `revoteGenuine])

/-! What a timed network adds: times, the full protocol, and causality of the timeout inputs. -/
run_meta (checkFields `NewProtocol.TimedNetwork
  [`toNetwork, `honestQuorum, `time, `timeMono, `protocol,
   `timeoutCertCausal, `oneHonestCausal, `authentic])

/-! What is assumed of timing and delivery after GST. -/
run_meta (checkFields `NewProtocol.Synchrony
  [`proposal, `revote, `cert1, `cert2, `cert2Spread, `certSpread, `lockSpread, `blockSpread,
   `timeoutCert, `timeoutCertSpread, `timeoutLockSpread, `epochChange,
   `proposalValid, `validatedSound, `validated, `header, `timeUnbounded, `timerNotEarly,
   `timerFires])

end Checks
end NewProtocol
