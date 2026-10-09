import Verso
import VersoManual
import NewProtocolSpec
import Style

open Verso.Genre
-- The specification's `Config` and `Block` are the ones reproduced definitions mean.
open Verso.Genre.Manual hiding Config Block
open Verso.Genre.Manual.InlineLean
open NewProtocol

set_option verso.code.warnLineLength 90

-- The larger docstrings, such as `Synchrony`'s, exceed both default budgets.
set_option maxHeartbeats 1000000
set_option maxRecDepth 100000

#doc (Manual) "New protocol: consensus specification" =>

%%%
tag := "top"
%%%

_What an honest consensus node must do, what the network is assumed to do, and what follows._

Consensus runs in numbered views, each with a leader. The leader of a view
proposes one block on a parent, which it names by the parent's certificate. Every
member of the committee then votes on the proposal twice.

A vote1 says the proposal is a fit extension: the block is valid and its parent
is safe to build on, given what may have been committed. Beyond what safety needs,
an honest node votes1 only on a proposal from the view's leader, and owes a vote1
once it holds its own share of the payload and was told the block is valid. A
quorum of vote1s is a {name NewProtocol.Cert1}`Cert1`. With it, any node may enter the next view, and
the next leader may build on the block. A node that also has the block's payload
may lock on it. A node's _lock_ is the latest {name NewProtocol.Cert1}`Cert1` it could lock on
({name NewProtocol.History.LockedOn}`History.LockedOn`).

A vote2 says the node holds the block's {name NewProtocol.Cert1}`Cert1` and has rebuilt its payload from
the shares, so the payload is available. A quorum of vote2s is a {name NewProtocol.Cert2}`Cert2`, and a
block with a {name NewProtocol.Cert2}`Cert2` is _committed_. A node that holds the block and both its
certificates _decides_ it: it delivers the block to the application, and with it may deliver ancestors
it has not delivered yet, as far back as it holds them. An ancestor it does not hold is skipped;
{ref "valid-decides"}[Valid decides] says when a skipped block is delivered later.

A view that produces no {name NewProtocol.Cert1}`Cert1` in time is given up. Each node's timer fires, and
the node sends a timeout vote. The vote names a {name NewProtocol.Cert1}`Cert1` the node holds, no
earlier than any block it voted2 on, called the vote's lock. A node that sees
timeout votes for a view from enough nodes that one of them is honest (the
_one-honest indication_) times the view out too. A quorum of timeout votes is a
{name NewProtocol.TimeoutCert}`TimeoutCert`, whose lock is the latest of its votes' locks. The next view starts
on it, and its leader must build on a block no earlier than the certificate's lock.
Timeout votes go to every node, so the honest signers of a certificate draw every
other node's vote through the one-honest indication, and the certificate forms
everywhere without being sent on
({name NewProtocol.Synchrony.timeoutOneHonest}`Synchrony.timeoutOneHonest`). Only a node handed a certificate
without having voted for its view sends it on
({name NewProtocol.Synchrony.timeoutCertForward}`Synchrony.timeoutCertForward`), and a node that keeps timing a
view out after others left it is answered with what took them past it
({name NewProtocol.Synchrony.timeoutCatchUp}`Synchrony.timeoutCatchUp`).

Blocks are grouped into epochs of a fixed number of blocks, and each epoch has its
own committee. The last block of an epoch must be committed by its own committee
before the next epoch starts. When it has a {name NewProtocol.Cert1}`Cert1` but no {name NewProtocol.Cert2}`Cert2`, the leader
asks the outgoing committee to vote on it again in a later view (a {name NewProtocol.RevoteRequest}`RevoteRequest`).
{ref "epochs"}[Crossing an epoch boundary] gives an overview.

*What is proved.* {name NewProtocol.NoFork}`NoFork`, {name NewProtocol.DecideAgreement}`DecideAgreement` and {name NewProtocol.DecidesValid}`DecidesValid` hold at all times,
before GST too, given the signing rules, genuine certificates, quorums of an epoch
that share an honest member, no hash collisions and a coherent configuration.
{name NewProtocol.ChainGrows}`ChainGrows` holds eventually after GST, given in addition the whole
protocol, promptness within `δ`, synchrony with bound `Δ`, a view timer `τ` longer
than `8Δ + 3δ`, honest quorums in every epoch, finitely many honest nodes, and
leaders honest in each epoch infinitely often. {ref "proved"}[What is proved] is the
one complete list of each result's premises.

*How the rules are stated.* Every rule is a property of one node's _history_: the inputs it received and the
outputs it produced, step by step. What a node holds, what it is locked on, and
which view and epoch it is in are derived from that history, not stored. So the
rules say nothing about how a node keeps state, and a recorded trace of a real node
can be checked against them directly.

The rules come in layers, each read by a different result:

* {name NewProtocol.SafeHistory}`SafeHistory`: when a node may sign a vote, and what a decide must show. The
  no-fork result reads nothing else of a node.
* {name NewProtocol.ProtocolHistory}`ProtocolHistory`: the rest of what a node may do, such as timing out and
  proposing. Liveness needs these.
* {name NewProtocol.Owed}`Owed`: what a node must eventually do. With a bound on how long anything stays
  owed ({name NewProtocol.Prompt}`Prompt`), liveness needs this too.

Everything shown here is spliced from the declaration that states it, and every
reproduced definition is checked against the source when this document is built.
The prose orders and connects them; it does not restate them. Read from the top,
each definition comes before its uses. The overview of epoch boundaries names
definitions that come after it, and links to them.

# Numbers and epochs

Views, epochs and block heights are separate types, so that no rule can mix
them up. A run has more views than blocks: a view that times out produces none.

{docstring NewProtocol.ViewNumber}

{docstring NewProtocol.ViewNumber.genesis}

{docstring NewProtocol.EpochNumber}

{docstring NewProtocol.BlockNumber}

```lean -show
namespace Spec.epochOf
open NewProtocol.History
```

:::spec NewProtocol.epochOf
  ```lean
  def epochOf (blockNumber : BlockNumber) (height : Nat) : EpochNumber :=
    if height = 0 then 0
    else if blockNumber = 0 then 1
    else if blockNumber.toNat % height = 0 then ⟨blockNumber.toNat / height⟩
    else ⟨blockNumber.toNat / height + 1⟩
  ```
:::

```lean -show
example : @Spec.epochOf.epochOf = @NewProtocol.epochOf := rfl
end Spec.epochOf
```

{includeDocstring NewProtocol.epochOf}

```lean -show
namespace Spec.IsLastBlock
open NewProtocol.History
```

:::spec NewProtocol.IsLastBlock
  ```lean
  def IsLastBlock (blockNumber : BlockNumber) (height : Nat) : Prop :=
    blockNumber ≠ 0 ∧ height ≠ 0 ∧ blockNumber.toNat % height = 0
  ```
:::

```lean -show
example : @Spec.IsLastBlock.IsLastBlock = @NewProtocol.IsLastBlock := rfl
end Spec.IsLastBlock
```

{includeDocstring NewProtocol.IsLastBlock}

The fact about this arithmetic the safety argument uses: from one block to the
next, the epoch either stays the same or increases by one, and it increases exactly
after a last block.

{docstring NewProtocol.epochOf_succ}

# The data

What nodes send each other. These reductions apply throughout. Hashes, keys and
commitments are opaque values that are only compared. Signatures are not modelled:
a vote names its signer, and a certificate is its data and its view. And a proposal
and the block it proposes are one object.

First the identifiers: a block's hash, its payload's commitment, and a node's key.

{docstring NewProtocol.BlockHash}

{docstring NewProtocol.PayloadCommit}

{docstring NewProtocol.PubKey}

A proposal carries the block header, its view, epoch and parent certificate, and
is the block itself.

{docstring NewProtocol.BlockHeader}

{docstring NewProtocol.Proposal}

{docstring NewProtocol.Block}

{expansion NewProtocol.Block}

{docstring NewProtocol.Proposal.payloadCommit}

{docstring NewProtocol.blockHash}

Validity and VID shares are reduced to what consensus reads of them.

{docstring NewProtocol.BlockValid}

{docstring NewProtocol.VidShare}

Votes and certificates. A vote1 and a vote2 sign the same fields, kept as two types
so that no rule can take a vote of one round for one of the other.

{docstring NewProtocol.Vote1Data}

{docstring NewProtocol.Vote2Data}

{docstring NewProtocol.Vote1Data.toVote2}

{docstring NewProtocol.TimeoutData}

{docstring NewProtocol.Vote}

{docstring NewProtocol.Vote1}

{expansion NewProtocol.Vote1}

{docstring NewProtocol.Vote2}

{expansion NewProtocol.Vote2}

{docstring NewProtocol.TimeoutVote}

{expansion NewProtocol.TimeoutVote}

{docstring NewProtocol.Certificate}

{docstring NewProtocol.Cert1}

{expansion NewProtocol.Cert1}

{docstring NewProtocol.Cert2}

{expansion NewProtocol.Cert2}

{docstring NewProtocol.TimeoutCert}

{expansion NewProtocol.TimeoutCert}

The last kind of data is a leader's request to vote again on an epoch's last block,
for when that block has a {name NewProtocol.Cert1}`Cert1` but no {name NewProtocol.Cert2}`Cert2` ({ref "epochs"}[Crossing an epoch boundary]).

{docstring NewProtocol.RevoteRequest}

# Configuration, inputs and outputs

The configuration is fixed before the network starts. It names the anchor block,
which nothing in the run votes on, and how many blocks an epoch holds. The anchor
can be at any view and height: the genesis block, or a block an earlier run
decided. A run starts at the anchor's view, and no rule reads an earlier one.

{docstring NewProtocol.Config}

{docstring NewProtocol.Config.anchorView}

{expansion NewProtocol.Config.anchorView}

The anchor is decided. The configuration vouches for a `Cert2` over it, and if it is
the last block of its epoch, the run starts in the next epoch.

{docstring NewProtocol.Config.anchorCert2}

{docstring NewProtocol.Config.startEpoch}

{docstring NewProtocol.ConfigCoherent}

A node's inputs are moments at which it comes to know something: a proposal
arrived, a payload is in hand, a block was found valid, a timer fired. A
certificate is an input whether the node assembled it from votes or was handed it.
Certificates arriving as inputs are already verified, which is what the network
assumptions below say.

{docstring NewProtocol.Input}

{docstring NewProtocol.Input.cert1}

{docstring NewProtocol.Input.cert2}

{docstring NewProtocol.Input.timeoutCert}

A node's outputs are messages to its peers and blocks delivered to the application.
No message names a recipient; who receives what, and when, is the network's part
({name NewProtocol.Synchrony}`Synchrony`).

{docstring NewProtocol.Message}

{docstring NewProtocol.Message.leaderView}

{docstring NewProtocol.Message.leaderEpoch}

{docstring NewProtocol.Input.sentBy}

{docstring NewProtocol.Output}

# Crossing an epoch boundary

%%%
tag := "epochs"
%%%

Most of the rules below have a case for epochs. This section gives an overview of
them, in the order a boundary is crossed. The definitions it names come later.

*The last block.* Block heights fix where each epoch ends ({name NewProtocol.IsLastBlock}`IsLastBlock`). A
proposal's epoch is the one its height falls in ({name NewProtocol.ProposalWellFormed}`ProposalWellFormed`), and the
proposal after a last block opens the next epoch ({name NewProtocol.EntersEpoch}`EntersEpoch`).

*Ending the epoch.* The next epoch may start only once the outgoing committee has
decided its last block. A proposal opening an epoch must name its parent by the
parent's own {name NewProtocol.Cert1}`Cert1`, at the parent's view, and its leader must hold a {name NewProtocol.Cert2}`Cert2`
over the parent ({name NewProtocol.OpensEpochJustified}`OpensEpochJustified`). An honest node votes1 for such a proposal only
when it holds that {name NewProtocol.Cert2}`Cert2` too. So a branch crosses a boundary only at a decided
block, and the safety argument never has to intersect the quorums of two committees.

*The re-vote.* If the last block has a {name NewProtocol.Cert1}`Cert1` but no {name NewProtocol.Cert2}`Cert2`, the next view's
leader may ask the outgoing committee to vote on it again ({name NewProtocol.RevoteRequest}`RevoteRequest`,
{name NewProtocol.RevoteWellFormed}`RevoteWellFormed`, {name NewProtocol.RevoteJustified}`RevoteJustified`). The vote1s form a {name NewProtocol.Cert1}`Cert1` over the
same block at the later view, and vote2s on that a {name NewProtocol.Cert2}`Cert2`, which commits the
block ({name NewProtocol.Commits}`Commits` allows a later view). The next epoch's first block still names
the block's own {name NewProtocol.Cert1}`Cert1`, not the re-vote's: the outgoing committee can form
re-vote certificates after the next epoch began, and building on one of those could
start the epoch a second time.

*The epoch change.* The last block travels with its own {name NewProtocol.Cert1}`Cert1` and a {name NewProtocol.Cert2}`Cert2` as
an epoch change ({name NewProtocol.Input.epochChange}`Input.epochChange`, {name NewProtocol.EpochChangeWellFormed}`EpochChangeWellFormed`). Once one honest
node holds the block's proposal and the {name NewProtocol.Cert2}`Cert2`, every node honest in that epoch or later
receives the epoch change within `Δ` of that, or of GST if that is later
({name NewProtocol.Synchrony.epochChange}`Synchrony.epochChange`). A node that takes one has grounds for the next epoch and
for the view after the {name NewProtocol.Cert2}`Cert2`, and may lock on the block without its payload
({name NewProtocol.History.Lockable}`History.Lockable`). That is what lets a node new to the incoming committee,
which holds no share of the outgoing epoch's blocks, start voting.

*Staying in step.* Re-votes of the outgoing committee and the views of the next
epoch must not mix. So a node votes only for its own epoch or a later one
({name NewProtocol.NotBehind}`NotBehind`), and locks and committed blocks are ordered by epoch first
({name NewProtocol.LockLE}`LockLE`, {name NewProtocol.CertNotAfter}`CertNotAfter`).

# Well-formed data

Conditions on a piece of data alone, with no node involved. A node checks these on
what it receives, and the proofs read them off the certificates.

{docstring NewProtocol.ProposalWellFormed}

```lean -show
namespace Spec.EntersEpoch
open NewProtocol.History
```

:::spec NewProtocol.EntersEpoch
  ```lean
  def EntersEpoch (cfg : Config) (p : Proposal) : Prop :=
    IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
  ```
:::

```lean -show
example : @Spec.EntersEpoch.EntersEpoch = @NewProtocol.EntersEpoch := rfl
end Spec.EntersEpoch
```

{includeDocstring NewProtocol.EntersEpoch}

{docstring NewProtocol.EpochChangeWellFormed}

```lean -show
namespace Spec.ShareMatches
open NewProtocol.History
```

:::spec NewProtocol.ShareMatches
  ```lean
  def ShareMatches (p : Proposal) (vid : VidShare) : Prop :=
    vid.view = p.viewNumber ∧ p.payloadCommit = vid.payloadCommit
  ```
:::

```lean -show
example : @Spec.ShareMatches.ShareMatches = @NewProtocol.ShareMatches := rfl
end Spec.ShareMatches
```

{includeDocstring NewProtocol.ShareMatches}

```lean -show
namespace Spec.Certifies
open NewProtocol.History
```

:::spec NewProtocol.Certifies
  ```lean
  def Certifies (c : Cert1) (b : Block) : Prop :=
    b.viewNumber ≤ c.view ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩
  ```
:::

```lean -show
example : @Spec.Certifies.Certifies = @NewProtocol.Certifies := rfl
end Spec.Certifies
```

{includeDocstring NewProtocol.Certifies}

```lean -show
namespace Spec.Commits
open NewProtocol.History
```

:::spec NewProtocol.Commits
  ```lean
  def Commits (c : Cert2) (b : Block) : Prop :=
    b.viewNumber ≤ c.view ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩
  ```
:::

```lean -show
example : @Spec.Commits.Commits = @NewProtocol.Commits := rfl
end Spec.Commits
```

{includeDocstring NewProtocol.Commits}

```lean -show
namespace Spec.Vote1For
open NewProtocol.History
```

:::spec NewProtocol.Vote1For
  ```lean
  def Vote1For (vote : Vote1) (b : Block) : Prop :=
    vote.view = b.viewNumber ∧ vote.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩
  ```
:::

```lean -show
example : @Spec.Vote1For.Vote1For = @NewProtocol.Vote1For := rfl
end Spec.Vote1For
```

{includeDocstring NewProtocol.Vote1For}

```lean -show
namespace Spec.RevoteWellFormed
open NewProtocol.History
```

:::spec NewProtocol.RevoteWellFormed
  ```lean
  def RevoteWellFormed (cfg : Config) (r : RevoteRequest) : Prop :=
    r.cert.view < r.view
      ∧ ((r.timeoutEvidence = none ∧ r.cert.view + 1 = r.view)
          ∨ ∃ tc, r.timeoutEvidence = some tc ∧ tc.view + 1 = r.view)
      ∧ IsLastBlock r.cert.data.blockNumber cfg.epochHeight
  ```
:::

```lean -show
example : @Spec.RevoteWellFormed.RevoteWellFormed = @NewProtocol.RevoteWellFormed := rfl
end Spec.RevoteWellFormed
```

{includeDocstring NewProtocol.RevoteWellFormed}

```lean -show
namespace Spec.Vote1Again
open NewProtocol.History
```

:::spec NewProtocol.Vote1Again
  ```lean
  def Vote1Again (vote : Vote1) (r : RevoteRequest) : Prop :=
    vote.view = r.view ∧ vote.data = r.cert.data
  ```
:::

```lean -show
example : @Spec.Vote1Again.Vote1Again = @NewProtocol.Vote1Again := rfl
end Spec.Vote1Again
```

{includeDocstring NewProtocol.Vote1Again}

Certificates are ordered by epoch first and by view within an epoch. Ordering by
view alone would let a re-vote of an epoch's last block, cast at a late view by the
outgoing committee, outrank certificates of the next epoch.

```lean -show
namespace Spec.EpochViewLE
```

:::spec NewProtocol.EpochViewLE
  ```lean
  def EpochViewLE (e : EpochNumber) (v : ViewNumber) (e' : EpochNumber)
      (v' : ViewNumber) : Prop :=
    e < e' ∨ (e = e' ∧ v ≤ v')
  ```
:::

```lean -show
example : @Spec.EpochViewLE.EpochViewLE = @NewProtocol.EpochViewLE := rfl
end Spec.EpochViewLE
```

{includeDocstring NewProtocol.EpochViewLE}

```lean -show
namespace Spec.LockLE
open NewProtocol.History
```

:::spec NewProtocol.LockLE
  ```lean
  def LockLE (a b : Cert1) : Prop := EpochViewLE a.data.epoch a.view b.data.epoch b.view
  ```
:::

```lean -show
example : @Spec.LockLE.LockLE = @NewProtocol.LockLE := rfl
end Spec.LockLE
```

{includeDocstring NewProtocol.LockLE}

```lean -show
namespace Spec.LockAllows
open NewProtocol.History
```

:::spec NewProtocol.LockAllows
  ```lean
  def LockAllows (lock pc : Cert1) (e : EpochNumber) : Prop :=
    lock.data.epoch < e
      ∨ (lock.data.epoch = pc.data.epoch ∧ (lock.view ≤ pc.view ∨ lock.data = pc.data))
  ```
:::

```lean -show
example : @Spec.LockAllows.LockAllows = @NewProtocol.LockAllows := rfl
end Spec.LockAllows
```

{includeDocstring NewProtocol.LockAllows}

```lean -show
namespace Spec.ChainLinked
open NewProtocol.History
```

:::spec NewProtocol.ChainLinked
  ```lean
  def ChainLinked : List Block → Prop
    | [] => True
    | [_] => True
    | b :: b' :: rest =>
        b'.viewNumber ≤ b.parentCert.view ∧ b.parentCert.data.blockHash = blockHash b'
          ∧ ChainLinked (b' :: rest)
  ```
:::

```lean -show
example : @Spec.ChainLinked.ChainLinked = @NewProtocol.ChainLinked := by
  funext l
  induction l with
  | nil => rfl
  | cons b rest ih =>
    cases rest with
    | nil => rfl
    | cons b' r => simp only [ChainLinked, NewProtocol.ChainLinked, ih]
end Spec.ChainLinked
```

{includeDocstring NewProtocol.ChainLinked}

# A node's history

%%%
tag := "history"
%%%

A node is described by its history. Everything the rules read about a node is
derived from it. None of these is a field an implementation must keep; they are
facts about what it has seen.

{docstring NewProtocol.Step}

{docstring NewProtocol.Step.take}

{expansion NewProtocol.Step.take}

{docstring NewProtocol.History}

{expansion NewProtocol.History}

A rule reads what the node knew when it acted by cutting the history at a step.

{docstring NewProtocol.History.upTo}

## What arrived and what went out

```lean -show
namespace Spec.History_Received
open NewProtocol.History
```

:::spec NewProtocol.History.Received
  ```lean
  def History.Received (h : History) (i : Input) : Prop := ∃ st ∈ h, st.input = i
  ```
:::

```lean -show
example : @Spec.History_Received.History.Received = @NewProtocol.History.Received := rfl
end Spec.History_Received
```

{includeDocstring NewProtocol.History.Received}

```lean -show
namespace Spec.History_Sent
open NewProtocol.History
```

:::spec NewProtocol.History.Sent
  ```lean
  def History.Sent (h : History) (m : Message) : Prop :=
    ∃ st ∈ h, Output.send m ∈ st.output
  ```
:::

```lean -show
example : @Spec.History_Sent.History.Sent = @NewProtocol.History.Sent := rfl
end Spec.History_Sent
```

{includeDocstring NewProtocol.History.Sent}

```lean -show
namespace Spec.History_SentAt
open NewProtocol.History
```

:::spec NewProtocol.History.SentAt
  ```lean
  def History.SentAt (h : History) (n : Nat) (m : Message) : Prop :=
    ∃ st, h[n]? = some st ∧ Output.send m ∈ st.output
  ```
:::

```lean -show
example : @Spec.History_SentAt.History.SentAt = @NewProtocol.History.SentAt := rfl
end Spec.History_SentAt
```

{includeDocstring NewProtocol.History.SentAt}

```lean -show
namespace Spec.History_DecidedView
open NewProtocol.History
```

:::spec NewProtocol.History.DecidedView
  ```lean
  def History.DecidedView (h : History) (v : ViewNumber) : Prop :=
    ∃ st ∈ h, ∃ blocks c1 c2 b, Output.decided blocks c1 c2 ∈ st.output ∧ b ∈ blocks
      ∧ b.viewNumber = v
  ```
:::

```lean -show
example : @Spec.History_DecidedView.History.DecidedView = @NewProtocol.History.DecidedView := rfl
end Spec.History_DecidedView
```

{includeDocstring NewProtocol.History.DecidedView}

```lean -show
namespace Spec.History_TimedOut
open NewProtocol.History
```

:::spec NewProtocol.History.TimedOut
  ```lean
  def History.TimedOut (h : History) (v : ViewNumber) : Prop :=
    ∃ vote : TimeoutVote, Sent h (.timeoutVote vote) ∧ v ≤ vote.view
  ```
:::

```lean -show
example : @Spec.History_TimedOut.History.TimedOut = @NewProtocol.History.TimedOut := rfl
end Spec.History_TimedOut
```

{includeDocstring NewProtocol.History.TimedOut}

## What the node holds

```lean -show
namespace Spec.History_HasCert1
open NewProtocol.History
```

:::spec NewProtocol.History.HasCert1
  ```lean
  def History.HasCert1 (cfg : Config) (h : History) (c : Cert1) : Prop :=
    c = cfg.anchorCert ∨ Received h (.certificate1 c)
      ∨ ∃ c2 p, Received h (.epochChange c c2 p)
  ```
:::

```lean -show
example : @Spec.History_HasCert1.History.HasCert1 = @NewProtocol.History.HasCert1 := rfl
end Spec.History_HasCert1
```

{includeDocstring NewProtocol.History.HasCert1}

```lean -show
namespace Spec.History_HasCert2
open NewProtocol.History
```

:::spec NewProtocol.History.HasCert2
  ```lean
  def History.HasCert2 (h : History) (c : Cert2) : Prop :=
    Received h (.certificate2 c) ∨ ∃ c1 p, Received h (.epochChange c1 c p)
  ```
:::

```lean -show
example : @Spec.History_HasCert2.History.HasCert2 = @NewProtocol.History.HasCert2 := rfl
end Spec.History_HasCert2
```

{includeDocstring NewProtocol.History.HasCert2}

```lean -show
namespace Spec.History_HasProposal
open NewProtocol.History
```

:::spec NewProtocol.History.HasProposal
  ```lean
  def History.HasProposal (cfg : Config) (h : History) (p : Proposal) :
      Prop :=
    p = cfg.anchorBlock ∨ (∃ sender share, Received h (.proposal sender p share))
      ∨ ∃ c1 c2, Received h (.epochChange c1 c2 p)
  ```
:::

```lean -show
example : @Spec.History_HasProposal.History.HasProposal = @NewProtocol.History.HasProposal := rfl
end Spec.History_HasProposal
```

{includeDocstring NewProtocol.History.HasProposal}

```lean -show
namespace Spec.History_HasPayload
open NewProtocol.History
```

:::spec NewProtocol.History.HasPayload
  ```lean
  def History.HasPayload (cfg : Config) (h : History) (v : ViewNumber)
      (pc : PayloadCommit) : Prop :=
    (v = cfg.anchorView ∧ pc = cfg.anchorBlock.payloadCommit)
      ∨ Received h (.blockReconstructed v pc)
  ```
:::

```lean -show
example : @Spec.History_HasPayload.History.HasPayload = @NewProtocol.History.HasPayload := rfl
end Spec.History_HasPayload
```

{includeDocstring NewProtocol.History.HasPayload}

```lean -show
namespace Spec.History_TookEpochChange
open NewProtocol.History
```

:::spec NewProtocol.History.TookEpochChange
  ```lean
  def History.TookEpochChange (cfg : Config) (h : History) (c1 : Cert1)
      (c2 : Cert2) (p : Proposal) : Prop :=
    Received h (.epochChange c1 c2 p) ∧ EpochChangeWellFormed cfg c1 c2 p
  ```
:::

```lean -show
example : @Spec.History_TookEpochChange.History.TookEpochChange = @NewProtocol.History.TookEpochChange := rfl
end Spec.History_TookEpochChange
```

{includeDocstring NewProtocol.History.TookEpochChange}

## The lock

The lock is not state either. It is the latest certificate, in {name NewProtocol.LockLE}`LockLE` order,
the node could lock on, and what it could lock on is fixed by what it holds. A node
needs a block's payload to lock on it, except for the last block of an epoch handed
to it in an epoch change, whose {name NewProtocol.Cert2}`Cert2` shows that a quorum had the payload.

```lean -show
namespace Spec.History_Lockable
open NewProtocol.History
```

:::spec NewProtocol.History.Lockable
  ```lean
  def History.Lockable (cfg : Config) (h : History) (c : Cert1) : Prop :=
    c = cfg.anchorCert
      ∨ (HasCert1 cfg h c
          ∧ ∃ b, HasProposal cfg h b ∧ Certifies c b
            ∧ HasPayload cfg h b.viewNumber b.payloadCommit)
      ∨ ∃ c2 p, TookEpochChange cfg h c c2 p
  ```
:::

```lean -show
example : @Spec.History_Lockable.History.Lockable = @NewProtocol.History.Lockable := rfl
end Spec.History_Lockable
```

{includeDocstring NewProtocol.History.Lockable}

```lean -show
namespace Spec.History_Buildable
open NewProtocol.History
```

:::spec NewProtocol.History.Buildable
  ```lean
  def History.Buildable (cfg : Config) (h : History) (c : Cert1) : Prop :=
    c = cfg.anchorCert ∨ (HasCert1 cfg h c ∧ ∃ b, HasProposal cfg h b ∧ Certifies c b)
  ```
:::

```lean -show
example : @Spec.History_Buildable.History.Buildable = @NewProtocol.History.Buildable := rfl
end Spec.History_Buildable
```

{includeDocstring NewProtocol.History.Buildable}

```lean -show
namespace Spec.History_LockedOn
open NewProtocol.History
```

:::spec NewProtocol.History.LockedOn
  ```lean
  def History.LockedOn (cfg : Config) (h : History) (c : Cert1) : Prop :=
    Lockable cfg h c ∧ ∀ c', Lockable cfg h c' → LockLE c' c
  ```
:::

```lean -show
example : @Spec.History_LockedOn.History.LockedOn = @NewProtocol.History.LockedOn := rfl
end Spec.History_LockedOn
```

{includeDocstring NewProtocol.History.LockedOn}

## Where the node is

The node's view and epoch are the latest it has grounds for. A {name NewProtocol.Cert1}`Cert1` alone is
grounds for the next view, whether or not the node can rebuild the block: a node
shown a certificate follows it. A {name NewProtocol.Cert1}`Cert1` over an epoch's last block is not
grounds for the next epoch, since it does not make the block final.

```lean -show
namespace Spec.History_ViewGround
open NewProtocol.History
```

:::spec NewProtocol.History.ViewGround
  ```lean
  def History.ViewGround (cfg : Config) (h : History) (v : ViewNumber) : Prop :=
    (∃ c, HasCert1 cfg h c ∧ v = c.view + 1)
      ∨ (∃ tc, Received h (.timeoutCertificate tc) ∧ v = tc.view + 1)
      ∨ ∃ c1 c2 p, TookEpochChange cfg h c1 c2 p ∧ v = c2.view + 1
  ```
:::

```lean -show
example : @Spec.History_ViewGround.History.ViewGround = @NewProtocol.History.ViewGround := rfl
end Spec.History_ViewGround
```

{includeDocstring NewProtocol.History.ViewGround}

```lean -show
namespace Spec.History_InView
open NewProtocol.History
```

:::spec NewProtocol.History.InView
  ```lean
  def History.InView (cfg : Config) (h : History) (v : ViewNumber) : Prop :=
    ViewGround cfg h v ∧ ∀ v', ViewGround cfg h v' → v' ≤ v
  ```
:::

```lean -show
example : @Spec.History_InView.History.InView = @NewProtocol.History.InView := rfl
end Spec.History_InView
```

{includeDocstring NewProtocol.History.InView}

```lean -show
namespace Spec.History_EpochGround
open NewProtocol.History
```

:::spec NewProtocol.History.EpochGround
  ```lean
  def History.EpochGround (cfg : Config) (h : History) (e : EpochNumber) :
      Prop :=
    e = cfg.startEpoch
      ∨ (∃ c1 c2 p, TookEpochChange cfg h c1 c2 p ∧ e = c2.data.epoch + 1)
      ∨ (∃ tc, Received h (.timeoutCertificate tc) ∧ e = tc.data.epoch)
      ∨ ∃ c, HasCert1 cfg h c
          ∧ c.data.epoch = epochOf c.data.blockNumber cfg.epochHeight ∧ e = c.data.epoch
  ```
:::

```lean -show
example : @Spec.History_EpochGround.History.EpochGround = @NewProtocol.History.EpochGround := rfl
end Spec.History_EpochGround
```

{includeDocstring NewProtocol.History.EpochGround}

```lean -show
namespace Spec.History_InEpoch
open NewProtocol.History
```

:::spec NewProtocol.History.InEpoch
  ```lean
  def History.InEpoch (cfg : Config) (h : History) (e : EpochNumber) : Prop :=
    EpochGround cfg h e ∧ ∀ e', EpochGround cfg h e' → e' ≤ e
  ```
:::

```lean -show
example : @Spec.History_InEpoch.History.InEpoch = @NewProtocol.History.InEpoch := rfl
end Spec.History_InEpoch
```

{includeDocstring NewProtocol.History.InEpoch}

```lean -show
namespace Spec.History_PastView
open NewProtocol.History
```

:::spec NewProtocol.History.PastView
  ```lean
  def History.PastView (h : History) (v : ViewNumber) : Prop :=
    h.TimedOut v ∨ ∃ tc, h.Received (.timeoutCertificate tc) ∧ v ≤ tc.view
  ```
:::

```lean -show
example : @Spec.History_PastView.History.PastView = @NewProtocol.History.PastView := rfl
end Spec.History_PastView
```

{includeDocstring NewProtocol.History.PastView}

```lean -show
namespace Spec.History_AfterFloor
open NewProtocol.History
```

:::spec NewProtocol.History.AfterFloor
  ```lean
  def History.AfterFloor (cfg : Config) (h : History) (v : ViewNumber) : Prop :=
    cfg.anchorView < v ∧ ∀ w, DecidedView h w → w - cfg.decideBuffer < v
  ```
:::

```lean -show
example : @Spec.History_AfterFloor.History.AfterFloor = @NewProtocol.History.AfterFloor := rfl
end Spec.History_AfterFloor
```

{includeDocstring NewProtocol.History.AfterFloor}

# The signing rules

%%%
tag := "signing"
%%%

When a node may sign, and what a decide must show, for the epochs the node is honest
in. Each rule applies when the node is honest in the epoch of every message the rule
relates. This is all the no-fork result reads of a node.

Within a view, at most one {name NewProtocol.Cert1}`Cert1` of an epoch forms: an honest node votes1 at most
once per epoch and view ({name NewProtocol.SafeHistory.vote1Once}`SafeHistory.vote1Once`), and two quorums of an epoch
share a node honest in it. Across views, these rules of {name NewProtocol.SafeHistory}`SafeHistory`, shown
below, keep a committed view from being skipped:

* A vote1 on a proposal that follows a timeout checks the parent against the
  timeout certificate's lock ({name NewProtocol.SafeParent}`SafeParent`).
* A timeout vote names a lock at least as late as every view the node voted2 in
  ({name NewProtocol.SafeHistory.timeoutLock}`SafeHistory.timeoutLock`).
* No vote2 follows a timeout of its view ({name NewProtocol.SafeHistory.vote2BeforeTimeout}`SafeHistory.vote2BeforeTimeout`).

Suppose a quorum committed a block at view `x`, and a later view's proposal skips
past `x` behind a timeout certificate of the same epoch. The certificate's quorum and
the committing quorum share a node honest in that epoch. That node voted2 at `x` before it timed out, so its
timeout vote's lock is at `x` or later, and so is the certificate's, which is at
least as late as each of its signers' locks. An honest vote1 on the proposal then requires its
parent to be no earlier than that lock, so the proposal cannot skip `x`.

```lean -show
namespace Spec.SafeEvidence
open NewProtocol.History
```

:::spec NewProtocol.SafeEvidence
  ```lean
  def SafeEvidence (ev : Option TimeoutCert) (c : Cert1) (e : EpochNumber) : Prop :=
    ∀ tc, ev = some tc → tc.data.epoch = e ∧ LockAllows tc.data.lock c e
  ```
:::

```lean -show
example : @Spec.SafeEvidence.SafeEvidence = @NewProtocol.SafeEvidence := rfl
end Spec.SafeEvidence
```

{includeDocstring NewProtocol.SafeEvidence}

```lean -show
namespace Spec.SafeParent
open NewProtocol.History
```

:::spec NewProtocol.SafeParent
  ```lean
  def SafeParent (p : Proposal) : Prop :=
    SafeEvidence p.timeoutEvidence p.parentCert p.epoch
  ```
:::

```lean -show
example : @Spec.SafeParent.SafeParent = @NewProtocol.SafeParent := rfl
end Spec.SafeParent
```

{includeDocstring NewProtocol.SafeParent}

```lean -show
namespace Spec.SafeRevote
open NewProtocol.History
```

:::spec NewProtocol.SafeRevote
  ```lean
  def SafeRevote (r : RevoteRequest) : Prop :=
    SafeEvidence r.timeoutEvidence r.cert r.cert.data.epoch
  ```
:::

```lean -show
example : @Spec.SafeRevote.SafeRevote = @NewProtocol.SafeRevote := rfl
end Spec.SafeRevote
```

{includeDocstring NewProtocol.SafeRevote}

```lean -show
namespace Spec.OpensEpochJustified
open NewProtocol.History
```

:::spec NewProtocol.OpensEpochJustified
  ```lean
  def OpensEpochJustified (cfg : Config) (h : History) (p : Proposal) : Prop :=
    EntersEpoch cfg p → (∃ parent, h.HasProposal cfg parent
        ∧ parent.viewNumber = p.parentCert.view
        ∧ p.parentCert.data.blockHash = blockHash parent)
      ∧ ∃ c2, (h.HasCert2 c2 ∨ c2 = cfg.anchorCert2) ∧ c2.view < p.viewNumber
        ∧ c2.data = p.parentCert.data.toVote2
  ```
:::

```lean -show
example : @Spec.OpensEpochJustified.OpensEpochJustified = @NewProtocol.OpensEpochJustified := rfl
end Spec.OpensEpochJustified
```

{includeDocstring NewProtocol.OpensEpochJustified}

{docstring NewProtocol.SafeHistory}

# The rest of the protocol

%%%
tag := "protocol"
%%%

What else an honest node may do: when it may time out, what it may propose, and
from whom it takes a proposal.

A proposal and a re-vote request name a certificate the leader may build on
({name NewProtocol.CertJustified}`CertJustified`). Without a timeout, the leader holds the certificate and
the block's proposal, though not necessarily its payload. After a timeout, it was locked on the certificate at some point after
it received the timeout certificate. A re-vote request without a timeout also
needs the block's payload ({name NewProtocol.RevoteJustified.lockable}`RevoteJustified.lockable`).

Whether the honest members accept the certificate, given the timeout
certificate's lock, is a separate condition ({name NewProtocol.ProposalReady.safe}`ProposalReady.safe`).

```lean -show
namespace Spec.NotBehind
open NewProtocol.History
```

:::spec NewProtocol.NotBehind
  ```lean
  def NotBehind (cfg : Config) (h : History) (e : EpochNumber) : Prop :=
    ∀ e', h.InEpoch cfg e' → e' ≤ e
  ```
:::

```lean -show
example : @Spec.NotBehind.NotBehind = @NewProtocol.NotBehind := rfl
end Spec.NotBehind
```

{includeDocstring NewProtocol.NotBehind}

```lean -show
namespace Spec.CertJustified
open NewProtocol.History
```

:::spec NewProtocol.CertJustified
  ```lean
  def CertJustified (cfg : Config) (h : History) (c : Cert1) :
      Option TimeoutCert → Prop
    | none => h.Buildable cfg c
    | some tc => h.HasCert1 cfg c ∧ ∃ m, (h.upTo m).Received (.timeoutCertificate tc)
        ∧ ((∃ l, (h.upTo m).LockedOn cfg l ∧ l.data = c.data)
          ∨ (IsLastBlock c.data.blockNumber cfg.epochHeight
            ∧ ∃ c2, (h.upTo m).HasCert2 c2 ∧ c2.data = c.data.toVote2))
  ```
:::

```lean -show
example : @Spec.CertJustified.CertJustified = @NewProtocol.CertJustified := rfl
end Spec.CertJustified
```

{includeDocstring NewProtocol.CertJustified}

```lean -show
namespace Spec.ParentJustified
open NewProtocol.History
```

:::spec NewProtocol.ParentJustified
  ```lean
  def ParentJustified (cfg : Config) (h : History) (p : Proposal) : Prop :=
    CertJustified cfg h p.parentCert p.timeoutEvidence
  ```
:::

```lean -show
example : @Spec.ParentJustified.ParentJustified = @NewProtocol.ParentJustified := rfl
end Spec.ParentJustified
```

{includeDocstring NewProtocol.ParentJustified}

{docstring NewProtocol.ProposalReady}

{docstring NewProtocol.ProposalJustified}

{docstring NewProtocol.RevoteJustified}

```lean -show
namespace Spec.ProposedIn
open NewProtocol.History
```

:::spec NewProtocol.ProposedIn
  ```lean
  def ProposedIn (h : History) (e : EpochNumber) (v : ViewNumber) : Prop :=
    (∃ p, h.Sent (.proposal p) ∧ p.viewNumber = v ∧ p.epoch = e)
      ∨ ∃ r, h.Sent (.revote r) ∧ r.view = v ∧ r.cert.data.epoch = e
  ```
:::

```lean -show
example : @Spec.ProposedIn.ProposedIn = @NewProtocol.ProposedIn := rfl
end Spec.ProposedIn
```

{includeDocstring NewProtocol.ProposedIn}

{docstring NewProtocol.ProtocolHistory}

# What a node owes

%%%
tag := "owed"
%%%

The rules above are permissions: a node that never acts satisfies all of them.
{name NewProtocol.Owed}`Owed` is the other side, what a node must eventually do. Each obligation is an
action that is justified by what the node holds, not yet taken, and not overtaken
by events. Every obligation ends, and none asks for anything the node would have to
fetch: a vote1 and a proposal are owed only in the node's current view and until it
times the view out, a vote2 until the node gives its view up, and a decide only
after the decide floor.

How long an obligation may stay owed is not said here. That is {name NewProtocol.Prompt}`Prompt`, below.

```lean -show
namespace Spec.ParentReady
open NewProtocol.History
```

:::spec NewProtocol.ParentReady
  ```lean
  def ParentReady (cfg : Config) (h : History) (p : Proposal) : Prop :=
    p.parentCert.view = cfg.anchorView ∨ EntersEpoch cfg p
      ∨ ∃ parent, h.HasProposal cfg parent ∧ parent.viewNumber ≤ p.parentCert.view
          ∧ p.parentCert.data.blockHash = blockHash parent
          ∧ h.HasPayload cfg parent.viewNumber parent.payloadCommit
  ```
:::

```lean -show
example : @Spec.ParentReady.ParentReady = @NewProtocol.ParentReady := rfl
end Spec.ParentReady
```

{includeDocstring NewProtocol.ParentReady}

{docstring NewProtocol.Obligation}

The vote and decide obligations have many conditions, so each is a structure with
named fields.

{docstring NewProtocol.OwedVote1}

{docstring NewProtocol.OwedVote1Again}

{docstring NewProtocol.OwedVote2}

{docstring NewProtocol.OwedDecide}

```lean -show
namespace Spec.Owed
open NewProtocol.History
```

:::spec NewProtocol.Owed
  ```lean
  def Owed (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey)
      (node : PubKey) (h : History) : Obligation → Prop
    | .vote1 p => OwedVote1 cfg leader h p
    | .vote1Again r => OwedVote1Again cfg leader h r
    | .vote2 c => OwedVote2 cfg h c
    | .decide c => OwedDecide cfg h c
    | .propose e v =>
        ((∃ p, ProposalJustified cfg leader node h p ∧ p.viewNumber = v ∧ p.epoch = e)
          ∨ ∃ r, RevoteJustified cfg leader node h r ∧ r.view = v ∧ r.cert.data.epoch = e)
        ∧ ¬ ProposedIn h e v ∧ ¬ h.TimedOut v ∧ h.InView cfg v
  ```
:::

```lean -show
example : @Spec.Owed.Owed = @NewProtocol.Owed := rfl
end Spec.Owed
```

{includeDocstring NewProtocol.Owed}

What a node owes is read for the epochs it is honest in. What it signs for another
epoch is unconstrained, so it does not count against what it owes in these.

{docstring NewProtocol.Output.SignedIn}

{docstring NewProtocol.History.restrict}

{docstring NewProtocol.Obligation.epoch}

```lean -show
namespace Spec.OwedIn
open NewProtocol.History
```

:::spec NewProtocol.OwedIn
  ```lean
  def OwedIn (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey)
      (node : PubKey) (honestIn : EpochNumber → Prop) (h : History)
      (o : Obligation) : Prop :=
    honestIn o.epoch ∧ Owed cfg leader node (h.restrict honestIn) o
  ```
:::

```lean -show
example : @Spec.OwedIn.OwedIn = @NewProtocol.OwedIn := rfl
end Spec.OwedIn
```

{includeDocstring NewProtocol.OwedIn}

# Networks and certificates

%%%
tag := "network"
%%%

The honest nodes and what connects them. Honesty is per epoch: a node honest in an
epoch signs every message of that epoch by the rules, and what it signs for any
other epoch is unconstrained. A node honest in no epoch has no trace at all. The network is
authenticated, so a node knows who sent it a message. Only votes are signed, since
they are aggregated into certificates that travel to nodes that never saw the votes.

{docstring NewProtocol.Committee}

{docstring NewProtocol.Committee.Honest}

{docstring NewProtocol.Committee.HonestFrom}

{docstring NewProtocol.Committee.HonestOften}

{docstring NewProtocol.Committee.Steady}

{docstring NewProtocol.Trace}

{expansion NewProtocol.Trace}

```lean -show
namespace Spec.Trace_history
open NewProtocol.History
```

:::spec NewProtocol.Trace.history
  ```lean
  def Trace.history (r : Trace) (n : Nat) : History := (List.range n).map r
  ```
:::

```lean -show
example : @Spec.Trace_history.Trace.history = @NewProtocol.Trace.history := rfl
end Spec.Trace_history
```

{includeDocstring NewProtocol.Trace.history}

```lean -show
namespace Spec.SentBy
open NewProtocol.History
```

:::spec NewProtocol.SentBy
  ```lean
  def SentBy (r : Trace) (m : Message) : Prop := ∃ n, Output.send m ∈ (r n).output
  ```
:::

```lean -show
example : @Spec.SentBy.SentBy = @NewProtocol.SentBy := rfl
end Spec.SentBy
```

{includeDocstring NewProtocol.SentBy}

A certificate is _backed_ when a quorum of its epoch's committee really cast the
votes behind it. Only the honest members of the quorum are held to it: what faulty
members did is unconstrained.

```lean -show
namespace Spec.Cert1Backed
open NewProtocol.History
```

:::spec NewProtocol.Cert1Backed
  ```lean
  def Cert1Backed {C : Committee} (trace : ∀ k, C.Honest k → Trace) (c : Cert1) : Prop :=
    ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest c.data.epoch k,
      SentBy (trace k (.of h)) (.vote1 ⟨c.data, c.view, k⟩)
  ```
:::

```lean -show
example : @Spec.Cert1Backed.Cert1Backed = @NewProtocol.Cert1Backed := rfl
end Spec.Cert1Backed
```

{includeDocstring NewProtocol.Cert1Backed}

```lean -show
namespace Spec.Cert2Backed
open NewProtocol.History
```

:::spec NewProtocol.Cert2Backed
  ```lean
  def Cert2Backed {C : Committee} (trace : ∀ k, C.Honest k → Trace) (c : Cert2) : Prop :=
    ∃ q, C.Quorum c.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest c.data.epoch k,
      SentBy (trace k (.of h)) (.vote2 ⟨c.data, c.view, k⟩)
  ```
:::

```lean -show
example : @Spec.Cert2Backed.Cert2Backed = @NewProtocol.Cert2Backed := rfl
end Spec.Cert2Backed
```

{includeDocstring NewProtocol.Cert2Backed}

```lean -show
namespace Spec.TimeoutVoteFor
open NewProtocol.History
```

:::spec NewProtocol.TimeoutVoteFor
  ```lean
  def TimeoutVoteFor (k : PubKey) (tc : TimeoutCert) (vote : TimeoutVote) : Prop :=
    vote.signer = k ∧ vote.view = tc.view ∧ vote.data.epoch = tc.data.epoch
      ∧ LockLE vote.data.lock tc.data.lock
  ```
:::

```lean -show
example : @Spec.TimeoutVoteFor.TimeoutVoteFor = @NewProtocol.TimeoutVoteFor := rfl
end Spec.TimeoutVoteFor
```

{includeDocstring NewProtocol.TimeoutVoteFor}

```lean -show
namespace Spec.TimeoutCertBacked
open NewProtocol.History
```

:::spec NewProtocol.TimeoutCertBacked
  ```lean
  def TimeoutCertBacked {C : Committee} (trace : ∀ k, C.Honest k → Trace)
      (tc : TimeoutCert) : Prop :=
    ∃ q, C.Quorum tc.data.epoch q ∧ ∀ k, q k → ∀ h : C.honest tc.data.epoch k,
      ∃ vote, TimeoutVoteFor k tc vote ∧ SentBy (trace k (.of h)) (.timeoutVote vote)
  ```
:::

```lean -show
example : @Spec.TimeoutCertBacked.TimeoutCertBacked = @NewProtocol.TimeoutCertBacked := rfl
end Spec.TimeoutCertBacked
```

{includeDocstring NewProtocol.TimeoutCertBacked}

```lean -show
namespace Spec.TimeoutLockChecked
open NewProtocol.History
```

:::spec NewProtocol.TimeoutLockChecked
  ```lean
  def TimeoutLockChecked {C : Committee} (trace : ∀ k, C.Honest k → Trace)
      (cfg : Config) (tc : TimeoutCert) : Prop :=
    (tc.data.lock = cfg.anchorCert ∨ Cert1Backed trace tc.data.lock)
      ∧ tc.data.lock.view ≤ tc.view
  ```
:::

```lean -show
example : @Spec.TimeoutLockChecked.TimeoutLockChecked = @NewProtocol.TimeoutLockChecked := rfl
end Spec.TimeoutLockChecked
```

{includeDocstring NewProtocol.TimeoutLockChecked}

A network is one trace per honest node, each obeying the signing rules, in which
every certificate an honest node is handed is backed. The fields saying so are named
_genuine_ ({name NewProtocol.Network.cert1Genuine}`Network.cert1Genuine` and the others); _backed_ is what they say of a
certificate. The two words name one notion: a genuine certificate is a backed one. Those fields are the
verification layer's contract and, with {name NewProtocol.CollisionFree}`CollisionFree`, the only cryptographic
assumptions.

{docstring NewProtocol.Network}

# Time

%%%
tag := "time"
%%%

What liveness needs beyond the rules: when steps happen, how promptly a node acts,
and what the network delivers once it has stabilised. The model is partial
synchrony. Before an unknown time GST, messages may be delayed without bound. After
it, what is sent at time `t` arrives by `max t GST + Δ`.

Delivery is stated per kind of input rather than per message, because inputs are
what a history records. Each assumption says what arrives, not how: an
implementation meets it however it likes, and that it does can be checked on its
traces.

{docstring NewProtocol.TimedNetwork}

Liveness speaks of what a node's history satisfies by a time, and of what spreads
from one honest node to the others.

{docstring NewProtocol.TimedNetwork.By}

{docstring NewProtocol.TimedNetwork.SentByTime}

{docstring NewProtocol.TimedNetwork.Propagates}

{docstring NewProtocol.TimedNetwork.Spreads}

{expansion NewProtocol.TimedNetwork.Spreads}

```lean -show
namespace Spec.Prompt
open NewProtocol.History
```

:::spec NewProtocol.Prompt
  ```lean
  def Prompt {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey}
      {C : Committee} (N : TimedNetwork cfg leader C) (δ : Nat) : Prop :=
    ∀ k h n (o : Obligation),
      OwedIn cfg leader k (C.honest · k) ((N.trace k h).history (n + 1)) o →
      ∃ m, n ≤ m ∧ N.time k h m ≤ N.time k h n + δ
        ∧ ¬ OwedIn cfg leader k (C.honest · k) ((N.trace k h).history (m + 1)) o
  ```
:::

```lean -show
example : @Spec.Prompt.Prompt = @NewProtocol.Prompt := rfl
end Spec.Prompt
```

{includeDocstring NewProtocol.Prompt}

The synchrony assumptions say what the network delivers within `Δ` after GST,
what a node's own modules deliver within the same bound (the builder's headers and
the validity reports), and how time and the view timer of length `τ` behave.

{docstring NewProtocol.Synchrony}

```lean -show
namespace Spec.LeaderRotation
open NewProtocol.History
```

:::spec NewProtocol.LeaderRotation
  ```lean
  def LeaderRotation (C : Committee) (leader : EpochNumber → ViewNumber → Option PubKey) :
      Prop :=
    ∀ e v, ∃ w, v ≤ w ∧ ∃ l, leader e w = some l ∧ C.honest e l ∧ C.members e l
  ```
:::

```lean -show
example : @Spec.LeaderRotation.LeaderRotation = @NewProtocol.LeaderRotation := rfl
end Spec.LeaderRotation
```

{includeDocstring NewProtocol.LeaderRotation}

# What is proved

%%%
tag := "proved"
%%%

Each property is a named proposition, proved for every configuration and committee.
Each premise below is a hypothesis of the proposition, or a field of the structures
it quantifies over.

* {name NewProtocol.NoFork}`NoFork`, proved by {name NewProtocol.noFork}`noFork`, and {name NewProtocol.DecideAgreement}`DecideAgreement`, proved by
  {name NewProtocol.decideAgreement}`decideAgreement`: the signing rules and genuine certificates ({name NewProtocol.Network}`Network`),
  quorum intersection ({name NewProtocol.Committee.intersect}`Committee.intersect`), {name NewProtocol.ConfigCoherent}`ConfigCoherent`, {name NewProtocol.CollisionFree}`CollisionFree`,
  and a block table that is {name NewProtocol.TreeCoherent}`TreeCoherent` and {name NewProtocol.Resolves}`Resolves` the honest nodes'
  blocks.
* {name NewProtocol.DecidesValid}`DecidesValid`, proved by {name NewProtocol.decidesValid}`decidesValid`: {name NewProtocol.Network}`Network`, quorum
  intersection, {name NewProtocol.ConfigCoherent}`ConfigCoherent` and {name NewProtocol.CollisionFree}`CollisionFree`.
* {name NewProtocol.ChainGrows}`ChainGrows`, proved by {name NewProtocol.Liveness.chainGrows}`Liveness.chainGrows`: a {name NewProtocol.TimedNetwork}`TimedNetwork`, which
  adds to {name NewProtocol.Network}`Network` the whole protocol and honest quorums in every epoch
  ({name NewProtocol.TimedNetwork.honestQuorum}`TimedNetwork.honestQuorum`); finitely many honest nodes
  ({name NewProtocol.Committee.honestFinite}`Committee.honestFinite`); {name NewProtocol.Synchrony}`Synchrony`, {name NewProtocol.Prompt}`Prompt`, `8Δ + 3δ < τ`,
  {name NewProtocol.LeaderRotation}`LeaderRotation`, {name NewProtocol.ConfigCoherent}`ConfigCoherent` and {name NewProtocol.CollisionFree}`CollisionFree`.

## No fork

The safety statement is about ancestry, and ancestry needs blocks to follow: a
certificate names only its block's hash. A block table answers which block a hash
names. It is a device of the statement, not something a node keeps. The statement
requires it to be honest ({name NewProtocol.TreeCoherent}`TreeCoherent`) and to
have every proposal an honest node holds ({name NewProtocol.Resolves}`Resolves`).

{docstring NewProtocol.BlockTable}

{expansion NewProtocol.BlockTable}

{docstring NewProtocol.Ancestor}

```lean -show
namespace Spec.TreeCoherent
open NewProtocol.History
```

:::spec NewProtocol.TreeCoherent
  ```lean
  def TreeCoherent (tree : BlockTable) : Prop :=
    ∀ h b, tree h = some b → blockHash b = h
  ```
:::

```lean -show
example : @Spec.TreeCoherent.TreeCoherent = @NewProtocol.TreeCoherent := rfl
end Spec.TreeCoherent
```

{includeDocstring NewProtocol.TreeCoherent}

```lean -show
namespace Spec.CollisionFree
open NewProtocol.History
```

:::spec NewProtocol.CollisionFree
  ```lean
  def CollisionFree : Prop :=
    ∀ b b' : Block, blockHash b = blockHash b' → b = b'
  ```
:::

```lean -show
example : @Spec.CollisionFree.CollisionFree = @NewProtocol.CollisionFree := rfl
end Spec.CollisionFree
```

{includeDocstring NewProtocol.CollisionFree}

```lean -show
namespace Spec.Resolves
open NewProtocol.History
```

:::spec NewProtocol.Resolves
  ```lean
  def Resolves (cfg : Config) (tree : BlockTable) {C : Committee}
      (N : Network cfg C) : Prop :=
    ∀ k h n b, ((N.trace k h).history n).HasProposal cfg b → tree (blockHash b) = some b
  ```
:::

```lean -show
example : @Spec.Resolves.Resolves = @NewProtocol.Resolves := rfl
end Spec.Resolves
```

{includeDocstring NewProtocol.Resolves}

```lean -show
namespace Spec.CertNotAfter
open NewProtocol.History
```

:::spec NewProtocol.CertNotAfter
  ```lean
  def CertNotAfter (c c' : Cert2) : Prop :=
    EpochViewLE c.data.epoch c.view c'.data.epoch c'.view
  ```
:::

```lean -show
example : @Spec.CertNotAfter.CertNotAfter = @NewProtocol.CertNotAfter := rfl
end Spec.CertNotAfter
```

{includeDocstring NewProtocol.CertNotAfter}

```lean -show
namespace Spec.NoFork
open NewProtocol.History
```

:::spec NewProtocol.NoFork
  ```lean
  def NoFork (cfg : Config) (C : Committee) : Prop :=
    ∀ (N : Network cfg C) (tree : BlockTable), ConfigCoherent cfg → TreeCoherent tree →
      CollisionFree → Resolves cfg tree N →
      ∀ c c', Cert2Backed N.trace c → Cert2Backed N.trace c' → CertNotAfter c c' →
        Ancestor cfg tree c.data.blockHash c'.data.blockHash
  ```
:::

```lean -show
example : @Spec.NoFork.NoFork = @NewProtocol.NoFork := rfl
end Spec.NoFork
```

{includeDocstring NewProtocol.NoFork}

{docstring NewProtocol.noFork}

The proof walks back from the later certificate's block, one parent link at a time,
and shows the walk cannot step over the earlier committed block.

Inside one epoch, each step back from a certified block, to its parent or to the
certificate a re-vote voted on again, skips no committed view of the epoch. This is
where the signing rules meet: without timeout evidence the step covers no view, and
with it the honest voter checked the evidence's lock.

{docstring NewProtocol.cert1_step}

{docstring NewProtocol.cert2_ancestor_epoch}

Across epochs, the walk passes through a committed last block, because a proposal
opening an epoch comes behind a {name NewProtocol.Cert2}`Cert2` over its parent. So the argument never
intersects the quorums of two committees.

{docstring NewProtocol.cert1_crosses_boundary}

## Agreement on decides

%%%
tag := "decides"
%%%

A decide counts when the node is honest in the epoch of the {name NewProtocol.Cert2}`Cert2` it decides on.
Such a decide carries a {name NewProtocol.Cert2}`Cert2` the node holds, which is backed, and every block it
delivers is an ancestor of that certificate's block. {name NewProtocol.noFork}`noFork` orders the
certificates, so what honest nodes decide lies on one chain.

```lean -show
namespace Spec.DecidedBlock
open NewProtocol.History
```

:::spec NewProtocol.DecidedBlock
  ```lean
  def DecidedBlock (cfg : Config) {C : Committee} (N : Network cfg C)
      (k : PubKey) (h : C.Honest k) (b : Block) : Prop :=
    ∃ n blocks c1 c2, Output.decided blocks c1 c2 ∈ (N.trace k h n).output ∧ b ∈ blocks
      ∧ C.honest c2.data.epoch k
  ```
:::

```lean -show
example : @Spec.DecidedBlock.DecidedBlock = @NewProtocol.DecidedBlock := rfl
end Spec.DecidedBlock
```

{includeDocstring NewProtocol.DecidedBlock}

```lean -show
namespace Spec.DecideAgreement
open NewProtocol.History
```

:::spec NewProtocol.DecideAgreement
  ```lean
  def DecideAgreement (cfg : Config) (C : Committee) : Prop :=
    ∀ (N : Network cfg C) (tree : BlockTable), ConfigCoherent cfg → TreeCoherent tree →
      CollisionFree → Resolves cfg tree N →
      ∀ k h k' h' b b', DecidedBlock cfg N k h b → DecidedBlock cfg N k' h' b' →
        Ancestor cfg tree (blockHash b) (blockHash b')
          ∨ Ancestor cfg tree (blockHash b') (blockHash b)
  ```
:::

```lean -show
example : @Spec.DecideAgreement.DecideAgreement = @NewProtocol.DecideAgreement := rfl
end Spec.DecideAgreement
```

{includeDocstring NewProtocol.DecideAgreement}

{docstring NewProtocol.decideAgreement}

## Valid decides

%%%
tag := "valid-decides"
%%%

Consensus never checks a block's validity itself. Each delivered block has a
backed {name NewProtocol.Cert1}`Cert1` over it, and an honest signer of that certificate votes1 only for
a valid block ({name NewProtocol.SafeHistory.vote1Justified}`SafeHistory.vote1Justified`). How a node learns validity is
outside the rules, under {ref "not-covered"}[what is not covered].

```lean -show
namespace Spec.DecidesValid
open NewProtocol.History
```

:::spec NewProtocol.DecidesValid
  ```lean
  def DecidesValid (cfg : Config) (C : Committee) : Prop :=
    ∀ (N : Network cfg C), ConfigCoherent cfg → CollisionFree →
      ∀ k h b, DecidedBlock cfg N k h b → BlockValid b
  ```
:::

```lean -show
example : @Spec.DecidesValid.DecidesValid = @NewProtocol.DecidesValid := rfl
end Spec.DecidesValid
```

{includeDocstring NewProtocol.DecidesValid}

{docstring NewProtocol.decidesValid}

What else the application is promised is a rule rather than a result: a decide
delivers each view at most once ({name NewProtocol.SafeHistory.decideOnce}`SafeHistory.decideOnce`).
Neither order nor completeness is promised:

* A decide delivers the committed block and a linked chain of ancestors the node holds, newest
  first, and may stop at any of them. It must stop at an ancestor it does not hold, so that block
  is skipped.
* A skipped block is delivered later only by a decide on a {name NewProtocol.Cert2}`Cert2` over
  it, or over another block between it and the delivered block after it: a decide never delivers
  a view twice, so its chain cannot pass through a block already delivered. Such a later decide
  delivers an earlier view.
* Only a decide on a {name NewProtocol.Cert2}`Cert2` the node holds is owed, and only after the
  decide floor ({name NewProtocol.Owed}`Owed`). A skipped block committed only through a later
  block's {name NewProtocol.Cert2}`Cert2` may never be delivered.

A node may fetch a proposal it is missing from any peer, faulty or not. A fetched proposal is held
like any other ({name NewProtocol.Input.proposal}`Input.proposal`, without a share), and nothing
else vouches for it: a decide that includes it is checked against its own contents, each block by
the parent certificate of the block after it. So {name NewProtocol.DecideAgreement}`DecideAgreement`
and {name NewProtocol.DecidesValid}`DecidesValid` cover fetched blocks as they cover any other.
What a node delivers lies on one chain, whatever order it arrives in, which is what lets the
application order it.

## The chain grows

After GST, decides keep coming, made by nodes on `Cert2`s of epochs they are honest
in. Every node honest in every epoch keeps deciding new views, and so does every
node honest in infinitely many epochs, when epochs have blocks. When they have
none, every node honest in the start epoch does. The premises are
those listed at the start of {ref "proved"}[What is proved].

A node keeps deciding when, for every time `t`, it decides after `t` a view later
than every view it had decided by `t`. Its decides count when they are on a `Cert2` of an epoch it is
honest in.

```lean -show
namespace Spec.DecidesAfter
open NewProtocol.History
variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}
```

:::spec NewProtocol.TimedNetwork.DecidesAfter
  ```lean
  def DecidesAfter (N : TimedNetwork cfg leader C) (k : PubKey) (h : C.Honest k)
      (t : Nat) : Prop :=
    ∃ v, (∃ n, (((N.trace k h).history n).restrict (C.honest · k)).DecidedView v)
      ∧ ∀ n, (∀ i, i < n → N.time k h i ≤ t) →
        ∀ w, (((N.trace k h).history n).restrict (C.honest · k)).DecidedView w → w < v
  ```
:::

```lean -show
example : @Spec.DecidesAfter.DecidesAfter = @NewProtocol.TimedNetwork.DecidesAfter := rfl
end Spec.DecidesAfter
```

```lean -show
namespace Spec.ChainGrows
open NewProtocol.History
```

:::spec NewProtocol.ChainGrows
  ```lean
  def ChainGrows (cfg : Config)
      (leader : EpochNumber → ViewNumber → Option PubKey) (C : Committee) : Prop :=
    ∀ (N : TimedNetwork cfg leader C) (GST Δ δ τ : Nat), ConfigCoherent cfg →
      CollisionFree → Synchrony N GST Δ τ → Prompt N δ → 8 * Δ + 3 * δ < τ →
      LeaderRotation C leader →
      (∀ t, ∃ k, ∃ h : C.Honest k, N.DecidesAfter k h t)
        ∧ ∀ t k (h : C.Honest k),
          (C.Steady k ∨ (cfg.epochHeight = 0 ∧ C.honest cfg.startEpoch k)
            ∨ (cfg.epochHeight ≠ 0 ∧ C.HonestOften k)) →
          N.DecidesAfter k h t
  ```
:::

```lean -show
example : @Spec.ChainGrows.ChainGrows = @NewProtocol.ChainGrows := rfl
end Spec.ChainGrows
```

{includeDocstring NewProtocol.ChainGrows}

{docstring NewProtocol.Liveness.chainGrows}

Both parts are proved by contradiction, from one argument
({name NewProtocol.Liveness.decides_unbounded}`Liveness.decides_unbounded`). For each epoch, take a node honest in it, and
suppose that the views each decides on `Cert2`s of epochs it is honest in are
bounded by `V`. For
the first part, these are nodes from each epoch's honest quorum, and the bound is
what all nodes had decided by `t`. For the second, the steady node is that node for
every epoch. A node honest only in infinitely many epochs takes one more step, at
the end of this section.

First, no honest node ever gets grounds for an epoch later than a bound. Grounds for
a later epoch come from an epoch change, from a certified block of that epoch,
whose chain entered it behind a last block an honest node held with a {name NewProtocol.Cert2}`Cert2`,
or from a timeout certificate, whose signers were already in the epoch. Either way
the epoch change reaches the node honest in that epoch, which then decides that
last block, and the last block's view grows with its epoch. Grounds other than a
timeout certificate are _solid_:

{docstring NewProtocol.Liveness.SolidGround}

{docstring NewProtocol.Liveness.solid_bound}

{docstring NewProtocol.Liveness.ground_solid}

So some epoch `E` is the latest any honest node ever has grounds for, and every node
honest in it or later gets grounds for it within `Δ` of the first honest node that
has them. Every other
honest node gets the epoch change that ended the last epoch it is honest in. From
a time `t0` on, `E` is stable: every node honest in it or later is in it, every
other honest node is past every epoch it is honest in, and no honest node holds
the epoch's last block with a {name NewProtocol.Cert2}`Cert2`.

{docstring NewProtocol.Liveness.Stable}

{docstring NewProtocol.Liveness.retires}

{docstring NewProtocol.Liveness.stabilises}

In `E` every view is reached. Otherwise the honest nodes come to rest in
the latest view any of them reaches, and time it out again until their timeout
votes, now all naming the stable epoch, form a certificate that takes them past it.
A member left in an earlier view keeps timing that one out, and is answered.

{docstring NewProtocol.Liveness.member_reaches}

{docstring NewProtocol.Liveness.views_unbounded}

Finally, take a view `w` of `E` with a leader honest in `E`, later than anything
reached at `t0`. Every honest node reaches it within `2Δ` of the
first, without any certificate being sent on in the usual case.

{docstring NewProtocol.Liveness.reach_spread}
 The leader proposes on its lock, opens the epoch on the previous epoch's last
block, or asks for a re-vote, within `4Δ + δ`.

{docstring NewProtocol.Liveness.leader_acts}

The members vote1, lock on the {name NewProtocol.Cert1}`Cert1` and vote2, and every node honest in the
epoch, member or not, gets the block and decides the view or a later one, all
before the timer of `τ` can fire. A re-vote cannot get this far in a stable epoch:
its {name NewProtocol.Cert2}`Cert2` would commit the epoch's last block. So the node honest in the
stable epoch decides a view past the bound, which contradicts the supposition.

{docstring NewProtocol.Liveness.stable_decides}

{docstring NewProtocol.Liveness.decides_unbounded}

{docstring NewProtocol.Liveness.late_decide}

When epochs have no blocks there is one epoch, the start epoch, and it becomes
stable. So a node honest in it decides past any bound, whatever it is in other
epochs.

{docstring NewProtocol.Liveness.single_epoch_decides}

A node honest in infinitely many epochs need not be honest in the stable epoch, so
for it the argument goes one step further, when epochs have blocks. Then no epoch
stays stable: each view of it with an honest leader commits a block of the epoch at
its own view, after the block an earlier such view committed, so with a larger
block number, and no block of the epoch has a larger block number than its last.

{docstring NewProtocol.Liveness.not_stable}

Were what the node decides bounded, it would be honest in some epoch past the
bound. The end of that epoch would reach it and it would decide past the bound, so
no honest node ever ends that epoch, and some epoch up to it is stable, which it
cannot be.

{docstring NewProtocol.Liveness.often_decides}

# The premises can be met

%%%
tag := "witness"
%%%

A premise nothing satisfies makes a result true for no reason, and no build
notices. So each set of premises is met by a concrete network, in which what the
results are about really happens.

For the safety premises, this package has one: a network in which a block is
committed and a proposal opens the next epoch behind it, so that the rule for
opening an epoch ({name NewProtocol.OpensEpochJustified}`OpensEpochJustified`) is met by doing something rather than by
having nothing to do.

{docstring NewProtocol.Witness.premises_met}

{name NewProtocol.DecideAgreement}`DecideAgreement` and {name NewProtocol.DecidesValid}`DecidesValid` read no premise beyond those of
{name NewProtocol.NoFork}`NoFork`, so the same network meets theirs.

Its committees change with the epoch, and so does honesty. The node that is the
committee of epoch one is faulty in epoch two, where it votes1 for two different
blocks in one view. Its trace breaks the signing rules taken over every epoch, and
the premises still hold, since a node is held to the rules only in the epochs it is
honest in.

{docstring NewProtocol.Witness.per_epoch}

For the liveness premises, the implementation package (`new-protocol-impl`) runs
its reference machine on fixed schedules and proves that each run meets every
premise of {name NewProtocol.ChainGrows}`ChainGrows`. Each witness exercises a different path:

* `EpochWitness` and `LongEpochWitness`: committees that change with each epoch, of
  one and of two blocks. At every boundary the leader sends its re-vote request and
  the next epoch's first block in the same view, and nobody times out.
* `RevoteWitness`: a re-vote the outgoing committee answers.
* `SplitWitness`: GST after the first timer, so the first epoch change reaches only
  some nodes, the first timeout votes name two epochs and form no certificate, and
  the timer's second round does. Every later boundary is crossed without a timer.
* `LateCertWitness`: an epoch's last block whose {name NewProtocol.Cert1}`Cert1` arrives after every node
  timed its view out. Nobody votes2 at the block's view, the epoch change carries the
  re-vote's {name NewProtocol.Cert2}`Cert2`, and the next epoch's first block names the block's own
  {name NewProtocol.Cert1}`Cert1`, behind a timeout certificate.
* `ByzantineWitness`, without epochs: rotating leaders, the faulty one proposing
  different blocks to two honest nodes; no {name NewProtocol.Cert1}`Cert1` forms, and the next honest
  leader builds behind the timeout certificate.
* `ByzantineWitness`, with three blocks to an epoch: the faulty leader's view
  follows each epoch's last block, the timeout certificate is of the new epoch, and
  the next honest leader opens it behind the certificate, with no re-vote. The
  faulty leader is honest in the first epoch, where it votes as a member of the
  committee, and equivocates only in the later epochs, where it is faulty.
* `MinorityLockWitness`: a timeout before GST whose certificate names a lock one
  honest signer could not lock on yet; that node locks on it after GST and votes
  for the block built on it.

Each states `premises_met` and `decides`, and their axiom footprints are checked in
`NewProtocolImpl.Checks`. Each takes `∀ b, BlockValid b` as a hypothesis, since
{name NewProtocol.BlockValid}`BlockValid` is opaque. These names are in the implementation package, which
this document does not import, so its build does not check them.

No witness has a node that is honest in infinitely many epochs but not in every
epoch. So the second case of {name NewProtocol.ChainGrows}`ChainGrows`'s second part is proved but not shown
to happen in an example. `ByzantineWitness`'s faulty leader is honest in the first
epoch only, and is not among the nodes its `decides` covers.

Every witness starts from an anchor at view and height zero. A run from a later
anchor, and one whose anchor ends its epoch, are proved but not shown in an example.

The witnesses are fixed schedules. Random ones are run by `just sim`
(`NewProtocolDiff.Sim`): several nodes run the machine over a simulated network
that meets the synchrony assumptions with random delays, with one silent node,
one equivocating leader, or a different member faulty in each epoch, and epochs of
one to three blocks or none. Each run is checked against the rules by the
verified checker, for agreement on one chain, and for growth.

# What is not covered

%%%
tag := "not-covered"
%%%

Some things a real deployment does are left out. Each omission is either harmless,
because no rule reads what is missing, or narrows what the results say. The second
kind is where an audit should spend its doubt.

*Harmless omissions.*

* _How a node stores, fetches and prunes._ Votes and proposals are owed only in the
  node's current view, and a decide only within `decideBuffer` views of its latest
  decide ({name NewProtocol.History.AfterFloor}`History.AfterFloor`), so a node may
  forget what is older. How it stores the rest, and how it fetches what it missed,
  are its own business. A fetched proposal arrives as an
  {name NewProtocol.Input.proposal}`Input.proposal` without a share.
* _Message assembly._ A proposal and the node's own payload share travel separately;
  {name NewProtocol.Input.proposal}`Input.proposal` delivers them together, or the
  proposal alone where the node has no share yet, or none at all.
* _Fields consensus does not read._ A real proposal carries data for light clients,
  state certificates and version upgrades. None of it is modelled.

*Omissions that narrow the results.*

* _Signatures._ A certificate is its data and its view. That a quorum really
  signed it is an assumption on the verification layer ({name NewProtocol.Network}`Network`), not something
  proved from cryptography. For a timeout certificate this includes the claim that
  its lock is at least as late as each of its signers' locks, which may be earlier
  ({name NewProtocol.TimeoutCertBacked}`TimeoutCertBacked`); how a real
  aggregate shows that is outside the model.
* _Where committees come from._ Committees are a parameter ({name NewProtocol.Committee}`Committee`), one per
  epoch, and so is the leader schedule. The stake table and the randomness that
  picks the next epoch's leaders are not described, so nothing here says a committee
  is the right one.
* _Application validity._ {name NewProtocol.BlockValid}`BlockValid` is opaque. A node learns validity from an
  input and believes it. The signing rules require a vote1's block to be valid, so
  the liveness result needs the reports to be true: a node owes a vote1 on a block
  reported valid, and a false report would leave it an obligation it may not meet
  ({name NewProtocol.Prompt}`Prompt`).
* _Crashes and restarts._ Nodes are honest or faulty, and an honest node never loses
  its history. Restarts are out of scope. {name NewProtocol.SafeHistory.vote1Once}`SafeHistory.vote1Once`,
  {name NewProtocol.SafeHistory.vote2Once}`SafeHistory.vote2Once` and {name NewProtocol.SafeHistory.timeoutLock}`SafeHistory.timeoutLock` read every vote the
  node ever sent, so an implementation meets them across a restart only if it
  persists what it signed.
* _A bound on how long._ {name NewProtocol.ChainGrows}`ChainGrows` says decides keep coming after GST, not
  how soon. The proof uses `8Δ + 3δ < τ` to fit a view into one timer, but no
  latency bound reaches the statement.
* _Before GST._ Safety holds at all times. Progress is claimed only after the network
  stabilises ({name NewProtocol.Synchrony}`Synchrony`).
* _Too many faults._ Each committee is assumed to be a quorum system whose quorums
  intersect in a member honest in that epoch ({name NewProtocol.Committee.intersect}`Committee.intersect`). Nothing is
  claimed beyond that.
* _Endless churn._ Finitely many nodes are honest over the whole run
  ({name NewProtocol.Committee.honestFinite}`Committee.honestFinite`). Liveness does not cover a run whose validators keep
  changing for ever.
* _Every block at every node._ The fault bound is per epoch: the members honest in
  an epoch are a quorum of its committee ({name NewProtocol.TimedNetwork.honestQuorum}`TimedNetwork.honestQuorum`), and the epoch keeps
  getting leaders honest in it ({name NewProtocol.LeaderRotation}`LeaderRotation`). What a node signs for an epoch it is
  not honest in does not count against what it owes in the epochs it is honest in
  ({name NewProtocol.OwedIn}`OwedIn`). {name NewProtocol.ChainGrows}`ChainGrows` promises that decides keep coming, and that a node honest in
  every epoch ({name NewProtocol.Committee.Steady}`Committee.Steady`) keeps deciding, as does a node honest in infinitely many
  epochs ({name NewProtocol.Committee.HonestOften}`Committee.HonestOften`) when epochs have blocks. A node that retires,
  honest in no epoch after some, is promised nothing after its last.
* _Every block._ Nothing promises every block to every node. A decide is owed only on a
  {name NewProtocol.Cert2}`Cert2` the node holds and only after its floor, and it may skip
  ancestors the node does not hold. A block committed only through a later block's
  {name NewProtocol.Cert2}`Cert2`, which is usual after a timeout, is never owed, and a node that
  does not hold it when it decides the later block may never deliver it. A node that fetches
  missing proposals and keeps deciding gaps above its floor delivers more, but no rule asks for
  that and no result counts on it ({ref "decides"}[Agreement on decides]).
* _Keys of past epochs._ A node honest in an epoch signs all of that epoch's
  messages by the rules, even after it turns faulty in a later one. A real
  deployment needs per-epoch keys, or keys that cannot be used once retired, for
  this to hold.
* _Migration._ Every run starts from the configured anchor block
  ({name NewProtocol.Config.anchorBlock}`Config.anchorBlock`), at any view and height. How the nodes come to agree on
  it, and a switch from an earlier protocol, are not described.

# How to audit this

%%%
tag := "audit"
%%%

The proofs are checked by the Lean kernel. No machine checks that a statement says
what was intended, so the definitions are to be read and the premises challenged.

*What to read.* The contract is the modules spliced here: `Base`, `Types`,
`Interface`, `Validity`, `History`, `Rules`, `Network`, `Timing` and `Properties`.
`Proofs/` holds the proofs, and `Lists` restates a history as finite lists for the
machine and the trace checker; neither needs reading to judge what is claimed.

*What the build checks.*

* Every definition of the contract and every main result is shown in this
  document, or the `coverage` executable fails.
* Every definition reproduced in a box above equals the one in the source, or this
  document fails to build.
* Every name a docstring of the specification mentions exists, or `lint` fails.
* Nothing rests on an axiom beyond Lean's own (`propext`, `Classical.choice`,
  `Quot.sound`), and no `sorry`; and the rule structures, the network premises and
  the synchrony assumptions have exactly the fields listed in
  `NewProtocolSpec.Checks`. A field added to one of them is then a deliberate change.

*The premises to challenge.*

* The configuration: {name NewProtocol.ConfigCoherent}`ConfigCoherent`.
* The cryptographic assumptions: {name NewProtocol.CollisionFree}`CollisionFree`, and the fields
  {name NewProtocol.Network.cert1Genuine}`Network.cert1Genuine`, {name NewProtocol.Network.cert2Genuine}`Network.cert2Genuine`, {name NewProtocol.Network.timeoutCertGenuine}`Network.timeoutCertGenuine` and
  {name NewProtocol.Network.revoteGenuine}`Network.revoteGenuine`, which say every certificate an honest node is handed
  is backed.
* The quorum system: the fields of {name NewProtocol.Committee}`Committee`, quorum intersection and
  finitely many honest nodes ({name NewProtocol.Committee.honestFinite}`Committee.honestFinite`).
* For liveness: the fields of {name NewProtocol.TimedNetwork}`TimedNetwork` and {name NewProtocol.Synchrony}`Synchrony`, the bound
  `8Δ + 3δ < τ`, and {name NewProtocol.LeaderRotation}`LeaderRotation`.
* {name NewProtocol.Resolves}`Resolves` and {name NewProtocol.TreeCoherent}`TreeCoherent`, which are about the block table of the
  statement rather than about the protocol. Without them the statement says nothing,
  since ancestry is followed through the table.

*Questions worth asking of the rules.*

* Does {name NewProtocol.SafeHistory}`SafeHistory` forbid everything a node must not sign? It is all the safety
  results read of a node.
* Does every obligation in {name NewProtocol.Owed}`Owed` end, and is every action it asks for permitted by
  {name NewProtocol.ProtocolHistory}`ProtocolHistory`? An action that is owed but forbidden would make the rules
  unsatisfiable; the witnesses show that they are not.
* Are the omissions listed under {ref "not-covered"}[what is not covered] safe to
  make?

What no reading can check is that this formalisation is the intended protocol. An
audit narrows that gap, and so does replaying a real node's traces against these
rules, which the differential harness in `new-protocol-diff` does.
