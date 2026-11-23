module

public import StakeTableImpl.Apply

/-!
# The node's checks meet the specification

On every state the invariant holds of, `apply` applies, skips and rejects exactly
the events `step` does, and gives the same state (`StakeTable.apply_outcome`). The
order of the checks and the errors reported are the implementation's own.

The invariant is needed only for the node's second overflow check, on the
delegator's own amount: since a delegation is part of the stake, it cannot fail
when the check on the stake passed.
-/

@[expose] public section

namespace StakeTable

open State

theorem applyRekey_outcome (s : State) (a : Address) (bls : BlsKey) (schnorr : SchnorrKey) :
    (s.applyRekey a bls schnorr).outcome = s.rekey a bls schnorr := by
  simp only [State.applyRekey, State.rekey]
  by_cases hr : s.registered a = true <;> by_cases hb : bls ∈ s.usedBls <;>
    by_cases hs : schnorr ∈ s.usedSchnorr <;> simp [hr, hb, hs, ApplyResult.outcome]

theorem admitRegistration_outcome (s : State) (a : Address) (keys : RegistrationKeys)
    (commission : Nat) (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) :
    (if s.schnorrTaken keys.schnorr then
      Outcome.applied (s.admit (.new a keys.bls none x25519 p2pAddr commission false))
    else .applied (s.admit (.new a keys.bls keys.schnorr x25519 p2pAddr commission keys.isVerified)))
      = .applied (s.admitRegistration a keys commission x25519 p2pAddr) := by
  simp only [State.admitRegistration]
  split <;> rfl

theorem bls_filter {s : State} (k? : Option BlsKey) :
    (k?.filter (· ∈ s.usedBls)).isSome = s.blsTaken k? := by
  cases k? <;> simp [State.blsTaken, Option.filter]; split <;> simp_all

theorem schnorr_filter {s : State} (k? : Option SchnorrKey) :
    (k?.filter (· ∈ s.usedSchnorr)).isSome = s.schnorrTaken k? := by
  cases k? <;> simp [State.schnorrTaken, Option.filter]; split <;> simp_all

theorem x25519_filter {s : State} (k? : Option X25519Key) :
    (k?.filter (· ∈ s.usedX25519)).isSome = s.x25519Taken k? := by
  cases k? <;> simp [State.x25519Taken, Option.filter]; split <;> simp_all

/-- `apply` comes to what `step` specifies, on every state that holds the invariant. -/
theorem apply_outcome {s : State} (h : Inv s) (e : Event) : (apply s e).outcome = step s e := by
  cases e with
  | register a bls schnorr commission =>
    simp only [apply, step]
    by_cases hx : a ∈ s.exits
    · simp [hx, ApplyResult.outcome]
    by_cases hr : s.registered a = true
    · simp [hx, hr, ApplyResult.outcome]
    have hb := bls_filter (s := s) bls
    have hs := schnorr_filter (s := s) schnorr
    cases hbf : bls.filter (· ∈ s.usedBls) with
    | some k =>
      rw [hbf] at hb
      simp [hx, hr, ← hb, ApplyResult.outcome]
    | none =>
      rw [hbf] at hb
      cases hsf : schnorr.filter (· ∈ s.usedSchnorr) with
      | some j =>
        rw [hsf] at hs
        simp [hx, hr, ← hb, ← hs, ApplyResult.outcome]
      | none =>
        rw [hsf] at hs
        simp [hx, hr, ← hb, ← hs, ApplyResult.outcome]
  | registerV2 a keys commission =>
    simp only [apply, step, State.registerNew]
    by_cases hx : a ∈ s.exits
    · simp [hx, ApplyResult.outcome]
    by_cases hr : s.registered a = true
    · simp [hx, hr, ApplyResult.outcome]
    have hb := bls_filter (s := s) keys.bls
    cases hbf : keys.bls.filter (· ∈ s.usedBls) with
    | some k =>
      rw [hbf] at hb
      simp [hx, hr, ← hb, ApplyResult.outcome]
    | none =>
      rw [hbf] at hb
      simp only [hx, hr, ← hb, State.x25519Taken, ApplyResult.outcome, Option.isSome_none,
        Bool.false_eq_true, or_false, ↓reduceIte]
      exact (admitRegistration_outcome ..).symm
  | registerV3 a keys commission x25519 p2pAddr =>
    simp only [apply, step, State.registerNew]
    by_cases hx : a ∈ s.exits
    · simp [hx, ApplyResult.outcome]
    by_cases hr : s.registered a = true
    · simp [hx, hr, ApplyResult.outcome]
    have hb := bls_filter (s := s) keys.bls
    have hk := x25519_filter (s := s) x25519
    cases hbf : keys.bls.filter (· ∈ s.usedBls) with
    | some k =>
      rw [hbf] at hb
      simp [hx, hr, ← hb, ApplyResult.outcome]
    | none =>
      rw [hbf] at hb
      cases hkf : x25519.filter (· ∈ s.usedX25519) with
      | some k =>
        rw [hkf] at hk
        simp [hx, hr, ← hb, ← hk, ApplyResult.outcome]
      | none =>
        rw [hkf] at hk
        simp only [hx, hr, ← hb, ← hk, ApplyResult.outcome, Option.isSome_none,
          Bool.false_eq_true, or_false, ↓reduceIte]
        exact (admitRegistration_outcome ..).symm
  | deregister a =>
    simp only [apply, step]
    by_cases hr : s.registered a = true <;> simp [hr, ApplyResult.outcome]
  | delegate d a amount =>
    simp only [apply, step]
    by_cases h0 : amount = 0
    · cases hf : s.find? a <;> simp [h0, ApplyResult.outcome]
    cases hf : s.find? a with
    | none => simp [h0, ApplyResult.outcome]
    | some v =>
      by_cases hb : U256 ≤ v.stake + amount
      · simp [h0, hb, ApplyResult.outcome]
      have hv := find?_some hf
      have hs := h.sound v hv.1
      have : ¬ U256 ≤ amountOf d v.delegators + amount := by
        have := amountOf_le_total d v.delegators
        rw [← hs.stakeTotal] at this
        omega
      simp [h0, hb, this, ApplyResult.outcome]
  | undelegate d a amount =>
    simp only [apply, step]
    cases hf : s.find? a with
    | none => simp [ApplyResult.outcome]
    | some v =>
      by_cases h1 : v.stake < amount
      · simp [h1, ApplyResult.outcome]
      by_cases h2 : delegates d v.delegators = false
      · simp [h1, h2, ApplyResult.outcome]
      by_cases h3 : amountOf d v.delegators < amount
      · simp [h1, h2, h3, ApplyResult.outcome]
      simp [h1, h2, h3, ApplyResult.outcome]
  | keyUpdate a bls schnorr =>
    cases bls <;> cases schnorr <;> simp only [apply, step, ApplyResult.outcome] <;>
      exact applyRekey_outcome ..
  | keyUpdateV2 a keys =>
    cases keys <;> simp only [apply, step, ApplyResult.outcome] <;> exact applyRekey_outcome ..
  | commissionUpdate a commission =>
    simp only [apply, step]
    by_cases hc : maxCommission < commission
    · simp [hc, ApplyResult.outcome]
    by_cases hr : s.registered a = true <;> simp [hc, hr, ApplyResult.outcome]
  | x25519Update a key =>
    simp only [apply, step]
    by_cases hr : s.registered a = true
    · cases key with
      | none => simp [hr, Option.filter, ApplyResult.outcome]
      | some k =>
        by_cases hk : k ∈ s.usedX25519 <;> simp [hr, hk, Option.filter, ApplyResult.outcome]
    · cases key <;> simp [hr, ApplyResult.outcome]
  | p2pAddrUpdate a p2pAddr =>
    simp only [apply, step]
    by_cases hr : s.registered a = true <;> simp [hr, ApplyResult.outcome]

/--
Apply events in order, as `validators_from_l1_events` does: an expected error skips
the event, a fatal one stops the replay.
-/
def applyAll (s : State) : List Event → Except StakeTableError State
  | [] => .ok s
  | e :: es =>
    match apply s e with
    | .ok (.ok s') => applyAll s' es
    | .ok (.error _) => applyAll s es
    | .error err => .error err

/-- The node's replay gives what the specification's does. -/
theorem applyAll_replay {s : State} (h : Inv s) (es : List Event) :
    (applyAll s es).toOption = replay s es := by
  induction es generalizing s with
  | nil => rfl
  | cons e es ih =>
    have ho := apply_outcome h e
    simp only [applyAll, replay]
    cases ha : apply s e with
    | error err =>
      rw [ha] at ho
      simp only [ApplyResult.outcome] at ho
      rw [← ho]; rfl
    | ok r =>
      cases r with
      | ok s' =>
        rw [ha] at ho
        simp only [ApplyResult.outcome] at ho
        rw [← ho]
        exact ih (step_inv h ho.symm)
      | error err =>
        rw [ha] at ho
        simp only [ApplyResult.outcome] at ho
        rw [← ho]
        exact ih h

end StakeTable
