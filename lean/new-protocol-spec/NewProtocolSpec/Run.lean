module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc

/-!
# Transitions and runs

What a node does in one step, and what it does over time.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
What happens at one step of a run: the node consumes an input and emits
outputs, or it collects.

Collection carries **no input**. It is a stutter in the input stream, not a
way of consuming it — an input is consumed only by the step that answers it.
This matters because `GcSpec` is satisfied by leaving the state alone, so a
collection step is always available: were it to carry an input, it would be a
way of discarding one, and with it the obligations that input triggers
(`StepSpec.cert2RelayOwed`, `timeoutVoteOwed`, `advanceOwed`,
`timeoutCertAdvanceOwed`). Those obligations are triggered by an input rather
than shaped like enabledness, so `WeaklyFair` cannot recover them: once the
input is gone, nothing stays enabled to be owed later.
-/
inductive Event where
  /-- Consume `input`, emit `output`. -/
  | consensus (input : Input) (output : List Output)

  /-- Collect. No input is consumed and nothing is emitted. -/
  | collect

/-- What an event emits; collection emits nothing. -/
def Event.outputs : Event → List Output
  | .consensus _ output => output
  | .collect => []

/--
What a consensus step may do: a relation on state, input, outputs, next state.

`StepSpec cfg leader node` and `SafetySpec cfg node` are the two that matter.
Naming the shape lets everything below be stated once and applied to either.
-/
abbrev StepRel := NodeState → Input → List Output → NodeState → Prop

/-- One transition of a node, whose consensus steps satisfy `S`. -/
inductive Transition (cfg : Config) (S : StepRel) (s : NodeState) : Event → NodeState → Prop where
  /-- A consensus step: an input consumed and outputs emitted, as `S` allows. -/
  | step {input output s'} : S s input output s' → Transition cfg S s (.consensus input output) s'

  /-- A collection step: state dropped, as `GcSpec` allows. -/
  | collect {s'} : GcSpec cfg s s' → Transition cfg S s .collect s'

/--
States reachable from `init` through `Transition`s.

Everything downstream quantifies over this — the safety statements and the
fairness obligations alike — so that no result is silently restricted to nodes
that never prune.
-/
inductive Reachable (cfg : Config) (S : StepRel) (init : NodeState) : NodeState → Prop where
  /-- `init` itself. -/
  | refl : Reachable cfg S init init

  /-- One transition on from a state already reachable. -/
  | step {s s' : NodeState} {event : Event} :
      Reachable cfg S init s → Transition cfg S s event s' → Reachable cfg S init s'

/-- An infinite run of one node: states and events indexed by time. -/
structure Run (cfg : Config) (S : StepRel) where
  /-- The state before each step. -/
  state : Nat → NodeState

  /-- What the node did at each step. -/
  event : Nat → Event

  /-- Each step relates its two states, under `S` or by collecting. -/
  transition : ∀ n, Transition cfg S (state n) (event n) (state (n + 1))

namespace Run

/--
`P` holds at every state from step `n` on, anchored at a point.

The anchor is what lets a fairness obligation tie the action it demands to the
point from which it was owed: `Run.EmitsFrom` is bounded by the same `n`. Weak
fairness is usually stated with that point existentially quantified; here it is
kept, so that the conclusion can name it.
-/
def AlwaysFrom {cfg : Config} {S : StepRel}
    (r : Run cfg S) (n : Nat) (P : NodeState → Prop) : Prop :=
  ∀ m, n ≤ m → P (Run.state r m)

/-! The two halves of an `Event`: what a step put out, and what it took in. -/

/-- Some step of the run emits an output satisfying `P`. -/
def Emits {cfg : Config} {S : StepRel}
    (r : Run cfg S) (P : Output → Prop) : Prop :=
  ∃ n o, o ∈ (Run.event r n).outputs ∧ P o

/--
Some step at or after `n` emits an output satisfying `P`.

What `WeaklyFair` concludes. The bound is the whole difference from `Run.Emits`,
and it buys convenience rather than strength: an emission before `n` is already
impossible when the action is owed from `n` on, because taking it sets a mark the
node may drop only where the action is barred anyway. What the bound saves is
every consumer having to re-derive that in order to rule the earlier action out.
-/
def EmitsFrom {cfg : Config} {S : StepRel}
    (r : Run cfg S) (n : Nat) (P : Output → Prop) : Prop :=
  ∃ j, n ≤ j ∧ ∃ o, o ∈ (Run.event r j).outputs ∧ P o

/-- The run consumed `i` at step `n`. -/
def Consumes {cfg : Config} {S : StepRel}
    (r : Run cfg S) (n : Nat) (i : Input) : Prop :=
  ∃ out, Run.event r n = .consensus i out

end Run

/--
The step that emitted an output.

`Run.Emits` gives an index; this turns that index into the step relation that
governed it. Collection emits nothing, so an emitting event is a consensus step.
-/
theorem emit_step {cfg : Config} {S : StepRel} (r : Run cfg S) {n : Nat} {o : Output}
    (he : o ∈ (Run.event r n).outputs) :
    ∃ input output, Run.event r n = .consensus input output
      ∧ S (Run.state r n) input output (Run.state r (n + 1)) ∧ o ∈ output := by
  have ht : Transition cfg S (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
    Run.transition r n
  cases hev : Run.event r n with
  | consensus input output =>
    rw [hev] at ht he
    cases ht with
    | step hs => exact ⟨input, output, rfl, hs, by simpa [Event.outputs] using he⟩
  | collect => rw [hev] at he; exact absurd he (by simp [Event.outputs])

/-- A transition under `S` is one under anything `S` implies. -/
theorem Transition.weaken {cfg : Config} {S T : StepRel} {s s' : NodeState} {e : Event}
    (ht : Transition cfg S s e s') (h : ∀ a i o b, S a i o b → T a i o b) :
    Transition cfg T s e s' := by
  cases ht with
  | step hs => exact .step (h _ _ _ _ hs)
  | collect hg => exact .collect hg

/--
Every run is a run of the safety clauses alone.

The one-way street between the two: a node obeying the whole specification
obeys `SafetySpec`, so a result proved of safety runs holds of real ones. The
converse does not hold, which is the point — `NewProtocolSpec.Network` is built
on safety runs, so nothing in the safety argument can appeal to a rule outside
`SafetySpec`.
-/
def Run.weaken (r : Run cfg (StepSpec cfg leader node)) : Run cfg (SafetySpec cfg node) where
  state := r.state
  event := r.event
  transition n := (r.transition n).weaken fun _ _ _ _ hs => StepSpec.toSafetySpec hs

end NewProtocol
