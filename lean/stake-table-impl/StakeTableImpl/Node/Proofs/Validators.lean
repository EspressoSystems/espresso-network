module

public import StakeTableImpl.Node.Step
public import StakeTableImpl.Node.Proofs.Delegations

/-!
# Changing the validator list

How finding, adding, changing and removing a validator interacts with the
invariant: each of `State.admit`, `State.update` and `State.remove` keeps it,
under conditions the events check.
-/

@[expose] public section

namespace StakeTable

open State

theorem registered_iff {s : State} {a : Address} :
    s.registered a = true ↔ ∃ v ∈ s.validators, v.account = a := by
  simp [State.registered, List.any_eq_true]

theorem not_registered {s : State} {a : Address} (h : ¬ s.registered a = true) :
    ∀ v ∈ s.validators, v.account ≠ a :=
  fun v hv ha => h (registered_iff.2 ⟨v, hv, ha⟩)

theorem find?_some {s : State} {a : Address} {v : Validator} (h : s.find? a = some v) :
    v ∈ s.validators ∧ v.account = a :=
  ⟨List.mem_of_find?_eq_some h, by simpa using List.find?_some h⟩

theorem find?_none {s : State} {a : Address} (h : s.find? a = none) :
    s.registered a = false := by
  simp only [State.find?, List.find?_eq_none, beq_iff_eq] at h
  cases hr : s.registered a
  · rfl
  · obtain ⟨v, hv, ha⟩ := registered_iff.1 hr
    exact absurd ha (h v hv)

theorem registered_of_find? {s : State} {a : Address} {v : Validator} (h : s.find? a = some v) :
    s.registered a = true :=
  registered_iff.2 ⟨v, find?_some h⟩

/-- With distinct accounts, the validator found under an account is the only one with it. -/
theorem eq_of_find? {vs : List Validator} {a : Address} {v w : Validator}
    (hn : (vs.map (·.account)).Nodup) (hf : vs.find? (·.account == a) = some v)
    (hw : w ∈ vs) (ha : w.account = a) : w = v := by
  induction vs with
  | nil => simp at hw
  | cons x xs ih =>
    simp only [List.map_cons, List.nodup_cons] at hn
    simp only [List.find?_cons] at hf
    simp only [List.mem_cons] at hw
    split at hf
    · rename_i hx
      simp only [beq_iff_eq] at hx
      cases hf
      rcases hw with rfl | hw
      · rfl
      · exact absurd (List.mem_map.2 ⟨w, hw, ha.trans hx.symm⟩) hn.1
    · rename_i hx
      simp only [beq_eq_false_iff_ne, ne_eq] at hx
      rcases hw with rfl | hw
      · exact absurd ha hx
      · exact ih hn.2 hf hw

theorem mem_update {s : State} {a : Address} {f : Validator → Validator} {w : Validator}
    (h : w ∈ (s.update a f).validators) :
    ∃ v ∈ s.validators, (v.account = a ∧ w = f v) ∨ (v.account ≠ a ∧ w = v) := by
  simp only [State.update, List.mem_map] at h
  obtain ⟨v, hv, rfl⟩ := h
  refine ⟨v, hv, ?_⟩
  by_cases ha : v.account = a
  · exact .inl ⟨ha, by simp [ha]⟩
  · exact .inr ⟨ha, by simp [ha]⟩

theorem accounts_update {s : State} {a : Address} {f : Validator → Validator}
    (hf : ∀ v, (f v).account = v.account) :
    (s.update a f).validators.map (·.account) = s.validators.map (·.account) := by
  simp only [State.update, List.map_map]
  apply List.map_congr_left
  intro v _
  by_cases ha : v.account = a <;> simp [ha, hf]

theorem KeysTracked.update {κ : Type} {vs : List Validator} {key : Validator → Option κ}
    {used new : List κ} {a : Address} {f : Validator → Validator}
    (h : KeysTracked vs key used) (hacc : ∀ v, (f v).account = v.account)
    (hkey : ∀ v ∈ vs, v.account = a → ∀ k, key (f v) = some k →
      key v = some k ∨ (k ∈ new ∧ k ∉ used)) :
    KeysTracked (vs.map fun v => if v.account = a then f v else v) key (new ++ used) := by
  have hmem : ∀ w ∈ vs.map (fun v => if v.account = a then f v else v),
      ∃ v ∈ vs, (v.account = a ∧ w = f v) ∨ (v.account ≠ a ∧ w = v) :=
    fun w hw => mem_update (s := ⟨vs, [], [], [], []⟩) hw
  constructor
  · intro w hw k hk
    obtain ⟨v, hv, ⟨ha, rfl⟩ | ⟨_, rfl⟩⟩ := hmem w hw
    · rcases hkey v hv ha k hk with hk | hk
      · exact List.mem_append_right _ (h.admitted v hv k hk)
      · exact List.mem_append_left _ hk.1
    · exact List.mem_append_right _ (h.admitted w hv k hk)
  · intro w1 hw1 w2 hw2 k hk1 hk2
    obtain ⟨v1, hv1, ⟨ha1, rfl⟩ | ⟨hn1, rfl⟩⟩ := hmem w1 hw1 <;>
    obtain ⟨v2, hv2, ⟨ha2, rfl⟩ | ⟨hn2, rfl⟩⟩ := hmem w2 hw2
    · rw [hacc, hacc, ha1, ha2]
    · rw [hacc]
      rcases hkey v1 hv1 ha1 k hk1 with hk | hk
      · exact h.unique v1 hv1 _ hv2 k hk hk2
      · exact absurd (h.admitted _ hv2 k hk2) hk.2
    · rw [hacc]
      rcases hkey v2 hv2 ha2 k hk2 with hk | hk
      · exact h.unique _ hv1 v2 hv2 k hk1 hk
      · exact absurd (h.admitted _ hv1 k hk1) hk.2
    · exact h.unique _ hv1 _ hv2 k hk1 hk2

theorem KeysTracked.append {κ : Type} {vs : List Validator} {key : Validator → Option κ}
    {used : List κ} {v : Validator} (h : KeysTracked vs key used)
    (hfresh : ∀ k, key v = some k → k ∉ used) :
    KeysTracked (vs ++ [v]) key ((key v).toList ++ used) := by
  constructor
  · intro w hw k hk
    simp only [List.mem_append, List.mem_singleton] at hw
    rcases hw with hw | rfl
    · exact List.mem_append_right _ (h.admitted w hw k hk)
    · simp [hk]
  · intro w1 hw1 w2 hw2 k hk1 hk2
    simp only [List.mem_append, List.mem_singleton] at hw1 hw2
    rcases hw1 with hw1 | rfl <;> rcases hw2 with hw2 | rfl
    · exact h.unique _ hw1 _ hw2 k hk1 hk2
    · exact absurd (h.admitted _ hw1 k hk1) (hfresh k hk2)
    · exact absurd (h.admitted _ hw2 k hk2) (hfresh k hk1)
    · rfl

theorem KeysTracked.filter {κ : Type} {vs : List Validator} {key : Validator → Option κ}
    {used : List κ} (h : KeysTracked vs key used) (p : Validator → Bool) :
    KeysTracked (vs.filter p) key used where
  admitted v hv := h.admitted v (List.mem_filter.1 hv).1
  unique v hv w hw := h.unique v (List.mem_filter.1 hv).1 w (List.mem_filter.1 hw).1

/-- A validator just registered has its stake add up. -/
theorem Validator.sound_new (a : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) (commission : Nat)
    (authenticated : Bool) :
    (Validator.new a bls schnorr x25519 p2pAddr commission authenticated).Sound where
  delegatorsNodup := by simp [Validator.new]
  delegationsPositive := by simp [Validator.new]
  stakeTotal := by simp [Validator.new, total]
  stakeBound := by simp [Validator.new, U256]

/--
Changing one validator keeps the invariant if the change keeps its account, its
stake adds up after, and each key it gains is new and admitted with it.
-/
theorem Inv.update {s : State} {a : Address} {f : Validator → Validator}
    (bls : List BlsKey) (schnorr : List SchnorrKey) (x25519 : List X25519Key)
    (h : Inv s) (hacc : ∀ v, (f v).account = v.account)
    (hsound : ∀ v ∈ s.validators, v.account = a → (f v).Sound)
    (hbls : ∀ v ∈ s.validators, v.account = a → ∀ k, (f v).blsKey = some k →
      v.blsKey = some k ∨ (k ∈ bls ∧ k ∉ s.usedBls))
    (hschnorr : ∀ v ∈ s.validators, v.account = a → ∀ k, (f v).schnorrKey = some k →
      v.schnorrKey = some k ∨ (k ∈ schnorr ∧ k ∉ s.usedSchnorr))
    (hx25519 : ∀ v ∈ s.validators, v.account = a → ∀ k, (f v).x25519Key = some k →
      v.x25519Key = some k ∨ (k ∈ x25519 ∧ k ∉ s.usedX25519)) :
    Inv { s.update a f with
      usedBls := bls ++ s.usedBls
      usedSchnorr := schnorr ++ s.usedSchnorr
      usedX25519 := x25519 ++ s.usedX25519 } where
  accountsNodup := by
    show ((s.update a f).validators.map (·.account)).Nodup
    rw [accounts_update hacc]; exact h.accountsNodup
  notExited w hw := by
    obtain ⟨v, hv, ⟨_, rfl⟩ | ⟨_, rfl⟩⟩ := mem_update hw
    · rw [hacc]; exact h.notExited v hv
    · exact h.notExited _ hv
  sound w hw := by
    obtain ⟨v, hv, ⟨ha, rfl⟩ | ⟨_, rfl⟩⟩ := mem_update hw
    · exact hsound v hv ha
    · exact h.sound _ hv
  bls := h.bls.update hacc hbls
  schnorr := h.schnorr.update hacc hschnorr
  x25519 := h.x25519.update hacc hx25519

/-- `Inv.update` for a change that leaves the keys alone. -/
theorem Inv.update_keys {s : State} {a : Address} {f : Validator → Validator}
    (h : Inv s) (hacc : ∀ v, (f v).account = v.account)
    (hsound : ∀ v ∈ s.validators, v.account = a → (f v).Sound)
    (hbls : ∀ v, (f v).blsKey = v.blsKey) (hschnorr : ∀ v, (f v).schnorrKey = v.schnorrKey)
    (hx25519 : ∀ v, (f v).x25519Key = v.x25519Key) :
    Inv (s.update a f) :=
  h.update [] [] [] hacc hsound (fun v _ _ _ hk => .inl ((hbls v).symm.trans hk))
    (fun v _ _ _ hk => .inl ((hschnorr v).symm.trans hk))
    (fun v _ _ _ hk => .inl ((hx25519 v).symm.trans hk))

/--
Adding a validator keeps the invariant if its account is neither registered nor
has left, its stake adds up, and its keys are new.
-/
theorem Inv.admit {s : State} {v : Validator} (h : Inv s)
    (hreg : ¬ s.registered v.account = true) (hexit : v.account ∉ s.exits) (hsound : v.Sound)
    (hbls : ∀ k, v.blsKey = some k → k ∉ s.usedBls)
    (hschnorr : ∀ k, v.schnorrKey = some k → k ∉ s.usedSchnorr)
    (hx25519 : ∀ k, v.x25519Key = some k → k ∉ s.usedX25519) :
    Inv (s.admit v) where
  accountsNodup := by
    simp only [State.admit, List.map_append, List.map_cons, List.map_nil, List.nodup_append]
    refine ⟨h.accountsNodup, by simp, ?_⟩
    intro x hx y hy
    simp only [List.mem_singleton] at hy
    obtain ⟨w, hw, rfl⟩ := List.mem_map.1 hx
    exact hy ▸ not_registered hreg w hw
  notExited w hw := by
    simp only [State.admit, List.mem_append, List.mem_singleton] at hw
    rcases hw with hw | rfl
    · exact h.notExited w hw
    · exact hexit
  sound w hw := by
    simp only [State.admit, List.mem_append, List.mem_singleton] at hw
    rcases hw with hw | rfl
    · exact h.sound w hw
    · exact hsound
  bls := h.bls.append hbls
  schnorr := h.schnorr.append hschnorr
  x25519 := h.x25519.append hx25519

/-- Removing a validator keeps the invariant. -/
theorem Inv.remove {s : State} (a : Address) (h : Inv s) : Inv (s.remove a) where
  accountsNodup := h.accountsNodup.sublist (List.filter_sublist.map _)
  notExited w hw := by
    simp only [State.remove, List.mem_filter, bne_iff_ne, ne_eq] at hw
    simp only [State.remove, List.mem_cons, not_or]
    exact ⟨hw.2, h.notExited w hw.1⟩
  sound w hw := h.sound w (List.mem_filter.1 hw).1
  bls := h.bls.filter _
  schnorr := h.schnorr.filter _
  x25519 := h.x25519.filter _

end StakeTable
