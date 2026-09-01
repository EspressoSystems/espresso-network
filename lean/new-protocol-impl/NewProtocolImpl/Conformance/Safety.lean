module

public import NewProtocolImpl.Conformance.Sound
public import NewProtocolImpl.Conformance.Gc

/-!
# The safety half of conformance

`Implements`: the machine starts where the specification starts, carries its
invariant, and every transition — consensus step or collection — satisfies the
rule for it.

Five of the six fields are already proved; the sixth is the one fact about the
initial state nobody needed until now, that its abstraction *is*
`NodeState.initial`. The anchor is the only entry in either, and the empty tables
abstract to the empty functions.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

/-- A fresh node abstracts to the specification's initial state. -/
theorem initial_abstract (cfg : Config) : (initial cfg).abstract = NodeState.initial cfg := by
  unfold State.abstract initial NodeState.initial
  simp only [NodeState.mk.injEq]
  repeat' apply And.intro
  all_goals first
    | trivial
    | (funext v; rw [get?_insert, get?_empty])
    | (funext v; exact get?_empty)
    | (funext v hb; exact get?_empty)
    | (funext v pc; simp)
    | (funext v; simp; try exact eq_comm)

/--
The machine implements the specification: it starts where the specification
starts, keeps its invariant, and satisfies `StepSpec` on every consensus
transition and `GcSpec` on every collection.
-/
theorem implements (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)
    (h : ConfigCoherent cfg) :
    Implements cfg leader node (initial cfg) (next cfg leader node) (State.gc cfg)
      State.abstract (WF cfg) ValidityReported where
  initial := initial_abstract cfg
  invInitial := initial_wf cfg h
  invStep := fun s i henv hwf => next_preservesWF cfg leader node s i henv hwf
  invCollect := fun s hwf => gc_preservesWF cfg s hwf
  sound := fun s i henv hwf => next_conforms cfg leader node s i henv hwf
  collectSound := fun s hwf => gc_conforms cfg s hwf

end Impl
end NewProtocol
