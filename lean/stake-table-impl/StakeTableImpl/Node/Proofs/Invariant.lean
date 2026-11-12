module

public import StakeTableImpl.Node.Proofs.Validators

/-!
# Every event keeps the invariant

`step_inv` takes each event in turn; `replay_inv` follows by induction.
-/

@[expose] public section

namespace StakeTable

open State

theorem Inv.empty : Inv State.empty where
  accountsNodup := by simp [State.empty]
  notExited := by simp [State.empty]
  sound := by simp [State.empty]
  bls := ⟨by simp [State.empty], by simp [State.empty]⟩
  schnorr := ⟨by simp [State.empty], by simp [State.empty]⟩
  x25519 := ⟨by simp [State.empty], by simp [State.empty]⟩

theorem blsTaken_false {s : State} {k? : Option BlsKey} (h : ¬ s.blsTaken k? = true) :
    ∀ k, k? = some k → k ∉ s.usedBls := by
  rintro k rfl; simpa [State.blsTaken] using h

theorem schnorrTaken_false {s : State} {k? : Option SchnorrKey} (h : ¬ s.schnorrTaken k? = true) :
    ∀ k, k? = some k → k ∉ s.usedSchnorr := by
  rintro k rfl; simpa [State.schnorrTaken] using h

theorem x25519Taken_false {s : State} {k? : Option X25519Key} (h : ¬ s.x25519Taken k? = true) :
    ∀ k, k? = some k → k ∉ s.usedX25519 := by
  rintro k rfl; simpa [State.x25519Taken] using h

/-- A change that leaves a validator's stake and delegations alone keeps them adding up. -/
theorem Validator.Sound.of_same {v w : Validator} (h : v.Sound)
    (hd : w.delegators = v.delegators) (hs : w.stake = v.stake) : w.Sound where
  delegatorsNodup := hd ▸ h.delegatorsNodup
  delegationsPositive := hd ▸ h.delegationsPositive
  stakeTotal := by rw [hd, hs]; exact h.stakeTotal
  stakeBound := hs ▸ h.stakeBound

theorem registerNew_inv {s s' : State} {a : Address} {keys : RegistrationKeys}
    {commission : Nat} {x25519 : Option X25519Key} {p2pAddr : Option NetAddr} (h : Inv s)
    (he : s.registerNew a keys commission x25519 p2pAddr = .applied s') : Inv s' := by
  simp only [State.registerNew] at he
  split at he
  · cases he
  · rename_i hc
    simp only [not_or] at hc
    split at he
    · cases he
      exact h.admit hc.2.1 hc.1 (Validator.sound_new ..) (blsTaken_false hc.2.2.1)
        (by simp [Validator.new]) (x25519Taken_false hc.2.2.2)
    · rename_i hs
      cases he
      exact h.admit hc.2.1 hc.1 (Validator.sound_new ..) (blsTaken_false hc.2.2.1)
        (schnorrTaken_false hs) (x25519Taken_false hc.2.2.2)

theorem rekey_inv {s s' : State} {a : Address} {bls : BlsKey} {schnorr : SchnorrKey}
    (h : Inv s) (he : s.rekey a bls schnorr = .applied s') : Inv s' := by
  simp only [State.rekey] at he
  split at he
  · cases he
  · rename_i hc
    simp only [not_or] at hc
    split at he
    · cases he
    · rename_i hs
      cases he
      refine h.update [bls] [schnorr] [] (fun _ => rfl)
        (fun v hv _ => (h.sound v hv).of_same rfl rfl) ?_ ?_ (fun v _ _ k hk => .inl hk)
      · intro v _ _ k hk
        cases hk
        exact .inr ⟨List.mem_singleton_self _, hc.2⟩
      · intro v _ _ k hk
        cases hk
        exact .inr ⟨List.mem_singleton_self _, hs⟩

theorem delegate_inv {s s' : State} {d a : Address} {amount : Nat} (h : Inv s)
    (he : step s (.delegate d a amount) = .applied s') : Inv s' := by
  simp only [step] at he
  split at he
  · cases he
  · rename_i v hf
    split at he
    · cases he
    · rename_i hc
      simp only [not_or] at hc
      cases he
      refine h.update_keys (fun _ => rfl) ?_ (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
      intro w hw ha
      have hwv : w = v := eq_of_find? h.accountsNodup hf hw ha
      have hs := h.sound w hw
      exact {
        delegatorsNodup := nodup_credit d amount hs.delegatorsNodup
        delegationsPositive := positive_credit d hs.delegationsPositive (Nat.pos_of_ne_zero hc.1)
        stakeTotal := by
          show w.stake + amount = total (credit d amount w.delegators)
          rw [total_credit, ← hs.stakeTotal]
        stakeBound := by
          show w.stake + amount < U256
          subst hwv; omega }

theorem undelegate_inv {s s' : State} {d a : Address} {amount : Nat} (h : Inv s)
    (he : step s (.undelegate d a amount) = .applied s') : Inv s' := by
  simp only [step] at he
  split at he
  · cases he
  · rename_i v hf
    split at he
    · cases he
    · rename_i hc
      simp only [not_or] at hc
      cases he
      refine h.update_keys (fun _ => rfl) ?_ (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
      intro w hw ha
      have hwv : w = v := eq_of_find? h.accountsNodup hf hw ha
      have hs := h.sound w hw
      subst hwv
      exact {
        delegatorsNodup := nodup_debit d amount hs.delegatorsNodup
        delegationsPositive := positive_debit d amount hs.delegationsPositive
        stakeTotal := by
          show w.stake - amount = total (debit d amount w.delegators)
          rw [total_debit d (by omega), ← hs.stakeTotal]
        stakeBound := by
          show w.stake - amount < U256
          have := hs.stakeBound; omega }

/-- Every event keeps the invariant. -/
theorem step_inv {s s' : State} {e : Event} (h : Inv s) (he : step s e = .applied s') :
    Inv s' := by
  cases e with
  | register a bls schnorr commission =>
    simp only [step] at he
    split at he
    · cases he
    · rename_i hc
      simp only [not_or] at hc
      split at he
      · cases he
      · rename_i hs
        cases he
        exact h.admit hc.2.1 hc.1 (Validator.sound_new ..) (blsTaken_false hc.2.2)
          (schnorrTaken_false hs) (by simp [Validator.new])
  | registerV2 => exact registerNew_inv h he
  | registerV3 => exact registerNew_inv h he
  | deregister a =>
    simp only [step] at he
    split at he
    · cases he; exact h.remove a
    · cases he
  | delegate => exact delegate_inv h he
  | undelegate => exact undelegate_inv h he
  | keyUpdate a bls schnorr =>
    cases bls <;> cases schnorr <;> simp only [step] at he <;> first | cases he | exact rekey_inv h he
  | keyUpdateV2 a keys =>
    cases keys <;> simp only [step] at he <;> first | cases he | exact rekey_inv h he
  | commissionUpdate a commission =>
    simp only [step] at he
    split at he
    · cases he
    · cases he
      exact h.update_keys (fun _ => rfl) (fun v hv _ => (h.sound v hv).of_same rfl rfl)
        (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
  | x25519Update a key =>
    cases key with
    | none =>
      simp only [step] at he
      split at he
      · cases he
        exact h.update [] [] [] (fun _ => rfl) (fun v hv _ => (h.sound v hv).of_same rfl rfl)
          (fun _ _ _ _ hk => .inl hk) (fun _ _ _ _ hk => .inl hk) (fun _ _ _ _ hk => by cases hk)
      · cases he
    | some k =>
      simp only [step] at he
      split at he
      · cases he
      · rename_i hc
        simp only [not_or] at hc
        cases he
        refine h.update [] [] [k] (fun _ => rfl) (fun v hv _ => (h.sound v hv).of_same rfl rfl)
          (fun _ _ _ _ hk => .inl hk) (fun _ _ _ _ hk => .inl hk) ?_
        intro v _ _ k' hk
        cases hk
        exact .inr ⟨List.mem_singleton_self _, hc.2⟩
  | p2pAddrUpdate a p2pAddr =>
    simp only [step] at he
    split at he
    · cases he
      exact h.update_keys (fun _ => rfl) (fun v hv _ => (h.sound v hv).of_same rfl rfl)
        (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
    · cases he

/-- Every state a replay reaches from one that holds the invariant holds it too. -/
theorem replay_inv {s s' : State} {es : List Event} (h : Inv s) (he : replay s es = some s') :
    Inv s' := by
  induction es generalizing s with
  | nil => simp only [replay, Option.some.injEq] at he; exact he ▸ h
  | cons e es ih =>
    simp only [replay] at he
    split at he
    · rename_i s'' hs
      exact ih (step_inv h hs) he
    · exact ih h he
    · cases he

/-- Every state built from L1 events holds the invariant. -/
theorem replay_empty_inv {s : State} {es : List Event} (he : replay State.empty es = some s) :
    Inv s :=
  replay_inv Inv.empty he

end StakeTable
