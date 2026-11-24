module

public import Init.Data.List.Perm

/-!
# Facts about lists

General facts the proofs use, about nothing in the stake table: sorted lists, lists
without duplicates, sublists, sums, and counting below a bound.
-/

@[expose] public section

namespace StakeTable

/--
Two lists with the same elements, each sorted by an asymmetric relation, are equal.

This is what makes an order stated by a strict comparison determine the list: the
event order and the schedule order.
-/
theorem eq_of_perm_of_pairwise {α : Type} {R : α → α → Prop}
    (asymm : ∀ a b, R a b → R b a → False) :
    ∀ {l₁ l₂ : List α}, l₁.Perm l₂ → l₁.Pairwise R → l₂.Pairwise R → l₁ = l₂
  | [], l₂, hp, _, _ => (List.Perm.nil_eq hp)
  | a :: t, [], hp, _, _ => absurd hp.length_eq (by simp)
  | a :: t, b :: t₂, hp, h₁, h₂ => by
    rw [List.pairwise_cons] at h₁ h₂
    by_cases hab : a = b
    · subst hab
      rw [eq_of_perm_of_pairwise asymm ((List.perm_cons a).1 hp) h₁.2 h₂.2]
    · have ha : a ∈ t₂ := by
        have := hp.subset (List.mem_cons_self ..)
        rcases List.mem_cons.1 this with h | h
        · exact absurd h hab
        · exact h
      have hb : b ∈ t := by
        have := hp.symm.subset (List.mem_cons_self ..)
        rcases List.mem_cons.1 this with h | h
        · exact absurd h.symm hab
        · exact h
      exact (asymm a b (h₁.1 b hb) (h₂.1 a ha)).elim

/-- With distinct keys, a member of `l` is fixed by its key. -/
theorem eq_of_key_eq {α β : Type} {f : α → β} {l : List α} (hk : (l.map f).Nodup) {p q : α}
    (hp : p ∈ l) (hq : q ∈ l) (h : f p = f q) : p = q := by
  induction l with
  | nil => simp at hp
  | cons t l ih =>
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hk
    simp only [List.mem_cons] at hp hq
    rcases hp with rfl | hp <;> rcases hq with rfl | hq
    · rfl
    · exact absurd ⟨q, hq, h.symm⟩ hk.1
    · exact absurd ⟨p, hp, h⟩ hk.1
    · exact ih hk.2 hp hq

theorem nodup_of_nodup_map {α β : Type} {f : α → β} {l : List α} (h : (l.map f).Nodup) :
    l.Nodup := by
  induction l with
  | nil => exact List.nodup_nil
  | cons a l ih =>
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at h
    exact List.nodup_cons.2 ⟨fun ha => h.1 ⟨a, ha, rfl⟩, ih h.2⟩

theorem nodup_map_of_injOn {α β : Type} {f : α → β} {l : List α}
    (hinj : ∀ a ∈ l, ∀ b ∈ l, f a = f b → a = b) (h : l.Nodup) : (l.map f).Nodup := by
  induction l with
  | nil => exact List.nodup_nil
  | cons a l ih =>
    rw [List.nodup_cons] at h
    simp only [List.map_cons, List.nodup_cons, List.mem_map]
    refine ⟨fun ⟨b, hb, hab⟩ => ?_, ih (fun x hx y hy => hinj x (.tail _ hx) y (.tail _ hy)) h.2⟩
    exact h.1 (hinj a (.head _) b (.tail _ hb) hab.symm ▸ hb)

/-- Two sublists of a list without duplicates that have the same members are equal. -/
theorem sublist_eq_of_mem_iff {α : Type} :
    ∀ {l l₁ l₂ : List α}, l₁.Sublist l → l₂.Sublist l → l.Nodup → (∀ x, x ∈ l₁ ↔ x ∈ l₂) →
      l₁ = l₂
  | [], l₁, l₂, h₁, h₂, _, _ => by
    rw [List.sublist_nil.1 h₁, List.sublist_nil.1 h₂]
  | a :: t, l₁, l₂, h₁, h₂, hn, hm => by
    rw [List.nodup_cons] at hn
    rw [List.sublist_cons_iff] at h₁ h₂
    rcases h₁ with h₁ | ⟨r₁, rfl, h₁⟩ <;> rcases h₂ with h₂ | ⟨r₂, rfl, h₂⟩
    · exact sublist_eq_of_mem_iff h₁ h₂ hn.2 hm
    · exact absurd (h₁.subset ((hm a).2 (List.mem_cons_self ..))) hn.1
    · exact absurd (h₂.subset ((hm a).1 (List.mem_cons_self ..))) hn.1
    · have hr : ∀ x, x ∈ r₁ ↔ x ∈ r₂ := by
        intro x
        have := hm x
        by_cases hx : x = a
        · subst hx
          exact ⟨fun h => absurd (h₁.subset h) hn.1, fun h => absurd (h₂.subset h) hn.1⟩
        · simpa [hx] using this
      rw [sublist_eq_of_mem_iff h₁ h₂ hn.2 hr]

/--
A list without duplicates adds up to no more than a list that holds each of its
members where `f` is not zero.
-/
theorem sum_map_le_of_subset {α : Type} (f : α → Nat) :
    ∀ {l₁ l₂ : List α}, l₁.Nodup → (∀ x ∈ l₁, f x ≠ 0 → x ∈ l₂) →
      (l₁.map f).sum ≤ (l₂.map f).sum
  | [], _, _, _ => by simp
  | a :: t, l₂, hn, hs => by
    rw [List.nodup_cons] at hn
    by_cases ha : f a = 0
    · have := sum_map_le_of_subset f hn.2 fun x hx => hs x (List.mem_cons_of_mem _ hx)
      simpa [ha] using this
    · obtain ⟨s, u, rfl⟩ := List.append_of_mem (hs a (List.mem_cons_self ..) ha)
      have ht : ∀ x ∈ t, f x ≠ 0 → x ∈ s ++ u := fun x hx hfx => by
        have := hs x (List.mem_cons_of_mem _ hx) hfx
        simp only [List.mem_append, List.mem_cons] at this ⊢
        rcases this with h | rfl | h
        · exact .inl h
        · exact absurd hx hn.1
        · exact .inr h
      have := sum_map_le_of_subset f hn.2 ht
      simp only [List.map_append, List.map_cons, List.sum_append, List.sum_cons] at this ⊢
      omega

/-- A list of distinct numbers each below its length is `0, 1, …` in some order. -/
theorem perm_range_of_nodup {l : List Nat} (hnd : l.Nodup) (hlt : ∀ x ∈ l, x < l.length) :
    l.Perm (List.range l.length) := by
  refine (List.perm_ext_iff_of_nodup hnd List.nodup_range).2 fun x => ⟨?_, ?_⟩
  · intro hx; exact List.mem_range.2 (hlt x hx)
  · intro hx
    rw [List.mem_range] at hx
    refine Classical.byContradiction fun hn => ?_
    have hsub : x :: l ⊆ List.range l.length := by
      intro y hy
      simp only [List.mem_cons] at hy
      rcases hy with rfl | hy
      · exact List.mem_range.2 hx
      · exact List.mem_range.2 (hlt y hy)
    have := List.Nodup.length_le_of_subset (List.nodup_cons.2 ⟨hn, hnd⟩) hsub
    simp only [List.length_cons, List.length_range] at this
    omega

/-- Of the numbers below `m`, `min n m` are below `n`. -/
theorem length_filter_range_lt (n m : Nat) :
    ((List.range m).filter fun x => decide (x < n)).length = min n m := by
  induction m with
  | zero => simp
  | succ m ih =>
    rw [List.range_succ, List.filter_append, List.length_append, ih]
    by_cases h : m < n
    · simp [h]; omega
    · simp [h]; omega

/-- Of the numbers below `T`, as many lie in `[lo, lo + s)` as the two ranges share. -/
theorem length_filter_range_interval (lo s T : Nat) :
    ((List.range T).filter fun b => decide (lo ≤ b ∧ b < lo + s)).length
      = min T (lo + s) - min T lo := by
  induction T with
  | zero => simp
  | succ T ih =>
    rw [List.range_succ, List.filter_append, List.length_append, ih]
    by_cases h : lo ≤ T ∧ T < lo + s
    · simp only [h, and_self, decide_true, List.filter_cons_of_pos, List.filter_nil,
        List.length_cons, List.length_nil]
      omega
    · simp only [List.filter_cons, List.filter_nil]
      have h' : ¬ (lo ≤ T ∧ T < lo + s) := h
      simp only [h', decide_false, Bool.false_eq_true, ite_false, List.length_nil]
      omega

end StakeTable
