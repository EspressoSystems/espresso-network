module

public import StakeTableImpl.Select
public import StakeTableImpl.Events

/-!
# Reading an epoch's committee

`StakeTable.committeeAt` reads a committee as the node does: it keeps the logs up to
the epoch root's finalized L1 block, sorts them into L1 order, replays their events
on the indexed state and selects the active validators. Where the replay finishes,
the result is the specification's committee (`StakeTable.committeeAt_spec`).
-/

@[expose] public section

namespace StakeTable

/--
The committee read from `logs` up to L1 block `B`, or `none` if the replay stops on an
event the contract should never emit.
-/
def committeeAt (usable : NetAddr → Bool) (logs : List (LogKey × Event)) (B : Nat) :
    Option (Option (List Member)) :=
  let events := (orderLogs (logs.filter fun l => decide (l.1.1 ≤ B))).map (·.2)
  (Indexed.applyAll Indexed.State.empty events).toOption.map fun is =>
    select usable is.summaries

/-- Where the replay finishes, `committeeAt` gives the specification's committee. -/
theorem committeeAt_spec {usable : NetAddr → Bool}
    {logs : List (LogKey × Event)} {B : Nat} (hk : (logs.map (·.1)).Nodup)
    {r : Option (List Member)} (h : committeeAt usable logs B = some r) :
    CommitteeAt usable logs B r := by
  simp only [committeeAt, Option.map_eq_some_iff] at h
  obtain ⟨is, his, rfl⟩ := h
  have his : Indexed.applyAll Indexed.State.empty
      ((orderLogs (logs.filter fun l => decide (l.1.1 ≤ B))).map (·.2)) = .ok is := by
    revert his
    cases Indexed.applyAll Indexed.State.empty
      ((orderLogs (logs.filter fun l => decide (l.1.1 ≤ B))).map (·.2)) <;>
      simp [Except.toOption]
  obtain ⟨hist, hr, hs⟩ := Indexed.summaries_table his
  refine ⟨_, hist, orderLogs_inL1Order ((List.filter_sublist.map _).nodup hk), hr, ?_⟩
  rw [← hs]
  exact select_spec usable (hs ▸ (table_distinct (reads_valid hr)).accounts)

end StakeTable
