module

public import NewProtocolSpec.Deadlock.Defs

/-!
# Deadlock freedom

That the rules leave the environment a way in, and that walking in makes the node
act. "A way in" is not unconditional: an input is ingested only where the slot it
writes is free or already holds it, which is what the `Writable` guards of
`ProposalAdmissible` and its companions say. The file has two halves.

The `*_unstalled` results are step-local: for each of the four actions, inputs
the environment may always supply, and the conclusion that after supplying them
the node either owes the action or has already taken it. The `*_forced` results
compose those with `NewProtocolSpec.Progress` over a run — deliver the inputs to
a node whose window is open, and the action happens.

The first half is what `NewProtocolSpec.Progress` takes for granted. There the
hypothesis is that an action is owed; here it is discharged, from the ingestion
obligations of `StepSpec`, the retention clause and the mark obligations.

Why it needs proving at all. Every enabledness predicate is a conjunction of
guards, and every guard reads state that only an input can write
(`SafetySpec.proposalProvenance` and the other provenance clauses). Nothing says
those conjunctions are jointly satisfiable, and a guard that no input can
establish would be a rule no node could ever be obliged to follow — safety would
still hold, and the specification would be describing a protocol that never
does anything. What is checked here is that each obligation's guards can all be
met at once, at the state a step consuming those inputs leaves. That such a step
exists at all is the other package's business (`NewProtocolImpl.conforms`).

What is *not* claimed. Nothing here says the inputs arrive: that is delivery,
which the specification does not model. Nothing says the room is there either —
the step-local results take a `Vote1Room` or its analogue, which says the node
has not committed something later, timed out, or pruned the view, and the
composed ones take a window, which says the same until the node acts. Both are
hypotheses, and both are what a partial-synchrony argument would have to supply.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-! ## The step the vote is owed at

Both vote results end the same way: the last input lands, and what the node
already holds becomes a justification. That step is shared, so it is proved once
here and used by the step-local result and by the composed one, which differ only
in how the holdings reached it.
-/

/--
The step that reports validity leaves a vote1 owed, or casts it.

Everything but the report is a hypothesis: the proposal admitted, the share held,
the parent known, and the room the node has left itself.
-/
theorem vote1_owed_of_validated {cfg : Config} {leader : ViewNumber → Option PubKey}
    {node : PubKey} {s s' : NodeState} {o : List Output} {p : Proposal} {vid : VidShare}
    (hs : StepSpec cfg leader node s (Input.blockValidated p.viewNumber (blockHash p)) o s')
    (hadm : s.admitted p.viewNumber = some p) (hvid : s.vidShares p.viewNumber = some vid)
    (hparent : ParentHeld s p) (hvalid : BlockValid p)
    (hroom : Vote1Room cfg s' p)
    (hwritable : Writable (s.validated p.viewNumber) (blockHash p))
    (hfresh : ¬ s.voted1Views p.viewNumber) :
    Vote1Enabled s' p
      ∨ ∃ vote : Vote1, Output.send (.vote1 vote) ∈ o ∧ vote.view = p.viewNumber
          ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = node := by
  have hfloor : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs.toSafetySpec hroom.floor
  have hkeep := retainsVote_of_step hs hfloor
  have hadm' : s'.admitted p.viewNumber = some p := hkeep.admitted p hadm
  have hval : s'.validated p.viewNumber = some (blockHash p) :=
    StepSpec.blockValidatedIngested hs p.viewNumber (blockHash p) rfl hwritable
  have hjust : Vote1Justification s' p := by
    refine { proposalAdmitted := hadm', blockValid := hvalid, vidShare := ?_
           , safeToExtend := hroom.lock.safe, parentLinked := ?_ }
    · rw [hkeep.vidShares vid hvid]; rfl
    · intro hne
      obtain ⟨parent, hpar, hhash, hrec⟩ := hparent hne
      have hd := retainsDecide_of_step hs
        (SafetySpec.floorMono hs.toSafetySpec (hroom.parentFloor hne))
      exact ⟨parent, hd.proposals parent hpar, hhash, hd.blocksReconstructed _ hrec⟩
  by_cases hmark : s'.voted1Views p.viewNumber
  · obtain ⟨vote, hmem, hview⟩ := StepSpec.vote1Marked hs p.viewNumber hfresh hmark
    obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote1Justified hs.toSafetySpec vote hmem
    have hqp : q = p := by
      have h1 := hjq.proposalAdmitted
      rw [← hviewq, hview] at h1
      exact Option.some_inj.mp (h1.symm.trans hadm')
    exact Or.inr ⟨vote, hmem, hview, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩
  · exact Or.inl ⟨hjust, hval, hmark, hroom.timedOut, hroom.bar, hroom.lock.below⟩

/--
The step that reports a reconstruction leaves a vote2 owed, or casts it.

As `vote1_owed_of_validated`, with the `Cert1` in place of the validity report.
-/
theorem vote2_owed_of_reconstructed {cfg : Config} {leader : ViewNumber → Option PubKey}
    {node : PubKey} {s s' : NodeState} {o : List Output} {p : Proposal} {c : Cert1}
    (hs : StepSpec cfg leader node s (Input.blockReconstructed p.viewNumber p.payloadCommit) o s')
    (hadm : s.admitted p.viewNumber = some p) (hcert : s.cert1s p.viewNumber = some c)
    (hhash : c.data.blockHash = blockHash p) (hepoch : c.data.epoch = p.epoch)
    (hroom : Vote2Room cfg s' p)
    (hfresh : ¬ s.voted2Views p.viewNumber) :
    Vote2Enabled cfg s' p
      ∨ ∃ vote : Vote2, Output.send (.vote2 vote) ∈ o ∧ vote.view = p.viewNumber
          ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch ∧ vote.signer = node := by
  have hfloor : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs.toSafetySpec hroom.floor
  have hadm' : s'.admitted p.viewNumber = some p :=
    (retainsVote_of_step hs hfloor).admitted p hadm
  have hcert' : s'.cert1s p.viewNumber = some c :=
    (retainsDecide_of_step hs hfloor).cert1s c hcert
  have hrec : s'.blocksReconstructed p.viewNumber p.payloadCommit :=
    StepSpec.reconstructedIngested hs p.viewNumber p.payloadCommit rfl
  by_cases hmark : s'.voted2Views p.viewNumber
  · obtain ⟨vote, hmem, hview⟩ := StepSpec.vote2Marked hs p.viewNumber hfresh hmark
    obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote2Justified hs.toSafetySpec vote hmem
    have hqp : q = p := by
      have h1 := hjq.proposalAdmitted
      rw [← hviewq, hview] at h1
      exact Option.some_inj.mp (h1.symm.trans hadm')
    exact Or.inr ⟨vote, hmem, hview, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩
  · exact Or.inl ⟨{ proposalAdmitted := hadm', certMatches := ⟨c, hcert', hhash, hepoch⟩
                  , reconstructed := hrec }
                 , hroom.noSkip, hmark, hroom.noCert2, hroom.notDecided, hroom.floor, hroom.bar⟩

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
    (hadmissible : ProposalAdmissible cfg s p vid) (hroom : Vote1Room cfg s₂ p)
    (hparent : ParentHeld s p) (hvalid : BlockValid p)
    (hwritable : Writable (s₁.validated p.viewNumber) (blockHash p))
    (hfresh : ¬ s.voted1Views p.viewNumber) :
    Vote1Enabled s₂ p
      ∨ ∃ vote : Vote1, (Output.send (.vote1 vote) ∈ o₁ ∨ Output.send (.vote1 vote) ∈ o₂)
          ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch
          ∧ vote.signer = node := by
  obtain ⟨hadm₁, -, hvid₁⟩ :=
    StepSpec.proposalIngested hs₁ sender p vid rfl hadmissible.bar hadmissible.admitted
      hadmissible.proposals hadmissible.vidShares hadmissible.safe hadmissible.wellFormed
      hadmissible.share
  -- The proposal step may cast the vote itself; otherwise the validity step decides.
  by_cases hmark₁ : s₁.voted1Views p.viewNumber
  · obtain ⟨vote, hmem, hview⟩ := StepSpec.vote1Marked hs₁ p.viewNumber hfresh hmark₁
    obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote1Justified hs₁.toSafetySpec vote hmem
    have hqp : q = p := by
      have h1 := hjq.proposalAdmitted
      rw [← hviewq, hview] at h1
      exact Option.some_inj.mp (h1.symm.trans hadm₁)
    exact Or.inr ⟨vote, Or.inl hmem, hview, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩
  · have hparent₁ : ParentHeld s₁ p := by
      intro hne
      obtain ⟨parent, hpar, hhash, hrec⟩ := hparent hne
      have hd := retainsDecide_of_step hs₁ (SafetySpec.floorMono hs₁.toSafetySpec
        (SafetySpec.floorMono hs₂.toSafetySpec (hroom.parentFloor hne)))
      exact ⟨parent, hd.proposals parent hpar, hhash, hd.blocksReconstructed _ hrec⟩
    rcases vote1_owed_of_validated hs₂ hadm₁ hvid₁ hparent₁ hvalid hroom hwritable hmark₁ with
      hen | ⟨vote, hmem, hview, hdata, hsigner⟩
    · exact Or.inl hen
    · exact Or.inr ⟨vote, Or.inr hmem, hview, hdata, hsigner⟩

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
    (hepoch : c.data.epoch = p.epoch) (hwritable : Writable (s.cert1s c.view) c)
    (hfresh : ¬ s.voted2Views p.viewNumber) :
    Vote2Enabled cfg s₂ p
      ∨ ∃ vote : Vote2, (Output.send (.vote2 vote) ∈ o₁ ∨ Output.send (.vote2 vote) ∈ o₂)
          ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch
          ∧ vote.signer = node := by
  have hfloor₁ : s₁.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs₂.toSafetySpec hroom.floor
  have hfloor₀ : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs₁.toSafetySpec hfloor₁
  have hcert₁ : s₁.cert1s p.viewNumber = some c :=
    hview ▸ StepSpec.cert1Ingested hs₁ c (Or.inl rfl) (hview ▸ hfloor₀) hwritable
  have hadm₁ : s₁.admitted p.viewNumber = some p :=
    (retainsVote_of_step hs₁ hfloor₀).admitted p hadmitted
  -- The certificate step may cast the vote itself; otherwise the payload step decides.
  by_cases hmark₁ : s₁.voted2Views p.viewNumber
  · obtain ⟨vote, hmem, hviewv⟩ := StepSpec.vote2Marked hs₁ p.viewNumber hfresh hmark₁
    obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote2Justified hs₁.toSafetySpec vote hmem
    have hqp : q = p := by
      have h1 := hjq.proposalAdmitted
      rw [← hviewq, hviewv] at h1
      exact Option.some_inj.mp (h1.symm.trans hadm₁)
    exact Or.inr ⟨vote, Or.inl hmem, hviewv, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩
  · rcases vote2_owed_of_reconstructed hs₂ hadm₁ hcert₁ hhash hepoch hroom hmark₁ with
      hen | ⟨vote, hmem, hviewv, hdata, hepv, hsigner⟩
    · exact Or.inl hen
    · exact Or.inr ⟨vote, Or.inr hmem, hviewv, hdata, hepv, hsigner⟩

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
    (hready : ProposeReady cfg leader node s p parent)
    (hwritable : Writable (s.headers p.viewNumber (blockHash parent)) p.blockHeader)
    (hroom : ProposeRoom cfg s₁ p)
    (hanchor : p.parentCert.view = ViewNumber.genesis → AnchorKept s s₁)
    (hfresh : ¬ s.proposedViews p.viewNumber) :
    ProposeEnabled cfg leader node s₁ p
      ∨ ∃ q : Proposal, Output.send (.proposal q) ∈ o ∧ q.viewNumber = p.viewNumber := by
  have hfloor₀ : s.aboveDecideFloor cfg p.viewNumber :=
    SafetySpec.floorMono hs.toSafetySpec hroom.floor
  have hparentFloor₀ : p.parentCert.view ≠ ViewNumber.genesis →
      s.aboveDecideFloor cfg p.parentCert.view :=
    fun hne => SafetySpec.floorMono hs.toSafetySpec (hroom.parentFloor hne)
  have hheader : s₁.headers p.viewNumber (blockHash parent) = some p.blockHeader :=
    StepSpec.headerIngested hs p.viewNumber (blockHash parent) p.blockHeader rfl hwritable
  have hkeepV := retainsVote_of_step hs hfloor₀
  have hprop : ∀ b, s.proposals p.parentCert.view = some b →
      s₁.proposals p.parentCert.view = some b := by
    by_cases hgen : p.parentCert.view = ViewNumber.genesis
    · rw [hgen]; exact (hanchor hgen).proposal
    · exact (retainsDecide_of_step hs (hparentFloor₀ hgen)).proposals
  have hcert : ∀ c, s.cert1s p.parentCert.view = some c →
      s₁.cert1s p.parentCert.view = some c := by
    by_cases hgen : p.parentCert.view = ViewNumber.genesis
    · rw [hgen]; exact (hanchor hgen).cert
    · exact (retainsDecide_of_step hs (hparentFloor₀ hgen)).cert1s
  have hjust : ProposalJustification cfg leader node s₁ p := by
    refine { leader := hready.leads, wellFormed := hready.wellFormed, justified := ?_
           , headerBuilt := ⟨parent, hprop parent hready.parentHeld.1,
               hready.parentHeld.2, hheader⟩ }
    have hj := hready.justified
    unfold ParentCertJustified at hj ⊢
    cases hte : p.timeoutEvidence with
    | some tc =>
      rw [hte] at hj
      exact ⟨hkeepV.timeoutCerts tc hj.1, hroom.lock (by rw [hte]; rfl)⟩
    | none =>
      rw [hte] at hj
      refine ⟨?_, hj.2⟩
      rw [parent_view_of_linked hj.2] at hj ⊢
      exact hcert _ hj.1
  by_cases hmark : s₁.proposedViews p.viewNumber
  · obtain ⟨q, hmem, hview⟩ := StepSpec.proposedMarked hs p.viewNumber hfresh hmark
    exact Or.inr ⟨q, hmem, hview⟩
  · exact Or.inl ⟨hjust, hmark, hroom.timedOut, hroom.bar⟩

/-!
## Delivery makes the node act

The two halves in one statement, once per action: the environment delivers the
inputs, the window is open, and the action is taken. `stepSpec_of_consumes` is
what joins them — a step of a run that consumed a named input is a transition the
step-local results apply to.

Each window is anchored at the step the delivery begins, not at the step it ends.
That is forced: the guard on a window's fields is that the action has not gone out
since the anchor, so a window anchored after the delivery would have its guard
hold for ever in the run where the node acted *during* the delivery, and would
demand the view stay unpruned for ever in the one run that does not need it. The
proofs move the anchor inward themselves (`Vote1Window.shift`), which is sound
because the delivery steps are consensus steps and a vote cast at one of them
would be on record at the next.

None of the four takes a room bundle. A window covers the delivery, so the room
the step-local results want follows from it — in the run where the node acted
during the delivery the window says nothing, but there the conclusion is the
action that was taken, so each proof splits on that first.

The action may be taken during the delivery itself rather than after it, and the
conclusions do not distinguish the two: what is claimed is that it happens, not
when. Nothing here says the inputs arrive, and the windows are still hypotheses;
these are the strongest statements the specification supports without a model of
delivery or duration.
-/

/--
**A node handed a proposal and its validity report casts a vote1 for it.**

The two inputs need not be adjacent, though their order is fixed: the proposal
first. Everything the vote reads is carried across the stretch between them by
the window — the floor at every state, the bar at
every successor — which is what `vote1Window_retainsVote` chains.
-/
theorem vote1_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n n₂ : Nat} {sender : PubKey} {p : Proposal}
    {vid : VidShare}
    (hfair : WeaklyFair r)
    (hin₁ : Run.Consumes r n (Input.proposal sender p vid))
    (hin₂ : Run.Consumes r n₂ (Input.blockValidated p.viewNumber (blockHash p)))
    (hlt : n < n₂)
    (hadmissible : ProposalAdmissible cfg (Run.state r n) p vid)
    (hparent : ParentHeld (Run.state r n) p) (hvalid : BlockValid p)
    (hwritable : Writable ((Run.state r n₂).validated p.viewNumber) (blockHash p))
    (hfresh : ¬ (Run.state r n).voted1Views p.viewNumber)
    (hwindow : Vote1Window r p n) :
    ∃ j, ∃ vote : Vote1, Output.send (.vote1 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch
      ∧ vote.signer = node := by
  obtain ⟨o₁, hs₁, _⟩ := stepSpec_of_consumes hin₁
  obtain ⟨o₂, hs₂, hout₂⟩ := stepSpec_of_consumes hin₂
  obtain ⟨hadm₁, -, hvid₁⟩ :=
    StepSpec.proposalIngested hs₁ sender p vid rfl hadmissible.bar hadmissible.admitted
      hadmissible.proposals hadmissible.vidShares hadmissible.safe hadmissible.wellFormed
      hadmissible.share
  by_cases hearly : ∃ j, n ≤ j ∧ j < n₂ + 1 ∧ ∃ vote : Vote1,
      Output.send (.vote1 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber
  · -- Cast on the way: the first such step signs the proposal that is admitted there.
    obtain ⟨j₀, hj₀⟩ := hearly
    obtain ⟨j, ⟨hj, hjlt, vote, hmem, hview⟩, hmin⟩ :=
      exists_least (P := fun j => n ≤ j ∧ j < n₂ + 1 ∧ ∃ vote : Vote1,
        Output.send (.vote1 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber)
        j₀ j₀ (Nat.le_refl j₀) hj₀
    have hpend : Vote1Pending r p n j := fun i hi him vote' hmem' hview' =>
      hmin i him ⟨hi, Nat.lt_trans him hjlt, vote', hmem', hview'⟩
    obtain ⟨input, output, -, hsj, hin⟩ := emit_step r hmem
    have hadmj : (Run.state r (j + 1)).admitted p.viewNumber = some p := by
      rcases Nat.eq_or_lt_of_le hj with rfl | hgt
      · exact hadm₁
      · exact (retainsVote_of_step hsj (hwindow.floor j hj hpend)).admitted p
          ((vote1Window_retainsVote hwindow (by omega) (by omega) hpend).admitted p hadm₁)
    obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote1Justified hsj.toSafetySpec vote hin
    have hqp : q = p := by
      have h1 := hjq.proposalAdmitted
      rw [← hviewq, hview] at h1
      exact Option.some_inj.mp (h1.symm.trans hadmj)
    exact ⟨j, vote, hmem, hview, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩
  · -- Not cast on the way: the vote is owed at the end of the delivery.
    have hpend : Vote1Pending r p n (n₂ + 1) := fun i hi him vote hmem hview =>
      hearly ⟨i, hi, him, vote, hmem, hview⟩
    have hpend₂ : Vote1Pending r p n n₂ := fun i hi him => hpend i hi (Nat.lt_succ_of_lt him)
    have hkeep := vote1Window_retainsVote hwindow (by omega) (show n + 1 ≤ n₂ by omega) hpend₂
    have hparent₂ : ParentHeld (Run.state r n₂) p := by
      intro hne
      obtain ⟨parent, hpar, hhash, hrec⟩ := hparent hne
      have hd := vote1Window_retainsParent hwindow hne (Nat.le_refl n) (by omega) hpend₂
      exact ⟨parent, hd.proposals parent hpar, hhash, hd.blocksReconstructed _ hrec⟩
    have hroom : Vote1Room cfg (Run.state r (n₂ + 1)) p :=
      { bar := hwindow.bar (n₂ + 1) (by omega) hpend
      , timedOut := hwindow.timedOut (n₂ + 1) (by omega) hpend
      , lock := hwindow.lock (n₂ + 1) (by omega) hpend
      , floor := hwindow.floor (n₂ + 1) (by omega) hpend
      , parentFloor := fun hne => hwindow.parentFloor hne (n₂ + 1) (by omega) hpend }
    have hmark : ¬ (Run.state r n₂).voted1Views p.viewNumber :=
      notVoted1_upTo (by omega) hpend₂ (Nat.le_refl n) hfresh
    rcases vote1_owed_of_validated hs₂ (hkeep.admitted p hadm₁) (hkeep.vidShares vid hvid₁)
      hparent₂ hvalid hroom hwritable hmark with
      hen | ⟨vote, hmem, hview, hdata, hsigner⟩
    · exact vote1_cast hfair ⟨n₂ + 1, hen, hwindow.shift (by omega) hpend⟩
    · exact ⟨n₂, vote, by rw [hout₂]; exact hmem, hview, hdata, hsigner⟩

/--
**A node handed the `Cert1` and the payload casts a vote2 for the block.**

As `vote1_forced`, and the two inputs need not be adjacent for the same reason.
Their order is fixed the same way: the certificate first.
-/
theorem vote2_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n n₂ : Nat} {p : Proposal} {c : Cert1}
    (hfair : WeaklyFair r)
    (hin₁ : Run.Consumes r n (Input.certificate1 c))
    (hin₂ : Run.Consumes r n₂ (Input.blockReconstructed p.viewNumber p.payloadCommit))
    (hlt : n < n₂)
    (hadmitted : (Run.state r n).admitted p.viewNumber = some p)
    (hview : c.view = p.viewNumber) (hhash : c.data.blockHash = blockHash p)
    (hepoch : c.data.epoch = p.epoch)
    (hwritable : Writable ((Run.state r n).cert1s c.view) c)
    (hfresh : ¬ (Run.state r n).voted2Views p.viewNumber)
    (hwindow : Vote2Window r p n) :
    ∃ j, ∃ vote : Vote2, Output.send (.vote2 vote) ∈ (Run.event r j).outputs
      ∧ vote.view = p.viewNumber ∧ vote.data.blockHash = blockHash p ∧ vote.data.epoch = p.epoch
      ∧ vote.signer = node := by
  obtain ⟨o₁, hs₁, _⟩ := stepSpec_of_consumes hin₁
  obtain ⟨o₂, hs₂, hout₂⟩ := stepSpec_of_consumes hin₂
  by_cases hearly : ∃ j, n ≤ j ∧ j < n₂ + 1 ∧ ∃ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber
  · obtain ⟨j₀, hj₀⟩ := hearly
    obtain ⟨j, ⟨hj, hjlt, vote, hmem, hviewv⟩, hmin⟩ :=
      exists_least (P := fun j => n ≤ j ∧ j < n₂ + 1 ∧ ∃ vote : Vote2,
        Output.send (.vote2 vote) ∈ (Run.event r j).outputs ∧ vote.view = p.viewNumber)
        j₀ j₀ (Nat.le_refl j₀) hj₀
    have hpend : Vote2Pending r p n j := fun i hi him vote' hmem' hview' =>
      hmin i him ⟨hi, Nat.lt_trans him hjlt, vote', hmem', hview'⟩
    obtain ⟨input, output, -, hsj, hin⟩ := emit_step r hmem
    have hadmj : (Run.state r (j + 1)).admitted p.viewNumber = some p :=
      (retainsVote_of_step hsj (hwindow.floor j hj hpend)).admitted p
        ((vote2Window_retainsVote hwindow (Nat.le_refl n) hj hpend).admitted p hadmitted)
    obtain ⟨q, hjq, hviewq, hsigner, hdata, hepq⟩ := SafetySpec.vote2Justified hsj.toSafetySpec vote hin
    have hqp : q = p := by
      have h1 := hjq.proposalAdmitted
      rw [← hviewq, hviewv] at h1
      exact Option.some_inj.mp (h1.symm.trans hadmj)
    exact ⟨j, vote, hmem, hviewv, hqp ▸ hdata, hqp ▸ hepq, hsigner⟩
  · have hpend : Vote2Pending r p n (n₂ + 1) := fun i hi him vote hmem hview =>
      hearly ⟨i, hi, him, vote, hmem, hview⟩
    have hpend₂ : Vote2Pending r p n n₂ := fun i hi him => hpend i hi (Nat.lt_succ_of_lt him)
    have hkeepV := vote2Window_retainsVote hwindow (by omega) (show n + 1 ≤ n₂ by omega) hpend₂
    have hkeepD := vote2Window_retainsDecide hwindow (by omega) (show n + 1 ≤ n₂ by omega) hpend₂
    have hcert₁ : (Run.state r (n + 1)).cert1s p.viewNumber = some c :=
      hview ▸ StepSpec.cert1Ingested hs₁ c (Or.inl rfl)
        (hview ▸ hwindow.floor n (Nat.le_refl n) (Vote2Pending.refl r p n)) hwritable
    have hadm₁ : (Run.state r (n + 1)).admitted p.viewNumber = some p :=
      (retainsVote_of_step hs₁
        (hwindow.floor n (Nat.le_refl n) (Vote2Pending.refl r p n))).admitted p hadmitted
    have hroom : Vote2Room cfg (Run.state r (n₂ + 1)) p :=
      { bar := hwindow.bar (n₂ + 1) (by omega) hpend
      , floor := hwindow.floor (n₂ + 1) (by omega) hpend
      , noSkip := hwindow.noSkip (n₂ + 1) (by omega) hpend
      , noCert2 := hwindow.noCert2 (n₂ + 1) (by omega) hpend
      , notDecided := hwindow.notDecided (n₂ + 1) (by omega) hpend }
    have hmark : ¬ (Run.state r n₂).voted2Views p.viewNumber :=
      notVoted2_upTo (by omega) hpend₂ (Nat.le_refl n) hfresh
    rcases vote2_owed_of_reconstructed hs₂ (hkeepV.admitted p hadm₁) (hkeepD.cert1s c hcert₁)
      hhash hepoch hroom hmark with hen | ⟨vote, hmem, hviewv, hdata, hepv, hsigner⟩
    · exact vote2_cast hfair ⟨n₂ + 1, hen, hwindow.shift (by omega) hpend⟩
    · exact ⟨n₂, vote, by rw [hout₂]; exact hmem, hviewv, hdata, hepv, hsigner⟩

/-- **A node handed the `Cert2` for a block it holds decides that view.** -/
theorem decide_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {v : ViewNumber} {p : Proposal}
    {c : Cert2}
    (hfair : WeaklyFair r)
    (hin : Run.Consumes r n (Input.certificate2 c))
    (hview : c.view = v)
    (hcert1 : ((Run.state r n).cert1s v).isSome)
    (hheld : (Run.state r n).proposals v = some p)
    (hhash : c.data.blockHash = blockHash p)
    (hwritable : Writable ((Run.state r n).cert2s c.view) c)
    (hfresh : ¬ (Run.state r n).decidedViews v)
    (hwindow : DecideWindow r v n) :
    ∃ j blocks c1 c2 b, Output.decided blocks c1 c2 ∈ (Run.event r j).outputs
      ∧ b ∈ blocks ∧ b.viewNumber = v := by
  obtain ⟨o, hs, hout⟩ := stepSpec_of_consumes hin
  by_cases hearly : ∃ blocks c1 c2 b, Output.decided blocks c1 c2 ∈ o ∧ b ∈ blocks
      ∧ b.viewNumber = v
  · obtain ⟨blocks, c1, c2, b, hmem, hb, hbv⟩ := hearly
    exact ⟨n, blocks, c1, c2, b, by rw [hout]; exact hmem, hb, hbv⟩
  · have hgap : DecidePending r v n (n + 1) := by
      intro i hi him blocks c1 c2 hmemi b hb hbv
      have hin : i = n := by omega
      subst hin
      rw [hout] at hmemi
      exact hearly ⟨blocks, c1, c2, b, hmemi, hb, hbv⟩
    rcases decide_unstalled hs (hwindow.floor (n + 1) (by omega) hgap) hview hcert1 hheld hhash
      hwritable hfresh with hen | ⟨blocks, c1, c2, b, hmem, hb, hbv⟩
    · exact decide_delivered hfair ⟨n + 1, hen, hwindow.shift (by omega) hgap⟩
    · exact ⟨n, blocks, c1, c2, b, by rw [hout]; exact hmem, hb, hbv⟩

/-- **A leader handed a block to propose sends the proposal.** -/
theorem propose_forced {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {p : Proposal} {parent : Block}
    (hfair : WeaklyFair r)
    (hin : Run.Consumes r n
      (Input.headerBuilt p.viewNumber (blockHash parent) p.blockHeader))
    (hready : ProposeReady cfg leader node (Run.state r n) p parent)
    (hwritable : Writable ((Run.state r n).headers p.viewNumber (blockHash parent))
      p.blockHeader)
    (hfresh : ¬ (Run.state r n).proposedViews p.viewNumber)
    (hwindow : ProposeWindow r p n) :
    ∃ j, ∃ q : Proposal, Output.send (.proposal q) ∈ (Run.event r j).outputs
      ∧ q.viewNumber = p.viewNumber := by
  obtain ⟨o, hs, hout⟩ := stepSpec_of_consumes hin
  by_cases hearly : ∃ q : Proposal, Output.send (.proposal q) ∈ o ∧ q.viewNumber = p.viewNumber
  · obtain ⟨q, hmem, hview⟩ := hearly
    exact ⟨n, q, by rw [hout]; exact hmem, hview⟩
  · have hgap : ProposePending r p n (n + 1) := by
      intro i hi him q hmemi hviewi
      have hin : i = n := by omega
      subst hin
      rw [hout] at hmemi
      exact hearly ⟨q, hmemi, hviewi⟩
    have hroom : ProposeRoom cfg (Run.state r (n + 1)) p :=
      { bar := hwindow.bar _ (by omega) hgap
      , timedOut := hwindow.timedOut _ (by omega) hgap
      , floor := hwindow.floor _ (by omega) hgap
      , parentFloor := fun hne => hwindow.parentFloor hne _ (by omega) hgap
      , lock := fun hte => hwindow.lock hte _ (by omega) hgap }
    rcases propose_unstalled hs hready hwritable hroom
      (fun hgen => hwindow.anchorKept hgen n (Nat.le_refl n) (ProposePending.refl r p n)) hfresh
      with hen | ⟨q, hmem, hview⟩
    · exact propose_sent hfair ⟨n + 1, hen, hwindow.shift (by omega) hgap⟩
    · exact ⟨n, q, by rw [hout]; exact hmem, hview⟩

end NewProtocol
