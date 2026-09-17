module

public import NewProtocolImpl.Protocol
public import NewProtocolImpl.Store

/-!
# The decide floor

The machine keeps the decided views in a finite set and reads the floor off
their maximum; the specification states the floor pointwise, as a condition on
every decided view (`NodeState.aboveDecideFloor`). The two agree as long as the
set is non-empty, which is what `Impl.aboveFloor_iff` says — the one place
the machine's `max?` meets the specification's quantifier.
-/

public section

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

/-- `≤` on view numbers is the comparison the tables order keys by. -/
theorem le_of_isLE {a b : ViewNumber} (h : (compare a b).isLE) : a ≤ b := by
  rw [compare_viewNumber, Nat.isLE_compare] at h
  exact h

/-- The maximum of the decided views is one of them. -/
theorem lastDecided_mem {s : State} (h : s.decidedViews.max?.isSome) :
    s.lastDecided ∈ s.decidedViews := by
  obtain ⟨m, hm⟩ := Option.isSome_iff_exists.mp h
  have hmem := (Std.TreeSet.max?_eq_some_iff_mem_and_forall.mp hm).1
  rwa [State.lastDecided, hm, Option.getD_some]

/-- No decided view is above the maximum. -/
theorem le_lastDecided {s : State} {w : ViewNumber} (h : w ∈ s.decidedViews) :
    w ≤ s.lastDecided := by
  obtain ⟨m, hm⟩ := Option.isSome_iff_exists.mp (Std.TreeSet.isSome_max?_of_mem h)
  have hle := (Std.TreeSet.max?_eq_some_iff_mem_and_forall.mp hm).2 w h
  rw [State.lastDecided, hm, Option.getD_some]
  exact le_of_isLE hle

/-- Subtracting the buffer is monotone. -/
theorem sub_le_sub {a b : ViewNumber} (n : Nat) (h : a ≤ b) : a - n ≤ b - n :=
  Nat.sub_le_sub_right h n

/--
The machine's floor test is the specification's, on a non-empty decided set.

Left to right the maximum dominates every decided view; right to left the
maximum is itself decided, so the pointwise condition applies to it.
-/
theorem aboveFloor_iff (cfg : Config) (s : State) (hne : s.decidedViews.max?.isSome)
    (v : ViewNumber) :
    s.aboveFloor cfg v = true ↔ ∀ w, w ∈ s.decidedViews → w - cfg.decideBuffer < v := by
  refine ⟨fun h w hw => ?_, fun h => ?_⟩
  · exact Nat.lt_of_le_of_lt (sub_le_sub _ (le_lastDecided hw)) (of_decide_eq_true h)
  · exact decide_eq_true (h _ (lastDecided_mem hne))

/--
The branch scan, as a question about the records it folds over.

`State.vote1Skipped` is a fold for the sake of the machine; every proof reads it
through this equation instead.
-/
theorem vote1Skipped_eq_any (s : State) (v : ViewNumber) :
    s.vote1Skipped v = s.vote1Branches.toList.any fun e => v < e.1 && e.2 < v := by
  rw [State.vote1Skipped, Std.TreeMap.foldl_eq_foldl_toList, foldl_or_eq_any]
  simp

end Impl
end NewProtocol
