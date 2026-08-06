module

public import NewProtocolSpec
public import NewProtocolImpl.Order
public import NewProtocolImpl.Store
public import NewProtocolImpl.Protocol
public import NewProtocolImpl.Floor
public import NewProtocolImpl.Conformance
public import NewProtocolImpl.Conformance.Gc
public import NewProtocolImpl.Conformance.Handle
public import NewProtocolImpl.Conformance.Rounds
public import NewProtocolImpl.Conformance.Seq
public import NewProtocolImpl.Conformance.Fields
public import NewProtocolImpl.Conformance.Cases
public import NewProtocolImpl.Conformance.Pass
public import NewProtocolImpl.Conformance.Shape
public import NewProtocolImpl.Conformance.Vote2
public import NewProtocolImpl.Conformance.Vote1
public import NewProtocolImpl.Conformance.Propose
public import NewProtocolImpl.Conformance.Chain
public import NewProtocolImpl.Conformance.Decide
public import NewProtocolImpl.Conformance.Safety
public import NewProtocolImpl.Conformance.Settle
public import NewProtocolImpl.Conformance.Guards
public import NewProtocolImpl.Conformance.Owed
public import NewProtocolImpl.Conformance.Acts
public import NewProtocolImpl.Demo
public import NewProtocolImpl.Conformance.Progress
public import NewProtocolImpl.Conformance.Sound
public import NewProtocolImpl.Conformance.Views
public import NewProtocolImpl.Conformance.Step
public import NewProtocolImpl.Conformance.Inputs
public import NewProtocolImpl.Conformance.Marks
public import NewProtocolImpl.Conformance.Lock

/-!
# A reference implementation

One implementation of `NewProtocolSpec`, and the proof that it is one.

* `NewProtocolImpl.Order`, `NewProtocolImpl.Store` — view-number order, and
  the finite tables the machine is built from
* `NewProtocolImpl.Protocol` — the machine: `Impl.State` and `Impl.next`
* `NewProtocolImpl.Floor` — the decide floor, where the machine's maximum meets
  the specification's quantifier
* `NewProtocolImpl.Conformance` — the abstraction into `NodeState`, the
  machine's representation invariant, and what conformance decomposes into
* `NewProtocolImpl.Conformance.Gc` — collection: `GcConforms`, `GcPreservesWF`
* `NewProtocolImpl.Conformance.Handle` — the input arms: provenance, ingestion,
  retention, the cursors, and the invariant
* `NewProtocolImpl.Conformance.Rounds` — the reaction rounds: what they frame,
  what they grow, and `NextPreservesWF`
* `NewProtocolImpl.Conformance.Seq` — sequencing: splitting a pass, and tracing an
  output back to the round that emitted it
* `NewProtocolImpl.Conformance.Fields` — which rounds leave the lock and the decided
  views alone
* `NewProtocolImpl.Conformance.Cases` — what each round does, in one equation
* `NewProtocolImpl.Conformance.Pass` — the segments of a pass, the states between
  them, and what is settled when
* `NewProtocolImpl.Conformance.Shape` — each round emits only its own kind of output
* `NewProtocolImpl.Conformance.Vote2`, `.Vote1`, `.Propose`, `.Decide` — everything
  `StepSpec` asks of each kind of action, at the state the step ends in
* `NewProtocolImpl.Conformance.Chain` — the decide walk: held, linked, unsettled,
  and stopping only where the specification allows
* `NewProtocolImpl.Conformance.Safety` — `Implements`, the safety half assembled
* `NewProtocolImpl.Conformance.Settle` — nothing owed before a step and after a
  collection, the second from `GcSpec` alone
* `NewProtocolImpl.Conformance.Guards` — the guards read in the completeness
  direction, and why the lock has reached every view it may
* `NewProtocolImpl.Conformance.Owed` — `NextSettles`: a step leaves nothing owed,
  and `Impl.conforms`
* `NewProtocolImpl.Conformance.Progress` — from those to `WeaklyFair` over runs

Nothing here is normative. The contract is the specification package, which
does not depend on this one; where it leaves freedom — scheduling, output
order, representation, the architecture-internal `Command`s — the choices made
here bind nobody. A second implementation with a different state
representation and a different scheduling discipline would be equally
conforming, and writing one from the specification alone is the sharpest
available test of whether the specification is complete.

The machine earns its place three ways:

* it **witnesses** that the specification is satisfiable — obligations written
  independently of one another can turn out to be jointly unachievable, and no
  amount of reading catches that; `ProtocolConforms` does;
* it is **executable**, so it can be driven against the production node in
  differential tests: `blockHash` is `@[irreducible]` rather than `opaque`, so no
  proof can see through it while a run still computes it, and hash comparisons
  mean what they say. A block that arrived carries the identity the network
  assigned it, and `NewProtocolDiff` replays real traces against the machine;
* it is a **worked example** of one way to discharge the obligations.

## The shape of the machine

Each step takes the input in (`Impl.handle`) and then does everything the new
state owes (`Impl.rounds`). Three choices in the second half are forced by the
obligations rather than chosen, and they are the ones to read first:

* **The rounds are ordered.** Decides first, since deciding raises the floor and
  `SafetySpec.vote2AboveFloor` reads the floor at the end of the step; then the
  lock; then vote1s; then vote2s, which must see this step's
  vote1 records for `SafetySpec.vote2NotInSkippedView`; proposals last, over the
  settled lock.
* **The lock moves before the votes, to the highest certificate the state
  licenses.** `Vote1Justification.safeToExtend` judges a vote1's proposal
  against the lock and `vote2LockOrdered` requires the lock to have reached
  every vote2's view — both judged on the state the step leaves behind —
  so where the lock ends up has to be settled before either round tests it.
* **Each round scans its own table, once.** A vote is owed only where a proposal
  was admitted, a decide only where a `Cert2` landed, a proposal only where a
  header was headers. Scanning those keys is a complete search, and one pass over
  them suffices, since an attempt can only retire opportunities and never create
  one. That is what makes the eagerness argument for progress available: nothing
  is left owed at a step boundary.

## What is proved

**Conformance is complete, both halves**:

```lean
theorem NewProtocol.Impl.conforms (cfg : Config) (leader : ViewNumber → Option PubKey)
    (node : PubKey) : ProtocolConforms cfg leader node
```

`#print axioms` on it reports only `propext`, `Classical.choice` and `Quot.sound`; there
are no `sorry`s. `ProtocolConforms` is `Conforms` for every configuration whose anchor
sits at genesis, every leader schedule and every node identity — so the machine is a
witness that the specification is satisfiable, and that nothing in it is contradictory
in combination with fairness.

It is relative to one assumption about the environment, and only one:
`ValidityReported`, the machine's `Implements.envOk`. `Vote1Justification.blockValid`
is a statement about the block, not about having been told about it, and consensus
does not interpret blocks — so nothing the machine can compute discharges it. What
does is `Impl.WF.validated`, the invariant that its validity table records only valid
blocks, and preserving *that* is where the assumption is consumed: `Input.blockValidated`
is the only arm that writes the table. The progress half is therefore quantified over
schedules that honour the assumption. Nothing else rests on it — in particular
`DecideSafety` never reads validity.

### Safety

`Impl.implements`: the machine starts where the specification starts, keeps its
representation invariant, satisfies `StepSpec` on every consensus transition and
`GcSpec` on every collection. Its six components, and the pieces behind them:

* `Impl.initial_abstract` — a fresh node abstracts to `NodeState.initial`
* `Impl.next_conforms` — every step satisfies `StepSpec` (`NextConforms`), assembled in
  `Conformance.Sound` from all of the below
* `Impl.gc_conforms` — collection satisfies `GcSpec` (`GcConforms`)
* `Impl.gc_preservesWF` — collection keeps the invariant (`GcPreservesWF`)
* `Impl.next_preservesWF` — every step keeps the invariant (`NextPreservesWF`)
* `Impl.initial_wf` — a fresh node satisfies it
* the content half of `StepSpec` over the input arms: provenance, ingestion,
  retention, the two cursors, and the rules of admission as decidable tests
  (`NewProtocolImpl.Conformance.Handle`)
* the frame and growth structure of the reaction rounds, closed under `seq`
  (`NewProtocolImpl.Conformance.Rounds`)
* what each round does when it acts, and what it leaves alone — the five
  `*_cases` equations, the field-preservation lemmas, and the pass decomposition
  with the states between its segments (`Cases`, `Fields`, `Pass`, `Seq`, `Shape`)
* the four action obligations of `StepSpec`, one per output kind:
  `Impl.pass_vote2` (including the three clauses the round order exists for — the
  lock has reached the vote's view, the view is above the floor the step ends with,
  and no branch record skips it), `Impl.pass_vote1` (with the share it travels
  with and the branch it records), `Impl.pass_propose` (both readings of the
  proposing rule), and `Impl.pass_decide` (with the chain's shape from
  `Conformance.Chain`)

### Progress

Progress is the converse of safety: not "the round only fires when the action is
justified" but "if the action is owed, the round fires". Every field of `WeaklyFair`
is discharged by *eagerness* — nothing is owed once a step ends
(`Impl.next_settles`), so the antecedent is never satisfied. That route is open
because every enabledness predicate reads only what the node holds; nothing owed
waits on state the node would have to procure from outside, so no obligation has to
be met by asking for something.

* `Impl.next_settles` (`NextSettles`) — a step leaves nothing owed. One lemma per
  action in `Conformance.Owed`, each reading a guard in the completeness direction
  and contradicting it with the action's own freshness clause. The rounds' order is
  what makes this possible, and `Impl.advanceLock_reached` is why a vote2,
  which the machine bars while the lock sits below its view, is never owed there
  (`Conformance.Guards`).
* `Impl.gc_settles` (`GcSettles`) and `Impl.initial_settled` (`InitialSettled`) —
  collection creates no obligation and a fresh node owes nothing (`Conformance.Settle`).
  The first is proved from `GcSpec` alone, so the machine's filters never enter it.
* `Impl.weaklyFair` — the passage from those to `WeaklyFair` over runs, which is the
  only place a whole run appears (`Conformance.Progress`).

### Where a decide stops

A decide delivers a chain, and the chain runs back only as far as the node holds it: at a
view already decided, at one at or below the floor, or at a block not in hand. A missing
ancestor truncates the chain rather than blocking the decide, and the view it names is
skipped — the decide stream promises agreement and finality, not completeness
(`NewProtocolSpec.DecideStream`). `Impl.chainFrom_last` is that the walk stops nowhere else,
and its fuel is why exhaustion is not a fourth place: a held proposal above genesis is
well-formed, so each link strictly descends.

That is what lets the decide round be a single scan. An obligation that waited for the
hole to fill would have had to be repaired from outside — the node cannot reach a block
nobody is obliged to send it — and `WeaklyFair` forces every implementation to leave
nothing owed at a step boundary, since an input-driven node emits nothing during a
collection and a schedule need never deliver another input. So the bar would have made
the obligation unsatisfiable rather than merely awkward. With ancestry out of
`DecideEnabled`, a decide owed at the end of a pass was owed at its start and the round's
own attempt took it: nothing an attempt does can make a view an earlier attempt passed
over become owed, deciding only raising the floor and only marking views decided.
-/
