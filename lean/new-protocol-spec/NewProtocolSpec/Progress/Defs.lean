module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run
public import NewProtocolSpec.Liveness
public import NewProtocolSpec.Network.Defs

/-!
# Conditional progress, as definitions

The definitions `NewProtocolSpec.Progress` is phrased with: a *window*, which
says what must stay true of a node for an action to stay owed, and
`LiveNetwork`, a network whose nodes obey the whole specification rather than
the safety half alone.

A window is a hypothesis, not a rule. Nothing here asks an implementation for
anything, and nothing here says a window ever opens — that is what the timing
and delivery assumptions listed as omissions would supply. What the results
built on these do is discharge the step from "the action is owed and nothing
overtakes it" to "the action is taken", which is the step that consumes
`WeaklyFair`.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-!
## What the lock must leave alone

`Vote1Enabled` reads the lock twice, through `SafeToExtend` and through its own
final clause, and the lock moves on its own between steps. Both readings are
collected here so that a window has one field for them rather than two.
-/

/--
The lock leaves room for a vote1 on `p`.

Exactly the two clauses of `Vote1Enabled` that read `NodeState.lockedCert`:
the proposal is still safe against the lock, and the lock has not reached the
proposal's view. A node that locks past `p` is not stalled — it has committed
something later — so this is where a window closes rather than something a
node owes.
-/
structure LockAllows (s : NodeState) (p : Proposal) : Prop where
  /-- Still safe to admit against the lock as it stands. -/
  safe : SafeToExtend s.lockedCert p

  /-- And the lock has not reached the proposal's own view. -/
  below : ∀ lock, s.lockedCert = some lock → lock.view < p.viewNumber

/-!
## While the action is outstanding

An obligation lapses when the action is taken, and a window has to stop
constraining the node there. What marks the action as taken cannot be the
freshness mark itself: a collection may drop the mark once the view is
abandoned (`GcSpec.voted1Retained`), so `¬ NodeState.voted1Views` comes back
after a node has voted and pruned, and a window guarded on it would be false in
exactly the runs that vote and then prune.

So the guard is the run's own history: nothing of that kind has gone out since
the window opened. Nothing can undo it, pruning included.
-/

/-- No vote1 for `p`'s view has gone out at a step in `[n, m)`. -/
def Vote1Pending {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n m : Nat) : Prop :=
  ∀ i, n ≤ i → i < m → ∀ vote : Vote1,
    Output.send (.vote1 vote) ∈ (Run.event r i).outputs → vote.view ≠ p.viewNumber

/-- No vote2 for `p`'s view has gone out at a step in `[n, m)`. -/
def Vote2Pending {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n m : Nat) : Prop :=
  ∀ i, n ≤ i → i < m → ∀ vote : Vote2,
    Output.send (.vote2 vote) ∈ (Run.event r i).outputs → vote.view ≠ p.viewNumber

/-- No decide naming view `v` has gone out at a step in `[n, m)`. -/
def DecidePending {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (v : ViewNumber) (n m : Nat) : Prop :=
  ∀ i, n ≤ i → i < m → ∀ blocks c1 c2,
    Output.decided blocks c1 c2 ∈ (Run.event r i).outputs → ∀ b ∈ blocks, b.viewNumber ≠ v

/-- No proposal for `p`'s view has gone out at a step in `[n, m)`. -/
def ProposePending {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n m : Nat) : Prop :=
  ∀ i, n ≤ i → i < m → ∀ q : Proposal,
    Output.send (.proposal q) ∈ (Run.event r i).outputs → q.viewNumber ≠ p.viewNumber

/-! Nothing has gone out yet at the step a window opens: the range is empty. -/

theorem ProposePending.refl {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n : Nat) : ProposePending r p n n :=
  fun i hi him => absurd (Nat.lt_of_lt_of_le him hi) (Nat.lt_irrefl i)

theorem Vote2Pending.refl {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n : Nat) : Vote2Pending r p n n :=
  fun i hi him => absurd (Nat.lt_of_lt_of_le him hi) (Nat.lt_irrefl i)

/--
The anchor a proposal extends is still there.

`Config.anchorBlock` and `Config.anchorCert` sit at `ViewNumber.genesis`, which
is below both watermarks for ever, so no retention clause reaches them: nothing
obliges a node to keep the anchor, and a node that dropped it can never propose
the chain's first view. That is a hypothesis of the propose results rather than
an obligation, and one any node that has not pruned the anchor discharges.
-/
structure AnchorKept (s s' : NodeState) : Prop where
  /-- The block the chain starts from is still held. -/
  proposal : ∀ b, s.proposals ViewNumber.genesis = some b →
    s'.proposals ViewNumber.genesis = some b

  /-- And the certificate over it. -/
  cert : ∀ c, s.cert1s ViewNumber.genesis = some c →
    s'.cert1s ViewNumber.genesis = some c

/-!
## Windows

One per action, each a conjunction of "nothing overtook this view while the
action was outstanding". Every field names state a node cannot be obliged to
preserve: a bar it may raise on a timeout, a floor it may raise by deciding, a
lock it may move by committing.

Every field is guarded by the action still being outstanding, and the guard is
what makes a window something a run can satisfy rather than a promise that the
node stops. A node that acts *does* abandon the view afterwards — the bar rises,
the floor rises, the lock moves past it — so a window demanded unconditionally
from some point on would be one no progressing node is in.

Read together with `NewProtocolSpec.Progress`: the results consume a window only
at states the action has yet to be taken at, and the step that takes it is the
last one they look at.
-/

/--
The window in which a vote1 for `p` stays owed.

`parentFloor` is conditional exactly as `Vote1Justification.parentLinked` is:
at genesis there is no parent to keep.
-/
structure Vote1Window {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n : Nat) : Prop where
  /-- The view is not abandoned while the vote is outstanding. -/
  bar : ∀ m, n ≤ m → Vote1Pending r p n m → (Run.state r m).barredView < p.viewNumber

  /-- Nor does it time out. -/
  timedOut : ∀ m, n ≤ m → Vote1Pending r p n m → (Run.state r m).timeoutView < p.viewNumber

  /-- The lock keeps leaving room for it. -/
  lock : ∀ m, n ≤ m → Vote1Pending r p n m → LockAllows (Run.state r m) p

  /-- The decide floor stays below it, so what the vote reads stays held. -/
  floor : ∀ m, n ≤ m → Vote1Pending r p n m →
    (Run.state r m).aboveDecideFloor cfg p.viewNumber

  /-- And below the parent's view, so the ancestry stays held. -/
  parentFloor : p.parentCert.view ≠ ViewNumber.genesis →
    ∀ m, n ≤ m → Vote1Pending r p n m →
      (Run.state r m).aboveDecideFloor cfg p.parentCert.view

/--
The window in which a vote2 for `p` stays owed.

Three of the five fields are the ways the view can be overtaken rather than
abandoned: a `Cert2` arriving, the view being decided as an ancestor, or one of
this node's own vote1s coming to skip it. Each ends the obligation without
anything having stalled, and `noSkip` is the one that is a genuine choice — see
`Vote1SkippedView`.
-/
structure Vote2Window {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n : Nat) : Prop where
  /-- The view is not abandoned while the vote is outstanding. -/
  bar : ∀ m, n ≤ m → Vote2Pending r p n m → (Run.state r m).barredView < p.viewNumber

  /-- The decide floor stays below it. -/
  floor : ∀ m, n ≤ m → Vote2Pending r p n m →
    (Run.state r m).aboveDecideFloor cfg p.viewNumber

  /-- No later vote1 of this node's endorses a branch skipping the view. -/
  noSkip : ∀ m, n ≤ m → Vote2Pending r p n m →
    ¬ Vote1SkippedView (Run.state r m) p.viewNumber

  /-- The certificate does not arrive ready-made. -/
  noCert2 : ∀ m, n ≤ m → Vote2Pending r p n m → (Run.state r m).cert2s p.viewNumber = none

  /-- And the view is not decided as some later block's ancestor. -/
  notDecided : ∀ m, n ≤ m → Vote2Pending r p n m →
    ¬ (Run.state r m).decidedViews p.viewNumber

/--
The window in which a decide for `v` stays owed.

One field, because deciding carries no bar: `DecideEnabled` reads the decide
path only, and `StepSpec.contentRetained` keeps all of it above the floor.
-/
structure DecideWindow {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (v : ViewNumber) (n : Nat) : Prop where
  /-- The decide floor stays below the view while the decide is outstanding. -/
  floor : ∀ m, n ≤ m → DecidePending r v n m → (Run.state r m).aboveDecideFloor cfg v

/--
The window in which a proposal `p` stays owed.

`lock` is conditional on the proposal carrying timeout evidence, because that is
the only branch of `ParentCertJustified` that reads the lock. A
proposal extending the immediately preceding view rests on a certificate
instead, which retention keeps.
-/
structure ProposeWindow {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n : Nat) : Prop where
  /-- The view is not abandoned while the proposal is outstanding. -/
  bar : ∀ m, n ≤ m → ProposePending r p n m → (Run.state r m).barredView < p.viewNumber

  /-- Nor does it time out. -/
  timedOut : ∀ m, n ≤ m → ProposePending r p n m → (Run.state r m).timeoutView < p.viewNumber

  /-- The decide floor stays below the view, so the built header stays held. -/
  floor : ∀ m, n ≤ m → ProposePending r p n m →
    (Run.state r m).aboveDecideFloor cfg p.viewNumber

  /--
  And below the parent's view, so the parent block and its certificate stay held.

  Conditional, as `Vote1Window.parentFloor` is, and for a sharper reason: genesis
  is below the floor in every state, so retention could not reach the anchor
  however the window was written. `anchorKept` is what covers that case.
  -/
  parentFloor : p.parentCert.view ≠ ViewNumber.genesis →
    ∀ m, n ≤ m → ProposePending r p n m →
      (Run.state r m).aboveDecideFloor cfg p.parentCert.view

  /-- A proposal extending the anchor needs the anchor to still be there. -/
  anchorKept : p.parentCert.view = ViewNumber.genesis →
    ∀ m, n ≤ m → ProposePending r p n m →
      AnchorKept (Run.state r m) (Run.state r (m + 1))

  /-- After a timeout, the lock stays on the certificate the proposal extends. -/
  lock : p.timeoutEvidence.isSome →
    ∀ m, n ≤ m → ProposePending r p n m →
      (Run.state r m).lockedCert = some p.parentCert

/-!
## What it is for an action to be owed

Each is a window together with the action being owed at the point it opens. This
is the hypothesis every result of `NewProtocolSpec.Progress` takes, and the shape
a delivery-and-timing argument would have to establish: not that the node is
enabled at one moment, but that it is enabled and stays so until it acts.
-/

/-- The node owes a vote1 for `p`, and nothing overtakes the view. -/
def Vote1Owed {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
  ∃ n, Vote1Enabled (Run.state r n) p ∧ Vote1Window r p n

/-- The node owes a vote2 for `p`, and nothing overtakes the view. -/
def Vote2Owed {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
  ∃ n, Vote2Enabled cfg (Run.state r n) p ∧ Vote2Window r p n

/-- The node owes a decide for `v`, and the floor stays below it. -/
def DecideOwed {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (v : ViewNumber) : Prop :=
  ∃ n, DecideEnabled cfg (Run.state r n) v ∧ DecideWindow r v n

/-- The node owes a proposal `p`, and nothing overtakes the view. -/
def ProposeOwed {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
  ∃ n, ProposeEnabled leader node (Run.state r n) p ∧ ProposeWindow r p n

/-!
## A network that makes progress
-/

/--
A network whose honest nodes obey the whole specification, and schedule their
own actions fairly.

`Network` is built on `SafetySpec` so that no safety argument can reach a clause
outside it (`NewProtocolSpec.Safety`). A progress argument needs the clauses that
oblige a node to act, and `WeaklyFair` besides, so it needs a different object.
`Run.weaken` is the bridge: the same states and the same events, read through the
weaker relation, which is why `net` can be *the* network the safety results speak
about rather than another one alongside it.

Nothing here is a new premise. `fair` and the stronger runs are what
`Conforms` obliges an implementation to exhibit, and the four verification-layer
premises are `net`'s, unchanged.
-/
structure LiveNetwork (cfg : Config) (leader : ViewNumber → Option PubKey) (C : Committee)
    where
  /-- One run per honest node, obeying every clause. -/
  run : ∀ k, C.honest k → Run cfg (StepSpec cfg leader k)

  /-- Each of them schedules its own obligations fairly. -/
  fair : ∀ k h, WeaklyFair (run k h)

  /-- The network the safety results are about. -/
  net : Network cfg C

  /-- And it is this one, read through the safety clauses alone. -/
  netRun : ∀ k h, net.run k h = Run.weaken cfg leader k (run k h)

end NewProtocol
