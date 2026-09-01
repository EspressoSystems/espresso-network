import Verso
import VersoManual
import NewProtocolSpec
import Style

open Verso.Genre Manual InlineLean
open NewProtocol

set_option verso.code.warnLineLength 90

#doc (Manual) "New protocol: consensus specification" =>

%%%
tag := "top"
%%%

_What one consensus node must do, and the whole contract an implementation owes._

Consensus runs in numbered views, each with a leader. The leader of a view
proposes one block, extending a parent that it names by certificate. What travels
is a proposal: the block's header, the view it is made in, and the certificate
for the parent. The transactions are erasure-coded into shares and dispersed, the
header commits to them. Every node then votes on that proposal twice, and each
round completes when a quorum has voted.

A vote1 says the proposal is a fit extension. The node casting it holds the
proposal, has been told the block it proposes is valid, holds its own share of the
payload and has checked that the proposal does not conflict with its lock. Unless
the parent is genesis, it also holds the parent proposal, with the parent's payload
reconstructed. A quorum of vote1s is a {name NewProtocol.Cert1}`Cert1`, which does
three jobs:
  * the next leader cites it to extend the chain,
  * any node may move to the next view on it, and
  * a node may lock on it.

A vote2 says the block is there to stay. It requires a {name NewProtocol.Cert1}`Cert1`
for the proposal and the payload reconstructed from enough shares — so the second
round is where availability is established, not just agreement, and where a proposal
becomes a block the node has in full. A quorum of vote2s is a
{name NewProtocol.Cert2}`Cert2`, and a {name NewProtocol.Cert2}`Cert2` decides the
block: the node hands it to the application, along with any ancestors it holds and
that it has not delivered before.

A view that produces no {name NewProtocol.Cert1}`Cert1` is abandoned. The node's timer
fires, it votes to give the view up, and a quorum of those votes is a
{name NewProtocol.TimeoutCert}`TimeoutCert`, on which the next view can start without a
{name NewProtocol.Cert1}`Cert1` for the one skipped.

Forks are prevented by two things a node knows about its own past, not by any test
on the branch it is asked to endorse. A node that has cast a vote2 in a view has
its lock at that view or beyond, which stops it endorsing a later proposal reaching
back past it. A node that has cast a vote1 remembers the branch it endorsed, which
stops it casting a vote2 in a view that branch skips.
{name NewProtocol.decideSafety}`decideSafety` is the proof that these two suffice.

Everything is stated per node — one node's state, one node's step — until the
network section, which is where a quorum's worth of nodes first appears, and which
is all that {name NewProtocol.decideSafety}`decideSafety` needs.

Every rule below is spliced from the declaration that states it. The prose here
orders and groups them; it does not restate them, so it cannot drift from what is
proved. Read top to bottom, nothing is used before it is defined.

# What is not covered

%%%
tag := "not-covered"
%%%

This specification covers the core protocol: a single, static committee of which this
node is a member, with the leader schedule left as a parameter, on crash-free nodes.
What is left out falls into two groups, and the difference decides how much judgment
each item deserves.

*Absent, and nothing proved depends on it.* An implementation supplies these, and no
rule here reads them, so how they work cannot affect the results.

* *Light-client certification of the chain's state.* Consensus only carries such a
  certificate for others to use; nothing in the protocol reads it.

* *Message assembly.* A proposal and the recipient's VID share travel separately and
  must be paired; {name NewProtocol.Input}`Input` delivers the pair already assembled.
  That the pairing is honest is still required rather than assumed:
  {name NewProtocol.StepSpec.vidShareProvenance}`StepSpec.vidShareProvenance` says a
  held share arrived with the proposal it belongs to.

* *Fetching what a node missed.* An implementation may ask peers for proposals and
  payloads it does not have. Here a fetched proposal arrives as any other does — held
  for ancestry when its admission guards fail, never votable — and no obligation
  depends on fetching, since the decide stream skips what is not in hand. Fetching is
  quality of service, not protocol.

*Absent, and the results are narrower for it.* Each of these is something a real
deployment does, and what is proved holds of the system with the piece removed. The
distance between that system and a real one is where an audit should spend its doubt.

* *Epochs and committee rotation.* Epoch numbers on messages, the arithmetic of epoch
  boundaries, the randomness that seeds the next committee, the handover, and
  stake-table membership. Only the leader schedule survives, as a parameter. Safety is
  stated for one fixed committee, so nothing here speaks to a handover between two.

* *Persistence and restart.* Parking a vote or a proposal until storage confirms it,
  and resuming afterwards, exist to make voting promises survive a crash. Nodes are
  assumed not to crash, so nothing is claimed of a node that does; what matters for
  safety is kept as ordering obligations — the lock is settled before a vote2 is
  released ({name NewProtocol.SafetySpec.vote2LockOrdered}`SafetySpec.vote2LockOrdered`),
  and a view voted in is never voted in twice
  ({name NewProtocol.SafetySpec.vote1Once}`SafetySpec.vote1Once`,
  {name NewProtocol.SafetySpec.vote2Once}`SafetySpec.vote2Once`).

* *Application-level block validity.* Consensus never sees a block's transactions, so
  {name NewProtocol.BlockValid}`BlockValid` is uninterpreted and arrives as an input to
  be believed. Every result is therefore conditional on
  {name NewProtocol.ValidityReported}`ValidityReported` below: a node told that an
  invalid block is valid will vote for it.

* *The network.* Nothing says a message is ever delivered, not even eventually, and
  with no notion of duration there is nothing for a synchrony assumption to be about.
  {name NewProtocol.WeaklyFair}`WeaklyFair` is the per-node half of liveness: it
  constrains how a node schedules its own actions and says nothing about its peers.
  End-to-end progress needs the other half; {ref "progress"}[what progress is worth]
  is how far the per-node half reaches on its own.

* *The view timer.* A timeout input is the timer firing; nothing says when, because
  nothing here has a notion of duration. {name NewProtocol.Network}`Network` orders
  events causally, which is a different thing: it says one event precedes another, never
  how long anything takes, and concurrent events stay incomparable. So resetting the
  timer on entering a view has no counterpart here, and neither does the request that
  would do it — see {name NewProtocol.Output}`Output`. What this costs is progress:
  partial synchrony would need the duration that is absent.

* *Agreement on what may be pruned.* {name NewProtocol.GcSpec}`GcSpec` lets each node
  prune unilaterally, bounded by two watermarks, and proves that nothing it may still
  owe is lost. Nothing coordinates the decision, which is sound only because nothing
  here says what it means to serve a peer what one has kept; a specification that had
  one would need the nodes to agree on the watermark before pruning past it.

* *Message routing.* No message names a recipient, so nothing here says whether one is
  broadcast or sent to a single peer, and for one message the choice is not free. Every
  honest node must be able to assemble a timeout certificate itself, which means the
  timeout votes must reach all of them: a node enters the next view only on a `Cert1`
  for the previous one or a timeout certificate over it, and it cannot time out of a
  view it never entered. Concentrate those votes on one node and that node can stop the
  network for good. Vote1s and vote2s are not like this — withholding them stalls one
  view, which the timeout path then carries. Neither fact is stated here, so end-to-end
  progress needs both, alongside a delivery model.

* *Migration from an earlier protocol.* Only what happens after such a boundary is
  described, and only for a node that ran this protocol from the start:
  {name NewProtocol.Config.anchorBlock}`Config.anchorBlock` anchors every run at
  genesis. The earlier protocol is not described here, so nothing is claimed of it, of
  the crossing, or of a node carrying state from before it.

Whether each omission is safe is a judgment for the reader; the specification does not
settle it. The second group is where that judgment is needed.

# The data

The objects the rules are about. Three reductions run through all of them.
Cryptographic values — hashes, keys, commitments — are one-field wrappers, because
the rules only ever compare and store them. Hashing is opaque: no rule can look
inside a hash, so every rule relates hash images only. And a block is reduced to the
proposal that carries it: {name NewProtocol.Block}`Block` is an alias for
{name NewProtocol.Proposal}`Proposal`, so what the rules call a block is its header,
view and parent link, never its transactions. Those sit behind
{name NewProtocol.PayloadCommit}`PayloadCommit`, and whether a node has them is
tracked apart from the proposal, as availability. Signatures are not modelled
either — where a message would carry one and a recipient would verify it, the
conclusion is stated as a proposition, and {name NewProtocol.Network}`Network` is
where those propositions are collected.

{docstring NewProtocol.ViewNumber}

{docstring NewProtocol.EpochNumber}

{docstring NewProtocol.EpochNumber.ext}

Epochs are not carried around as a separate notion: a block's epoch is a
function of its height, and the committee that may certify it is the one that
epoch names. Two functions say all of it.

{docstring NewProtocol.epochOf}

{docstring NewProtocol.IsLastBlock}

Four facts about that arithmetic are what the epoch-crossing argument runs on.
The first two say the epoch moves only at a boundary and then only by one, so a
branch cannot slip between epochs unnoticed; the third that epochs are numbered
from one, so there is a first; the last two that an epoch's blocks stop where its
last block is.

{docstring NewProtocol.epochOf_one}

{docstring NewProtocol.epochOf_succ}

{docstring NewProtocol.epochOf_pos}

{docstring NewProtocol.lastBlock_height}

{docstring NewProtocol.epochOf_height_le}

{docstring NewProtocol.BlockHash}

{docstring NewProtocol.PayloadCommit}

{docstring NewProtocol.PubKey}

{docstring NewProtocol.BlockHeader}

{docstring NewProtocol.Proposal}

{docstring NewProtocol.Block}

{expansion NewProtocol.Block}

{docstring NewProtocol.BlockTable}

{expansion NewProtocol.BlockTable}

{docstring NewProtocol.Vote1Data}

{docstring NewProtocol.Vote2Data}

Certificates share one shape, differing in what their votes signed.
{name NewProtocol.Cert1}`Cert1` and {name NewProtocol.Cert2}`Cert2` are kept apart
at the type level so that no rule can mistake a vote of one round for the other.

{docstring NewProtocol.Certificate}

{docstring NewProtocol.Cert1}

{expansion NewProtocol.Cert1}

{docstring NewProtocol.Cert2}

{expansion NewProtocol.Cert2}

{docstring NewProtocol.TimeoutCert}

{expansion NewProtocol.TimeoutCert}

{docstring NewProtocol.Vote}

{docstring NewProtocol.Vote1}

{expansion NewProtocol.Vote1}

{docstring NewProtocol.Vote2}

{expansion NewProtocol.Vote2}

{docstring NewProtocol.TimeoutVote}

{expansion NewProtocol.TimeoutVote}

{docstring NewProtocol.CatchupEvidence}

{docstring NewProtocol.VidShare}

{docstring NewProtocol.Config}

One function over that data is used throughout, and it is not a rule.

{docstring NewProtocol.blockHash}

# What a node is told, and what it says

Nothing else crosses the boundary. In particular a node never asks for anything,
so there are no request outputs: an implementation's own subsystems are its
business, and the specification only names what arrives.

Both are architecture-neutral *events*. An input is a moment at which the node
comes to know something — a proposal arrived, a payload is in hand, a block was
found valid, a timer fired — never a module answering a call, so every
implementation has these moments however it is decomposed. Which of them a node
*owes* is not settled here; that is what the obligations of
{name NewProtocol.StepSpec}`StepSpec` name.

{docstring NewProtocol.Input}

{docstring NewProtocol.Output}

{docstring NewProtocol.Message}

# What a node remembers

{docstring NewProtocol.NodeState}

The other watermark is not a field. The bar is stored, because a node chooses when
to move it; the decide floor is derived, because it follows the views the node has
decided, and so moves on its own as the chain advances.

```lean -show
namespace Spec
```

:::spec NewProtocol.NodeState.aboveDecideFloor
  ```lean
  def NodeState.aboveDecideFloor (cfg : NewProtocol.Config) (s : NodeState)
      (v : ViewNumber) : Prop :=
    ∀ w, s.decidedViews w → w - cfg.decideBuffer < v
  ```
:::

```lean -show
example : @Spec.NodeState.aboveDecideFloor = @NewProtocol.NodeState.aboveDecideFloor := rfl
end Spec
```

{includeDocstring NewProtocol.NodeState.aboveDecideFloor}

What a step may not discard is split along the same seam. Each names one view's
worth of state, and the two halves go stale at different times, which is what lets
a collection drop one and keep the other.

{docstring NewProtocol.RetainsDecide +hideFields}

{docstring NewProtocol.RetainsVote +hideFields}

{docstring NewProtocol.Retains +hideFields}

```lean -show
namespace Spec
```

:::spec NewProtocol.Writable
  ```lean
  def Writable {α : Type} (o : Option α) (x : α) : Prop :=
    o = none ∨ o = some x
  ```
:::

```lean -show
example : @Spec.Writable = @NewProtocol.Writable := rfl
end Spec
```

{includeDocstring NewProtocol.Writable}

{docstring NewProtocol.Event}

{docstring NewProtocol.StepRel}

{expansion NewProtocol.StepRel}

Collection is a step of the machine like any other: a `.consensus` event is
justified by the step relation, a `.collect` event by
{name NewProtocol.GcSpec}`GcSpec`. What a node prunes is therefore bound by a
rule, rather than happening between steps where nothing constrains it.

{docstring NewProtocol.Transition}

{docstring NewProtocol.Reachable}

{docstring NewProtocol.Run}

Four readings of a run are used below, and nothing else looks inside one: what it
emitted, what it emitted from a step on, what it consumed, and what holds from a
step on.

```lean -show
namespace Spec
```

:::spec NewProtocol.Run.Emits
  ```lean
  def Run.Emits {cfg : NewProtocol.Config} {S : StepRel}
      (r : Run cfg S) (P : NewProtocol.Output → Prop) : Prop :=
    ∃ n o, o ∈ (Run.event r n).outputs ∧ P o
  ```
:::

```lean -show
example : @Spec.Run.Emits = @NewProtocol.Run.Emits := rfl
end Spec
```

{includeDocstring NewProtocol.Run.Emits}

```lean -show
namespace Spec
```

:::spec NewProtocol.Run.EmitsFrom
  ```lean
  def Run.EmitsFrom {cfg : NewProtocol.Config} {S : StepRel}
      (r : Run cfg S) (n : Nat) (P : NewProtocol.Output → Prop) : Prop :=
    ∃ j, n ≤ j ∧ ∃ o, o ∈ (Run.event r j).outputs ∧ P o
  ```
:::

```lean -show
example : @Spec.Run.EmitsFrom = @NewProtocol.Run.EmitsFrom := rfl
end Spec
```

{includeDocstring NewProtocol.Run.EmitsFrom}

```lean -show
namespace Spec
```

:::spec NewProtocol.Run.Consumes
  ```lean
  def Run.Consumes {cfg : NewProtocol.Config} {S : StepRel}
      (r : Run cfg S) (n : Nat) (i : Input) : Prop :=
    ∃ out, Run.event r n = .consensus i out
  ```
:::

```lean -show
example : @Spec.Run.Consumes = @NewProtocol.Run.Consumes := rfl
end Spec
```

{includeDocstring NewProtocol.Run.Consumes}

```lean -show
namespace Spec
```

:::spec NewProtocol.Run.AlwaysFrom
  ```lean
  def Run.AlwaysFrom {cfg : NewProtocol.Config} {S : StepRel}
      (r : Run cfg S) (n : Nat) (P : NodeState → Prop) : Prop :=
    ∀ m, n ≤ m → P (Run.state r m)
  ```
:::

```lean -show
example : @Spec.Run.AlwaysFrom = @NewProtocol.Run.AlwaysFrom := rfl
end Spec
```

{includeDocstring NewProtocol.Run.AlwaysFrom}

None of this is a liveness notion. {ref "network"}[The network] and the no-fork
result are built on runs too, and both read a node's history rather than its
future; progress is stated separately, in {ref "when-owed"}[when an action is
owed], so nothing here need be read as a promise that anything happens.

# The rules the votes appeal to

Both votes, and the rule for proposing, are stated in terms of these. Each reads
only the proposal in front of the node and what the node already holds.

```lean -show
namespace Spec
```

:::spec NewProtocol.ProposalWellFormed
  ```lean
  def ProposalWellFormed (cfg : NewProtocol.Config) (p : Proposal) : Prop :=
    p.parentCert.view < p.viewNumber
      ∧ (p.parentCert.view + 1 = p.viewNumber
          ∨ ∃ tc, p.timeoutEvidence = some tc ∧ tc.view + 1 = p.viewNumber)
      ∧ p.epoch = epochOf p.blockHeader.blockNumber cfg.epochHeight
  ```
:::

```lean -show
example : @Spec.ProposalWellFormed = @NewProtocol.ProposalWellFormed := rfl
end Spec
```

{includeDocstring NewProtocol.ProposalWellFormed}


```lean -show
namespace Spec
```

:::spec NewProtocol.SafeToExtend
  ```lean
  def SafeToExtend (locked : Option Cert1) (p : Proposal) : Prop :=
    match locked with
    | none      => True
    | some lock =>
      if lock.view = p.viewNumber then
        lock.data.blockHash = blockHash p
      else
        p.parentCert.data = lock.data ∨ lock.view < p.parentCert.view
  ```
:::

```lean -show
example : @Spec.SafeToExtend = @NewProtocol.SafeToExtend := rfl
end Spec
```

{includeDocstring NewProtocol.SafeToExtend}

```lean -show
namespace Spec
```

:::spec NewProtocol.ShareMatches
  ```lean
  def ShareMatches (p : Proposal) (vid : VidShare) : Prop :=
    vid.view = p.viewNumber ∧ p.payloadCommit = vid.payloadCommit
  ```
:::

```lean -show
example : @Spec.ShareMatches = @NewProtocol.ShareMatches := rfl
end Spec
```

{includeDocstring NewProtocol.ShareMatches}

```lean -show
namespace Spec
```

:::spec NewProtocol.ChainLinked
  ```lean
  def ChainLinked : List NewProtocol.Block → Prop
    | []              => True
    | [_]             => True
    | b :: b' :: rest => b.parentCert.view = b'.viewNumber
        ∧ b.parentCert.data.blockHash = blockHash b'
        ∧ ChainLinked (b' :: rest)
  ```
:::

```lean -show
example : @Spec.ChainLinked = @NewProtocol.ChainLinked := by
  funext l
  induction l with
  | nil => rfl
  | cons b rest ih =>
    cases rest with
    | nil => rfl
    | cons b' r => simp only [Spec.ChainLinked, NewProtocol.ChainLinked, ih]
end Spec
```

{includeDocstring NewProtocol.ChainLinked}

```lean -show
namespace Spec
```

:::spec NewProtocol.Vote1SkippedView
  ```lean
  def Vote1SkippedView (s : NodeState) (v : ViewNumber) : Prop :=
    ∃ w u, v < w ∧ s.vote1Branches w = some u ∧ u < v
  ```
:::

```lean -show
example : @Spec.Vote1SkippedView = @NewProtocol.Vote1SkippedView := rfl
end Spec
```

{includeDocstring NewProtocol.Vote1SkippedView}

Its three branches are what the safety argument turns on.

{docstring NewProtocol.Vote1Justification}

{docstring NewProtocol.Vote2Justification}

{docstring NewProtocol.ProposalJustification}

# The clauses safety rests on

Twenty-one, collected in {name NewProtocol.SafetySpec}`SafetySpec`. No-fork rests on
exactly these; `NewProtocolSpec.Checks` fails the build if that changes silently.

They come in six groups, and the fields below are in that order.

* *Where a node's state comes from.* Nothing enters a node's state except through
  an input, so no rule can be satisfied by inventing the state that justifies it.
* *Views left behind.* A node that has abandoned a view must not act in it, and
  collection is the only thing that may move the bar.
* *What a node may not forget.* Safety compares two actions of one node, so what it
  did must still be known when the second action comes due.
* *The first vote.* What a vote1 requires, and what it leaves on the record.
* *The second vote.* A vote2 commits a view for good, so it is the more constrained
  of the two.
* *The lock, and timeouts.* The lock is a certificate the node holds and only ever
  moves forward. Timing out endorses no branch, which is why it bars so little.

{docstring NewProtocol.SafetySpec}

Of those, {name NewProtocol.SafetySpec.vote2NotInSkippedView}`vote2NotInSkippedView`
is the one whose absence is hardest to picture, so here is the run it forbids.

:::scenario "the fork a withheld certificate would buy"

Four nodes, so a quorum is three and one may be faulty. `D` is the faulty one.
`A` and `B` are honest, and everything they do below passes every other rule.

1. *View 10, led by `D`.* `D` proposes a block justified by the `Cert1` for view
   9, and sends the proposal to `A` and `B` only. Both vote1. `D` assembles the `Cert1`
   for view 10 from their votes and its own, and keeps it. `A` and `B` now want
   to vote2 and cannot: {name NewProtocol.Vote2Justification}`Vote2Justification`
   needs that `Cert1`. Their locks are still at 9.

2. *Views 10 and 11 time out.* A timeout certificate forms for each.

3. *View 12, led by `C`.* `C` never saw view 10's block, so its lock is still at
   9, and it proposes justified by the `Cert1` for view 9 plus the timeout
   certificate — well-formed by
   {name NewProtocol.ProposalWellFormed}`ProposalWellFormed`. `A` and `B` check
   {name NewProtocol.SafeToExtend}`SafeToExtend`: the proposal's parent
   certificate *is* their lock, so it passes. `A`, `B` and `C` vote1, then
   vote2, and view 12 gets a `Cert2`. The chain runs 9 → 12, and view 10 is
   skipped.

4. *`D` releases the certificate it withheld.* Now `A` and `B` hold everything
   view 10's vote2 asks for — the proposal, its payload, and a matching
   `Cert1`. Nothing else stops them: their locks are at 12, which
   {name NewProtocol.SafetySpec.vote2LockOrdered}`SafetySpec.vote2LockOrdered`
   is content with, and
   {name NewProtocol.Vote2Enabled}`Vote2Enabled` carries no timeout bar, so the
   two expired views do not stop it either. Their two votes plus `D`'s make a
   `Cert2` for view 10.

Two blocks would then hold a `Cert2`, and neither is an ancestor of the other:
view 12's block descends from view 9 without passing through view 10, and view
10's block has no descendants at all. That is precisely what
{name NewProtocol.DecideSafety}`DecideSafety` denies.

The vote1 `A` and `B` cast in step 3 is what forbids their vote2 in step 4:
it was justified at view 9, so it recorded view 10 as skipped, and
{name NewProtocol.Vote1SkippedView}`Vote1SkippedView` holds there ever after.
Note which votes it is stated about — the *earlier* pair of votes constrains the
later one, in the opposite order to the one the run takes.

A bar on the timeout view would stop this run too. Two arguments reach that
conclusion, and they differ in what they need.

The first goes through the quorum. The timeout certificate is a quorum of
timeout votes for view 11, so
{name NewProtocol.Committee.intersect}`Committee.intersect` puts an honest node
in both that quorum and view 10's, and
{name NewProtocol.SafetySpec.timeoutVoteSound}`SafetySpec.timeoutVoteSound` has
already carried its bar past view 10.

The second needs nothing of the kind, but does need a rule this specification
does not have. Were the bar to rise on *holding* a valid timeout certificate,
`A` and `B` would carry theirs to 11 the moment they admitted `C`'s proposal —
which is where that certificate reaches them. View 10 is below it, so their
vote2 is refused, and the whole argument is about one node's own state with no
quorum in it. As things stand the bar rises only on
a node's own timer or on enough timeout votes to prove that an honest node timed
out ({name NewProtocol.StepSpec.timeoutViewJustified}`StepSpec.timeoutViewJustified`),
and a certificate that arrives inside a proposal is never recorded
({name NewProtocol.StepSpec.timeoutCertIngested}`StepSpec.timeoutCertIngested`).

The bar is not the rule used because it is coarser than a record of what the
node endorsed. A bar at 11 refuses a vote2 in *every* view at or below 11,
including view 9 — the branch's own parent — where
{name NewProtocol.Vote1SkippedView}`Vote1SkippedView` refuses only the views the
branch skipped. What that costs is a vote rather than a block: a view the chain
extends is still delivered as an ancestor once a descendant commits, which
{name NewProtocol.Vote2Enabled}`Vote2Enabled` sets out.
:::

# When an action is owed

%%%
tag := "when-owed"
%%%

An action is owed when it is justified, still fresh, and not barred. These are
the predicates {name NewProtocol.WeaklyFair}`WeaklyFair` quantifies over.

Progress has to be stated over runs, because no step-local specification can
force it: a node that stores everything it is sent and never votes, proposes or
decides violates no clause of {name NewProtocol.StepSpec}`StepSpec` about those
four actions, every one of them being either a permission or a mark obligation.

What the mark obligations forbid is silent retirement: consuming the mark that
makes an action unrepeatable without emitting the action. That closes one escape
and not the others. An owed action also stops being owed when the node times out,
abandons the view, prunes past it, locks past it, or sees the certificate arrive
ready-made — none of which is a stall, and all of which leave
{name NewProtocol.WeaklyFair}`WeaklyFair`'s antecedent false. So "becomes owed"
does not imply "stays owed", and fairness alone does not turn one into the other.
What closes the gap is {ref "progress"}[what progress is worth], where a window
is the statement that none of those happened before the node acted.

This is the per-node half only. End-to-end progress — after the network settles,
a view with an honest leader decides — needs the delivery and synchrony
assumptions listed under {ref "not-covered"}[what is not covered]; the predicates
below are the hypotheses such a proof would consume, and
{ref "progress"}[what progress is worth] is what they yield already.

```lean -show
namespace Spec
```

:::spec NewProtocol.Vote1Enabled
  ```lean
  def Vote1Enabled (s : NodeState) (p : Proposal) : Prop :=
    Vote1Justification s p
      ∧ s.validated p.viewNumber = some (blockHash p)
      ∧ ¬ s.voted1Views p.viewNumber ∧ s.timeoutView < p.viewNumber
      ∧ s.barredView < p.viewNumber
      ∧ ∀ lock, s.lockedCert = some lock → lock.view < p.viewNumber
  ```
:::

```lean -show
example : @Spec.Vote1Enabled = @NewProtocol.Vote1Enabled := rfl
end Spec
```

{includeDocstring NewProtocol.Vote1Enabled}


```lean -show
namespace Spec
```

:::spec NewProtocol.Vote2Enabled
  ```lean
  def Vote2Enabled (cfg : NewProtocol.Config) (s : NodeState) (p : Proposal) : Prop :=
    Vote2Justification s p ∧ ¬ Vote1SkippedView s p.viewNumber
      ∧ ¬ s.voted2Views p.viewNumber ∧ s.cert2s p.viewNumber = none
      ∧ ¬ s.decidedViews p.viewNumber ∧ s.aboveDecideFloor cfg p.viewNumber
      ∧ s.barredView < p.viewNumber
  ```
:::

```lean -show
example : @Spec.Vote2Enabled = @NewProtocol.Vote2Enabled := rfl
end Spec
```

{includeDocstring NewProtocol.Vote2Enabled}


```lean -show
namespace Spec
```

:::spec NewProtocol.DecideEnabled
  ```lean
  def DecideEnabled (cfg : NewProtocol.Config) (s : NodeState) (v : ViewNumber) : Prop :=
    ¬ s.decidedViews v ∧ s.aboveDecideFloor cfg v ∧ (s.cert1s v).isSome
      ∧ ∃ c2 p, s.cert2s v = some c2 ∧ s.proposals v = some p
          ∧ c2.data.blockHash = blockHash p
  ```
:::

```lean -show
example : @Spec.DecideEnabled = @NewProtocol.DecideEnabled := rfl
end Spec
```

{includeDocstring NewProtocol.DecideEnabled}


```lean -show
namespace Spec
```

:::spec NewProtocol.ProposeEnabled
  ```lean
  def ProposeEnabled (cfg : NewProtocol.Config)
      (leader : ViewNumber → Option PubKey) (node : PubKey)
      (s : NodeState) (p : Proposal) : Prop :=
    ProposalJustification cfg leader node s p ∧ ¬ s.proposedViews p.viewNumber
      ∧ s.timeoutView < p.viewNumber ∧ s.barredView < p.viewNumber
  ```
:::

```lean -show
example : @Spec.ProposeEnabled = @NewProtocol.ProposeEnabled := rfl
end Spec
```

{includeDocstring NewProtocol.ProposeEnabled}


Progress is not proved but owed: {name NewProtocol.Conforms}`Conforms` carries
{name NewProtocol.WeaklyFair}`WeaklyFair` as a field, so an implementation must exhibit
it. Both speak of a node's whole history rather than of one step.

{docstring NewProtocol.WeaklyFair}

# Everything else a node owes

{name NewProtocol.StepSpec}`StepSpec` extends {name NewProtocol.SafetySpec}`SafetySpec`
with thirty-seven further clauses. No safety result consumes them, which is not to say
they are optional: they are what makes a node useful rather than merely harmless.

{docstring NewProtocol.StepSpec}

# Pruning

Pruning is a transition of its own rather than something a step does on the side,
for two reasons: it takes no input, so a node that receives nothing can still
reclaim memory, and it is the only place the staleness watermarks apply, which is
what lets a node forget a vote it can no longer repeat. A consensus step, by
contrast, never prunes above the decide floor
({name NewProtocol.StepSpec.contentRetained}`StepSpec.contentRetained`) and never
moves the bar
({name NewProtocol.SafetySpec.barredViewUnchanged}`SafetySpec.barredViewUnchanged`).

Two watermarks decide what a collection may drop. The bar covers what only a vote1
or a proposal would read, and once a view is at or below it none of that can be
acted on again. The decide floor, lower, covers what a late `Cert2` could still
need — the vote2 marks among it, because a view between the floor and the bar is
one this node has given up proposing in and may still commit.

{docstring NewProtocol.Shrinks +hideFields}

{docstring NewProtocol.GcSpec}

The floor is what makes those cuts safe, and it never descends. A collection may
drop a decided view once the floor has passed it, which is why `floorStable` is
assumed above; a consensus step keeps every decided view, so there the same fact
is a theorem.

{docstring NewProtocol.SafetySpec.floorMono}

# The network, and what certificates guarantee

%%%
tag := "network"
%%%

A node's own rules say nothing about certificates, which are the act of many. The
committee supplies the one thing the argument takes from stake, and the ordering
supplies the one thing it takes from time.

{docstring NewProtocol.Committee}

A certificate is not a signature here. It is the votes behind it: to say a `Cert1`
is backed is to say some quorum of honest nodes each emitted that very vote at some
point in its own run. So the two predicates below are about runs, not about
cryptography, and the verification layer's job is to make the certificates a node
is handed match them.

```lean -show
namespace Spec
```

:::spec NewProtocol.CastVote1
  ```lean
  def CastVote1 {cfg : NewProtocol.Config} {node : PubKey}
      (r : Run cfg (SafetySpec cfg node)) (v : Vote1) : Prop :=
    Run.Emits r fun o => o = NewProtocol.Output.send (.vote1 v)
  ```
:::

```lean -show
example : @Spec.CastVote1 = @NewProtocol.CastVote1 := rfl
end Spec
```

{includeDocstring NewProtocol.CastVote1}

```lean -show
namespace Spec
```

:::spec NewProtocol.CastTimeout
  ```lean
  def CastTimeout {cfg : NewProtocol.Config} {node : PubKey}
      (r : Run cfg (SafetySpec cfg node)) (d : TimeoutData) (v : ViewNumber) : Prop :=
    Run.Emits r fun o => ∃ e, o = NewProtocol.Output.send (.timeoutVote ⟨d, v, node⟩ e)
  ```
:::

```lean -show
example : @Spec.CastTimeout = @NewProtocol.CastTimeout := rfl
end Spec
```

{includeDocstring NewProtocol.CastTimeout}

```lean -show
namespace Spec
```

:::spec NewProtocol.Cert1Backed
  ```lean
  def Cert1Backed {cfg : NewProtocol.Config} {C : Committee}
      (run : ∀ k, C.honest k → NewProtocol.Run cfg (SafetySpec cfg k))
      (c : Cert1) : Prop :=
    ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest k,
      CastVote1 (run k h) ⟨c.data, c.view, k⟩
  ```
:::

```lean -show
example : @Spec.Cert1Backed = @NewProtocol.Cert1Backed := rfl
end Spec
```

{includeDocstring NewProtocol.Cert1Backed}

{docstring NewProtocol.NodeStep}

```lean -show
namespace Spec
```

:::spec NewProtocol.Cert1BackedBefore
  ```lean
  def Cert1BackedBefore {cfg : NewProtocol.Config} {C : Committee}
      (run : ∀ k, C.honest k → NewProtocol.Run cfg (SafetySpec cfg k))
      (Before : NodeStep C → NodeStep C → Prop) (c : Cert1) (s : NodeStep C) : Prop :=
    ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest k,
      ∃ n, NewProtocol.Output.send (.vote1 ⟨c.data, c.view, k⟩)
            ∈ (NewProtocol.Run.event (run k h) n).outputs
        ∧ Before ⟨k, h, n⟩ s
  ```
:::

```lean -show
example : @Spec.Cert1BackedBefore = @NewProtocol.Cert1BackedBefore := rfl
end Spec
```

{includeDocstring NewProtocol.Cert1BackedBefore}

```lean -show
namespace Spec
```

:::spec NewProtocol.TimeoutCertBacked
  ```lean
  def TimeoutCertBacked {cfg : NewProtocol.Config} {C : Committee}
      (run : ∀ k, C.honest k → NewProtocol.Run cfg (SafetySpec cfg k))
      (tc : TimeoutCert) : Prop :=
    ∃ q, C.Quorum tc.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest k,
      CastTimeout (run k h) tc.data tc.view
  ```
:::

```lean -show
example : @Spec.TimeoutCertBacked = @NewProtocol.TimeoutCertBacked := rfl
end Spec
```

{includeDocstring NewProtocol.TimeoutCertBacked}

{docstring NewProtocol.Network}

```lean -show
namespace Spec
```

:::spec NewProtocol.Network.Cast1
  ```lean
  def Network.Cast1 (cfg : NewProtocol.Config) {C : Committee}
      (N : NewProtocol.Network cfg C) (k : PubKey) (h : C.honest k) (v : Vote1) : Prop :=
    Run.Emits (N.run k h) fun o => o = NewProtocol.Output.send (.vote1 v)
  ```
:::

```lean -show
example : @Spec.Network.Cast1 = @NewProtocol.Network.Cast1 := rfl
end Spec
```

{includeDocstring NewProtocol.Network.Cast1}

```lean -show
namespace Spec
```

:::spec NewProtocol.Network.Cast2
  ```lean
  def Network.Cast2 (cfg : NewProtocol.Config) {C : Committee}
      (N : NewProtocol.Network cfg C) (k : PubKey) (h : C.honest k) (v : Vote2) : Prop :=
    Run.Emits (N.run k h) fun o => o = NewProtocol.Output.send (.vote2 v)
  ```
:::

```lean -show
example : @Spec.Network.Cast2 = @NewProtocol.Network.Cast2 := rfl
end Spec
```

{includeDocstring NewProtocol.Network.Cast2}


A certificate is valid when a quorum signed it, and only the quorum's honest
members are held to having voted — which is the strength quorum intersection
needs.

```lean -show
namespace Spec
```

:::spec NewProtocol.Network.ValidCert1
  ```lean
  def Network.ValidCert1 (cfg : NewProtocol.Config) {C : Committee}
      (N : NewProtocol.Network cfg C) (c : Cert1) : Prop :=
    ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest k,
      NewProtocol.Network.Cast1 cfg N k h ⟨c.data, c.view, k⟩
  ```
:::

```lean -show
example : @Spec.Network.ValidCert1 = @NewProtocol.Network.ValidCert1 := rfl
end Spec
```

{includeDocstring NewProtocol.Network.ValidCert1}

```lean -show
namespace Spec
```

:::spec NewProtocol.Network.ValidCert2
  ```lean
  def Network.ValidCert2 (cfg : NewProtocol.Config) {C : Committee}
      (N : NewProtocol.Network cfg C) (c : Cert2) : Prop :=
    ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest k,
      NewProtocol.Network.Cast2 cfg N k h ⟨c.data, c.view, k⟩
  ```
:::

```lean -show
example : @Spec.Network.ValidCert2 = @NewProtocol.Network.ValidCert2 := rfl
end Spec
```

{includeDocstring NewProtocol.Network.ValidCert2}


All six results below were once premises. Four were discharged outright, and two
more became theorems once the ordering arrived —
{name NewProtocol.cert1_backed}`cert1_backed`, that a certificate an honest node
holds is one a quorum really signed, and
{name NewProtocol.oneHonest_reached}`oneHonest_reached`, that one-honest evidence
follows a view an honest node was really in. What used to be trusted about
certificates is now proved from the two premises above.

{docstring NewProtocol.cert1_unique}

{docstring NewProtocol.cert2_unique}

{docstring NewProtocol.cert2_implies_cert1}

{docstring NewProtocol.timeoutCert_reached}

{docstring NewProtocol.cert1_backed}

{docstring NewProtocol.oneHonest_reached}

# What is assumed

Besides the fields of {name NewProtocol.Committee}`Committee` and
{name NewProtocol.Network}`Network` above, five premises are stated outright.
Two of the five arrived with epochs, and both are conditions on the block table
rather than on the protocol.

Ancestry is followed through a block table, so the statement is worth nothing
unless the table holds the blocks the network actually has.

{docstring NewProtocol.Ancestor}

```lean -show
namespace Spec
```

:::spec NewProtocol.Resolves
  ```lean
  def Resolves (tree : NewProtocol.BlockTable)
      {cfg : NewProtocol.Config} {C : Committee} (N : Network cfg C) : Prop :=
    ∀ k (h : C.honest k) n v b,
      (Run.state (N.run k h) n).proposals v = some b → tree (blockHash b) = some b
  ```
:::

```lean -show
example : @Spec.Resolves = @NewProtocol.Resolves := rfl
end Spec
```

{includeDocstring NewProtocol.Resolves}


```lean -show
namespace Spec
```

:::spec NewProtocol.CollisionFree
  ```lean
  def CollisionFree : Prop :=
    ∀ b b' : NewProtocol.Block, blockHash b = blockHash b' → b = b'
  ```
:::

```lean -show
example : @Spec.CollisionFree = @NewProtocol.CollisionFree := rfl
end Spec
```

{includeDocstring NewProtocol.CollisionFree}

```lean -show
namespace Spec
```

:::spec NewProtocol.HeightSucceedsParent
  ```lean
  def HeightSucceedsParent (tree : NewProtocol.BlockTable) {cfg : NewProtocol.Config}
      {C : Committee} (N : Network cfg C) : Prop :=
    ∀ c1 : Cert1, Network.ValidCert1 cfg N c1 → ∀ b parent : NewProtocol.Block,
      tree c1.data.blockHash = some b → b ≠ cfg.anchorBlock →
      tree b.parentCert.data.blockHash = some parent →
      b.blockHeader.blockNumber = parent.blockHeader.blockNumber + 1
  ```
:::

```lean -show
example : @Spec.HeightSucceedsParent = @NewProtocol.HeightSucceedsParent := rfl
end Spec
```

{includeDocstring NewProtocol.HeightSucceedsParent}

```lean -show
namespace Spec
```

:::spec NewProtocol.AnchorRooted
  ```lean
  def AnchorRooted (tree : NewProtocol.BlockTable) (cfg : NewProtocol.Config) : Prop :=
    tree cfg.anchorBlock.parentCert.data.blockHash = none
  ```
:::

```lean -show
example : @Spec.AnchorRooted = @NewProtocol.AnchorRooted := rfl
end Spec
```

{includeDocstring NewProtocol.AnchorRooted}

Whether those five can be met at once, and met by a network that certifies
anything, is not something a reader should have to take on trust: a premise
nothing satisfies makes the no-fork result true for no reason.
{name NewProtocol.Witness.certificate_exists}`Witness.certificate_exists`
exhibits a network that meets them all and certifies a block, and
`NewProtocolSpec.Checks` fails the build if it stops doing so.


```lean -show
namespace Spec
```

:::spec NewProtocol.TreeCoherent
  ```lean
  def TreeCoherent (tree : NewProtocol.BlockTable) : Prop :=
    ∀ (h : BlockHash) (b : NewProtocol.Block), tree h = some b → blockHash b = h
  ```
:::

```lean -show
example : @Spec.TreeCoherent = @NewProtocol.TreeCoherent := rfl
end Spec
```

{includeDocstring NewProtocol.TreeCoherent}


```lean -show
namespace Spec
```

:::spec NewProtocol.ValidityReported
  ```lean
  def ValidityReported (i : Input) : Prop :=
    ∀ v h, i = Input.blockValidated v h →
      ∀ b : NewProtocol.Block, blockHash b = h → BlockValid b
  ```
:::

```lean -show
example : @Spec.ValidityReported = @NewProtocol.ValidityReported := rfl
end Spec
```

{includeDocstring NewProtocol.ValidityReported}


{docstring NewProtocol.BlockValid}

One further premise is a hypothesis of {name NewProtocol.decideSafety}`decideSafety`
itself and appears in its signature: the configured anchor must sit where the rules
assume it does.

{docstring NewProtocol.ConfigCoherent}

# What is proved

The point of everything above. `decideSafety` is stated here rather than where
its own definitions are introduced, because it is phrased with all of them: a
committee, nodes obeying the safety clauses, and the three conditions on the
block table.

Every other result is stated where the definitions it speaks about are
introduced. The table is the whole list, and says where each one is.

:::rows +header
*
  * Result
  * What it says
  * Stated in
*
  * {name NewProtocol.decideSafety}`decideSafety`
  * no forks: the `Cert2`-certified blocks form one chain
  * this section
*
  * {name NewProtocol.decideInv_reachable}`decideInv_reachable`
  * a decide is never taken back
  * {ref "decide-stream"}[What the application is promised]
*
  * {name NewProtocol.admitted_held}`admitted_held`
  * what a node admitted above the floor, it holds
  * {ref "invariants"}[What a node's state satisfies]
*
  * {name NewProtocol.cert1_unique}`cert1_unique`
  * at most one `Cert1` per view
  * {ref "network"}[The network, and what certificates guarantee]
*
  * {name NewProtocol.cert2_unique}`cert2_unique`
  * at most one `Cert2` per view
  * {ref "network"}[The network, and what certificates guarantee]
*
  * {name NewProtocol.cert2_implies_cert1}`cert2_implies_cert1`
  * a `Cert2` presupposes its `Cert1`
  * {ref "network"}[The network, and what certificates guarantee]
*
  * {name NewProtocol.timeoutCert_reached}`timeoutCert_reached`
  * a quorum timeout means an honest node was in that view
  * {ref "network"}[The network, and what certificates guarantee]
:::

Progress is not among them, and is not that kind of result. What a node owes is
{ref "when-owed"}[When an action is owed]; what an owed action is *worth* is
{ref "progress"}[What progress is worth], which is conditional throughout.

```lean -show
namespace Spec
```

:::spec NewProtocol.DecideSafety
  ```lean
  def DecideSafety (tree : NewProtocol.BlockTable)
      {cfg : NewProtocol.Config} {C : Committee} (N : Network cfg C) : Prop :=
    TreeCoherent tree → CollisionFree → Resolves tree N → HeightSucceedsParent tree N →
      AnchorRooted tree cfg →
      ∀ c c', Network.ValidCert2 cfg N c → Network.ValidCert2 cfg N c' →
        c.view ≤ c'.view → Ancestor tree c.data.blockHash c'.data.blockHash
  ```
:::

```lean -show
example : @Spec.DecideSafety = @NewProtocol.DecideSafety := rfl
end Spec
```

{includeDocstring NewProtocol.DecideSafety}


{docstring NewProtocol.decideSafety}

# What the application is promised

%%%
tag := "decide-stream"
%%%

A decide is never taken back. That is a property of one node's stream, not
agreement between nodes, which is what the previous section settles.

{docstring NewProtocol.DecideInv}

{docstring NewProtocol.decideInv_reachable}

# What a node's state satisfies

%%%
tag := "invariants"
%%%

The other invariant of a reachable state, and the same kind of claim: not an
obligation but a consequence of the clauses already collected, so an
implementation satisfies it whether it meant to or not. What it is for is the one
place two rules read different fields for the same purpose.
{name NewProtocol.Vote2Enabled}`Vote2Enabled` reads
{name NewProtocol.NodeState.admitted}`NodeState.admitted`, and casting the vote2
it licenses moves the lock, which
{name NewProtocol.SafetySpec.lockJustified}`SafetySpec.lockJustified` judges
against {name NewProtocol.NodeState.proposals}`NodeState.proposals`. Nothing in
either says the two agree.

{docstring NewProtocol.admitted_held}

# What progress is worth

%%%
tag := "progress"
%%%

{name NewProtocol.WeaklyFair}`WeaklyFair` says a node does not sit on an
obligation for ever. On its own that is a statement about a run and nothing about
the network: it does not say the obligation ever arises, nor that anything reaches
the node that would make it arise. This section is what can be proved without
either of the two assumptions the specification does not have — delivery and
duration.

Two directions, and they meet in the middle.

*From owed to done.* An action owed, and not overtaken, is taken. The hypothesis
is a *window*: the action is owed at some step, and until the node acts nothing
raises the bar past the view, times it out, moves the lock past it or prunes it.
Every field of a window is guarded by the action not having gone out yet, because
a node that acts *does* abandon the view afterwards: an unconditional window
would be one no progressing node is ever in. The guard reads the run's own
history rather than the freshness mark, which a collection may drop once the view
is abandoned.

*From delivered to owed.* Inputs the specification itself admits make the action
owed. This is the direction that cannot be read off the clause list: every
enabledness predicate is a conjunction of guards over state only an input can
write, and nothing says those conjunctions are jointly satisfiable. A guard no
input can establish would leave a rule that never obliges anything — safe, and
describing a protocol that never acts.

Composed, they are `vote1_forced` and its three companions: deliver the inputs to
a node whose window is open, and the action happens. Nothing here says the inputs
arrive.

:::rows +header
*
  * Result
  * What it says
*
  * {name NewProtocol.vote1_cast}`vote1_cast`
  * an owed vote1 is cast, for the block the node admitted
*
  * {name NewProtocol.vote2_cast}`vote2_cast`
  * an owed vote2 is cast, for the block the node admitted
*
  * {name NewProtocol.decide_delivered}`decide_delivered`
  * an owed decide is delivered, with the view in the chain
*
  * {name NewProtocol.propose_sent}`propose_sent`
  * an owed proposal is sent, in the view it was owed for
*
  * {name NewProtocol.cert1_forms}`cert1_forms`
  * a quorum's vote1s are the `Cert1`
*
  * {name NewProtocol.cert2_forms}`cert2_forms`
  * a quorum's vote2s are the `Cert2`
*
  * {name NewProtocol.quorum_on_chain}`quorum_on_chain`
  * and what it commits lies on the one chain
*
  * {name NewProtocol.vote1_unstalled}`vote1_unstalled`
  * deliver the proposal and its validity, and a vote1 is owed or already cast
*
  * {name NewProtocol.vote2_unstalled}`vote2_unstalled`
  * deliver the `Cert1` and the payload, and a vote2 is owed or already cast
*
  * {name NewProtocol.decide_unstalled}`decide_unstalled`
  * deliver the `Cert2`, and the decide is owed or already delivered
*
  * {name NewProtocol.propose_unstalled}`propose_unstalled`
  * deliver the block to propose, and the proposal is owed or already sent
*
  * {name NewProtocol.vote1_forced}`vote1_forced`
  * the two halves, composed: deliver, and the vote1 is cast
*
  * {name NewProtocol.vote2_forced}`vote2_forced`
  * likewise the vote2
*
  * {name NewProtocol.decide_forced}`decide_forced`
  * likewise the decide
*
  * {name NewProtocol.propose_forced}`propose_forced`
  * and likewise the proposal
:::

## The window

A window's fields are guarded by the action still being outstanding, which is
read off the run's own history rather than the freshness mark: a collection may
drop the mark once the view is abandoned, and nothing can undo an emission.

```lean -show
namespace Spec
```

:::spec NewProtocol.Vote1Pending
  ```lean
  def Vote1Pending {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (p : Proposal)
      (n m : Nat) : Prop :=
    ∀ i, n ≤ i → i < m → ∀ vote : Vote1,
      NewProtocol.Output.send (.vote1 vote) ∈ (Run.event r i).outputs
        → vote.view ≠ p.viewNumber
  ```
:::

:::spec NewProtocol.Vote2Pending
  ```lean
  def Vote2Pending {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (p : Proposal)
      (n m : Nat) : Prop :=
    ∀ i, n ≤ i → i < m → ∀ vote : Vote2,
      NewProtocol.Output.send (.vote2 vote) ∈ (Run.event r i).outputs
        → vote.view ≠ p.viewNumber
  ```
:::

:::spec NewProtocol.DecidePending
  ```lean
  def DecidePending {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (v : ViewNumber)
      (n m : Nat) : Prop :=
    ∀ i, n ≤ i → i < m → ∀ blocks c1 c2,
      NewProtocol.Output.decided blocks c1 c2 ∈ (Run.event r i).outputs
        → ∀ b ∈ blocks, b.viewNumber ≠ v
  ```
:::

:::spec NewProtocol.ProposePending
  ```lean
  def ProposePending {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (p : Proposal)
      (n m : Nat) : Prop :=
    ∀ i, n ≤ i → i < m → ∀ q : Proposal,
      NewProtocol.Output.send (.proposal q) ∈ (Run.event r i).outputs
        → q.viewNumber ≠ p.viewNumber
  ```
:::

```lean -show
example : @Spec.Vote1Pending = @NewProtocol.Vote1Pending := rfl
example : @Spec.Vote2Pending = @NewProtocol.Vote2Pending := rfl
example : @Spec.DecidePending = @NewProtocol.DecidePending := rfl
example : @Spec.ProposePending = @NewProtocol.ProposePending := rfl
end Spec
```

{docstring NewProtocol.LockAllows}

{docstring NewProtocol.Vote1Window}

{docstring NewProtocol.Vote2Window}

{docstring NewProtocol.DecideWindow}

{docstring NewProtocol.ProposeWindow}

{docstring NewProtocol.AnchorKept}

Each action being owed is its window together with the action being enabled at the
step the window opens. These four are the hypothesis every result below takes.

```lean -show
namespace Spec
```

:::spec NewProtocol.Vote1Owed
  ```lean
  def Vote1Owed {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
    ∃ n, Vote1Enabled (Run.state r n) p ∧ Vote1Window r p n
  ```
:::

:::spec NewProtocol.Vote2Owed
  ```lean
  def Vote2Owed {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
    ∃ n, Vote2Enabled cfg (Run.state r n) p ∧ Vote2Window r p n
  ```
:::

:::spec NewProtocol.DecideOwed
  ```lean
  def DecideOwed {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (v : ViewNumber) : Prop :=
    ∃ n, DecideEnabled cfg (Run.state r n) v ∧ DecideWindow r v n
  ```
:::

:::spec NewProtocol.ProposeOwed
  ```lean
  def ProposeOwed {cfg : NewProtocol.Config} {leader : ViewNumber → Option PubKey}
      {node : PubKey} (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
    ∃ n, ProposeEnabled cfg leader node (Run.state r n) p ∧ ProposeWindow r p n
  ```
:::

```lean -show
example : @Spec.Vote1Owed = @NewProtocol.Vote1Owed := rfl
example : @Spec.Vote2Owed = @NewProtocol.Vote2Owed := rfl
example : @Spec.DecideOwed = @NewProtocol.DecideOwed := rfl
example : @Spec.ProposeOwed = @NewProtocol.ProposeOwed := rfl
end Spec
```

{includeDocstring NewProtocol.Vote1Owed}

## A network that makes progress

{docstring NewProtocol.LiveNetwork}

## From owed to done

{docstring NewProtocol.vote1_cast}

{docstring NewProtocol.vote2_cast}

{docstring NewProtocol.decide_delivered}

{docstring NewProtocol.propose_sent}

## A quorum forms the certificate

{docstring NewProtocol.cert1_forms}

{docstring NewProtocol.cert2_forms}

{docstring NewProtocol.cert1_forms_of_owed}

{docstring NewProtocol.cert2_forms_of_owed}

{docstring NewProtocol.quorum_on_chain}

## From delivered to owed

The hypotheses come in two kinds, and the split is the point: what the arriving
input has to satisfy for the ingestion clauses to fire, and what the node must not
have done to itself in the meantime.

{docstring NewProtocol.ProposalAdmissible}

{docstring NewProtocol.Vote1Room}

{docstring NewProtocol.Vote2Room}

{docstring NewProtocol.ProposeRoom}

{docstring NewProtocol.ProposeReady}

```lean -show
namespace Spec
```

:::spec NewProtocol.ParentHeld
  ```lean
  def ParentHeld (s : NodeState) (p : Proposal) : Prop :=
    p.parentCert.view ≠ ViewNumber.genesis →
      ∃ parent, s.proposals p.parentCert.view = some parent
        ∧ p.parentCert.data.blockHash = blockHash parent
        ∧ s.blocksReconstructed p.parentCert.view parent.payloadCommit
  ```
:::

```lean -show
example : @Spec.ParentHeld = @NewProtocol.ParentHeld := rfl
end Spec
```

{includeDocstring NewProtocol.ParentHeld}

{docstring NewProtocol.vote1_owed_of_validated}

{docstring NewProtocol.vote2_owed_of_reconstructed}

{docstring NewProtocol.vote1_unstalled}

{docstring NewProtocol.vote2_unstalled}

{docstring NewProtocol.decide_unstalled}

{docstring NewProtocol.propose_unstalled}

## Delivery makes the node act

The two directions composed, once per action. `Run.Consumes` is what places a
delivery in a run: the step consumed that input, whatever else the schedule did
before or after it. The action may be taken during the delivery itself rather than
after it, and none of the four distinguishes the two — what is claimed is that it
happens, not when.

{docstring NewProtocol.vote1_forced}

{docstring NewProtocol.vote2_forced}

{docstring NewProtocol.decide_forced}

{docstring NewProtocol.propose_forced}

# A round completes

%%%
tag := "round"
%%%

The results above are hops. This section composes them into the round they belong
to: a quorum handed a proposal forms the {name NewProtocol.Cert1}`Cert1` over it,
a quorum handed that certificate forms the {name NewProtocol.Cert2}`Cert2`, and a
node handed *that* decides the view. Each hop's conclusion is the next hop's
hypothesis as the same object, which is what makes this a composition rather than
four statements about the same block.

It is still conditional throughout, and in the same two ways. Nothing says a
delivery happens, so every hop takes its arrivals as hypotheses. And nothing
carries a node's state from one hop to the next, since the steps between two
deliveries are unconstrained. So the admission clause of
{name NewProtocol.Vote2Delivery}`Vote2Delivery` is a hypothesis where reading the
round in order would suggest a consequence.

The leader's proposal is not part of it.
{name NewProtocol.propose_sent}`propose_sent` says a leader that owes a proposal
sends one, but a proposal sent and a proposal arriving at a node are joined only
by delivery. The round starts where the arrivals do.

## What a delivery supplies

One bundle per hop, each the hypotheses of the matching result collected into a
structure so that a statement about a quorum can quantify over them. Nothing is
added or weakened in the collecting.

{docstring NewProtocol.Vote1Delivery}

{docstring NewProtocol.Vote2Delivery}

{docstring NewProtocol.DecideDelivery}

## The hops, composed

{docstring NewProtocol.vote1_cast_of_delivered}

{docstring NewProtocol.vote2_cast_of_delivered}

{docstring NewProtocol.round_cert1}

{docstring NewProtocol.round_cert2}

{docstring NewProtocol.round_completes}

{docstring NewProtocol.round_decided}

# What conformance means

{docstring NewProtocol.Implements}

{docstring NewProtocol.Conforms}

{docstring NewProtocol.Run.weaken}

# How to audit this

Everything hinges on the statements. The proofs are checked by the Lean kernel,
but no machine checks that a statement says what was intended. So the definitions
are to be read and judged, and the premises challenged.

The source is arranged for that. A results module comes in up to three parts:
`X/Defs.lean` holds the definitions the statements are phrased with,
`X/Lemmas.lean` the kernel-checked scaffolding, and `X.lean` the results. An audit
reads the first and the third, and everything spliced into this document comes
from those. Nothing in `Lemmas` needs reading to judge what is claimed, and that
every result reaches this document is checked on every build rather than left to
whoever added one.

Nothing is assumed beyond what is collected above. There are no `axiom`s and no
`sorry`s, so the axiom footprint of every theorem here is Lean's own — `propext`,
`Classical.choice`, `Quot.sound` — and that is checked on every build rather than
asserted, along with the clause lists and the premise lists. Alongside them
`NewProtocolSpec.Checks.Examples` exhibits six states that owe an action, including on
the branches a first-view proposal would leave exempt — so the claim
{ref "progress"}[what progress is worth] rests on is checked rather than argued.

*Definitions — read and judge.* In particular:

* The types and the interface: do they faithfully represent the objects that really
  travel, given the deliberate reductions — signatures become propositions, hashing is
  opaque, cryptographic values are one-field wrappers?

* The rules: are {name NewProtocol.ProposalWellFormed}`ProposalWellFormed` and
  {name NewProtocol.SafeToExtend}`SafeToExtend` the right admission rules; are the
  justification gates complete; do the mark obligations and the input-triggered ones
  cover everything a node must not withhold; are the ingestion obligations and
  {name NewProtocol.StepSpec.contentRetained}`StepSpec.contentRetained` enough that a
  node cannot escape an obligation by never holding what triggers it; does each
  obligation say what its docstring claims; and does every bar on an output have a
  counterpart in the enabledness predicate that owes it? That last one is not
  decoration: an action a node is forbidden to emit must also not be owed, or the two
  obligations ask for opposite things and nothing can satisfy both
  ({name NewProtocol.StepSpec.vote1Bar}`StepSpec.vote1Bar` against
  {name NewProtocol.Vote1Enabled}`Vote1Enabled`,
  {name NewProtocol.SafetySpec.vote2NotInSkippedView}`SafetySpec.vote2NotInSkippedView` against
  {name NewProtocol.Vote2Enabled}`Vote2Enabled`).

* Pruning: is {name NewProtocol.GcSpec}`GcSpec` weak enough for a real implementation
  and strong enough that safety survives pruning? Note that what a node may discard is
  set by {name NewProtocol.StepSpec.contentRetained}`StepSpec.contentRetained`, which
  binds every step; {name NewProtocol.GcSpec}`GcSpec` governs only the spontaneous,
  input-free case.

* Progress: is {name NewProtocol.WeaklyFair}`WeaklyFair` the right progress assumption,
  and are the enabledness predicates exactly the actions a node owes? Are the windows
  of {ref "progress"}[what progress is worth] no more than what a node needs to be left
  alone with — each field a way the node itself moved on, none of them a stall?

* Conformance: is {name NewProtocol.Conforms}`Conforms` the whole of what an
  implementation owes? Its abstraction argument is trusted glue supplied by the
  implementation — a wrong abstraction makes conformance vacuous, so it is the first
  thing to read in any conformance claim.

* The omissions above: whether each is safe to omit is an audit judgment.

*Premises — challenge.* They are the ones collected above:
{name NewProtocol.TreeCoherent}`TreeCoherent` and
{name NewProtocol.CollisionFree}`CollisionFree` stated outright, and the
verification-layer fields of {name NewProtocol.Committee}`Committee` and
{name NewProtocol.Network}`Network`. Certificate uniqueness and cert2-implies-cert1 are
not among them: they are theorems.

{name NewProtocol.Resolves}`Resolves`,
{name NewProtocol.HeightSucceedsParent}`HeightSucceedsParent` and
{name NewProtocol.AnchorRooted}`AnchorRooted` are worth challenging alongside
them. All three are hypotheses of {name NewProtocol.DecideSafety}`DecideSafety`
rather than standing premises, and all three are conditions on the block table,
which is a device of the statement rather than a part of the protocol. The
statement says nothing without them: ancestry is followed through the table, so
an empty one would collapse it to equality of hashes, a table whose heights do
not succeed would let a branch cross an epoch boundary without passing through
it, and one that answers past the anchor would let a walk leave the chain.

Their satisfiability is the thing to check first, and the check is written down:
{name NewProtocol.Witness.certificate_exists}`Witness.certificate_exists` meets
every one of them in a network that certifies a block. Three times a premise
here could not be met, and each time every result guarded by it was true for no
reason while the build stayed green.

The node's seam with its environment is not a premise but an obligation: the
provenance clauses say nothing enters a node's state except through an input, and
the ingestion clauses say an input's content is taken.

What no reading can check is the correspondence between this formalisation and the
intended protocol. That gap is closed by the audit itself, and narrowed by
replaying a real execution against a machine proved to satisfy these rules.

Everything here is a proposition. That these obligations are jointly satisfiable is
not something the specification can tell you — it is discharged by exhibiting a
conforming implementation.
