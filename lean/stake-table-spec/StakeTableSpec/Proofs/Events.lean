module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Lists

/-!
# What the event order gives

The L1 order of a set of logs is unique (`StakeTable.inL1Order_unique`), so the
node's replay does not depend on how the logs were read; logs with distinct keys have
one, sorted by key (`StakeTable.inL1Order_exists`). In particular a read that
resumes from kept events gives the order of all the logs, as long as the kept events
are those of the blocks up to some block, in order, and the fetched ones those of
the blocks after it, in order (`StakeTable.resume_inL1Order`).

The logs of a range of blocks are the logs of each of its blocks, one block after
another (`StakeTable.blocks_logs_perm`), so a read in chunks that cover the range
block by block gives the logs of the range (`StakeTable.chunked_reads_all`).
-/

@[expose] public section

namespace StakeTable

theorem LogKey.before_asymm {a b : LogKey} (h : a.before b = true) (h' : b.before a = true) :
    False := by
  simp only [LogKey.before, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h h'
  omega

theorem LogKey.before_irrefl (a : LogKey) : a.before a = false := by
  simp [LogKey.before]

/-- The L1 order of a set of logs is unique. -/
theorem inL1Order_unique {α : Type} {logs o o' : List (LogKey × α)} (h : InL1Order logs o)
    (h' : InL1Order logs o') : o = o' :=
  eq_of_perm_of_pairwise (fun _ _ hab hba => LogKey.before_asymm hab hba) (h.1.trans h'.1.symm)
    h.2 h'.2

/-- `a` comes no later than `b` on L1. -/
def LogKey.le (a b : LogKey) : Bool := !b.before a

/-- Logs with distinct keys, sorted by key, are in L1 order. -/
theorem inL1Order_mergeSort {α : Type} {logs : List (LogKey × α)} (hk : (logs.map (·.1)).Nodup) :
    InL1Order logs (logs.mergeSort fun a b => a.1.le b.1) := by
  have hperm : (logs.mergeSort fun a b => a.1.le b.1).Perm logs := List.mergeSort_perm ..
  have hle : (logs.mergeSort fun a b => a.1.le b.1).Pairwise fun a b => a.1.le b.1 = true :=
    List.pairwise_mergeSort
      (fun a b c hab hbc => by
        simp only [LogKey.le, LogKey.before, Bool.not_eq_true', Bool.or_eq_false_iff,
          Bool.and_eq_false_imp, decide_eq_false_iff_not, decide_eq_true_eq] at hab hbc ⊢
        omega)
      (fun a b => by
        simp only [LogKey.le, LogKey.before, Bool.or_eq_true, Bool.not_eq_true',
          Bool.or_eq_false_iff, Bool.and_eq_false_imp, decide_eq_false_iff_not,
          decide_eq_true_eq]
        omega)
      logs
  have hne : (logs.mergeSort fun a b => a.1.le b.1).Pairwise fun a b => a.1 ≠ b.1 :=
    List.pairwise_map.1 ((hperm.map Prod.fst).nodup_iff.2 hk)
  refine ⟨hperm, (hle.and hne).imp fun {a b} ⟨h1, h2⟩ => ?_⟩
  simp only [LogKey.le, LogKey.before, Bool.not_eq_true', Bool.or_eq_false_iff,
    Bool.and_eq_false_imp, decide_eq_false_iff_not, decide_eq_true_eq, Bool.or_eq_true,
    Bool.and_eq_true] at h1 ⊢
  have : a.1.1 ≠ b.1.1 ∨ a.1.2 ≠ b.1.2 := by
    by_cases h : a.1.1 = b.1.1
    · exact .inr fun h' => h2 (Prod.ext h h')
    · exact .inl h
  omega

/-- Logs with distinct keys have an L1 order. -/
theorem inL1Order_exists {α : Type} {logs : List (LogKey × α)} (hk : (logs.map (·.1)).Nodup) :
    ∃ o, InL1Order logs o :=
  ⟨_, inL1Order_mergeSort hk⟩

theorem dedupAdjacent_of_pairwise_ne {α : Type} [DecidableEq α] :
    ∀ {l : List α}, l.Pairwise (· ≠ ·) → dedupAdjacent l = l
  | [], _ => rfl
  | [_], _ => rfl
  | a :: b :: l, h => by
    have hab : a ≠ b := (List.pairwise_cons.1 h).1 b (List.mem_cons_self ..)
    simp only [dedupAdjacent, hab, ite_false]
    rw [dedupAdjacent_of_pairwise_ne (List.pairwise_cons.1 h).2]

/--
Resuming from kept events gives the L1 order of all the logs, when the kept events
are the logs up to block `B` in order and the fetched ones the logs after it.
-/
theorem resume_inL1Order {α : Type} [DecidableEq α] {logs kept fetched : List (LogKey × α)}
    {B : Nat} (hk : InL1Order (logs.filter fun l => decide (l.1.1 ≤ B)) kept)
    (hf : InL1Order (logs.filter fun l => decide (B < l.1.1)) fetched) :
    InL1Order logs (resume kept fetched) := by
  have hperm : (kept ++ fetched).Perm logs := by
    refine (hk.1.append hf.1).trans ?_
    have hc : (logs.filter fun l => decide (B < l.1.1))
        = logs.filter fun l => !decide (l.1.1 ≤ B) := by
      apply List.filter_congr
      intro x _
      by_cases h : B < x.1.1 <;> simp [h] <;> omega
    rw [hc]
    exact List.filter_append_perm _ _
  have hpair : (kept ++ fetched).Pairwise fun a b => a.1.before b.1 = true := by
    rw [List.pairwise_append]
    refine ⟨hk.2, hf.2, fun a ha b hb => ?_⟩
    have ha' := List.mem_filter.1 (hk.1.subset ha)
    have hb' := List.mem_filter.1 (hf.1.subset hb)
    simp only [decide_eq_true_eq] at ha' hb'
    simp only [LogKey.before, Bool.or_eq_true, decide_eq_true_eq]
    exact .inl (Nat.lt_of_le_of_lt ha'.2 hb'.2)
  have hne : (kept ++ fetched).Pairwise (· ≠ ·) :=
    hpair.imp fun {a b} h hab => by subst hab; simp [LogKey.before_irrefl] at h
  unfold resume
  rw [dedupAdjacent_of_pairwise_ne hne]
  exact ⟨hperm, hpair⟩

/--
The logs of the blocks `lo` to `hi` are the logs of each of these blocks, one block
after another.
-/
theorem blocks_logs_perm {α : Type} (logs : List (LogKey × α)) (lo hi : Nat) :
    ((List.range' lo (hi + 1 - lo)).flatMap fun b => logs.filter fun l => decide (l.1.1 = b)).Perm
      (logs.filter fun l => decide (lo ≤ l.1.1 ∧ l.1.1 ≤ hi)) := by
  induction hn : hi + 1 - lo generalizing lo with
  | zero =>
    refine .of_eq (Eq.symm (List.filter_eq_nil_iff.2 fun l _ => ?_))
    simp only [decide_eq_true_eq]
    omega
  | succ n ih =>
    rw [List.range'_succ, List.flatMap_cons]
    refine ((ih (lo + 1) (by omega)).append_left _).trans ?_
    -- The logs of the range are those of block `lo` and those of the blocks after it.
    have hs := List.filter_append_perm (fun l : LogKey × α => decide (l.1.1 = lo))
      (logs.filter fun l => decide (lo ≤ l.1.1 ∧ l.1.1 ≤ hi))
    rw [List.filter_filter, List.filter_filter] at hs
    refine (List.Perm.of_eq ?_).trans hs
    congr 1 <;> apply List.filter_congr <;> intro l _ <;> rw [Bool.eq_iff_iff] <;>
      simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq,
        decide_eq_false_iff_not] <;> omega

/-- A read in chunks loses no log and repeats none. -/
theorem chunked_reads_all {α : Type} (logs : List (LogKey × α)) {first last size : Nat}
    {chunks : List (Nat × Nat)} (hc : Chunked first last size chunks) :
    (chunks.flatMap fun c => logs.filter fun l => decide (c.1 ≤ l.1.1 ∧ l.1.1 ≤ c.2)).Perm
      (logs.filter fun l => decide (first ≤ l.1.1 ∧ l.1.1 ≤ last)) := by
  have hb := blocks_logs_perm logs first last
  rw [← hc.1, List.flatMap_assoc] at hb
  refine .trans ?_ hb
  clear hb hc
  induction chunks with
  | nil => exact .nil
  | cons c cs ih =>
    rw [List.flatMap_cons, List.flatMap_cons]
    exact (blocks_logs_perm logs c.1 c.2).symm.append ih

end StakeTable
