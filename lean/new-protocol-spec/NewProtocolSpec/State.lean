module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface

@[expose] public section

namespace NewProtocol

/--
The state of one consensus participant, as the specification observes it.

Every field is a record of inputs received or outputs emitted, not a
prescription of data structures — the maps and sets are plain functions and
predicates, and an implementation refines this state via a simulation
relation, choosing whatever representation it likes.
-/
structure NodeState where
  /--
  Proposals we hold, by view — however obtained.

  Wider than `NodeState.admitted`, and deliberately so: an arrival that
  fails the admission guards may still be *held*
  (`SafetySpec.proposalProvenance`), which is how a block an implementation
  fetched for itself enters a state the specification never asked it to
  fetch. Only `admitted` is safe to vote on; this map exists for ancestry —
  linking a parent, walking the decide chain — and it outlives `admitted`,
  being pruned at the decide floor rather than at the bar (`GcSpec`).

  Keyed by view, which is also why leader equivocation needs no rule of its
  own: a node cannot hold two proposals for one view, so the consequences of
  equivocation reach the argument only through `cert1_unique` and
  `cert2_unique` — that it cannot produce two certificates in a view either.
  -/
  proposals : ViewNumber → Option Proposal

  /--
  Proposals that entered through the admission rule, by view.

  The block a vote signs is read from here and nowhere else; ancestry may be
  read from `NodeState.proposals`, which is wider. Keeping the two apart is
  what lets a node hold a block it may not vote on — one arriving against a
  stale lock, or without the matching share — as ancestry material only
  (`SafetySpec.admissionJustified`).
  -/
  admitted : ViewNumber → Option Proposal

  /-- Views we proposed in (at most one proposal per view). -/
  proposedViews : ViewNumber → Prop

  /-- Our VID share per view. -/
  vidShares : ViewNumber → Option VidShare

  /--
  Blocks reported valid, by view and block hash.

  The node's knowledge of `BlockValid`, which it cannot compute for itself.
  `Vote1Justification.blockValid` is the rule — an invalid block may not be
  voted for — and this is what makes the vote *owed* (`Vote1Enabled`): an
  obligation may only read what the node holds.
  -/
  validated : ViewNumber → Option BlockHash

  /-- Payloads reconstructed from VID shares (or dispersed by us). -/
  blocksReconstructed : ViewNumber → PayloadCommit → Prop

  /--
  Block content available to propose, by view and parent block hash.

  We may only propose a header built for exactly this view and parent
  (`ProposalJustification.headerBuilt`), and we can only be *owed* a proposal
  once one exists (`ProposeEnabled`) — a node cannot be obliged to propose a
  block that does not exist yet, which is why what is available to propose is
  part of the observed state.
  -/
  headers : ViewNumber → BlockHash → Option BlockHeader

  /-- `Cert1`s by view. -/
  cert1s : ViewNumber → Option Cert1

  /-- `Cert2`s by view. -/
  cert2s : ViewNumber → Option Cert2

  /-- Timeout certificates, keyed by the view they advanced *into* (one past the timed-out view). -/
  timeoutCerts : ViewNumber → Option TimeoutCert

  /--
  The locked certificate.

  The highest `Cert1` whose block we saw reconstructed and matching its proposal.
  -/
  lockedCert : Option Cert1

  /--
  Views actually emitted in an `Output.decided`.

  Not a contiguous range: a gap view can decide late, below already-decided
  views.
  -/
  decidedViews : ViewNumber → Prop

  /-- Views we cast a vote1 in. -/
  voted1Views : ViewNumber → Prop

  /--
  For each view this node voted1 in, the view that vote's proposal was
  justified at.

  A proposal at `w` justified at `u` extends the chain straight from `u` to
  `w`, leaving the views between them empty. Voting for it endorses that
  branch, gaps included, so the pair is worth remembering: it is what lets a
  later vote2 be refused in one of the skipped views. Casting that vote2 would
  put this node in the quorum committing the view *and* in the quorum
  certifying a branch with nothing there. Quorums overlap, so a single node in
  both is the whole difference between a fork and none.

  Its watermark is the decide floor rather than `NodeState.barredView`: a
  vote2 may lag several views behind, so the bar on it has to outlive the
  view it applies to.
  -/
  vote1Branches : ViewNumber → Option ViewNumber

  /-- Views we cast a vote2 in. -/
  voted2Views : ViewNumber → Prop

  /-- Highest view that timed out; we neither vote(1) nor propose at or below it. -/
  timeoutView : ViewNumber

  /--
  Views at or below this one are abandoned.

  Nothing is admitted, voted or proposed there again
  (`SafetySpec.vote1NotBarred` and friends), which is what makes it safe to
  forget having voted there and to collect the state a vote would have read.

  Deciding is *not* barred: a decide is retrospective, and a `Cert2` for an
  old view may still arrive. The decide path is bounded by the decide floor
  instead — a separate, lower watermark.

  Also the watermark below which local pruning is free.
  -/
  barredView : ViewNumber

  /-- The view we are currently in. -/
  currentView : ViewNumber

/--
The decide path's state at view `v` survives.

What a decide reads: the block, the certificates over it, and the payload
that reconstructs it. Its watermark is the decide floor, because a `Cert2`
for an old view may still arrive and decide it.
-/
structure RetainsDecide (s s' : NodeState) (v : ViewNumber) : Prop where
  proposals : ∀ p, s.proposals v = some p → s'.proposals v = some p
  blocksReconstructed : ∀ pc, s.blocksReconstructed v pc → s'.blocksReconstructed v pc
  cert1s : ∀ c, s.cert1s v = some c → s'.cert1s v = some c
  cert2s : ∀ c, s.cert2s v = some c → s'.cert2s v = some c

/--
The vote path's state at view `v` survives.

What a vote or a proposal reads, and nothing else. Its watermark is
`NodeState.barredView`: once a view is abandoned none of this can be acted
on again, so it may go — while the decide path at the same view lives on.
-/
structure RetainsVote (s s' : NodeState) (v : ViewNumber) : Prop where
  admitted : ∀ p, s.admitted v = some p → s'.admitted v = some p
  vidShares : ∀ sh, s.vidShares v = some sh → s'.vidShares v = some sh
  validated : ∀ h, s.validated v = some h → s'.validated v = some h
  headers : ∀ h hd, s.headers v h = some hd → s'.headers v h = some hd
  timeoutCerts : ∀ tc, s.timeoutCerts v = some tc → s'.timeoutCerts v = some tc

/--
Everything `s` holds at view `v` is still held in `s'`.

Stated as implications rather than equalities: `s'` may hold *more* at `v`.
This is what a node may not silently discard — the content counterpart of the
mark obligations, and the reason a step cannot destroy its own enabledness.
A consensus step never prunes; collection is where the two watermarks apply
separately.
-/
structure Retains (s s' : NodeState) (v : ViewNumber) : Prop where
  decide : RetainsDecide s s' v
  vote : RetainsVote s s' v

/--
Writing `x` into the slot `o` loses nothing: the slot is free, or already
holds exactly `x`.

Above the decide floor the state is append-only — `Retains` keeps every
slot that is filled, and the ingestion obligations only ever ask for a slot
this predicate admits. The two would otherwise contradict each other on any
slot an input rewrites, since a slot holds one value and `Retains` and
ingestion would name different ones.
-/
def Writable {α : Type} (o : Option α) (x : α) : Prop :=
  o = none ∨ o = some x

/--
The configuration's anchor sits where the rules assume it does.

Both the block and its certificate belong to `ViewNumber.genesis`; their
hashes need not agree (see `Config.anchorBlock`).
-/
structure ConfigCoherent (cfg : Config) : Prop where
  /-- The anchor block sits at genesis. -/
  anchorBlockView : cfg.anchorBlock.viewNumber = ViewNumber.genesis

  /-- And so does the certificate over it. -/
  anchorCertView : cfg.anchorCert.view = ViewNumber.genesis

  /--
  The chain is rooted at the first block.

  `Config.anchorBlock` says restart is not covered, so the anchor is genesis and
  genesis is block zero. The epoch arithmetic reads it: no boundary falls at
  block zero (`IsLastBlock` asks for a non-zero block), so the first proposal
  after the anchor never opens a new epoch, which is what lets the walk stop at
  the anchor without looking for a boundary certificate that nothing could have
  formed.
  -/
  anchorBlockNumber : cfg.anchorBlock.blockHeader.blockNumber = 0

  /--
  The certificate the chain is rooted at certifies the block it is rooted at.

  Without this the anchor certificate carries data unrelated to any block, and
  a proposal naming it as its parent claims a parent that does not exist. The
  walk every safety argument performs would then have nothing at its far end:
  `Network.parentCertValid` exempts the anchor from being quorum-backed,
  because nothing votes at genesis, so this is what stands in its place.
  -/
  anchorCertBlock : cfg.anchorCert.data.blockHash = blockHash cfg.anchorBlock

  /--
  The anchor's parent link points at its own view.

  Genesis has no parent, so the link is a placeholder. It must not point
  *forwards*, or the chain that every decide walks would not terminate at the
  anchor.
  -/
  anchorParentView : cfg.anchorBlock.parentCert.view = ViewNumber.genesis


namespace NodeState

/--
The state of a fresh node starting from the configured anchor.

The anchor block and its certificate are *present from the start* rather
than delivered: they come with the configuration, and a step that introduced
them would violate `SafetySpec.proposalProvenance` / `cert1Provenance`. The
anchor is held, not admitted — no rule votes on it — and its view is decided,
so the decide walk stops there.
-/
def initial (cfg : Config) : NodeState where
  proposals := fun v => if v = ViewNumber.genesis then some cfg.anchorBlock else none
  admitted := fun _ => none
  proposedViews := fun _ => False
  vidShares := fun _ => none
  validated := fun _ => none
  blocksReconstructed := fun _ _ => False
  headers := fun _ _ => none
  cert1s := fun v => if v = ViewNumber.genesis then some cfg.anchorCert else none
  cert2s := fun _ => none
  timeoutCerts := fun _ => none
  lockedCert := none
  decidedViews := fun v => v = ViewNumber.genesis
  voted1Views := fun _ => False
  vote1Branches := fun _ => none
  voted2Views := fun _ => False
  timeoutView := ViewNumber.genesis
  barredView := ViewNumber.genesis
  currentView := ViewNumber.genesis

/--
The epoch a node takes itself to be in.

The one its lock names, or the anchor's before anything is locked. A node's
epoch is not otherwise tracked here — no rule reads it except the one that says
what a timeout vote signs — and it has to be read off something the node holds,
because a timeout vote answers a timer rather than a block and so has no
proposal to take an epoch from.

Two honest nodes holding the same lock name the same epoch, which is what makes
a timeout certificate possible at all: `TimeoutCertBacked` asks for a quorum of
one epoch, so if honest nodes disagreed no certificate could be backed and
`Network.evidenceValid` would hold only where no proposal carries evidence.
-/
def epoch (cfg : Config) (s : NodeState) : EpochNumber :=
  match s.lockedCert with
  | some c => c.data.epoch
  | none => cfg.anchorCert.data.epoch

/--
`v` is above the decide floor.

The floor is `lastDecided - decideBuffer` for the maximal decided view
`lastDecided`; stated pointwise so that no maximum over a propositional set
is needed.
-/
def aboveDecideFloor (cfg : Config) (s : NodeState) (v : ViewNumber) : Prop :=
  ∀ w, s.decidedViews w → w - cfg.decideBuffer < v

end NodeState

end NewProtocol
