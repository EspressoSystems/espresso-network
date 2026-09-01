module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types

/-!
# Interface

The configuration and the inputs and outputs of one consensus participant —
shared between the specification and implementations.
-/

@[expose] public section

namespace NewProtocol

/-- Static protocol configuration. -/
structure Config where
  /--
  The block the node starts from, at `ViewNumber.genesis`.

  Genesis on a fresh network. It is *held* but never *admitted*: nothing
  votes on it, and it exists so that the first real proposal has a parent to
  name and a header key to be built under. Its hash need not match
  `Config.anchorCert` — the genesis exemptions in the proposing and
  parent-linking rules are exactly this.

  On restart this is the last decided block. Restart is
  not covered, so here it is always genesis.
  -/
  anchorBlock : Block

  /--
  The certificate for `Config.anchorBlock`, at `ViewNumber.genesis`.

  The chain's root of trust: it arrives with the configuration rather than
  over the wire, which is why it can be present without violating
  `SafetySpec.cert1Provenance` — no step introduces it.
  -/
  anchorCert : Cert1

  /--
  Views to retain decide inputs behind the decided view.

  Lets a late-broadcast `Cert2` decide an older gap view.
  -/
  decideBuffer : Nat := 20

  /--
  Blocks to an epoch.

  Fixes where every boundary falls, and so which committee governs each block,
  via `epochOf`. Configuration rather than state: it is agreed before the
  network starts and no rule changes it.

  Zero means epochs are not in use. Every block then reports epoch zero and no
  boundary ever falls, which is the static-committee run the results below were
  first proved for.
  -/
  epochHeight : Nat := 0
deriving Repr

/--
Inputs to the consensus transition function.

Certificates arriving here are already assumed signature-verified.
-/
inductive Input where
  /--
  This node has the payload of the block at `v`.

  Reconstructed from peers' VID shares, or dispersed by this node itself when
  it was the proposer — the specification does not distinguish, since what
  every rule reads is only that the payload is *in hand*
  (`NodeState.blocksReconstructed`).
  -/
  | blockReconstructed (v : ViewNumber) (c : PayloadCommit)

  /-- A verified `Cert1` arrived. -/
  | certificate1 (c : Cert1)

  /-- A verified `Cert2`. -/
  | certificate2 (c : Cert2)

  /--
  A `Cert1` requiring us to advance our view.

  The same payload as `Input.certificate1`, but the two differ in what they oblige.
  This one is an instruction: the node must be past `c.view` when the step ends
  (`StepSpec.advanceOwed`).

  The certificate alone, without the block it certifies. Nothing is lost by
  advancing without it: proposing on that block needs it
  (`ProposalJustification.headerBuilt`) and so does locking on it
  (`SafetySpec.lockJustified`), so requiring it here would only stop a node
  tracking a view it can see the network has reached.
  -/
  | advanceView (c : Cert1)

  /--
  A block is available for this leader to propose at `v` on the given parent.

  Where the content comes from is not the specification's business. What the
  rules need is only that *something* is available before a proposal can be
  owed (`ProposeEnabled`), since a node cannot be obliged to propose a block
  that does not exist yet.
  -/
  | headerBuilt (v : ViewNumber) (parent : BlockHash) (h : BlockHeader)

  /--
  A validated proposal together with this node's VID share for its payload.

  Every proposal arrival is this input, however the implementation obtained
  it — the leader's broadcast or a fetch of its own devising. Admission is a
  separate, guarded obligation: an arrival whose guards fail (a stale lock, a
  mismatching share) may still be *held* for ancestry
  (`SafetySpec.proposalProvenance`) without ever entering `NodeState.admitted`,
  and only admitted proposals are votable. Holding is safe whatever the
  block's provenance: `proposals` is only consumed certificate-anchored, so a
  block no certificate names is inert.
  -/
  | proposal (sender : PubKey) (p : Proposal) (vid : VidShare)

  /--
  The block with this hash at view `v` is valid.

  Validity is the application's notion, not consensus's — a block whose
  requests satisfy the application's conditions and are consistent with its
  ancestors — so it arrives from outside rather than being computed here.
  What the specification takes on trust is that this input is only ever
  supplied for blocks that really are valid; see `NewProtocolSpec.Assumptions`.

  There is deliberately no counterpart reporting *failure*. A block never
  reported valid is never votable, so nothing has to be undone; it sits in
  `NodeState.proposals` as inert ancestry, and `StepSpec.contentRetained`
  keeps it there — which is why an implementation should decline arrivals it
  has no use for rather than take them and need the slot back.
  -/
  | blockValidated (v : ViewNumber) (h : BlockHash)

  /-- The local timer for this view fired. -/
  | timeout (v : ViewNumber)

  /-- A verified timeout certificate. -/
  | timeoutCertificate (c : TimeoutCert)

  /-- Timeout votes reached the one-honest threshold; vote timeout too. -/
  | timeoutOneHonest (v : ViewNumber)

deriving DecidableEq, Repr

/--
Network messages of the protocol.

What peers see. Together with `Output.decided` these are the only outputs any
obligation requires; no message is required merely because it can be sent.

No message names a recipient. Who receives what is not specified and no rule
reads a destination, but the choice is not free everywhere: every honest node
must be able to assemble a timeout certificate itself, since a node enters the
next view only on a `Cert1` for the previous one or a timeout certificate over
it, and cannot time out of a view it never entered. If one node alone could
assemble that certificate, it could stop the network for good.

The three certificate relays differ in status. `cert2` is owed
(`StepSpec.cert2RelayOwed`): a `Cert2` has no other route through
the network, since only the view's vote collector can assemble one. The
`cert1` and `timeoutCert` relays are optimisations — a `Cert1` travels on the
next proposal as its `parentCert`, and a timeout certificate can be reassembled
by anyone holding the timeout votes, which is the reach required above.
-/
inductive Message where
  /-- The block we propose for our view. -/
  | proposal (p : Proposal)

  /-- Our vote1 on a proposal. -/
  | vote1 (v : Vote1)

  /-- Our vote2 on a proposal. -/
  | vote2 (v : Vote2)

  /-- Our vote to give up a view, with our best catchup evidence. -/
  | timeoutVote (v : TimeoutVote) (e : Option CatchupEvidence)

  /-- A timeout certificate, advancing into `v`. -/
  | timeoutCert (c : TimeoutCert) (v : ViewNumber)

  /-- A `Cert1`. -/
  | cert1 (c : Cert1)

  /-- A `Cert2`, so peers that could not assemble it from votes can decide. -/
  | cert2 (c : Cert2)

  /-- Our own VID share, so peers can reconstruct the block. -/
  | vidShare (s : VidShare)

deriving DecidableEq, Repr

/--
Outputs of the consensus transition function.

Two kinds, and both are observable outside the node: messages to peers, and
the decide stream to the application.

Deliberately absent is any request to the node's own modules — for a block to
propose, for a validity verdict, for a VID dispersal, for the view timer to be
reset. Each of those is one half of a seam whose other half is an `Input`,
and the specification keeps only the half that says what the node *knows*: a
request carries no protocol content, constrains nothing, and naming one would
prescribe a decomposition into modules that an implementation is free not to
have.

Nothing here obliges a node to ask: an input that never arrives leaves the action
it would have justified unenabled, and an unenabled action is owed nothing. So a
node that starves its own subsystems satisfies fairness vacuously rather than
failing it. That a node asks for what it needs is a property of an implementation
and not of this specification.
-/
inductive Output where
  /-- Send a protocol message to peers. -/
  | send (m : Message)

  /--
  A chain of blocks is decided, newest first — the externally visible
  result, delivered to the application.

  `c1`/`c2` certify the newest block; each older block's `Cert1` is the next
  block's `parentCert`, so the event is checkable against nothing but its
  own contents. The chain stops at a block the node does not hold: the view
  it names is skipped, not owed later — see `StepSpec.decideJustified` and
  `DecideInv` for the delivery contract.
  -/
  | decided (blocks : List Block) (c1 : Cert1) (c2 : Cert2)

deriving DecidableEq, Repr

end NewProtocol
