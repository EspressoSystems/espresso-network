module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run
public import NewProtocolSpec.Liveness

/-!
# Conformance

What it means to satisfy this specification: `Implements` for safety,
`Conforms` for safety and progress together.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
`step` implements the specification under the abstraction `abs`, given the
representation invariant `inv` and what it assumes of each input, `envOk`.
-/
structure Implements {σ : Type} (init : σ) (step : σ → Input → σ × List Output)
    (collect : σ → σ) (abs : σ → NodeState) (inv : σ → Prop)
    (envOk : Input → Prop) : Prop where
  /-- The implementation starts where the specification starts. -/
  initial : abs init = NodeState.initial cfg

  /-- The invariant holds initially and is preserved, so it is available at every step. -/
  invInitial : inv init

  /-- The invariant survives a consensus step on an input the environment allows. -/
  invStep : ∀ s input, envOk input → inv s → inv (step s input).1

  /-- And survives a collection. -/
  invCollect : ∀ s, inv s → inv (collect s)

  /-- Every consensus transition satisfies the step specification. -/
  sound : ∀ s input, envOk input → inv s →
    StepSpec cfg leader node (abs s) input (step s input).2 (abs (step s input).1)

  /--
  Every collection satisfies the pruning rule.

  An implementation that never prunes takes `collect` to be the identity,
  which satisfies `GcSpec` — but it must say so, because unconstrained
  pruning could discard anything.
  -/
  collectSound : ∀ s, inv s → GcSpec cfg (abs s) (abs (collect s))

namespace Implements

variable {σ : Type} {init : σ} {step : σ → Input → σ × List Output} {collect : σ → σ}
  {abs : σ → NodeState} {inv : σ → Prop} {envOk : Input → Prop}

/--
What the environment does at one point in time: deliver an input, or let the
node collect.

Collection is the environment's to schedule for the same reason it carries no
input: it is a stutter, available at any time and consuming nothing.
-/
abbrev Schedule := Nat → Option Input

/-- Every input the schedule delivers satisfies what the implementation assumes of it. -/
def Schedule.Honours (envOk : Input → Prop) (ι : Schedule) : Prop :=
  ∀ n i, ι n = some i → envOk i

/-- The states the implementation passes through under the schedule `ι`. -/
def trace (init : σ) (step : σ → Input → σ × List Output) (collect : σ → σ)
    (ι : Schedule) : Nat → σ
  | 0 => init
  | n + 1 =>
    match ι n with
    | some i => (step (trace init step collect ι n) i).1
    | none => collect (trace init step collect ι n)

theorem inv_trace (h : Implements cfg leader node init step collect abs inv envOk)
    {ι : Schedule} (hι : Schedule.Honours envOk ι) :
    ∀ n, inv (trace init step collect ι n)
  | 0 => Implements.invInitial h
  | n + 1 => by
    have ih := inv_trace h hι n
    unfold trace
    split
    · next i he => exact Implements.invStep h _ _ (hι n i he) ih
    · exact Implements.invCollect h _ ih

/--
The run the implementation performs under the schedule `ι`.

Schedules are total, so "nothing further ever happens" is not a run: a
deferred action always gets another chance, which is what makes deferring for
ever a choice rather than an accident.
-/
def run (h : Implements cfg leader node init step collect abs inv envOk)
    {ι : Schedule} (hι : Schedule.Honours envOk ι) : Run cfg (StepSpec cfg leader node) where
  state n := abs (trace init step collect ι n)
  event n :=
    match ι n with
    | some i => .consensus i (step (trace init step collect ι n) i).2
    | none => .collect
  transition n := by
    have hinv := inv_trace cfg leader node h hι n
    show Transition cfg (StepSpec cfg leader node) (abs (trace init step collect ι n))
        (match ι n with
          | some i => .consensus i (step (trace init step collect ι n) i).2
          | none => .collect)
        (abs (match ι n with
          | some i => (step (trace init step collect ι n) i).1
          | none => collect (trace init step collect ι n)))
    cases he : ι n with
    | some i => exact .step (Implements.sound h _ i (hι n i he) hinv)
    | none => exact .collect (Implements.collectSound h _ hinv)

end Implements

/--
**Conformance**: the implementation is safe, and it makes progress.

Neither half can be dropped. Safety is step-local, and no step-local property
can force an action, since deferring by one step is always legal — so a node
that does nothing is safe, and it is progress that it fails.

The two halves and `StepSpec`'s ingestion obligations interlock, and all three
are needed. Progress is conditioned on enabledness, and enabledness on what the
node holds: a node that consumed every input and stored nothing would enable
nothing and so satisfy fairness vacuously. Ingestion forces it to hold what it
was sent; the mark obligations stop it retiring an opportunity in silence;
fairness stops it deferring one for ever.

Progress does not require acting in the step that enables an action. An
implementation may defer, so long as it does not defer for ever. An *eager*
implementation — one that leaves nothing enabled at a step boundary — satisfies
every field of `WeaklyFair` vacuously, the antecedent never being true. That
route is open to any implementation because every enabledness predicate reads
only what the node holds: nothing owed waits on state the node would have to
procure from outside, so nothing has to be asked for before it can be done.

`envOk` is what the implementation assumes of its environment, and only one
thing needs it: an implementation cannot know that `Input.blockValidated` names
a block that really is `BlockValid`, since consensus does not interpret blocks.
So it assumes it of every input it is handed, and conformance holds relative to
schedules that honour the assumption (`Implements.Schedule.Honours`). An
implementation with nothing to assume takes `envOk` to be `fun _ => True`, and
then every schedule honours it. The dependency is confined to building a node:
`DecideSafety` never reads validity, so no-fork does not rest on it.
-/
structure Conforms {σ : Type} (init : σ) (step : σ → Input → σ × List Output)
    (collect : σ → σ) (abs : σ → NodeState) (inv : σ → Prop)
    (envOk : Input → Prop) : Prop where
  /-- Every transition satisfies the step specification. -/
  safety : Implements cfg leader node init step collect abs inv envOk

  /-- And every fair schedule makes progress. -/
  progress : ∀ (ι : Implements.Schedule) (hι : Implements.Schedule.Honours envOk ι),
    WeaklyFair (Implements.run cfg leader node safety hι)

end NewProtocol
