module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Events
public import StakeTableSpec.Proofs.Select
public import StakeTableSpec.Proofs.Table

/-!
# What reading events gives

Events make a valid history (`StakeTable.reads_valid`), at most one
(`StakeTable.reads_unique`): each event either adds the change it comes to or, if the
node passes over it, adds nothing, and an event the node passes over comes to no
change the table takes (`StakeTable.passes_not_taken`). So the logs on L1 up to a
block fix the committee read from them (`StakeTable.committeeAt_unique`).
-/

@[expose] public section

namespace StakeTable

/-- Events make a valid history. -/
theorem reads_valid {es : List Event} {h : History} (hr : Reads es h) : Valid h := by
  induction hr with
  | nil => exact .nil
  | take _ _ hc ih => exact .snoc ih hc
  | drop _ _ ih => exact ih

/-- An event the node passes over comes to no change the table takes. -/
theorem passes_not_taken {h : History} {e : Event} {c : Change} (hp : Passes h e)
    (hc : changeOf h e = some c) : ¬ Accepts h c := by
  cases e with
  | register a bls schnorr com =>
    cases schnorr with
    | none => exact hp.elim
    | some k =>
      simp only [changeOf, Option.some.injEq] at hc
      subst hc
      exact fun hA => hA.2.2.1 k rfl hp.1
  | keyUpdate a bls schnorr =>
    cases bls <;> cases schnorr <;> simp only [changeOf, reduceCtorEq] at hc
    simp only [Option.some.injEq] at hc
    subst hc
    exact fun hA => hA.2.2 hp.1
  | keyUpdateV2 a keys =>
    cases keys <;> simp only [changeOf, reduceCtorEq] at hc
    simp only [Option.some.injEq] at hc
    subst hc
    exact fun hA => hA.2.2 hp.1
  | _ => exact hp.elim

theorem eq_of_append_singleton {es es' : List Event} {e e' : Event}
    (h : es ++ [e] = es' ++ [e']) : es = es' ∧ e = e' := by
  have := List.append_inj' h rfl
  exact ⟨this.1, List.singleton_inj.1 this.2⟩

/-- Events make at most one history. -/
theorem reads_unique {es : List Event} {h h' : History} (hr : Reads es h) (hr' : Reads es h') :
    h = h' := by
  induction hr generalizing h' with
  | nil =>
    generalize hx : ([] : List Event) = x at hr'
    cases hr' with
    | nil => rfl
    | take => simp at hx
    | drop => simp at hx
  | take hr hc ha ih =>
    rename_i es h e c
    generalize hx : es ++ [e] = x at hr'
    cases hr' with
    | nil => simp at hx
    | take hr' hc' ha' =>
      obtain ⟨rfl, rfl⟩ := eq_of_append_singleton hx
      cases ih hr'
      rw [hc] at hc'
      cases hc'
      rfl
    | drop hr' hn =>
      obtain ⟨rfl, rfl⟩ := eq_of_append_singleton hx
      cases ih hr'
      exact absurd ha (passes_not_taken hn hc)
  | drop hr hn ih =>
    rename_i es h e
    generalize hx : es ++ [e] = x at hr'
    cases hr' with
    | nil => simp at hx
    | take hr' hc' ha' =>
      obtain ⟨rfl, rfl⟩ := eq_of_append_singleton hx
      cases ih hr'
      exact absurd ha' (passes_not_taken hn hc')
    | drop hr' _ =>
      obtain ⟨rfl, rfl⟩ := eq_of_append_singleton hx
      exact ih hr'

/-- The logs on L1 up to a block fix the committee read from them. -/
theorem committeeAt_unique {usable : NetAddr → Bool}
    {logs : List (LogKey × Event)} {B : Nat} {r r' : Option (List Member)}
    (h : CommitteeAt usable logs B r) (h' : CommitteeAt usable logs B r') :
    r = r' := by
  obtain ⟨out, hist, ho, hr, ha⟩ := h
  obtain ⟨out', hist', ho', hr', ha'⟩ := h'
  cases inL1Order_unique ho ho'
  cases reads_unique hr hr'
  exact isActiveSet_unique (table_distinct (reads_valid hr)).accounts ha ha'

end StakeTable
