module

public import StakeTableImpl.Node.Delegations

/-!
# Crediting and debiting delegations

Both keep each delegator once and every amount positive, and change the total by
exactly the amount.
-/

@[expose] public section

namespace StakeTable

theorem total_nil : total [] = 0 := rfl

theorem total_cons (e : Address × Nat) (es : List (Address × Nat)) :
    total (e :: es) = e.2 + total es := by
  simp [total]

theorem amountOf_le_total (d : Address) (ds : List (Address × Nat)) :
    amountOf d ds ≤ total ds := by
  induction ds with
  | nil => simp [amountOf, total_nil]
  | cons e es ih =>
    simp only [amountOf, total_cons]
    split <;> omega

theorem total_credit (d : Address) (amount : Nat) (ds : List (Address × Nat)) :
    total (credit d amount ds) = total ds + amount := by
  induction ds with
  | nil => simp [credit, total_cons, total_nil]
  | cons e es ih =>
    simp only [credit]
    split
    · simp only [total_cons]; omega
    · simp only [total_cons, ih]; omega

theorem mem_keys_credit {d x : Address} {amount : Nat} {ds : List (Address × Nat)}
    (h : x ∈ (credit d amount ds).map (·.1)) : x = d ∨ x ∈ ds.map (·.1) := by
  induction ds with
  | nil => simp [credit] at h; exact .inl h
  | cons e es ih =>
    simp only [credit] at h
    split at h
    · rename_i he
      simp only [List.map_cons, List.mem_cons] at h ⊢
      rcases h with h | h
      · exact .inl h
      · exact .inr (.inr h)
    · simp only [List.map_cons, List.mem_cons] at h ⊢
      rcases h with h | h
      · exact .inr (.inl h)
      · rcases ih h with h | h
        · exact .inl h
        · exact .inr (.inr h)

theorem nodup_credit (d : Address) (amount : Nat) {ds : List (Address × Nat)}
    (h : (ds.map (·.1)).Nodup) : ((credit d amount ds).map (·.1)).Nodup := by
  induction ds with
  | nil => simp [credit]
  | cons e es ih =>
    simp only [List.map_cons, List.nodup_cons] at h
    simp only [credit]
    split
    · rename_i he
      simp only [List.map_cons, List.nodup_cons]
      exact ⟨he ▸ h.1, h.2⟩
    · rename_i he
      simp only [List.map_cons, List.nodup_cons]
      refine ⟨fun hm => ?_, ih h.2⟩
      rcases mem_keys_credit hm with hm | hm
      · exact he hm
      · exact h.1 hm

theorem positive_credit (d : Address) {amount : Nat} {ds : List (Address × Nat)}
    (h : ∀ e ∈ ds, 0 < e.2) (ha : 0 < amount) : ∀ e ∈ credit d amount ds, 0 < e.2 := by
  induction ds with
  | nil => simp [credit]; exact ha
  | cons e es ih =>
    simp only [List.mem_cons, forall_eq_or_imp] at h
    simp only [credit]
    split
    · intro x hx
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · simp; omega
      · exact h.2 x hx
    · intro x hx
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact h.1
      · exact ih h.2 x hx

theorem total_debit (d : Address) {amount : Nat} {ds : List (Address × Nat)}
    (h : amount ≤ amountOf d ds) : total (debit d amount ds) = total ds - amount := by
  induction ds with
  | nil => simp [amountOf] at h; simp [debit, h]
  | cons e es ih =>
    simp only [amountOf] at h
    simp only [debit]
    split
    · rename_i he
      simp only [he, ↓reduceIte] at h
      split
      · simp only [total_cons]; omega
      · simp only [total_cons]; omega
    · rename_i he
      simp only [he, ↓reduceIte] at h
      have := amountOf_le_total d es
      simp only [total_cons, ih h]; omega

theorem keys_debit_sublist (d : Address) (amount : Nat) (ds : List (Address × Nat)) :
    ((debit d amount ds).map (·.1)).Sublist (ds.map (·.1)) := by
  induction ds with
  | nil => simp [debit]
  | cons e es ih =>
    simp only [debit]
    split
    · rename_i he
      split
      · simp only [List.map_cons]; exact .cons _ (.refl _)
      · simp only [List.map_cons, he]; exact .refl _
    · simp only [List.map_cons]; exact .cons_cons _ ih

theorem nodup_debit (d : Address) (amount : Nat) {ds : List (Address × Nat)}
    (h : (ds.map (·.1)).Nodup) : ((debit d amount ds).map (·.1)).Nodup :=
  h.sublist (keys_debit_sublist d amount ds)

theorem positive_debit (d : Address) (amount : Nat) {ds : List (Address × Nat)}
    (h : ∀ e ∈ ds, 0 < e.2) : ∀ e ∈ debit d amount ds, 0 < e.2 := by
  induction ds with
  | nil => simp [debit]
  | cons e es ih =>
    simp only [List.mem_cons, forall_eq_or_imp] at h
    simp only [debit]
    split
    · split
      · exact h.2
      · rename_i hz
        intro x hx
        simp only [List.mem_cons] at hx
        rcases hx with rfl | hx
        · simp; omega
        · exact h.2 x hx
    · intro x hx
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact h.1
      · exact ih h.2 x hx

end StakeTable
