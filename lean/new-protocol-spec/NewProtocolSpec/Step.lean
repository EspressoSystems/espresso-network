module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State

/-! # Voting rules and the step specification -/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-! ## Admission, voting and proposing rules -/

/--
Shape rules a proposal must satisfy to enter the protocol at all.

Chain links point strictly backwards, and a proposal that does not extend
the immediately preceding view must carry a timeout certificate for the gap.
Without the first rule the decide walk is not a walk (a proposal could name
itself as its own parent); without the second, a leader could skip views
unchallenged.

Both are stated here rather than left to a verification layer: the second is
what such a layer would naturally check, but nothing else in the spec implies
the first.
-/
def ProposalWellFormed (p : Proposal) : Prop :=
  p.parentCert.view < p.viewNumber
    ∧ (p.parentCert.view + 1 = p.viewNumber
        ∨ ∃ tc, p.timeoutEvidence = some tc ∧ tc.view + 1 = p.viewNumber)

/--
The VID share delivered with a proposal is the share of *that* proposal's
payload.

A share we broadcast is only useful to peers if it reconstructs the block we
voted for.
-/
def ShareMatches (p : Proposal) (vid : VidShare) : Prop :=
  vid.view = p.viewNumber ∧ p.payloadCommit = vid.payloadCommit

/--
The lock-based safety/liveness rule guarding proposal admission.

A proposal may enter the protocol iff nothing is locked yet, or it *is* the
locked block (certificate and block arrived before the proposal), or its
`parentCert` extends the locked block (safety) or is newer than the lock
(liveness). An implementation may compare commitments of the certificate data
where the spec compares the data itself; the two agree up to hash collisions.
-/
def SafeToExtend (locked : Option Cert1) (p : Proposal) : Prop :=
  match locked with
  | none      => True
  | some lock =>
    if lock.view = p.viewNumber then
      lock.data.blockHash = blockHash p
    else
      p.parentCert.data = lock.data ∨ lock.view < p.parentCert.view

/--
A `parentCert`-linked chain of blocks, newest first.

Each block's `parentCert` certifies the next (older) entry; this is the
shape of the decide chain-walk.
-/
def ChainLinked : List Block → Prop
  | []              => True
  | [_]             => True
  | b :: b' :: rest => b.parentCert.view = b'.viewNumber
      ∧ b.parentCert.data.blockHash = blockHash b'
      ∧ ChainLinked (b' :: rest)

/--
What must hold of state `s` for the node to cast a vote1 for
proposal `p`.

Freshness and the timeout bar are separate, see `Vote1Enabled`.
-/
structure Vote1Justification (s : NodeState) (p : Proposal) : Prop where
  /--
  The proposal was *admitted*, not merely held.

  I.e. it entered through the admission rule (`SafetySpec.admissionJustified`);
  a proposal held for ancestry only can never be voted on.
  -/
  proposalAdmitted : s.admitted p.viewNumber = some p

  /--
  The block is valid.

  The rule, not the evidence: consensus may not put its weight behind a block
  the application would reject, so a `Cert1` is a quorum's worth of independent
  confirmations that the block passed. How a node comes to know this is its own
  business, and *being obliged* to vote is a separate question —
  `Vote1Enabled` requires the node to have been told
  (`NodeState.validated`), because an obligation may only read what the
  node holds.
  -/
  blockValid : BlockValid p

  /-- We hold our VID share for the proposed payload. -/
  vidShare : (s.vidShares p.viewNumber).isSome

  /--
  Still safe against the lock as it stands now.

  Admission checked this once, but the lock may have moved since: a vote2
  vote in the meantime locks this node on a view the proposal's branch may
  skip. Voting here would then count it towards a quorum certifying that
  branch and towards the one committing the view — the same double count
  `Vote1SkippedView` prevents in the opposite order.
  -/
  safeToExtend : SafeToExtend s.lockedCert p

  /--
  Unless the parent is genesis: the parent proposal is known,
  `parentCert` certifies exactly it, and its block is reconstructed.

  The parent may be held without having been admitted — ancestry needs no
  admission, only the certificate that names it.

  An implementation may also accept a lock matching the parent as proof of
  reconstruction, which is a restart concern and not covered here.
  -/
  parentLinked : p.parentCert.view ≠ ViewNumber.genesis →
    ∃ parent, s.proposals p.parentCert.view = some parent
      ∧ p.parentCert.data.blockHash = blockHash parent
      ∧ s.blocksReconstructed p.parentCert.view parent.payloadCommit

/--
What must hold of state `s` for the node to cast a vote2 for proposal `p`.

Freshness and the lock ordering are separate, see `Vote2Enabled` and
`SafetySpec.vote2LockOrdered`.
-/
structure Vote2Justification (s : NodeState) (p : Proposal) : Prop where
  /-- The proposal was admitted (see `Vote1Justification.proposalAdmitted`). -/
  proposalAdmitted : s.admitted p.viewNumber = some p

  /-- A `Cert1` formed over exactly this proposal. -/
  certMatches : ∃ c1, s.cert1s p.viewNumber = some c1 ∧ c1.data.blockHash = blockHash p

  /-- The proposed block was reconstructed from VID shares. -/
  reconstructed : s.blocksReconstructed p.viewNumber p.payloadCommit

/--
The parent certificate a proposal names is one the node may build on.

After a timeout it is the locked certificate, together with the timeout
certificate as evidence; otherwise it is the certificate of the immediately
preceding view. Named rather than written inline so that a statement about
proposing has one place to point at.
-/
def ParentCertJustified (s : NodeState) (p : Proposal) : Prop :=
  match p.timeoutEvidence with
  | some tc => s.timeoutCerts p.viewNumber = some tc ∧ s.lockedCert = some p.parentCert
  | none => s.cert1s (p.viewNumber - 1) = some p.parentCert
      ∧ p.parentCert.view + 1 = p.viewNumber

/--
What must hold of state `s` for node `node` to propose `p`.

Freshness and the timeout bar are separate, see `ProposeEnabled`.
-/
structure ProposalJustification (s : NodeState) (p : Proposal) : Prop where
  /-- We lead the proposal's view. -/
  leader : leader p.viewNumber = some node

  /-- We only emit proposals that would pass our own admission check. -/
  wellFormed : ProposalWellFormed p

  /-- The parent certificate is justified; see `ParentCertJustified`. -/
  justified : ParentCertJustified s p

  /--
  We hold the parent block, and the header we propose is the one built for
  exactly this view and parent.

  Genesis is exempt from the hash match: the stored genesis proposal is a
  local placeholder, not the block the anchor certificate names.
  -/
  headerBuilt : ∃ parent, s.proposals p.parentCert.view = some parent
    ∧ (p.parentCert.view ≠ ViewNumber.genesis → blockHash parent = p.parentCert.data.blockHash)
    ∧ s.headers p.viewNumber (blockHash parent) = some p.blockHeader

/-! ## When an action is owed -/

/--
The node owes a vote1 for `p`: justified, *known* valid, not yet cast,
above both bars, and with the lock still below its view.

Validity appears twice for a reason. `Vote1Justification.blockValid` is the
rule — an invalid block may not be voted for, whoever the voter is — and the
second clause here is the node's knowledge of it. An obligation may only read
what the node holds, or it would demand a vote the node cannot justify; a
permission may name the world as it is. That asymmetry is the whole of the
seam between consensus and the layer that judges blocks.

The last clause bounds the obligation only. Nothing forbids a vote1 in a view
the lock has reached — `NewProtocolSpec.Network` derives safety from the order
votes were cast in rather than from a bar on this one — but nothing owes it
either, since such a vote can add to no certificate that does not already exist.
Here the lock is what makes that concrete. A node holding a `Cert1` over
an admitted, reconstructed proposal at `v` is *owed* the vote2, which it
cannot cast without moving the lock to `v` (`SafetySpec.vote2LockOrdered`); the
lock never moves back (`SafetySpec.lockMono`). Should `v` then be reported valid
— the ordinary race, the certificate and the block having arrived first — the
vote1 at `v` becomes justified, and its equal-view branch of
`SafeToExtend` is satisfied by the very certificate the node locked on.
Without this clause that vote is owed for ever and forbidden for ever.

`SafeToExtend` is not repeated here; it sits in `Vote1Justification`, so it
binds what a node *may* do and not only what it is owed. That placement is what
makes the lock check load-bearing rather than hardening: a node that has locked
since admitting the proposal must not vote for it, or it would join the quorum
certifying a branch that skips the view it just committed.

The lock alone does not make the protocol safe, though. It guards against a
branch reaching *back* past a committed view and says nothing about one built
*forward* on a certificate above the lock, which `SafeToExtend` admits by
design and must, or a node that fell behind could never rejoin. Safety needs the
conflicting certificate never to form, and the two halves of that are this
condition and `Vote1SkippedView` — one for each order in which a node could
otherwise end up in both quorums.
-/
def Vote1Enabled (s : NodeState) (p : Proposal) : Prop :=
  Vote1Justification s p
    ∧ s.validated p.viewNumber = some (blockHash p)
    ∧ ¬ s.voted1Views p.viewNumber ∧ s.timeoutView < p.viewNumber
    ∧ s.barredView < p.viewNumber
    ∧ ∀ lock, s.lockedCert = some lock → lock.view < p.viewNumber

/--
One of this node's vote1s endorsed a branch that skips view `v`.

A vote1 at `w` on a proposal justified at `u` endorses a chain that runs from
`u` to `w` with nothing in between. If `v` is one of the views skipped that
way, a vote2 at `v` afterwards would count this node towards the quorum
committing `v` *and* towards one certifying a branch that has no block there.
Quorums overlap, so a single node in both is the whole difference between a
fork and none.

Nothing orders the two votes on its own. A vote2 waits on a certificate and a
reconstructed block, and both may arrive long after the node has cast vote1
elsewhere, so the bar has to be stated rather than inferred from timing.

Timing out is deliberately not covered: a timeout vote endorses no branch, so
barring the vote2 on that basis would withhold a vote from a view that can
still commit. See `Vote2Enabled` for what that costs.
-/
def Vote1SkippedView (s : NodeState) (v : ViewNumber) : Prop :=
  ∃ w u, v < w ∧ s.vote1Branches w = some u ∧ u < v

/--
The node owes a vote2 for `p`: justified, in a view no earlier vote1 skipped,
not yet cast, and still useful — no `Cert2` yet, the view undecided and above
the decide floor.

There is no timeout bar here, unlike `Vote1Enabled`. Timing out at a view says
the node gave up waiting, not that it took a side; the view may still be
certified and its block still arrive.

Such a bar would cost a vote rather than a block. Deciding carries no bar
(`DecideEnabled`), so the node still decides the view once the `Cert2` reaches
it, and a view the chain extends is delivered as an ancestor by
`StepSpec.decideJustified`. What it would take away is a view's chance to commit
in its own right, once enough of its voters had timed out.

What the node must not do is commit a view one of its own vote1s skipped over;
`Vote1SkippedView` says why.

The block is read from `NodeState.admitted`, while casting the vote2 moves the
lock, and `SafetySpec.lockJustified` reads `NodeState.proposals`. The two agree
where it matters: `admitted_held` says what is admitted above the decide floor is
held, so a vote2 that is owed is also one the node may cast. It is stated of a
node started from `NodeState.initial`, which is what `Network.start` gives; a bare
`Run` does not say where it began.
-/
def Vote2Enabled (s : NodeState) (p : Proposal) : Prop :=
  Vote2Justification s p ∧ ¬ Vote1SkippedView s p.viewNumber
    ∧ ¬ s.voted2Views p.viewNumber ∧ s.cert2s p.viewNumber = none
    ∧ ¬ s.decidedViews p.viewNumber ∧ s.aboveDecideFloor cfg p.viewNumber
    ∧ s.barredView < p.viewNumber

/--
The node owes a decide for view `v`: undecided, above the floor, with a
`Cert2` over exactly the proposal it holds and the matching `Cert1`.

Nothing about ancestry. A certified block is final whether or not this node
holds what came before it, so a missing ancestor truncates the delivered
chain (`StepSpec.decideJustified`) instead of postponing the decide. A node
that waited for the hole to fill would wait on a block nothing obliges
anyone to still serve, stalling the frontier behind history — and history
is the sync layer's job, not this stream's.
-/
def DecideEnabled (s : NodeState) (v : ViewNumber) : Prop :=
  ¬ s.decidedViews v ∧ s.aboveDecideFloor cfg v ∧ (s.cert1s v).isSome
    ∧ ∃ c2 p, s.cert2s v = some c2 ∧ s.proposals v = some p ∧ c2.data.blockHash = blockHash p

/-- The node owes a proposal `p`: justified, not yet proposed, above the timeout bar. -/
def ProposeEnabled (s : NodeState) (p : Proposal) : Prop :=
  ProposalJustification leader node s p ∧ ¬ s.proposedViews p.viewNumber
    ∧ s.timeoutView < p.viewNumber ∧ s.barredView < p.viewNumber

/-! ## The step specification -/

/--
The clauses decide safety rests on, and nothing else.

`NewProtocolSpec.Safety` proves that a network of nodes satisfying *these*
cannot fork. `StepSpec` extends this structure with everything else it asks,
so an implementation conforming to the specification satisfies these by
construction; the split exists so that what safety needs is a field list rather
than a fact about a proof.

The boundary is enforced rather than documented: `Network` holds runs of this
relation, so no argument in the safety layer can reach a clause outside it, and
widening the layer's dependencies means adding a field here — which the
implementation must then discharge.

Every clause is a *permission*: a bound on when a node may vote, lock or admit,
or a record it may not lose. None obliges a node to act. That is the shape of
the guarantee — a node that does nothing at all satisfies all of this, and the
worst an implementation ignoring the rest of `StepSpec` can do is stall.

`leader` is absent, and that is a result rather than an oversight: no clause
below mentions it, so no fork can be blamed on who proposed. Leader election
bears on progress alone.
-/
structure SafetySpec (s : NodeState) (input : Input) (output : List Output) (s' : NodeState) : Prop
where
  -- **Where the state's content comes from**
  --
  -- The environment seam: nothing enters a node's state except through an
  -- input. Without these, every rule below could be satisfied by inventing the
  -- state that justifies it.

  /--
  A held proposal was delivered, and is well-formed.

  This is what makes the admission rule meaningful: proposals cannot appear
  from nowhere, so the only way into `admitted` is
  `SafetySpec.admissionJustified`. Holding asks for less than admission —
  arrival and well-formedness only — so an arrival whose admission guards
  fail may still be kept for ancestry, which is how a fetched block enters
  an implementation that fetches.

  Well-formedness is required of *everything* held, not only of what is
  admitted, and the reason is the ancestry: chain links must point strictly
  backwards or the decide walk is not a walk. A node may decline an arrival —
  nothing obliges it to hold one — but it may not keep a malformed one and
  call it an ancestor.
  -/
  proposalProvenance : ∀ v p, s'.proposals v = some p →
    s.proposals v = some p
      ∨ ((∃ sender vid, input = Input.proposal sender p vid)
          ∧ p.viewNumber = v ∧ ProposalWellFormed p)

  /--
  A proposal enters `admitted` only by passing the admission rule against
  the lock held on arrival, and only from an `Input.proposal`, carrying a
  matching VID share.

  A proposal held for ancestry only never passes through here, which is
  precisely why it can never be voted on: `admitted` is the only map the
  vote rules read.
  -/
  admissionJustified : ∀ v p, s'.admitted v = some p →
    s.admitted v = some p
      ∨ ∃ sender vid, input = Input.proposal sender p vid
          ∧ p.viewNumber = v
          ∧ s.barredView < v
          ∧ SafeToExtend s.lockedCert p
          ∧ ProposalWellFormed p
          ∧ ShareMatches p vid
          ∧ s'.vidShares v = some vid
          ∧ s'.proposals v = some p

  /--
  A certificate is stored under the view it names, and only on arrival.

  The keying half is what lets the rules below look an entry up by the view
  a *value* names.
  -/
  cert1Provenance : ∀ v c, s'.cert1s v = some c →
    s.cert1s v = some c ∨ ((input = Input.certificate1 c ∨ input = Input.advanceView c) ∧ c.view = v)

  -- **Abandoned views**
  --
  -- A view at or below the bar is inert: nothing is admitted, voted or
  -- proposed there (see also `admissionJustified`). That is what makes it safe
  -- for collection to forget a vote cast there and to drop the state the vote
  -- read — the action can never be repeated, so the record of it is no longer
  -- load-bearing.
  --
  -- Deciding is deliberately absent from this list. A decide is retrospective
  -- and is bounded by the decide floor instead, which is why the two watermarks
  -- cannot be merged.
  /-- Only collection moves the bar; a consensus step leaves it alone. -/
  barredViewUnchanged : s'.barredView = s.barredView

  /-- No vote1 at or below the abandoned-view bar. -/
  vote1NotBarred : ∀ v, Output.send (.vote1 v) ∈ output → s'.barredView < v.view

  /-- No vote2 at or below the abandoned-view bar. -/
  vote2NotBarred : ∀ v, Output.send (.vote2 v) ∈ output → s'.barredView < v.view

  -- **What the state may not lose**
  --
  -- The lock advances, a view once marked stays marked, and content above the
  -- decide floor stays held. Below the floor, any step may prune; that is what
  -- the floor is for.
  /-- The lock never moves to an older certificate. -/
  lockMono : ∀ lock, s.lockedCert = some lock →
    ∃ lock', s'.lockedCert = some lock' ∧ lock.view ≤ lock'.view

  /-- A decided view stays decided. -/
  decidedRetained : ∀ v, s.decidedViews v → s'.decidedViews v

  /-- A view voted1 in stays voted1 in. -/
  voted1Retained : ∀ v, s.voted1Views v → s'.voted1Views v

  /--
  A branch record is never dropped by a consensus step.

  Unconditional, unlike `Retains`: the conditional content retention has an
  escape for a view whose state validation failed, and the bar on vote2s
  must not be forgettable by receiving a late failure for the view voted in.
  Collection prunes it, bounded by the decide floor (`GcSpec`).
  -/
  vote1BranchesRetained : ∀ v u, s.vote1Branches v = some u → s'.vote1Branches v = some u

  /-- A view voted2 in stays voted2 in. -/
  voted2Retained : ∀ v, s.voted2Views v → s'.voted2Views v

  -- **Vote1**
  /-- A vote1 is cast at most once per view. -/
  vote1Once : ∀ v, Output.send (.vote1 v) ∈ output → ¬ s.voted1Views v.view ∧ s'.voted1Views v.view

  /-- A vote1 signs a proposal satisfying `Vote1Justification`. -/
  vote1Justified : ∀ v, Output.send (.vote1 v) ∈ output → ∃ p, Vote1Justification s' p
      ∧ v.view = p.viewNumber
      ∧ v.signer = node
      ∧ v.data.blockHash = blockHash p

  /--
  A vote1 records the branch it endorsed.

  Without this the bar on vote2s could be evaded by forgetting, which is
  the same escape the mark obligations close for the vote marks themselves.
  -/
  vote1Records : ∀ v, Output.send (.vote1 v) ∈ output →
    ∃ p, s'.admitted v.view = some p ∧ s'.vote1Branches v.view = some p.parentCert.view

  -- **Vote2**
  /-- A vote2 is cast at most once per view. -/
  vote2Once : ∀ v, Output.send (.vote2 v) ∈ output → ¬ s.voted2Views v.view ∧ s'.voted2Views v.view

  /-- A vote2 signs a proposal satisfying `Vote2Justification`. -/
  vote2Justified : ∀ v, Output.send (.vote2 v) ∈ output → ∃ p, Vote2Justification s' p
      ∧ v.view = p.viewNumber
      ∧ v.signer = node
      ∧ v.data.blockHash = blockHash p

  /--
  The linchpin: a vote2 is only cast once our lock has reached the vote's view.
  -/
  vote2LockOrdered : ∀ v, Output.send (.vote2 v) ∈ output →
    ∃ lock, s'.lockedCert = some lock ∧ v.view ≤ lock.view

  /--
  No vote2 in a view an earlier vote1 skipped.

  This is what keeps one node out of both quorums of a fork; see
  `Vote1SkippedView`. Unlike `SafetySpec.vote2LockOrdered` it is not a
  condition on the branch being voted for, but on what this node has already
  endorsed elsewhere — which is why no check made at the moment of admission
  can stand in for it.
  -/
  vote2NotInSkippedView : ∀ v, Output.send (.vote2 v) ∈ output → ¬ Vote1SkippedView s' v.view

  /--
  No vote2 below the decide floor, where the node has given up and the state a
  vote reads may already be gone.
  -/
  vote2AboveFloor : ∀ v, Output.send (.vote2 v) ∈ output → s'.aboveDecideFloor cfg v.view

  -- **The lock**
  /--
  The lock only moves forward, onto a certificate over a block we hold and
  whose payload we have.

  Held, not admitted: locking forward on a block obtained outside the
  admission path is what lets a node that fell behind rejoin, and the
  certificate is better evidence than this node's own admission check would
  be. What the payload is needed for is that the lock is a commitment — a node
  must not stake one on data it cannot produce.
  -/
  lockJustified : ∀ lock, s'.lockedCert = some lock → s.lockedCert = some lock
      ∨ ((∀ old, s.lockedCert = some old → old.view < lock.view)
          ∧ s'.cert1s lock.view = some lock
          ∧ ∃ p, s'.proposals lock.view = some p
              ∧ blockHash p = lock.data.blockHash
              ∧ s'.blocksReconstructed lock.view p.payloadCommit)

  -- **Timeouts**
  /--
  A timeout vote is cast in response to a timeout, and raises the bar over its
  view. The two timeout inputs differ in which view they may name.

  A *local* timer fires for the view the node is in, so its vote names exactly
  that view. This is what makes a timeout certificate evidence that some honest
  node **reached** the view it certifies — and that, rather than the lock
  alone, is what stops a stale proposal from existing before the view it skips
  has been left behind. A node free to time out for a distant future view could
  manufacture the certificate that licenses a gap proposal, before anyone had
  reason to abandon the views it skips.

  A vote joined on the one-honest threshold is different: the node is following
  evidence that others have already timed out, so it may be behind the view in
  question.
  -/
  timeoutVoteSound : ∀ v evidence, Output.send (.timeoutVote v evidence) ∈ output → v.signer = node
      ∧ v.view ≤ s'.timeoutView
      ∧ ((input = Input.timeout v.view ∧ v.view = s.currentView)
          ∨ (input = Input.timeoutOneHonest v.view ∧ s.currentView ≤ v.view))

/--
Relational specification of one consensus transition of node `node`:
state `s`, input, emitted outputs, state `s'`.

Anything a transition does must satisfy every field, `SafetySpec`'s included;
an implementation conforms exactly when every transition it makes satisfies
them. Whatever the fields do not constrain is the
implementation's own: scheduling, output order, representation, and everything
it asks of its own subsystems in order to make the inputs below arrive.

The fields here are the ones no safety result consumes, which is not to say
they are optional: they are what makes a node useful rather than merely
harmless. They fall into three kinds — that state moves only when an input
says so and that an input is not ignored, which is what makes the machine
determinate; the remaining bounds on acting; and the obligations to act, which
`NewProtocolSpec.Liveness` builds progress from.

Grouping and order follow `SafetySpec`'s, so the two read as one list split
in half. Within a group the rule for an action is stated in both directions:
when it *may* be taken, and what may not happen without taking it.
-/
structure StepSpec (s : NodeState) (input : Input) (output : List Output) (s' : NodeState)
    : Prop extends SafetySpec cfg node s input output s' where
  -- **Where the state's content comes from**
  --
  -- The environment seam: nothing enters a node's state except through an
  -- input. Without these, every rule below could be satisfied by inventing the
  -- state that justifies it.
  /-- A held VID share arrived with the proposal it belongs to. -/
  vidShareProvenance : ∀ v sh, s'.vidShares v = some sh →
    s.vidShares v = some sh ∨ ∃ sender p, input = Input.proposal sender p sh ∧ p.viewNumber = v

  /-- A validity we hold was reported to us. -/
  validatedProvenance : ∀ v h, s'.validated v = some h →
    s.validated v = some h ∨ input = Input.blockValidated v h

  /-- A payload we hold reconstructed was reported to us. -/
  reconstructedProvenance : ∀ v pc, s'.blocksReconstructed v pc →
    s.blocksReconstructed v pc ∨ input = Input.blockReconstructed v pc

  /-- A built header was handed to us for that view and parent. -/
  headerProvenance : ∀ v h hd, s'.headers v h = some hd →
    s.headers v h = some hd ∨ input = Input.headerBuilt v h hd

  /-- A held `Cert2` arrived as one, for the view it certifies. -/
  cert2Provenance : ∀ v c, s'.cert2s v = some c →
    s.cert2s v = some c ∨ (input = Input.certificate2 c ∧ c.view = v)

  /-- A held timeout certificate arrived as one, keyed by the view it advances into. -/
  timeoutCertProvenance : ∀ v tc, s'.timeoutCerts v = some tc →
    s.timeoutCerts v = some tc ∨ (input = Input.timeoutCertificate tc ∧ v = tc.view + 1)

  -- **That an input's content is taken**
  --
  -- The other direction of provenance, and what makes `NodeState` the record
  -- of received inputs it claims to be. Everything a node owes is conditioned
  -- on what it holds — the mark obligations and `WeaklyFair` both run through
  -- enabledness — so a node that stores nothing owes nothing.
  --
  -- The guards are the rules that would reject the content anyway, so state
  -- stays bounded: at most one admitted proposal and one certificate of each
  -- kind per view above the decide floor.
  --
  -- Every one of them also requires the target slot to be `Writable`. Above
  -- the floor the state is append-only — a filled slot is kept, without
  -- exception (`contentRetained`) — so an obligation to write a *different*
  -- value into a filled slot could not be met by any node: retention would
  -- demand the old value in the same breath.

  /--
  A proposal that passes admission is admitted.

  An arrival that fails the guards is owed nothing: a node may hold it for
  ancestry (`proposalProvenance`) or discard it, and discarding only loses
  ancestry it never promised to deliver.
  -/
  proposalIngested : ∀ sender p vid, input = Input.proposal sender p vid →
    s.barredView < p.viewNumber →
    Writable (s.admitted p.viewNumber) p → Writable (s.proposals p.viewNumber) p →
    Writable (s.vidShares p.viewNumber) vid →
    SafeToExtend s.lockedCert p → ProposalWellFormed p → ShareMatches p vid →
      s'.admitted p.viewNumber = some p
        ∧ s'.proposals p.viewNumber = some p
        ∧ s'.vidShares p.viewNumber = some vid

  /-- A `Cert1` for a view that can still act is recorded. -/
  cert1Ingested : ∀ c, (input = Input.certificate1 c ∨ input = Input.advanceView c) →
    s.aboveDecideFloor cfg c.view → Writable (s.cert1s c.view) c → s'.cert1s c.view = some c

  /-- A `Cert2` still worth having is kept. -/
  cert2Ingested : ∀ c, input = Input.certificate2 c →
    s.aboveDecideFloor cfg c.view → Writable (s.cert2s c.view) c → s'.cert2s c.view = some c

  /-- A timeout certificate that has not been overtaken is kept. -/
  timeoutCertIngested : ∀ tc, input = Input.timeoutCertificate tc →
    s.currentView ≤ tc.view + 1 → Writable (s.timeoutCerts (tc.view + 1)) tc →
      s'.timeoutCerts (tc.view + 1) = some tc

  /-- A reported validity is kept. -/
  blockValidatedIngested : ∀ v h, input = Input.blockValidated v h →
    Writable (s.validated v) h → s'.validated v = some h

  /--
  A reconstructed payload is recorded.

  No `Writable` guard: `blocksReconstructed` is a predicate, not a slot, so
  recording one payload never displaces another.
  -/
  reconstructedIngested : ∀ v pc, input = Input.blockReconstructed v pc →
    s'.blocksReconstructed v pc

  /-- A built header is kept. -/
  headerIngested : ∀ v parent h, input = Input.headerBuilt v parent h →
    Writable (s.headers v parent) h → s'.headers v parent = some h

  -- **How the cursors move**
  --
  -- `currentView` and `timeoutView` are the two fields with no content to
  -- trace, so they carry their own provenance: each only advances, and only
  -- for a reason. Without it they are free variables — and since three of the
  -- four enabledness predicates are gated on one or the other, a node could
  -- retire its own obligations by declaring itself past every view.

  /-- The view never goes backwards. -/
  currentViewMono : s.currentView ≤ s'.currentView

  /--
  A view is only entered on evidence that the previous one is settled: a
  `Cert1` for it, or a timeout certificate over it.

  The evidence may be held already or arrive with this step; below the decide
  floor an implementation need not have kept it, which is why the input counts.
  -/
  currentViewJustified : s.currentView ≠ s'.currentView →
    ∃ v, s'.currentView = v + 1
      ∧ ((∃ c, s'.cert1s v = some c)
          ∨ (∃ tc, s'.timeoutCerts (v + 1) = some tc)
          ∨ (∃ c, input = Input.advanceView c ∧ c.view = v)
          ∨ (∃ tc, input = Input.timeoutCertificate tc ∧ tc.view = v))

  /-- The timeout bar never goes backwards. -/
  timeoutViewMono : s.timeoutView ≤ s'.timeoutView

  /--
  The timeout bar only rises to a view whose timer actually fired.

  `Vote1Enabled` and `ProposeEnabled` are both gated on the bar, so a node
  free to raise it at will could stop voting and stop proposing for ever and
  still conform. The behaviour that would model — a node all of whose views
  time out — is exactly what a timeout input attests, so the attestation is
  required.
  -/
  timeoutViewJustified : s.timeoutView ≠ s'.timeoutView →
    input = Input.timeout s'.timeoutView ∨ input = Input.timeoutOneHonest s'.timeoutView

  -- **Abandoned views**
  --
  -- A view at or below the bar is inert: nothing is admitted, voted or
  -- proposed there (see also `admissionJustified`). That is what makes it safe
  -- for collection to forget a vote cast there and to drop the state the vote
  -- read — the action can never be repeated, so the record of it is no longer
  -- load-bearing.
  --
  -- Deciding is deliberately absent from this list. A decide is retrospective
  -- and is bounded by the decide floor instead, which is why the two watermarks
  -- cannot be merged.
  /-- No proposal at or below the abandoned-view bar. -/
  proposeNotBarred : ∀ p, Output.send (.proposal p) ∈ output → s'.barredView < p.viewNumber

  -- **What the state may not lose**
  --
  -- The lock advances, a view once marked stays marked, and content above the
  -- decide floor stays held. Below the floor, any step may prune; that is what
  -- the floor is for.

  /--
  Above the decide floor, a step keeps what it holds.

  Provenance says where content may come from, not that it stays; this says it
  stays. A step that dropped the proposal, share or certificate justifying an
  action it owes would destroy its own enabledness, and no amount of fairness
  can recover an obligation that is no longer enabled. Safety does not depend
  on this — every justification is checked against `s'`, at the step that acts
  — so what is at stake here is progress.

  Above the floor this makes the state append-only — without exception, which
  is why every ingestion obligation is guarded by `Writable`: an obligation
  to overwrite a filled slot would contradict this one.

  There is no escape for a block that turns out to be invalid, and none is
  needed: a block never reported valid is never votable, so holding it costs
  nothing but the slot. The slot is the one thing it does cost, and an
  implementation that holds every arrival can therefore wedge a view an
  equivocating leader has sent it two proposals for. Nothing obliges a node to
  hold what it cannot admit, so the rule for an implementation is to hold
  arrivals it has a use for; a specification loophole permitting the drop
  would instead let a node silently retire a decide it owes.
  -/
  contentRetained : ∀ v, s.aboveDecideFloor cfg v → Retains s s' v

  /-- A view proposed in stays proposed in. -/
  proposedRetained : ∀ v, s.proposedViews v → s'.proposedViews v

  -- **Vote1**
  /-- No vote1 at or below the timeout bar. -/
  vote1Bar : ∀ v, Output.send (.vote1 v) ∈ output → s'.timeoutView < v.view

  /--
  A vote1 travels with our VID share, so peers can reconstruct the block we
  just voted for — without it, no peer can form the vote2 justification.
  -/
  vote1CarriesShare : ∀ v, Output.send (.vote1 v) ∈ output →
    ∃ share, s'.vidShares v.view = some share ∧ Output.send (.vidShare share) ∈ output

  /--
  A view is only marked voted by emitting the vote.

  The converse of `SafetySpec.vote1Once`, and the reason the specification
  cannot be met by silently retiring an opportunity: a node may defer a
  vote, but it may not consume the mark that makes the vote unrepeatable.
  Same for the three marks further down.
  -/
  vote1Marked : ∀ v, ¬ s.voted1Views v → s'.voted1Views v →
    ∃ vote, Output.send (.vote1 vote) ∈ output ∧ vote.view = v

  /-- The record only ever names a proposal this node voted on. -/
  vote1BranchesSound : ∀ v u, s.vote1Branches v = none → s'.vote1Branches v = some u →
    ∃ vote, Output.send (.vote1 vote) ∈ output ∧ vote.view = v

  -- **Vote2**

  /--
  No vote2 for a view that already has a `Cert2`.

  Our vote can no longer contribute; voting after seeing the certificate is
  pointless.
  -/
  vote2NotAfterCert2 : ∀ v, Output.send (.vote2 v) ∈ output → s.cert2s v.view = none

  /-- A fresh vote2 mark means a vote2 went out for that view. -/
  vote2Marked : ∀ v, ¬ s.voted2Views v → s'.voted2Views v →
    ∃ vote, Output.send (.vote2 vote) ∈ output ∧ vote.view = v

  -- **Proposals**

  /-- We propose at most once per view. -/
  proposeOnce : ∀ p, Output.send (.proposal p) ∈ output →
    ¬ s.proposedViews p.viewNumber ∧ s'.proposedViews p.viewNumber

  /-- No proposal at or below the timeout bar. -/
  proposeBar : ∀ p, Output.send (.proposal p) ∈ output → s'.timeoutView < p.viewNumber

  /-- A proposal satisfies `ProposalJustification`. -/
  proposeJustified : ∀ p, Output.send (.proposal p) ∈ output →
    ProposalJustification leader node s' p

  /-- A fresh proposal mark means a proposal went out for that view. -/
  proposedMarked : ∀ v, ¬ s.proposedViews v → s'.proposedViews v →
    ∃ p, Output.send (.proposal p) ∈ output ∧ p.viewNumber = v

  -- **Deciding**

  /--
  A decide event is a fresh, `parentCert`-linked chain above the decide
  floor.

  The newest block carries a matching `Cert2` (and its `Cert1` is
  registered); every emitted block is a proposal we hold and becomes
  decided. Held, not admitted: an ancestor need never have passed the
  admission rule, and the chain of certificates is what vouches for it.

  Every event is *self-certifying*: the head is vouched for by its `Cert2`,
  each older block by the next one's `parentCert`, and nothing by anything
  outside the event. That is also the whole of the bar on backfilling — a
  block the chain walked past can only ever be delivered by a late `Cert2`
  of its own, never on ancestry evidence alone, because an event headed by
  anything less than a `Cert2` would need the stream's history to justify
  it.

  The chain stops only where continuing is impossible or pointless: at a
  view already decided, at one at or below the floor, or at a block not in
  hand. A missing ancestor thus truncates the chain rather than blocking the
  decide, and the view it names is skipped — the stream promises no
  completeness (see `DecideInv`). Nothing else may cut the
  chain short: a held ancestor may not be withheld, or a node could strand
  blocks it holds behind a delivery it is never obliged to repeat.
  -/
  decideJustified : ∀ blocks c1 c2, Output.decided blocks c1 c2 ∈ output →
    (∃ head rest, blocks = head :: rest
        ∧ c2.view = head.viewNumber
        ∧ c2.data.blockHash = blockHash head
        ∧ s'.cert2s head.viewNumber = some c2
        ∧ s'.cert1s head.viewNumber = some c1)
      ∧ ChainLinked blocks
      ∧ (∀ last, blocks.getLast? = some last →
          s'.decidedViews last.parentCert.view
            ∨ ¬ s'.aboveDecideFloor cfg last.parentCert.view
            ∨ ¬ ∃ q, s'.proposals last.parentCert.view = some q
                ∧ blockHash q = last.parentCert.data.blockHash)
      ∧ ∀ b ∈ blocks,
          s.aboveDecideFloor cfg b.viewNumber
            ∧ ¬ s.decidedViews b.viewNumber
            ∧ s'.decidedViews b.viewNumber
            ∧ s'.proposals b.viewNumber = some b

  /-- A fresh decide mark means that view was in a delivered chain. -/
  decidedMarked : ∀ v, ¬ s.decidedViews v → s'.decidedViews v →
    ∃ blocks c1 c2 b, Output.decided blocks c1 c2 ∈ output ∧ b ∈ blocks ∧ b.viewNumber = v

  -- **Certificate relay**

  /--
  A `Cert2` we see first is relayed.

  Unlike a `Cert1`, which travels on the next proposal as its `parentCert`,
  a `Cert2` has no other route to the rest of the network: only the view's
  vote collector can assemble it. Without this relay, only that node ever
  decides.
  -/
  cert2RelayOwed : ∀ c, input = Input.certificate2 c → s.cert2s c.view = none →
    ¬ s.decidedViews c.view → s.aboveDecideFloor cfg c.view → Output.send (.cert2 c) ∈ output

  -- **View changes**

  /-- A certificate that permits a view change moves us into the next view. -/
  advanceOwed : ∀ c, input = Input.advanceView c → c.view + 1 ≤ s'.currentView

  /--
  We only relay a timeout certificate we received, and it advances us
  into the view past the one it certifies.
  -/
  timeoutCertSound : ∀ tc v, Output.send (.timeoutCert tc v) ∈ output →
    input = Input.timeoutCertificate tc ∧ v = tc.view + 1 ∧ v ≤ s'.currentView

  /-- A timeout certificate moves us into the view it advances into. -/
  timeoutCertAdvanceOwed : ∀ tc, input = Input.timeoutCertificate tc →
    tc.view + 1 ≤ s'.currentView

  -- **Timeouts**

  /-- A timeout the node is entitled to answer is always answered. -/
  timeoutVoteOwed : ∀ v, (input = Input.timeout v ∧ v = s.currentView)
      ∨ (input = Input.timeoutOneHonest v ∧ s.currentView ≤ v) →
    ∃ e, Output.send (.timeoutVote ⟨(), v, node⟩ e) ∈ output

/--
A consensus step never lowers the decide floor.

`SafetySpec.decidedRetained` keeps every decided view, and the floor is read off
the decided views, so a view above the floor afterwards was above it before. The
collection counterpart is `GcSpec.floorStable`, which has to be assumed: a
collection may drop a decided view, provided the floor has passed it.
-/
theorem SafetySpec.floorMono {cfg : Config} {node : PubKey}
    {s : NodeState} {input : Input} {output : List Output} {s' : NodeState}
    (hs : SafetySpec cfg node s input output s') {v : ViewNumber}
    (h : s'.aboveDecideFloor cfg v) : s.aboveDecideFloor cfg v :=
  fun _ hu => h _ (SafetySpec.decidedRetained hs _ hu)

end NewProtocol
