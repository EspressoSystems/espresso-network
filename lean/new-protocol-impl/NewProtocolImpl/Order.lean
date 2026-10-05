module

public import Std.Data.TreeSet
public import NewProtocolSpec.Base
public import NewProtocolSpec.Types

/-!
# Order lawfulness

`TransOrd` and `LawfulEqOrd` instances for the tree key types, unlocking the
standard library's `TreeMap`/`TreeSet` lemmas.

Each key type is a one-field wrapper whose `Ord` is derived, so its `compare`
is not syntactically the `Nat` one; a transfer lemma relates the two and the
laws are then inherited. Products are covered by the core instances for
`lexOrd`.
-/

public section

open Std

namespace NewProtocol

/-! ## View numbers are natural numbers

Order and arithmetic on `ViewNumber` are those of the `Nat` it wraps. The
lemmas below push a view-number statement down to that payload, where `omega`
settles it; the development uses them rather than reproving the `Nat` order
facts on the wrapper.
-/

namespace ViewNumber

theorem ext {a b : ViewNumber} (h : a.toNat = b.toNat) : a = b := by
  obtain ⟨a⟩ := a; obtain ⟨b⟩ := b; exact congrArg ViewNumber.mk h

theorem eq_iff {a b : ViewNumber} : a = b ↔ a.toNat = b.toNat :=
  ⟨fun h => h ▸ rfl, ext⟩

theorem le_def {a b : ViewNumber} : a ≤ b ↔ a.toNat ≤ b.toNat := Iff.rfl

theorem lt_def {a b : ViewNumber} : a < b ↔ a.toNat < b.toNat := Iff.rfl

@[simp] theorem toNat_add (a : ViewNumber) (n : Nat) : (a + n).toNat = a.toNat + n := rfl

@[simp] theorem toNat_sub (a : ViewNumber) (n : Nat) : (a - n).toNat = a.toNat - n := rfl

@[simp] theorem toNat_max (a b : ViewNumber) : (max a b).toNat = max a.toNat b.toNat := by
  by_cases h : a.toNat ≤ b.toNat
  · rw [show max a b = b from if_pos h, Nat.max_def, if_pos h]
  · rw [show max a b = a from if_neg h, Nat.max_def, if_neg h]

@[simp] theorem toNat_min (a b : ViewNumber) : (min a b).toNat = min a.toNat b.toNat := by
  by_cases h : a.toNat ≤ b.toNat
  · rw [show min a b = a from if_pos h, Nat.min_def, if_pos h]
  · rw [show min a b = b from if_neg h, Nat.min_def, if_neg h]

@[simp] theorem toNat_genesis : ViewNumber.genesis.toNat = 0 := rfl

theorem le_max_left (a b : ViewNumber) : a ≤ max a b := by rw [le_def]; simp; omega

theorem le_max_right (a b : ViewNumber) : b ≤ max a b := by rw [le_def]; simp; omega

theorem min_le_left (a b : ViewNumber) : min a b ≤ a := by rw [le_def]; simp; omega

theorem min_le_right (a b : ViewNumber) : min a b ≤ b := by rw [le_def]; simp; omega

/-- A maximum is one of the two, which is how a cursor's new value is traced back. -/
theorem max_eq_or (a b : ViewNumber) : max a b = a ∨ max a b = b := by
  by_cases h : a.toNat ≤ b.toNat
  · exact Or.inr (if_pos h)
  · exact Or.inl (if_neg h)

theorem max_eq_right_of_ne {a b : ViewNumber} (h : max a b ≠ a) : max a b = b :=
  (max_eq_or a b).resolve_left h

theorem max_eq_right_of_le {a b : ViewNumber} (h : a ≤ b) : max a b = b := if_pos h

end ViewNumber

/-- The derived comparison on `ViewNumber` is the one on its `Nat` payload. -/
theorem compare_viewNumber (a b : ViewNumber) : compare a b = compare a.toNat b.toNat := by
  obtain ⟨a⟩ := a; obtain ⟨b⟩ := b; simp [compare, instOrdViewNumber.ord]

instance : TransOrd ViewNumber where
  eq_swap {a b} := by
    rw [compare_viewNumber, compare_viewNumber]
    exact OrientedCmp.eq_swap (cmp := (compare : Nat → Nat → Ordering))
  isLE_trans {a b c} h₁ h₂ := by
    rw [compare_viewNumber] at h₁ h₂ ⊢
    exact TransCmp.isLE_trans (cmp := (compare : Nat → Nat → Ordering)) h₁ h₂

instance : LawfulEqOrd ViewNumber where
  compare_self {a} := by
    rw [compare_viewNumber]
    exact ReflCmp.compare_self (cmp := (compare : Nat → Nat → Ordering))
  eq_of_compare {a b} h := by
    rw [compare_viewNumber] at h
    obtain ⟨a⟩ := a; obtain ⟨b⟩ := b
    exact congrArg ViewNumber.mk
      (LawfulEqCmp.eq_of_compare (cmp := (compare : Nat → Nat → Ordering)) h)

/-- The derived comparison on `BlockHash` is the one on its `Nat` payload. -/
theorem compare_blockHash (a b : BlockHash) : compare a b = compare a.toNat b.toNat := by
  obtain ⟨a⟩ := a; obtain ⟨b⟩ := b; simp [compare, instOrdBlockHash.ord]

instance : TransOrd BlockHash where
  eq_swap {a b} := by
    rw [compare_blockHash, compare_blockHash]
    exact OrientedCmp.eq_swap (cmp := (compare : Nat → Nat → Ordering))
  isLE_trans {a b c} h₁ h₂ := by
    rw [compare_blockHash] at h₁ h₂ ⊢
    exact TransCmp.isLE_trans (cmp := (compare : Nat → Nat → Ordering)) h₁ h₂

instance : LawfulEqOrd BlockHash where
  compare_self {a} := by
    rw [compare_blockHash]
    exact ReflCmp.compare_self (cmp := (compare : Nat → Nat → Ordering))
  eq_of_compare {a b} h := by
    rw [compare_blockHash] at h
    obtain ⟨a⟩ := a; obtain ⟨b⟩ := b
    exact congrArg BlockHash.mk
      (LawfulEqCmp.eq_of_compare (cmp := (compare : Nat → Nat → Ordering)) h)

/-- The derived comparison on `PayloadCommit` is the one on its `Nat` payload. -/
theorem compare_payloadCommit (a b : PayloadCommit) : compare a b = compare a.toNat b.toNat := by
  obtain ⟨a⟩ := a; obtain ⟨b⟩ := b; simp [compare, instOrdPayloadCommit.ord]

instance : TransOrd PayloadCommit where
  eq_swap {a b} := by
    rw [compare_payloadCommit, compare_payloadCommit]
    exact OrientedCmp.eq_swap (cmp := (compare : Nat → Nat → Ordering))
  isLE_trans {a b c} h₁ h₂ := by
    rw [compare_payloadCommit] at h₁ h₂ ⊢
    exact TransCmp.isLE_trans (cmp := (compare : Nat → Nat → Ordering)) h₁ h₂

instance : LawfulEqOrd PayloadCommit where
  compare_self {a} := by
    rw [compare_payloadCommit]
    exact ReflCmp.compare_self (cmp := (compare : Nat → Nat → Ordering))
  eq_of_compare {a b} h := by
    rw [compare_payloadCommit] at h
    obtain ⟨a⟩ := a; obtain ⟨b⟩ := b
    exact congrArg PayloadCommit.mk
      (LawfulEqCmp.eq_of_compare (cmp := (compare : Nat → Nat → Ordering)) h)

end NewProtocol
