module

public import NewProtocolSpec.Interface

/-!
# Well-formed data

Conditions on a piece of data alone, with no reference to any node: a proposal's
shape, an epoch change's consistency, what a certificate or a vote is over.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config)

/--
A proposal's shape.

Its parent is at an earlier view. Without timeout evidence the parent is at the
view before; with it, the evidence is a timeout certificate for the view before,
and the parent may be at any earlier view. Its epoch is the one its height falls
in, and its height is one more than its parent's. These make the epoch arithmetic
about a chain meaningful: a certificate is often all a node has of a block, and it
carries the block's height and epoch.
-/
structure ProposalWellFormed (p : Proposal) : Prop where
  /-- The parent is at an earlier view. -/
  parentEarlier : p.parentCert.view < p.viewNumber

  /--
  Without timeout evidence the parent is at the view before; with it, the timeout
  certificate is for the view before.
  -/
  covered : (p.timeoutEvidence = none ∧ p.parentCert.view + 1 = p.viewNumber)
    ∨ ∃ tc, p.timeoutEvidence = some tc ∧ tc.view + 1 = p.viewNumber

  /-- The epoch is the one the height falls in. -/
  epoch : p.epoch = epochOf p.blockHeader.blockNumber cfg.epochHeight

  /-- The height is one more than the parent's. -/
  height : p.parentCert.data.blockNumber + 1 = p.blockHeader.blockNumber

/-- The proposal opens a new epoch: its parent was the last block of the previous one. -/
def EntersEpoch (p : Proposal) : Prop :=
  IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight

/--
Evidence that an epoch ended is consistent.

The `Cert1` is the block's own, at its view, the `Cert2` is over the block at that
view or a later one, the block is the proposal carried, and it is the last block
of its epoch. The `Cert2` names the block's epoch, which a well-formed proposal
ties to its height.

The `Cert2` may be a re-vote's (`RevoteRequest`). The `Cert1` is the block's own because
the next epoch's first block is built on that one (`OpensEpochJustified`).
-/
structure EpochChangeWellFormed (c1 : Cert1) (c2 : Cert2) (p : Proposal) : Prop where
  /-- The `Cert2` is at the block's view or a later one. -/
  cert2View : p.viewNumber ≤ c2.view

  /-- The `Cert2` is over what the `Cert1` is over. -/
  sameData : c1.data.toVote2 = c2.data

  /-- The `Cert1` is over the block. -/
  cert1Data : c1.data = ⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩

  /-- The `Cert1` is at the block's view: it is the block's own. -/
  cert1View : p.viewNumber = c1.view

  /-- The proposal is well formed. -/
  wellFormed : ProposalWellFormed cfg p

  /-- The block is the last of its epoch. -/
  last : IsLastBlock p.blockHeader.blockNumber cfg.epochHeight

/-- A VID share is a share of this proposal's payload. -/
def ShareMatches (p : Proposal) (vid : VidShare) : Prop :=
  vid.view = p.viewNumber ∧ p.payloadCommit = vid.payloadCommit

/--
A `Cert1` is over exactly this block: its hash, epoch and height, at the block's
view or a later one.

A later view is a re-vote's (`RevoteRequest`): the votes are over the block again, in
the view they are cast in.
-/
def Certifies (c : Cert1) (b : Block) : Prop :=
  b.viewNumber ≤ c.view ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩

/--
A `Cert2` is over exactly this block, at its view or, after a re-vote, a later one.

The same relation as `Certifies`, for the other certificate.
-/
def Commits (c : Cert2) (b : Block) : Prop :=
  b.viewNumber ≤ c.view ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩

/-- A vote1 is for exactly this block. -/
def Vote1For (vote : Vote1) (b : Block) : Prop :=
  vote.view = b.viewNumber ∧ vote.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩

/--
A re-vote request's shape.

Its certificate is at an earlier view, at the view before or behind a timeout
certificate for the view before, as for a proposal. The block it is over is the last of its
epoch: only there does a missing `Cert2` hold the chain up.
-/
def RevoteWellFormed (r : RevoteRequest) : Prop :=
  r.cert.view < r.view
    ∧ ((r.timeoutEvidence = none ∧ r.cert.view + 1 = r.view)
        ∨ ∃ tc, r.timeoutEvidence = some tc ∧ tc.view + 1 = r.view)
    ∧ IsLastBlock r.cert.data.blockNumber cfg.epochHeight

/--
Epoch `e` at view `v` is no later than epoch `e'` at view `v'`: the epoch is
earlier, or the same and the view no later.

The order every certificate is compared in. Epochs come first: a re-vote certifies
the last block of an epoch again, and the outgoing committee may do so at a view
after the next epoch began. A certificate of the next epoch is later than any
re-vote's, whatever their views.
-/
def EpochViewLE (e : EpochNumber) (v : ViewNumber) (e' : EpochNumber) (v' : ViewNumber) : Prop :=
  e < e' ∨ (e = e' ∧ v ≤ v')

/-- Lock order: `a` is no later than `b` (`EpochViewLE`). -/
def LockLE (a b : Cert1) : Prop := EpochViewLE a.data.epoch a.view b.data.epoch b.view

/--
A timeout certificate's lock lets a parent certificate through, for a block of
epoch `e`, in three cases:

* The lock is of an earlier epoch than `e`. What the lock guards is the commits of
  the block's own epoch `e`, so a lock of an earlier epoch never stands in the
  way. This is the case of an epoch's first block.
* The lock is of the parent's epoch and at the parent's view or earlier. This is
  the ordinary case: the parent is no earlier than anything the signers voted2 on.
* The lock is of the parent's epoch and over the same block. This is for a
  re-vote request after a timeout (`SafeRevote`): a lock on an earlier re-vote's
  certificate over the same block still admits the block's first certificate.
-/
def LockAllows (lock pc : Cert1) (e : EpochNumber) : Prop :=
  lock.data.epoch < e
    ∨ (lock.data.epoch = pc.data.epoch ∧ (lock.view ≤ pc.view ∨ lock.data = pc.data))

/-- A vote1 answers this re-vote request: the certificate's data, at the request's view. -/
def Vote1Again (vote : Vote1) (r : RevoteRequest) : Prop :=
  vote.view = r.view ∧ vote.data = r.cert.data

/--
A chain of blocks, newest first, each the parent of the one before.

The shape of a decide: each block's parent certificate names the next.
-/
def ChainLinked : List Block → Prop
  | [] => True
  | [_] => True
  | b :: b' :: rest =>
      b'.viewNumber ≤ b.parentCert.view ∧ b.parentCert.data.blockHash = blockHash b'
        ∧ ChainLinked (b' :: rest)

end NewProtocol
