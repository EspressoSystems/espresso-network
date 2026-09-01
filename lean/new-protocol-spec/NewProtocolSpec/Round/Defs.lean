module

public import NewProtocolSpec.Deadlock

/-!
# What one delivery supplies

The hypothesis bundles `NewProtocolSpec.Round` takes, one per hop of a round.
Each is the hypotheses of the matching result of `NewProtocolSpec.Deadlock`
collected into a structure, with the step indices existentially quantified by the
`*Delivered` abbreviation beside it.

Bundling is what makes a statement about a quorum readable: a round quantifies
over the members of a quorum, and each member has its own run, its own step
indices and its own window. Nothing is added or weakened in the collecting, and
each field is named after the hypothesis it carries.

The certificates are written out rather than left general. A `Cert1` over `p` in
`p`'s own view is the only one the round is about, and it is the very object
`cert1_forms` produces, so fixing it here is what lets one hop's conclusion be the
next hop's hypothesis.
-/

@[expose] public section

namespace NewProtocol

/-! ## The vote1 hop -/

/--
A proposal and its validity report reach the node, which has room to vote1.

The hypotheses of `vote1_forced` at named steps: the proposal arrives at `n` and
the report at the later `n₂`. `arrival` carries the sender and the share
together with the proposal, since the admission guards are about the share the
proposal travelled with.
-/
structure Vote1Delivery {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n n₂ : Nat) : Prop where
  /-- The proposal arrives at `n`, and passes admission there. -/
  arrival : ∃ sender vid, Run.Consumes r n (Input.proposal sender p vid)
    ∧ ProposalAdmissible cfg (Run.state r n) p vid

  /-- The validity report arrives at `n₂`. -/
  validated : Run.Consumes r n₂ (Input.blockValidated p.viewNumber (blockHash p))

  /-- In that order. -/
  order : n < n₂

  /-- The node holds the block the proposal extends. -/
  parentHeld : ParentHeld (Run.state r n) p

  /-- The block really is valid; see `ValidityReported`. -/
  valid : BlockValid p

  /-- The report has somewhere to go. -/
  writable : Writable ((Run.state r n₂).validated p.viewNumber) (blockHash p)

  /-- The node has not already voted1 in the view. -/
  fresh : ¬ (Run.state r n).voted1Views p.viewNumber

  /-- And nothing overtakes the view while the vote is outstanding. -/
  window : Vote1Window r p n

/-- A vote1 delivery reaches the node at some pair of steps. -/
def Vote1Delivered {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
  ∃ n n₂, Vote1Delivery r p n n₂

/-! ## The vote2 hop -/

/--
The `Cert1` over `p` and `p`'s payload reach the node, which has room to vote2.

The hypotheses of `vote2_forced`, with the certificate fixed to the one a vote1
quorum forms. `admitted` is what the vote1 hop left behind at this node, and is a
hypothesis here rather than a consequence: nothing in the specification carries a
node's state from one delivery to the next, since the steps between them are not
constrained.
-/
structure Vote2Delivery {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n n₂ : Nat) : Prop where
  /-- The certificate arrives at `n`. -/
  certArrival : Run.Consumes r n (Input.certificate1 ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩)

  /-- The payload arrives at the later `n₂`. -/
  payloadArrival : Run.Consumes r n₂ (Input.blockReconstructed p.viewNumber p.payloadCommit)

  /-- In that order. -/
  order : n < n₂

  /-- The proposal is admitted when the certificate arrives. -/
  admitted : (Run.state r n).admitted p.viewNumber = some p

  /-- The certificate has somewhere to go. -/
  writable : Writable ((Run.state r n).cert1s p.viewNumber) ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩

  /-- The node has not already voted2 in the view. -/
  fresh : ¬ (Run.state r n).voted2Views p.viewNumber

  /-- And nothing overtakes the view while the vote is outstanding. -/
  window : Vote2Window r p n

/-- A vote2 delivery reaches the node at some pair of steps. -/
def Vote2Delivered {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
  ∃ n n₂, Vote2Delivery r p n n₂

/-! ## The decide hop -/

/--
The `Cert2` over `p` reaches a node that holds the block.

The hypotheses of `decide_forced`, with the certificate fixed to the one a vote2
quorum forms. `cert1Held` and `blockHeld` are what a decide reads besides the
certificate, and neither can be procured from outside, so both are hypotheses
(see `DecideInv`).
-/
structure DecideDelivery {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) (n : Nat) : Prop where
  /-- The certificate arrives at `n`. -/
  arrival : Run.Consumes r n (Input.certificate2 ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩)

  /-- The node holds the `Cert1` over the view. -/
  cert1Held : ((Run.state r n).cert1s p.viewNumber).isSome

  /-- And the block itself. -/
  blockHeld : (Run.state r n).proposals p.viewNumber = some p

  /-- The certificate has somewhere to go. -/
  writable : Writable ((Run.state r n).cert2s p.viewNumber) ⟨⟨blockHash p, p.epoch⟩, p.viewNumber⟩

  /-- The view is not already decided. -/
  fresh : ¬ (Run.state r n).decidedViews p.viewNumber

  /-- And the decide floor stays below it while the decide is outstanding. -/
  window : DecideWindow r p.viewNumber n

/-- A decide delivery reaches the node at some step. -/
def DecideDelivered {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (p : Proposal) : Prop :=
  ∃ n, DecideDelivery r p n

end NewProtocol
