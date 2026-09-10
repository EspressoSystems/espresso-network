module

public import NewProtocolSpec.Progress

/-!
# What a delivery supplies, and what the node must already have

The hypothesis bundles `NewProtocolSpec.Deadlock` takes. Each is one of two
kinds: what an arriving input must satisfy for the ingestion clauses of
`StepSpec` to fire, and what the node must not have done to itself in the
meantime. Keeping them apart is the point — the first is a condition on the
environment, the second on the node.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-! ## What the environment supplies, and what the node must already have -/

/--
The guards `StepSpec.proposalIngested` reads, met at `s`.

Exactly its hypotheses: the view is not abandoned, the three slots the arrival
writes are free or already hold it, and the proposal passes admission. A node
handed a proposal meeting these must admit it.
-/
structure ProposalAdmissible (s : NodeState) (p : Proposal) (vid : VidShare) : Prop where
  /-- The view is not abandoned. -/
  bar : s.barredView < p.viewNumber

  /-- The admitted slot is free, or already holds this proposal. -/
  admitted : Writable (s.admitted p.viewNumber) p

  /-- As is the held slot. -/
  proposals : Writable (s.proposals p.viewNumber) p

  /-- And the share slot. -/
  vidShares : Writable (s.vidShares p.viewNumber) vid

  /-- The proposal is safe against the lock as it stands. -/
  safe : SafeToExtend s.lockedCert p

  /-- Its chain link points backwards, with evidence for any gap. -/
  wellFormed : ProposalWellFormed p

  /-- And the share delivered with it is a share of its payload. -/
  share : ShareMatches p vid

/--
The ancestry a vote1 for `p` reads, held at `s`.

`Vote1Justification.parentLinked`, as a hypothesis on its own: consensus cannot
fetch a parent, so a node that holds none is not stalled but behind, and
recovering history is the sync layer's job (see `DecideInv`).
-/
def ParentHeld (s : NodeState) (p : Proposal) : Prop :=
  p.parentCert.view ≠ ViewNumber.genesis →
    ∃ parent, s.proposals p.parentCert.view = some parent
      ∧ p.parentCert.data.blockHash = blockHash parent
      ∧ s.blocksReconstructed p.parentCert.view parent.payloadCommit

/--
The room a vote1 for `p` needs, at the state the delivery leaves.

The four ways a node closes the window on itself, and the fifth is the parent's
floor. None of them is a stall: a node that has abandoned the view, timed out,
locked past it or pruned it has moved on.
-/
structure Vote1Room (s : NodeState) (p : Proposal) : Prop where
  /-- The view is not abandoned. -/
  bar : s.barredView < p.viewNumber

  /-- Nor has it timed out. -/
  timedOut : s.timeoutView < p.viewNumber

  /-- The lock leaves room for the vote. -/
  lock : LockAllows s p

  /-- The view is above the decide floor. -/
  floor : s.aboveDecideFloor cfg p.viewNumber

  /-- And so is the parent's, where there is one. -/
  parentFloor : p.parentCert.view ≠ ViewNumber.genesis →
    s.aboveDecideFloor cfg p.parentCert.view

/-- The room a vote2 for `p` needs, at the state the delivery leaves. -/
structure Vote2Room (s : NodeState) (p : Proposal) : Prop where
  /-- The view is not abandoned. -/
  bar : s.barredView < p.viewNumber

  /-- It is above the decide floor. -/
  floor : s.aboveDecideFloor cfg p.viewNumber

  /-- No vote1 of this node's endorsed a branch skipping it. -/
  noSkip : ¬ Vote1SkippedView s p.viewNumber

  /-- No `Cert2` for it has arrived. -/
  noCert2 : s.cert2s p.viewNumber = none

  /-- And it is not decided as some later block's ancestor. -/
  notDecided : ¬ s.decidedViews p.viewNumber

/-! ## A proposal cannot be stalled -/

/--
What a proposal needs beyond the header itself, held at `s`.

`ProposalJustification` with `ProposalJustification.headerBuilt` split in two: the
parent stays here, the header is what the delivery supplies.
-/
structure ProposeReady (s : NodeState) (p : Proposal) (parent : Block) : Prop where
  /-- We lead the view. -/
  leads : leader p.viewNumber = some node

  /-- The proposal's own shape is sound. -/
  wellFormed : ProposalWellFormed p

  /-- The parent certificate is justified; the same condition `ProposalJustification` reads. -/
  justified : ParentCertJustified s p

  /-- And we hold the block it names. -/
  parentHeld : s.proposals p.parentCert.view = some parent
    ∧ (p.parentCert.view ≠ ViewNumber.genesis → blockHash parent = p.parentCert.data.blockHash)

/-- The room a proposal `p` needs, at the state the delivery leaves. -/
structure ProposeRoom (s : NodeState) (p : Proposal) : Prop where
  /-- The view is not abandoned. -/
  bar : s.barredView < p.viewNumber

  /-- Nor has it timed out. -/
  timedOut : s.timeoutView < p.viewNumber

  /-- It is above the decide floor. -/
  floor : s.aboveDecideFloor cfg p.viewNumber

  /-- And so is the parent's view, where the floor can reach it. -/
  parentFloor : p.parentCert.view ≠ ViewNumber.genesis →
    s.aboveDecideFloor cfg p.parentCert.view

  /-- After a timeout, the lock is on the certificate the proposal extends. -/
  lock : p.timeoutEvidence.isSome → s.lockedCert = some p.parentCert

end NewProtocol
