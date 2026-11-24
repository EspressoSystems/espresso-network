module

public import StakeTableSpec

/-!
# Ordering logs, and splitting a read into chunks

`StakeTable.orderLogs` sorts logs by their key with `mergeSort`, which puts logs with
distinct keys in L1 order (`StakeTable.orderLogs_inL1Order`). `StakeTable.chunks`
splits a block range as the node's `block_range_chunks` does
(`StakeTable.chunks_chunked`).
-/

@[expose] public section

namespace StakeTable

/-- Logs sorted by key, the earliest first. -/
def orderLogs {α : Type} (logs : List (LogKey × α)) : List (LogKey × α) :=
  logs.mergeSort fun a b => a.1.le b.1

theorem orderLogs_inL1Order {α : Type} {logs : List (LogKey × α)} (hk : (logs.map (·.1)).Nodup) :
    InL1Order logs (orderLogs logs) :=
  inL1Order_mergeSort hk

/--
Split the blocks `first` to `last` into ranges of at most `size` blocks, as the
node's `block_range_chunks` does. The node's chunk size is never zero; a size of zero
gives no ranges here.
-/
def chunks (first last size : Nat) : List (Nat × Nat) :=
  if h : 0 < size ∧ first ≤ last then
    (first, min (first + size - 1) last) :: chunks (min (first + size - 1) last + 1) last size
  else []
termination_by last + 1 - first

/-- The chunks cover the range in order, each at most `size` blocks. -/
theorem chunks_chunked {size : Nat} (hs : 0 < size) (first last : Nat) :
    Chunked first last size (chunks first last size) := by
  rw [chunks]
  split
  · rename_i h
    obtain ⟨hcov, hbnd⟩ := chunks_chunked hs (min (first + size - 1) last + 1) last
    refine ⟨?_, ?_⟩
    · rw [List.flatMap_cons, hcov]
      have := @List.range'_append first (min (first + size - 1) last + 1 - first)
        (last + 1 - (min (first + size - 1) last + 1)) 1
      rw [show first + 1 * (min (first + size - 1) last + 1 - first)
          = min (first + size - 1) last + 1 by omega,
        show min (first + size - 1) last + 1 - first + (last + 1 - (min (first + size - 1) last + 1))
          = last + 1 - first by omega] at this
      exact this
    · intro c hc
      rcases List.mem_cons.1 hc with rfl | hc
      · simp only; omega
      · exact hbnd c hc
  · refine ⟨?_, fun c hc => by simp at hc⟩
    have : last + 1 - first = 0 := by omega
    simp [this]
termination_by last + 1 - first
decreasing_by omega

end StakeTable
