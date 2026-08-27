module

public import NewProtocolSpec.Progress

/-!
# Deadlock freedom

That the rules leave the environment a way in. For each of the four actions,
inputs the environment may always supply, and the conclusion that after
supplying them the node either owes the action or has already taken it.

This is the half `NewProtocolSpec.Progress` takes for granted. There the
hypothesis is that an action is owed; here it is discharged, from the ingestion
obligations of `StepSpec` and nothing else. Together: deliver the inputs, and the
node acts.

Why it needs proving at all. Every enabledness predicate is a conjunction of
guards, and every guard reads state that only an input can write
(`SafetySpec.proposalProvenance` and the other provenance clauses). Nothing says
those conjunctions are jointly satisfiable, and a guard that no input can
establish would be a rule no node could ever be obliged to follow — safety would
still hold, and the specification would be describing a protocol that never
does anything. What is checked here is that each obligation's guards can all be
met at once, by inputs the specification itself admits.

What is *not* claimed. Nothing here says the inputs arrive: that is delivery,
which the specification does not model. Nothing says the room is there either —
each result takes a `Vote1Room` or its analogue, which says the node has not
committed something later, timed out, or pruned the view. Both are hypotheses,
and both are what a partial-synchrony argument would have to supply.

The results are stated over single transitions rather than runs, because that is
all they need: two steps at most, and no fairness. `Vote1Owed` and its companions
are how they meet the run-level results.
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

The three ways a node closes the window on itself, and the fourth is the parent's
floor. None of them is a stall: a node that has timed out, locked past the view
or pruned it has moved on.
-/
structure Vote1Room (s : NodeState) (p : Proposal) : Prop where
  /-- The view has not timed out. -/
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

/-! ## A vote1 cannot be stalled -/

/--
**Deliver the proposal and the validity report, and a vote1 for it is owed.**

Two steps, and the only two the environment has to take: `Input.proposal`
admits the block (`StepSpec.proposalIngested`) and records the share, and
`Input.blockValidated` supplies the one thing consensus cannot compute
(`StepSpec.blockValidatedIngested`). Everything else `Vote1Enabled` reads is
either kept by `StepSpec.contentRetained` or a hypothesis here.

The disjunction is not a weakness. A node may vote during the very steps that
deliver, and then the vote is not owed afterwards because it has been cast —
which is the conclusion wanted, not a failure of it. Either way a vote1 for
`p`'s block goes out or is owed.

The floor is assumed at the *last* state rather than the first, which is the
weaker assumption: `SafetySpec.floorMono` carries it backwards, and a floor that
rises during the delivery would be a node deciding past the view it is being
handed.
-/
theorem vote1_unstalled {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s₁ s₂ : NodeState} {o₁ o₂ : List Output} {sender : PubKey} {p : Proposal} {vid : VidShare}
    (hs₁ : StepSpec cfg leader node s (Input.proposal sender p vid) o₁ s₁)
    (hs₂ : StepSpec cfg leader node s₁ (Input.blockValidated p.viewNumber (blockHash p)) o₂ s₂)
    (hadmissible : ProposalAdmissible s p vid) (hroom : Vote1Room cfg s₂ p)
    (hparent : ParentHeld s p) (hvalid : BlockValid p)
    (hwritable : Writable (s₁.validated p.viewNumber) (blockHash p))
    (hfresh : ¬ s.voted1Views p.viewNumber) :
    Vote1Enabled s₂ p
      ∨ ∃ vote : Vote1, (Output.send (.vote1 vote) ∈ o₁ ∨ Output.send (.vote1 vote) ∈ o₂)
          ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p
          ∧ vote.signer = node := by
  -- The floor and the bar, carried to the states that need them.
  have hfloor₁ : s₁.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs₂.toSafetySpec hroom.floor
  have hfloor₀ : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs₁.toSafetySpec hfloor₁
  have hbar₁ : s₁.barredView < p.viewNumber := by
    rw [SafetySpec.barredViewUnchanged hs₁.toSafetySpec]; exact hadmissible.bar
  have hbar₂ : s₂.barredView < p.viewNumber := by
    rw [SafetySpec.barredViewUnchanged hs₂.toSafetySpec]; exact hbar₁
  -- What the two deliveries put in the state.
  obtain ⟨hadm₁, hprop₁, hvid₁⟩ :=
    StepSpec.proposalIngested hs₁ sender p vid rfl hadmissible.bar hadmissible.admitted
      hadmissible.proposals hadmissible.vidShares hadmissible.safe hadmissible.wellFormed
      hadmissible.share
  have hvalidated₂ : s₂.validated p.viewNumber = some (blockHash p) :=
    StepSpec.blockValidatedIngested hs₂ p.viewNumber (blockHash p) rfl hwritable
  have hkeep₂ := retainsVote_of_step hs₂ hfloor₁
  have hadm₂ : s₂.admitted p.viewNumber = some p := hkeep₂.admitted p hadm₁
  -- The vote's own justification, at the state the delivery leaves.
  have hjust : Vote1Justification s₂ p := by
    refine { proposalAdmitted := hadm₂, blockValid := hvalid, vidShare := ?_
           , safeToExtend := hroom.lock.safe, parentLinked := ?_ }
    · rw [hkeep₂.vidShares vid hvid₁]; rfl
    · intro hne
      obtain ⟨parent, hpar, hhash, hrec⟩ := hparent hne
      have hd₁ := retainsDecide_of_transition (Transition.step hs₁)
        (SafetySpec.floorMono hs₁.toSafetySpec
          (SafetySpec.floorMono hs₂.toSafetySpec (hroom.parentFloor hne)))
      have hd₂ := retainsDecide_of_transition (Transition.step hs₂)
        (SafetySpec.floorMono hs₂.toSafetySpec (hroom.parentFloor hne))
      exact ⟨parent, hd₂.proposals parent (hd₁.proposals parent hpar), hhash,
        hd₂.blocksReconstructed _ (hd₁.blocksReconstructed _ hrec)⟩
  -- Either the mark is still clear, and the vote is owed, or it was cast on the way.
  by_cases hmark : s₂.voted1Views p.viewNumber
  · refine Or.inr ?_
    by_cases hmark₁ : s₁.voted1Views p.viewNumber
    · obtain ⟨vote, hmem, hview⟩ := StepSpec.vote1Marked hs₁ p.viewNumber hfresh hmark₁
      obtain ⟨q, hjq, hviewq, hsigner, hdata⟩ :=
        SafetySpec.vote1Justified hs₁.toSafetySpec vote hmem
      have hqp : q = p := by
        have h1 := hjq.proposalAdmitted
        rw [← hviewq, hview] at h1
        exact Option.some_inj.mp (h1.symm.trans hadm₁)
      exact ⟨vote, Or.inl hmem, hview, hqp ▸ hdata, hsigner⟩
    · obtain ⟨vote, hmem, hview⟩ := StepSpec.vote1Marked hs₂ p.viewNumber hmark₁ hmark
      obtain ⟨q, hjq, hviewq, hsigner, hdata⟩ :=
        SafetySpec.vote1Justified hs₂.toSafetySpec vote hmem
      have hqp : q = p := by
        have h1 := hjq.proposalAdmitted
        rw [← hviewq, hview] at h1
        exact Option.some_inj.mp (h1.symm.trans hadm₂)
      exact ⟨vote, Or.inr hmem, hview, hqp ▸ hdata, hsigner⟩
  · exact Or.inl ⟨hjust, hvalidated₂, hmark, hroom.timedOut, hbar₂, hroom.lock.below⟩

/-! ## A vote2 cannot be stalled -/

/--
**Deliver the certificate and the payload, and a vote2 for the block is owed.**

The counterpart of `vote1_unstalled` for the second round:
`Input.certificate1` records the `Cert1` (`StepSpec.cert1Ingested`) and
`Input.blockReconstructed` the payload (`StepSpec.reconstructedIngested`).
The proposal must already be admitted, which is what the first round left behind.

`Vote2Justification` is all three of those, so nothing else has to be supplied —
in particular no validity report, the `Cert1` being a quorum's worth of them
already.
-/
theorem vote2_unstalled {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s₁ s₂ : NodeState} {o₁ o₂ : List Output} {p : Proposal} {c : Cert1}
    (hs₁ : StepSpec cfg leader node s (Input.certificate1 c) o₁ s₁)
    (hs₂ : StepSpec cfg leader node s₁
      (Input.blockReconstructed p.viewNumber p.payloadCommit) o₂ s₂)
    (hroom : Vote2Room cfg s₂ p) (hadmitted : s.admitted p.viewNumber = some p)
    (hview : c.view = p.viewNumber) (hhash : c.data.blockHash = blockHash p)
    (hwritable : Writable (s.cert1s c.view) c)
    (hfresh : ¬ s.voted2Views p.viewNumber) :
    Vote2Enabled cfg s₂ p
      ∨ ∃ vote : Vote2, (Output.send (.vote2 vote) ∈ o₁ ∨ Output.send (.vote2 vote) ∈ o₂)
          ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p
          ∧ vote.signer = node := by
  have hfloor₁ : s₁.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs₂.toSafetySpec hroom.floor
  have hfloor₀ : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs₁.toSafetySpec hfloor₁
  -- What the two deliveries put in the state.
  have hcert₁ : s₁.cert1s c.view = some c :=
    StepSpec.cert1Ingested hs₁ c (Or.inl rfl) (hview ▸ hfloor₀) hwritable
  have hrec₂ : s₂.blocksReconstructed p.viewNumber p.payloadCommit :=
    StepSpec.reconstructedIngested hs₂ p.viewNumber p.payloadCommit rfl
  have hkeep₁ := retainsVote_of_step hs₁ hfloor₀
  have hkeep₂ := retainsVote_of_step hs₂ hfloor₁
  have hadm₂ : s₂.admitted p.viewNumber = some p :=
    hkeep₂.admitted p (hkeep₁.admitted p hadmitted)
  have hcert₂ : s₂.cert1s p.viewNumber = some c := by
    have := (retainsDecide_of_step hs₂ hfloor₁).cert1s c (hview ▸ hcert₁)
    exact hview ▸ this
  have hjust : Vote2Justification s₂ p :=
    { proposalAdmitted := hadm₂, certMatches := ⟨c, hcert₂, hhash⟩, reconstructed := hrec₂ }
  by_cases hmark : s₂.voted2Views p.viewNumber
  · refine Or.inr ?_
    by_cases hmark₁ : s₁.voted2Views p.viewNumber
    · obtain ⟨vote, hmem, hviewv⟩ := StepSpec.vote2Marked hs₁ p.viewNumber hfresh hmark₁
      obtain ⟨q, hjq, hviewq, hsigner, hdata⟩ :=
        SafetySpec.vote2Justified hs₁.toSafetySpec vote hmem
      have hqp : q = p := by
        have h1 := hjq.proposalAdmitted
        rw [← hviewq, hviewv] at h1
        exact Option.some_inj.mp (h1.symm.trans (hkeep₁.admitted p hadmitted))
      exact ⟨vote, Or.inl hmem, hviewv, hqp ▸ hdata, hsigner⟩
    · obtain ⟨vote, hmem, hviewv⟩ := StepSpec.vote2Marked hs₂ p.viewNumber hmark₁ hmark
      obtain ⟨q, hjq, hviewq, hsigner, hdata⟩ :=
        SafetySpec.vote2Justified hs₂.toSafetySpec vote hmem
      have hqp : q = p := by
        have h1 := hjq.proposalAdmitted
        rw [← hviewq, hviewv] at h1
        exact Option.some_inj.mp (h1.symm.trans hadm₂)
      exact ⟨vote, Or.inr hmem, hviewv, hqp ▸ hdata, hsigner⟩
  · exact Or.inl ⟨hjust, hroom.noSkip, hmark, hroom.noCert2, hroom.notDecided, hroom.floor,
      hroom.bar⟩

/-! ## A decide cannot be stalled -/

/--
**Deliver the `Cert2`, and the decide is owed.**

One input, because `DecideEnabled` reads nothing else: the node already holds the
block and its `Cert1` — that is what the two voting rounds left — and
`StepSpec.cert2Ingested` supplies the last piece.

Nothing about ancestry appears, which is the point of `DecideEnabled` having none:
a node missing an ancestor decides anyway and delivers a truncated chain
(`StepSpec.decideJustified`), so a hole in history cannot stall the frontier.
-/
theorem decide_unstalled {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s₁ : NodeState} {o : List Output} {v : ViewNumber} {p : Proposal} {c : Cert2}
    (hs : StepSpec cfg leader node s (Input.certificate2 c) o s₁)
    (hfloor : s₁.aboveDecideFloor cfg v) (hview : c.view = v)
    (hcert1 : (s.cert1s v).isSome) (hheld : s.proposals v = some p)
    (hhash : c.data.blockHash = blockHash p)
    (hwritable : Writable (s.cert2s c.view) c)
    (hfresh : ¬ s.decidedViews v) :
    DecideEnabled cfg s₁ v
      ∨ ∃ blocks c1 c2 b, Output.decided blocks c1 c2 ∈ o ∧ b ∈ blocks ∧ b.viewNumber = v := by
  have hfloor₀ : s.aboveDecideFloor cfg v := SafetySpec.floorMono hs.toSafetySpec hfloor
  have hcert₂ : s₁.cert2s v = some c := by
    have := StepSpec.cert2Ingested hs c rfl (hview ▸ hfloor₀) hwritable
    exact hview ▸ this
  have hkeep := retainsDecide_of_step hs hfloor₀
  by_cases hmark : s₁.decidedViews v
  · obtain ⟨blocks, c1, c2, b, hmem, hb, hbv⟩ := StepSpec.decidedMarked hs v hfresh hmark
    exact Or.inr ⟨blocks, c1, c2, b, hmem, hb, hbv⟩
  · refine Or.inl ⟨hmark, hfloor, ?_, c, p, hcert₂, hkeep.proposals p hheld, hhash⟩
    cases hx : s.cert1s v with
    | none => exact absurd hcert1 (by rw [hx]; simp)
    | some c' => rw [hkeep.cert1s c' hx]; rfl

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

  /-- The parent certificate is the locked one, or the previous view's. -/
  justified : match p.timeoutEvidence with
    | some tc => s.timeoutCerts p.viewNumber = some tc ∧ s.lockedCert = some p.parentCert
    | none => s.cert1s (p.viewNumber - 1) = some p.parentCert
        ∧ p.parentCert.view + 1 = p.viewNumber

  /-- And we hold the block it names. -/
  parentHeld : s.proposals p.parentCert.view = some parent
    ∧ (p.parentCert.view ≠ ViewNumber.genesis → blockHash parent = p.parentCert.data.blockHash)

/--
**Deliver the block to propose, and the proposal is owed.**

The leader's side of the same argument. `Input.headerBuilt` is the one input
(`StepSpec.headerIngested`); the rest of `ProposalJustification` is what the
leader already holds, and `ProposeReady` is exactly that.

The header must be built for the parent the node holds, which is what the input's
middle argument says. That is not a restriction on the environment: the parent is
read from the state, and a header built against anything else is one the node
could not propose (`ProposalJustification.headerBuilt`).
-/
theorem propose_unstalled {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s₁ : NodeState} {o : List Output} {p : Proposal} {parent : Block}
    (hs : StepSpec cfg leader node s
      (Input.headerBuilt p.viewNumber (blockHash parent) p.blockHeader) o s₁)
    (hready : ProposeReady leader node s p parent)
    (hwritable : Writable (s.headers p.viewNumber (blockHash parent)) p.blockHeader)
    (hfloor : s₁.aboveDecideFloor cfg p.viewNumber)
    (hparentFloor : s₁.aboveDecideFloor cfg p.parentCert.view)
    (hbar : s₁.barredView < p.viewNumber) (htimedOut : s₁.timeoutView < p.viewNumber)
    (hlock : p.timeoutEvidence.isSome → s₁.lockedCert = some p.parentCert)
    (hfresh : ¬ s.proposedViews p.viewNumber) :
    ProposeEnabled leader node s₁ p
      ∨ ∃ q : Proposal, Output.send (.proposal q) ∈ o ∧ q.viewNumber = p.viewNumber := by
  have hfloor₀ : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs.toSafetySpec hfloor
  have hparentFloor₀ : s.aboveDecideFloor cfg p.parentCert.view :=
    SafetySpec.floorMono hs.toSafetySpec hparentFloor
  have hheader : s₁.headers p.viewNumber (blockHash parent) = some p.blockHeader :=
    StepSpec.headerIngested hs p.viewNumber (blockHash parent) p.blockHeader rfl hwritable
  have hkeepD := retainsDecide_of_step hs hparentFloor₀
  have hkeepV := retainsVote_of_step hs hfloor₀
  have hjust : ProposalJustification leader node s₁ p := by
    refine { leader := hready.leads, wellFormed := hready.wellFormed, justified := ?_
           , headerBuilt := ⟨parent, hkeepD.proposals parent hready.parentHeld.1,
               hready.parentHeld.2, hheader⟩ }
    have hj := hready.justified
    cases hte : p.timeoutEvidence with
    | some tc =>
      rw [hte] at hj
      exact ⟨hkeepV.timeoutCerts tc hj.1, hlock (by rw [hte]; rfl)⟩
    | none =>
      rw [hte] at hj
      refine ⟨?_, hj.2⟩
      rw [parent_view_of_linked hj.2] at hj ⊢
      exact hkeepD.cert1s _ hj.1
  by_cases hmark : s₁.proposedViews p.viewNumber
  · obtain ⟨q, hmem, hview⟩ := StepSpec.proposedMarked hs p.viewNumber hfresh hmark
    exact Or.inr ⟨q, hmem, hview⟩
  · exact Or.inl ⟨hjust, hmark, htimedOut, hbar⟩

/-!
## Delivery makes the node act

The two halves in one statement, once per action: the environment delivers the
inputs, the window is open, and the action is taken. `stepSpec_of_consumes` is
what joins them — a step of a run that consumed a named input is a transition the
step-local results apply to.

The action may be taken during the delivery itself rather than after it, and the
conclusions do not distinguish the two: what is claimed is that it happens, not
when. Nothing here says the inputs arrive, and the windows are still hypotheses;
these are the strongest statements the specification supports without a model of
delivery or duration.
-/

/-- **A node handed a proposal and its validity report casts a vote1 for it.** -/
theorem vote1_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {sender : PubKey} {p : Proposal}
    {vid : VidShare}
    (hfair : WeaklyFair r)
    (hin₁ : Run.Consumes r n (Input.proposal sender p vid))
    (hin₂ : Run.Consumes r (n + 1) (Input.blockValidated p.viewNumber (blockHash p)))
    (hadmissible : ProposalAdmissible (Run.state r n) p vid)
    (hroom : Vote1Room cfg (Run.state r (n + 1 + 1)) p)
    (hparent : ParentHeld (Run.state r n) p) (hvalid : BlockValid p)
    (hwritable : Writable ((Run.state r (n + 1)).validated p.viewNumber) (blockHash p))
    (hfresh : ¬ (Run.state r n).voted1Views p.viewNumber)
    (hwindow : Vote1Window r p (n + 1 + 1)) :
    ∃ j, ∃ vote : Vote1, Output.send (.vote1 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p
      ∧ vote.signer = node := by
  obtain ⟨o₁, hs₁, hout₁⟩ := stepSpec_of_consumes hin₁
  obtain ⟨o₂, hs₂, hout₂⟩ := stepSpec_of_consumes hin₂
  rcases vote1_unstalled hs₁ hs₂ hadmissible hroom hparent hvalid hwritable hfresh with
    hen | ⟨vote, hmem, hview, hdata, hsigner⟩
  · exact vote1_cast hfair ⟨n + 1 + 1, hen, hwindow⟩
  · rcases hmem with hm | hm
    · exact ⟨n, vote, by rw [hout₁]; exact hm, hview, hdata, hsigner⟩
    · exact ⟨n + 1, vote, by rw [hout₂]; exact hm, hview, hdata, hsigner⟩

/-- **A node handed the `Cert1` and the payload casts a vote2 for the block.** -/
theorem vote2_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {p : Proposal} {c : Cert1}
    (hfair : WeaklyFair r)
    (hin₁ : Run.Consumes r n (Input.certificate1 c))
    (hin₂ : Run.Consumes r (n + 1) (Input.blockReconstructed p.viewNumber p.payloadCommit))
    (hroom : Vote2Room cfg (Run.state r (n + 1 + 1)) p)
    (hadmitted : (Run.state r n).admitted p.viewNumber = some p)
    (hview : c.view = p.viewNumber) (hhash : c.data.blockHash = blockHash p)
    (hwritable : Writable ((Run.state r n).cert1s c.view) c)
    (hfresh : ¬ (Run.state r n).voted2Views p.viewNumber)
    (hwindow : Vote2Window r p (n + 1 + 1)) :
    ∃ j, ∃ vote : Vote2, Output.send (.vote2 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p
      ∧ vote.signer = node := by
  obtain ⟨o₁, hs₁, hout₁⟩ := stepSpec_of_consumes hin₁
  obtain ⟨o₂, hs₂, hout₂⟩ := stepSpec_of_consumes hin₂
  rcases vote2_unstalled hs₁ hs₂ hroom hadmitted hview hhash hwritable hfresh with
    hen | ⟨vote, hmem, hviewv, hdata, hsigner⟩
  · exact vote2_cast hfair ⟨n + 1 + 1, hen, hwindow⟩
  · rcases hmem with hm | hm
    · exact ⟨n, vote, by rw [hout₁]; exact hm, hviewv, hdata, hsigner⟩
    · exact ⟨n + 1, vote, by rw [hout₂]; exact hm, hviewv, hdata, hsigner⟩

/-- **A node handed the `Cert2` for a block it holds decides that view.** -/
theorem decide_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {v : ViewNumber} {p : Proposal}
    {c : Cert2}
    (hfair : WeaklyFair r)
    (hin : Run.Consumes r n (Input.certificate2 c))
    (hfloor : (Run.state r (n + 1)).aboveDecideFloor cfg v) (hview : c.view = v)
    (hcert1 : ((Run.state r n).cert1s v).isSome)
    (hheld : (Run.state r n).proposals v = some p)
    (hhash : c.data.blockHash = blockHash p)
    (hwritable : Writable ((Run.state r n).cert2s c.view) c)
    (hfresh : ¬ (Run.state r n).decidedViews v)
    (hwindow : DecideWindow r v (n + 1)) :
    ∃ j blocks c1 c2 b, Output.decided blocks c1 c2 ∈ (Run.event r j).outputs
      ∧ b ∈ blocks ∧ b.viewNumber = v := by
  obtain ⟨o, hs, hout⟩ := stepSpec_of_consumes hin
  rcases decide_unstalled hs hfloor hview hcert1 hheld hhash hwritable hfresh with
    hen | ⟨blocks, c1, c2, b, hmem, hb, hbv⟩
  · exact decide_delivered hfair ⟨n + 1, hen, hwindow⟩
  · exact ⟨n, blocks, c1, c2, b, by rw [hout]; exact hmem, hb, hbv⟩

/-- **A leader handed a block to propose sends the proposal.** -/
theorem propose_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {p : Proposal} {parent : Block}
    (hfair : WeaklyFair r)
    (hin : Run.Consumes r n
      (Input.headerBuilt p.viewNumber (blockHash parent) p.blockHeader))
    (hready : ProposeReady leader node (Run.state r n) p parent)
    (hwritable : Writable ((Run.state r n).headers p.viewNumber (blockHash parent))
      p.blockHeader)
    (hfloor : (Run.state r (n + 1)).aboveDecideFloor cfg p.viewNumber)
    (hparentFloor : (Run.state r (n + 1)).aboveDecideFloor cfg p.parentCert.view)
    (hbar : (Run.state r (n + 1)).barredView < p.viewNumber)
    (htimedOut : (Run.state r (n + 1)).timeoutView < p.viewNumber)
    (hlock : p.timeoutEvidence.isSome →
      (Run.state r (n + 1)).lockedCert = some p.parentCert)
    (hfresh : ¬ (Run.state r n).proposedViews p.viewNumber)
    (hwindow : ProposeWindow r p (n + 1)) :
    ∃ j, ∃ q : Proposal, Output.send (.proposal q) ∈ (Run.event r j).outputs
      ∧ q.viewNumber = p.viewNumber := by
  obtain ⟨o, hs, hout⟩ := stepSpec_of_consumes hin
  rcases propose_unstalled hs hready hwritable hfloor hparentFloor hbar htimedOut hlock hfresh with
    hen | ⟨q, hmem, hview⟩
  · exact propose_sent hfair ⟨n + 1, hen, hwindow⟩
  · exact ⟨n, q, by rw [hout]; exact hmem, hview⟩

end NewProtocol
