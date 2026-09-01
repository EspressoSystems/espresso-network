module

public import NewProtocolImpl.Conformance.Inputs

/-!
# One step, taken apart

A step is the input arm, then the reaction pass. This file names those two
pieces of `Impl.next`.

Everything else is already proved — `NewProtocolImpl.Conformance.Handle` and
`.Inputs` for the arm, `.Vote1` through `.Marks` for the pass.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {s : State} {i : Input}

/-- The state a step ends in is the state the pass ends in. -/
theorem next_state : (next cfg leader node s i).1
    = st5 cfg leader node (ingest cfg node s i) :=
  pass_state cfg leader node (ingest cfg node s i)

/-- What a step emits: the arm's outputs, then the pass's. -/
theorem next_out : (next cfg leader node s i).2
    = ingestOut cfg node s i
      ++ (seq (rounds cfg leader node (ingest cfg node s i))
            (ingest cfg node s i)).2 := rfl

/-- An output of a step comes from the arm or the pass. -/
theorem mem_next_out {o : Output} (h : o ∈ (next cfg leader node s i).2) :
    o ∈ ingestOut cfg node s i
      ∨ o ∈ (seq (rounds cfg leader node (ingest cfg node s i))
              (ingest cfg node s i)).2 := by
  rw [next_out] at h
  exact List.mem_append.mp h

/-! ## The state the specification sees

The abstraction is a relabelling, so each field of `NodeState` is the same
lookup; the one thing that needs a proof is the decide floor, where the
machine's maximum meets the specification's quantifier.
-/

theorem abstract_aboveDecideFloor {t : State} (hwf : WF cfg t) {v : ViewNumber}
    (h : t.aboveFloor cfg v = true) : t.abstract.aboveDecideFloor cfg v :=
  (aboveFloor_abstract hwf v).mp h

theorem aboveFloor_of_abstract {t : State} (hwf : WF cfg t) {v : ViewNumber}
    (h : t.abstract.aboveDecideFloor cfg v) : t.aboveFloor cfg v = true :=
  (aboveFloor_abstract hwf v).mpr h

/-- The invariant holds of the state the arm leaves and of the state the step ends in. -/
theorem next_wf (henv : ValidityReported i) (hwf : WF cfg s) :
    WF cfg (ingest cfg node s i) ∧ WF cfg (next cfg leader node s i).1 := by
  refine ⟨ingest_wf henv hwf, ?_⟩
  rw [next_state]
  exact (st5_stage (ingest_wf henv hwf)).2.2

end Impl
end NewProtocol
