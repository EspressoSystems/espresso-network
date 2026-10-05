module

public import Std.Data.TreeMap
public import Std.Data.TreeSet
public import NewProtocolImpl.Order

/-!
# The tables

The three facts the machine's tables are ever used through — what a lookup
returns after an insertion, an erasure or a filter — plus the two that relate a
table to the list of keys the reaction pass scans.

Everything below the machine speaks `get?` and `∈`; nothing outside this file
unfolds either into the standard library's `getElem?` and `compare`. Keeping
that boundary is what makes the rest of the development independent of the
representation: swapping `TreeMap` for another finite map means reproving this
file and nothing else.
-/

public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {α β : Type} [Ord α] [DecidableEq α] [Std.TransOrd α] [Std.LawfulEqOrd α]

/-- Comparison decides equality, on any key type the tables use. -/
theorem compare_eq_iff {a b : α} : compare a b = .eq ↔ a = b :=
  ⟨Std.LawfulEqCmp.eq_of_compare, fun h => h ▸ Std.ReflCmp.compare_self⟩

/-! ## Maps -/

section Maps

variable {t : TreeMap α β} {k a : α} {x y : β}

theorem get?_insert : (t.insert k x).get? a = if a = k then some x else t.get? a := by
  rw [Std.TreeMap.get?_eq_getElem?, Std.TreeMap.getElem?_insert, Std.TreeMap.get?_eq_getElem?]
  by_cases h : a = k
  · rw [if_pos h, if_pos (compare_eq_iff.mpr h.symm)]
  · rw [if_neg h, if_neg (fun he => h (compare_eq_iff.mp he).symm)]

@[simp] theorem get?_insert_self : (t.insert k x).get? k = some x := by
  rw [get?_insert, if_pos rfl]

/-- A lookup that succeeds after an insertion found the value inserted, or one already there. -/
theorem get?_insert_cases (h : (t.insert k x).get? a = some y) :
    (a = k ∧ y = x) ∨ t.get? a = some y := by
  rw [get?_insert] at h
  split at h
  · exact Or.inl ⟨‹a = k›, by simpa using h.symm⟩
  · exact Or.inr h

/-- An insertion elsewhere disturbs nothing. -/
theorem get?_insert_of_ne (hne : a ≠ k) (h : t.get? a = some y) :
    (t.insert k x).get? a = some y := by
  rw [get?_insert, if_neg hne]; exact h

/--
An insertion into a slot that was free or already held the same value disturbs
nothing anywhere: the slot discipline of `Impl.handle`, as the retention
obligation needs it.
-/
theorem get?_insert_of_writable (hw : t.get? k = none ∨ t.get? k = some x)
    (h : t.get? a = some y) : (t.insert k x).get? a = some y := by
  rw [get?_insert]
  split
  · rename_i he
    subst he
    rcases hw with hn | hs
    · exact absurd (hn.symm.trans h) (by simp)
    · exact hs.symm.trans h
  · exact h

theorem get?_erase : (t.erase k).get? a = if a = k then none else t.get? a := by
  rw [Std.TreeMap.get?_eq_getElem?, Std.TreeMap.getElem?_erase, Std.TreeMap.get?_eq_getElem?]
  by_cases h : a = k
  · rw [if_pos h, if_pos (compare_eq_iff.mpr h.symm)]
  · rw [if_neg h, if_neg (fun he => h (compare_eq_iff.mp he).symm)]

/-- A lookup that succeeds after an erasure found a value that was already there. -/
theorem get?_of_get?_erase (h : (t.erase k).get? a = some y) : t.get? a = some y := by
  rw [get?_erase] at h
  split at h
  · exact absurd h (by simp)
  · exact h

/-- A filtered map holds exactly the entries that were there and pass the filter. -/
theorem get?_filter {f : α → β → Bool} :
    (t.filter f).get? a = some x ↔ t.get? a = some x ∧ f a x := by
  rw [Std.TreeMap.get?_eq_getElem?, Std.TreeMap.getElem?_filter', Option.filter_eq_some_iff,
    ← Std.TreeMap.get?_eq_getElem?]

theorem contains_eq_isSome : t.contains k = (t.get? k).isSome := by
  rw [Std.TreeMap.contains_eq_isSome_getElem?, Std.TreeMap.get?_eq_getElem?]

theorem exists_get?_of_contains (h : t.contains k = true) : ∃ y, t.get? k = some y :=
  Option.isSome_iff_exists.mp (by rw [← contains_eq_isSome]; exact h)

theorem contains_of_get? (h : t.get? k = some x) : t.contains k = true :=
  contains_eq_isSome.trans (by rw [h]; rfl)

/-- What the reaction pass scans: exactly the views the map has an entry at. -/
theorem mem_keys : k ∈ t.keys ↔ (t.get? k).isSome := by
  rw [Std.TreeMap.mem_keys, Std.TreeMap.mem_iff_isSome_getElem?, Std.TreeMap.get?_eq_getElem?]

theorem mem_keys_of_get? (h : t.get? k = some x) : k ∈ t.keys :=
  mem_keys.mpr (h ▸ rfl)

theorem mem_toList_of_get? (h : t.get? k = some x) : (k, x) ∈ t.toList :=
  Std.TreeMap.mem_toList_iff_getElem?_eq_some.mpr (Std.TreeMap.get?_eq_getElem? ▸ h)

theorem get?_of_mem_toList (h : (k, x) ∈ t.toList) : t.get? k = some x :=
  Std.TreeMap.get?_eq_getElem? ▸ Std.TreeMap.mem_toList_iff_getElem?_eq_some.mp h

/-- `TreeMap.empty` is the `EmptyCollection` the standard lemmas speak of. -/
theorem empty_eq_emptyc {cmp : α → α → Ordering} :
    (TreeMap.empty : TreeMap α β cmp) = ∅ := rfl

@[simp] theorem get?_empty : (TreeMap.empty : TreeMap α β).get? k = none := by
  rw [empty_eq_emptyc, Std.TreeMap.get?_eq_getElem?, Std.TreeMap.getElem?_emptyc]

end Maps

/-! ## Sets -/

section Sets

variable {t : TreeSet α} {k a : α}

theorem mem_insert : a ∈ t.insert k ↔ a = k ∨ a ∈ t := by
  rw [Std.TreeSet.mem_insert]
  exact or_congr_left ⟨fun h => (compare_eq_iff.mp h).symm, fun h => compare_eq_iff.mpr h.symm⟩

theorem mem_insert_of_mem (h : a ∈ t) : a ∈ t.insert k := mem_insert.mpr (Or.inr h)

@[simp] theorem mem_insert_self : k ∈ t.insert k := mem_insert.mpr (Or.inl rfl)

theorem mem_filter {f : α → Bool} : k ∈ t.filter f ↔ k ∈ t ∧ f k := by
  rw [Std.TreeSet.mem_filter]
  refine ⟨fun ⟨h, hf⟩ => ⟨h, ?_⟩, fun ⟨h, hf⟩ => ⟨h, ?_⟩⟩ <;>
    rwa [Std.TreeSet.get_eq] at *

@[simp] theorem not_mem_empty : k ∉ (TreeSet.empty : TreeSet α) := by
  show k ∉ (∅ : TreeSet α); simp

theorem contains_iff_mem : t.contains k = true ↔ k ∈ t := (Std.TreeSet.mem_iff_contains).symm

end Sets

/-! ## Folding a disjunction

A fold of `||` is a search, which is how a scan written as a fold over a tree is
read back as a question about its entries.
-/

theorem foldl_or_eq_any {γ : Type} (p : γ → Bool) :
    ∀ (l : List γ) (b : Bool), l.foldl (fun found x => found || p x) b = (b || l.any p) := by
  intro l
  induction l with
  | nil => intro b; simp
  | cons a l ih => intro b; simp [ih, Bool.or_assoc]

end Impl
end NewProtocol
