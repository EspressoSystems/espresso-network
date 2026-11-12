module

import StakeTableImpl.Indexed.Refines
import StakeTableImpl.Select
import StakeTableImpl.Reward
import StakeTableImpl.Events
import StakeTableImpl.Schedule
import StakeTableImpl.Committee
import StakeTableImpl.Payout
meta import StakeTableImpl.Indexed.Refines
meta import StakeTableImpl.Reward
meta import StakeTableImpl.Events
meta import StakeTableImpl.Schedule
meta import Lean.Elab.Command

/-!
# Checks on the implementation

A few event sequences run through `apply` and `Indexed.apply`, with the result the
node gives, and a check, failing the build, that nothing in the specification or the
implementation rests on an axiom beyond Lean's own, `sorry` among them.
-/

namespace StakeTable

private def alice : Address := ⟨1⟩
private def bob : Address := ⟨2⟩
private def carol : Address := ⟨3⟩

private def run (es : List Event) : Except StakeTableError State :=
  applyAll State.empty es

private def stakeOf (es : List Event) (a : Address) : Option Nat :=
  match run es with
  | .ok s => (s.find? a).map (·.stake)
  | .error _ => none

-- Delegating and undelegating add up; a delegation emptied is removed.
#guard stakeOf [.register alice (some ⟨10⟩) (some ⟨20⟩) 500,
  .delegate bob alice 7, .delegate carol alice 5, .undelegate bob alice 7] alice == some 5
#guard (match run [.register alice (some ⟨10⟩) (some ⟨20⟩) 500, .delegate bob alice 7,
  .undelegate bob alice 7] with
  | .ok s => (s.find? alice).map (·.delegators) == some []
  | .error _ => false)

-- A reused consensus key is fatal; a reused light client key skips a first-version registration.
#guard (match run [.register alice (some ⟨10⟩) (some ⟨20⟩) 0, .register bob (some ⟨10⟩) none 0] with
  | .error (.blsKeyAlreadyUsed _) => true
  | _ => false)
#guard (match run [.register alice (some ⟨10⟩) (some ⟨20⟩) 0, .register bob (some ⟨11⟩) (some ⟨20⟩) 0] with
  | .ok s => s.registered bob == false
  | .error _ => false)

-- From `StakeTableV2` on, the reused light client key is dropped instead.
#guard (match run [.register alice (some ⟨10⟩) (some ⟨20⟩) 0,
  .registerV2 bob (.verified ⟨11⟩ ⟨20⟩) 0] with
  | .ok s => (s.find? bob).map (fun v => (v.schnorrKey, v.authenticated)) == some (none, false)
  | .error _ => false)

-- A validator that left cannot register again, and its keys stay admitted.
#guard (match run [.register alice (some ⟨10⟩) (some ⟨20⟩) 0, .deregister alice,
  .register alice (some ⟨12⟩) (some ⟨22⟩) 0] with
  | .error (.validatorAlreadyExited _) => true
  | _ => false)
#guard (match run [.register alice (some ⟨10⟩) (some ⟨20⟩) 0, .deregister alice,
  .register bob (some ⟨10⟩) (some ⟨22⟩) 0] with
  | .error (.blsKeyAlreadyUsed _) => true
  | _ => false)

-- A key update with a key that does not decode is skipped, even for an unknown validator.
#guard (match run [.keyUpdate carol none (some ⟨1⟩)] with
  | .ok s => s == State.empty
  | .error _ => false)

-- The indexed state gives the same stakes and the same registration order.
private def mixed : List Event :=
  [.register alice (some ⟨10⟩) (some ⟨20⟩) 500, .registerV2 bob (.verified ⟨11⟩ ⟨21⟩) 100,
   .registerV3 carol (.unverified (some ⟨12⟩) none) 0 (some ⟨30⟩) none,
   .delegate carol alice 9, .delegate alice bob 4, .undelegate carol alice 2, .deregister bob,
   .keyUpdateV2 alice (.verified ⟨13⟩ ⟨23⟩), .x25519Update alice (some ⟨31⟩)]

#guard (match run mixed, Indexed.applyAll Indexed.State.empty mixed with
  | .ok s, .ok is =>
    is.order.toList == s.validators.map (·.account)
      && s.validators.all (fun v => (is.validators[v.account]?).map (·.stake) == some v.stake)
  | _, _ => false)

-- The block reward: with 1% of the supply staked, 0.03 / sqrt(0.02) of the stake a year.
#guard blockReward 100 1 2000 == some 0
#guard blockReward (10 ^ 27) (10 ^ 25) 2000 == some 134533253650408585
#guard blockReward 100 101 2000 == none
#guard blockReward 100 1 0 == none

-- Logs in L1 order, and a read split into chunks.
#guard (orderLogs [((2, 0), 0), ((1, 5), 1), ((1, 2), 2)]).map (·.2) == [2, 1, 0]
#guard chunks 0 25 10 == [(0, 9), (10, 19), (20, 25)]
#guard chunks 5 4 10 == []

-- A breakpoint picks the validator whose share contains it.
#guard (List.range 6).map (pickIndex [⟨[1], 2⟩, ⟨[2], 3⟩, ⟨[3], 1⟩]) ==
  [some 0, some 0, some 1, some 1, some 1, some 2]

/-- Nothing the specification or the implementation declares rests on an axiom beyond Lean's own. -/
meta def checkAxioms : Lean.MetaM Unit := do
  let env ← Lean.getEnv
  let allowed : List Lean.Name := [`propext, `Classical.choice, `Quot.sound]
  let packages : List Lean.Name := [`StakeTableSpec, `StakeTableImpl]
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
