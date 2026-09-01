module

public import NewProtocolImpl.Conformance.Settle

/-!
# From the step obligations to fairness

`WeaklyFair` is a statement about runs; the obligations proved for the machine
are statements about single transitions. This file is the passage between them,
and it is the only place a whole run appears.

A run is a schedule: at each point the environment delivers an input or lets the
node collect, and the states it passes through are `Implements.trace`. One
induction over that sequence is all that is needed: nothing is ever owed, at any
point of any run (`Impl.trace_settled`) — the initial state owes nothing, a
step leaves nothing owed, and a collection creates nothing. `WeaklyFair`'s
antecedent says an action is owed from the step it anchors on, so it is false at
that very step, and every field holds without the run being examined further.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-- The machine's implementation record, as the run is built over it. -/
abbrev Impl : Prop :=
  Implements cfg leader node (initial cfg) (next cfg leader node) (State.gc cfg)
    State.abstract (WF cfg) ValidityReported

/-- The states the machine passes through under the schedule `ι`. -/
abbrev traceOf (ι : Implements.Schedule) : Nat → State :=
  Implements.trace (initial cfg) (next cfg leader node) (State.gc cfg) ι

variable {cfg leader node}

/-- The invariant holds at every point of every run: `Implements` carries it. -/
theorem trace_wf (himpl : Impl cfg leader node) {ι : Implements.Schedule}
    (hι : Implements.Schedule.Honours ValidityReported ι) (n : Nat) :
    WF cfg (traceOf cfg leader node ι n) :=
  Implements.inv_trace cfg leader node himpl hι n

/-! ## Nothing is ever owed -/

/--
No state of any run owes anything.

The three ways a state arises are the three obligations: it is the fresh state,
or a step left it, or a collection did.
-/
theorem trace_settled (himpl : Impl cfg leader node) (hcoh : ConfigCoherent cfg)
    (hns : NextSettles cfg leader node) {ι : Implements.Schedule}
    (hι : Implements.Schedule.Honours ValidityReported ι) :
    ∀ n, Settled cfg leader node (traceOf cfg leader node ι n)
  | 0 => initial_settled cfg leader node hcoh
  | n + 1 => by
    cases he : ι n with
    | some i =>
      simp only [Implements.trace, he]
      exact hns _ i (hι n i he) (trace_wf himpl hι n)
    | none =>
      simp only [Implements.trace, he]
      exact gc_settles cfg leader node _ (trace_wf himpl hι n)
        (trace_settled himpl hcoh hns hι n)

/-! ## Fairness -/

/--
Every run of the machine is weakly fair.

Every field is the same eagerness argument: the antecedent is false, because
`Impl.trace_settled` says nothing is owed at any point of any run. The
antecedent's own anchor is where it is read — it says the action is owed from
that step on, and nothing is owed there.
-/
theorem weaklyFair (himpl : Impl cfg leader node) (hcoh : ConfigCoherent cfg)
    (hns : NextSettles cfg leader node) {ι : Implements.Schedule}
    (hι : Implements.Schedule.Honours ValidityReported ι) :
    WeaklyFair (Implements.run cfg leader node himpl hι) := by
  refine { vote1 := ?_, vote2 := ?_, decide := ?_, propose := ?_ }
  · intro p n hn
    obtain ⟨h1, _, _, _⟩ := trace_settled himpl hcoh hns hι n
    exact absurd (hn n (Nat.le_refl n)) (h1 p)
  · intro p n hn
    obtain ⟨_, h2, _, _⟩ := trace_settled himpl hcoh hns hι n
    exact absurd (hn n (Nat.le_refl n)) (h2 p)
  · intro v n hn
    obtain ⟨_, _, hd, _⟩ := trace_settled himpl hcoh hns hι n
    exact absurd (hn n (Nat.le_refl n)) (hd v)
  · intro p n hn
    obtain ⟨_, _, _, hp⟩ := trace_settled himpl hcoh hns hι n
    exact absurd (hn n (Nat.le_refl n)) (hp p)

/--
The machine conforms, given that its steps settle.

The last obligation is `NewProtocolImpl.Conformance.Owed`; everything else
this needs is proved.
-/
theorem protocol_conforms (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)
    (hns : NextSettles cfg leader node) : ProtocolConforms cfg leader node := fun hcoh =>
  { safety := implements cfg leader node hcoh
    progress := fun _ hι => weaklyFair (implements cfg leader node hcoh) hcoh hns hι }

end Impl
end NewProtocol
