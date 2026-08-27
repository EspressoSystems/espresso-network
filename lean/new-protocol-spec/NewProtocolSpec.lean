module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run
public import NewProtocolSpec.Liveness
public import NewProtocolSpec.DecideStream
public import NewProtocolSpec.Assumptions
public import NewProtocolSpec.Network
public import NewProtocolSpec.Safety
public import NewProtocolSpec.Progress
public import NewProtocolSpec.Deadlock
public import NewProtocolSpec.Implements
public import NewProtocolSpec.Checks

/-!
# The consensus specification

What one consensus node must do, and the whole contract an implementation owes:
`SafetySpec` and `StepSpec` for a step, `GcSpec` for a collection, `decideSafety`
for the result they are built for, and `Conforms` for what an implementation owes.

## Modules

* `Base` — view numbers
* `Types` — the data a node sends, receives and stores
* `Interface` — the configuration, and the inputs and outputs of one step
* `State` — `NodeState`, the history a node keeps
* `Step` — the rules, when an action is owed, and the two structures that
  collect them: `SafetySpec` and `StepSpec`, which extends it
* `Gc` — pruning, the rule for the other kind of step
* `Run` — the two kinds of step as one relation, and the runs they generate,
  over whichever of the two structures a result needs
* `Liveness` — progress, as fairness over runs
* `Implements` — what it means to conform: safety *and* progress
* `DecideStream` — what the decide stream guarantees to the application, and
  what it deliberately does not
* `Assumptions` — the premises taken rather than proved, in one place
* `Network` — the committee, certificates as the votes behind them, and what
  those certificates guarantee
* `Safety` — the no-fork property
* `Progress` — what an owed action is worth: fairness turned into an output, and a
  quorum's worth of owed votes turned into a certificate
* `Deadlock` — that an action can always be made owed, by inputs the specification
  itself admits
* `Checks` — the claims this specification makes about itself, checked at build
  time

`Network`, `Safety`, `DecideStream` and `Progress` come in three parts each: `X/Defs.lean`
holds the definitions the statements are phrased with, `X/Lemmas.lean` the
kernel-checked scaffolding, and `X.lean` the results. An audit reads the first
and the third.

Everything from `Network` upwards talks about nodes that obey `SafetySpec` and
nothing else. So a safety proof cannot reach for another clause: there is none in
scope to reach for. `Run.weaken` says a node obeying all of `StepSpec` is one
of these, so the results apply to it too.

`Progress` and `Deadlock` are the other direction, and are the only places the
obligations to act are used. Both are conditional throughout: a node acts if the
environment delivers and nothing overtakes the view, and neither of those is
something the specification models. What they establish is that the rules
themselves leave no way to stall — which no reading of a clause list can settle,
since it is a property of the conjunction.
-/
