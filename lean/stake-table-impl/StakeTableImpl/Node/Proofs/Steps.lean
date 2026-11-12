module

public import StakeTableImpl.Node.Step
public import StakeTableImpl.Node.Proofs.Invariant

/-!
# What an applied event can do

An event that applies adds a validator, removes one, or changes one in place
(`StakeTable.step_shape`). From that: admitted keys and exits are never forgotten
(`StakeTable.replay_grows`), a validator that left never returns
(`StakeTable.replay_exited`), and validators keep their relative order, so new ones
only come after (`StakeTable.step_order`).
-/

@[expose] public section

namespace StakeTable

open State

/-- The ways an applied event changes the state. -/
inductive Shape (s : State) : State → Prop where
  /-- A validator not yet registered is added. -/
  | admit (v : Validator) (hreg : ¬ s.registered v.account = true) : Shape s (s.admit v)
  /-- The validator under `a` is removed. -/
  | remove (a : Address) : Shape s (s.remove a)
  /-- The validator under `a` is changed in place, and keys may be admitted. -/
  | update (a : Address) (f : Validator → Validator) (bls : List BlsKey)
      (schnorr : List SchnorrKey) (x25519 : List X25519Key)
      (hacc : ∀ v, (f v).account = v.account) :
      Shape s { s.update a f with
        usedBls := bls ++ s.usedBls
        usedSchnorr := schnorr ++ s.usedSchnorr
        usedX25519 := x25519 ++ s.usedX25519 }

theorem registerNew_shape {s s' : State} {a : Address} {keys : RegistrationKeys}
    {commission : Nat} {x25519 : Option X25519Key} {p2pAddr : Option NetAddr}
    (he : s.registerNew a keys commission x25519 p2pAddr = .applied s') : Shape s s' := by
  simp only [State.registerNew] at he
  split at he
  · cases he
  · rename_i hc
    simp only [not_or] at hc
    split at he <;> (cases he; exact .admit _ hc.2.1)

theorem rekey_shape {s s' : State} {a : Address} {bls : BlsKey} {schnorr : SchnorrKey}
    (he : s.rekey a bls schnorr = .applied s') : Shape s s' := by
  simp only [State.rekey] at he
  split at he
  · cases he
  · split at he
    · cases he
    · cases he; exact .update a _ [bls] [schnorr] [] (fun _ => rfl)

/-- An applied event adds, removes or changes one validator. -/
theorem step_shape {s s' : State} {e : Event} (he : step s e = .applied s') : Shape s s' := by
  cases e with
  | register a bls schnorr commission =>
    simp only [step] at he
    split at he
    · cases he
    · rename_i hc
      simp only [not_or] at hc
      split at he
      · cases he
      · cases he; exact .admit _ hc.2.1
  | registerV2 => exact registerNew_shape he
  | registerV3 => exact registerNew_shape he
  | deregister a =>
    simp only [step] at he
    split at he
    · cases he; exact .remove a
    · cases he
  | delegate d a amount =>
    simp only [step] at he
    split at he
    · cases he
    · split at he
      · cases he
      · cases he; exact .update a _ [] [] [] (fun _ => rfl)
  | undelegate d a amount =>
    simp only [step] at he
    split at he
    · cases he
    · split at he
      · cases he
      · cases he; exact .update a _ [] [] [] (fun _ => rfl)
  | keyUpdate a bls schnorr =>
    cases bls <;> cases schnorr <;> simp only [step] at he <;> first | cases he | exact rekey_shape he
  | keyUpdateV2 a keys =>
    cases keys <;> simp only [step] at he <;> first | cases he | exact rekey_shape he
  | commissionUpdate a commission =>
    simp only [step] at he
    split at he
    · cases he
    · cases he; exact .update a _ [] [] [] (fun _ => rfl)
  | x25519Update a key =>
    cases key with
    | none =>
      simp only [step] at he
      split at he
      · cases he; exact .update a _ [] [] [] (fun _ => rfl)
      · cases he
    | some k =>
      simp only [step] at he
      split at he
      · cases he
      · cases he; exact .update a _ [] [] [k] (fun _ => rfl)
  | p2pAddrUpdate a p2pAddr =>
    simp only [step] at he
    split at he
    · cases he; exact .update a _ [] [] [] (fun _ => rfl)
    · cases he

theorem Grows.refl (s : State) : Grows s s :=
  ⟨fun _ h => h, fun _ h => h, fun _ h => h, fun _ h => h⟩

theorem Grows.trans {s s' s'' : State} (h : Grows s s') (h' : Grows s' s'') : Grows s s'' :=
  ⟨fun _ hx => h'.usedBls (h.usedBls hx), fun _ hx => h'.usedSchnorr (h.usedSchnorr hx),
   fun _ hx => h'.usedX25519 (h.usedX25519 hx), fun _ hx => h'.exits (h.exits hx)⟩

theorem Shape.grows {s s' : State} (h : Shape s s') : Grows s s' := by
  cases h with
  | admit v => exact ⟨fun _ hx => List.mem_append_right _ hx, fun _ hx => List.mem_append_right _ hx,
      fun _ hx => List.mem_append_right _ hx, fun _ hx => hx⟩
  | remove a => exact ⟨fun _ hx => hx, fun _ hx => hx, fun _ hx => hx,
      fun _ hx => List.mem_cons_of_mem _ hx⟩
  | update a f bls schnorr x25519 => exact ⟨fun _ hx => List.mem_append_right _ hx,
      fun _ hx => List.mem_append_right _ hx, fun _ hx => List.mem_append_right _ hx, fun _ hx => hx⟩

theorem replay_grows {s s' : State} {es : List Event} (he : replay s es = some s') :
    Grows s s' := by
  induction es generalizing s with
  | nil => simp only [replay, Option.some.injEq] at he; exact he ▸ Grows.refl s
  | cons e es ih =>
    simp only [replay] at he
    split at he
    · rename_i s'' hs
      exact (step_shape hs).grows.trans (ih he)
    · exact ih he
    · cases he

/-- A validator that left is never registered again. -/
theorem replay_exited {s s' : State} {es : List Event} {a : Address} (h : Inv s)
    (ha : a ∈ s.exits) (he : replay s es = some s') : s'.registered a = false := by
  cases hr : s'.registered a
  · rfl
  · obtain ⟨v, hv, rfl⟩ := registered_iff.1 hr
    exact absurd ((replay_grows he).exits ha) ((replay_inv h he).notExited v hv)

/--
Validators keep their relative order.

The validators of `s` still registered after the event come first in `s'`, in the
same order; a validator registered by the event comes after them. Active set
selection breaks ties in stake by this order.
-/
theorem step_order {s s' : State} {e : Event} (he : step s e = .applied s') :
    (s.accounts.filter (fun x => decide (x ∈ s'.accounts))) <+: s'.accounts := by
  cases step_shape he with
  | admit v =>
    have hall : s.accounts.filter (fun x => decide (x ∈ (s.admit v).accounts)) = s.accounts := by
      apply List.filter_eq_self.2
      intro x hx
      simp only [State.accounts, State.admit, List.map_append, decide_eq_true_eq] at hx ⊢
      exact List.mem_append_left _ hx
    rw [hall]
    simp only [State.accounts, State.admit, List.map_append]
    exact List.prefix_append _ _
  | remove a =>
    have hr : (s.remove a).accounts = s.accounts.filter (· != a) := by
      simp only [State.accounts, State.remove, List.filter_map]
      rfl
    rw [hr]
    have : s.accounts.filter (fun x => decide (x ∈ s.accounts.filter (· != a)))
        = s.accounts.filter (· != a) := by
      apply List.filter_congr
      intro x hx
      by_cases hxa : x = a <;> simp [List.mem_filter, hx, hxa]
    rw [this]
    exact List.prefix_refl _
  | update a f bls schnorr x25519 hacc =>
    have hu : ({ s.update a f with
        usedBls := bls ++ s.usedBls
        usedSchnorr := schnorr ++ s.usedSchnorr
        usedX25519 := x25519 ++ s.usedX25519 } : State).accounts = s.accounts :=
      accounts_update hacc
    rw [hu, List.filter_eq_self.2 (fun x hx => by simpa using hx)]
    exact List.prefix_refl _

end StakeTable
