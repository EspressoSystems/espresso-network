module

public import NewProtocolSpec.Progress.Defs

/-!
# Working lemmas for the progress results

Kernel-checked scaffolding for `NewProtocolSpec.Progress`; nothing here is part
of the contract, and an audit can skip the file. It does four things:

* turns `StepSpec.contentRetained` and the two `GcSpec` retention clauses into
  one statement per transition, whichever kind it was;
* carries each freshness mark forward across a step that does not take the
  action, from the mark obligations alone;
* propagates each enabledness predicate along a run inside its window, up to a
  bound, which is where the retention clauses are consumed;
* finds the *first* step an action was taken at, which is the only one whose
  window is still open and so the only one the block a vote signed can be read
  off.
-/

@[expose] public section

namespace NewProtocol

/-! ## Reading a `StepSpec` run through the safety clauses -/

/-- The same run, read through the safety clauses alone. -/
abbrev safeRun {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) : Run cfg (SafetySpec cfg node) :=
  Run.weaken cfg leader node r

theorem safeRun_event {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) (n : Nat) :
    Run.event (safeRun r) n = Run.event r n := rfl

/-- The step relation of a step that consumed a named input. -/
theorem stepSpec_of_consumes {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {n : Nat} {i : Input}
    (h : Run.Consumes r n i) :
    ∃ output, StepSpec cfg leader node (Run.state r n) i output (Run.state r (n + 1))
      ∧ (Run.event r n).outputs = output := by
  obtain ⟨output, hev⟩ := h
  have ht := Run.transition r n
  rw [hev] at ht
  cases ht with
  | step hs => exact ⟨output, hs, by rw [hev]; simp [Event.outputs]⟩

/-! ## Votes of a live network

`Network.ValidCert1` is defined over the safety runs of a `Network`, and a
progress argument has the stronger runs of a `LiveNetwork` in hand.
`LiveNetwork.netRun` says they are the same runs, so a vote emitted in one is a
vote the other records.
-/

/-- A vote1 cast in `LiveNetwork.run` is one `Network.Cast1` records. -/
theorem cast1_of_emit {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) (k : PubKey) (h : C.honest k) (j : Nat) (vote : Vote1)
    (hmem : Output.send (.vote1 vote) ∈ (Run.event (N.run k h) j).outputs) :
    Network.Cast1 cfg N.net k h vote := by
  refine ⟨j, Output.send (.vote1 vote), ?_, rfl⟩
  rw [N.netRun k h, safeRun_event]
  exact hmem

/-- A vote2 cast in `LiveNetwork.run` is one `Network.Cast2` records. -/
theorem cast2_of_emit {cfg : Config} {leader : ViewNumber → Option PubKey} {C : Committee}
    (N : LiveNetwork cfg leader C) (k : PubKey) (h : C.honest k) (j : Nat) (vote : Vote2)
    (hmem : Output.send (.vote2 vote) ∈ (Run.event (N.run k h) j).outputs) :
    Network.Cast2 cfg N.net k h vote := by
  refine ⟨j, Output.send (.vote2 vote), ?_, rfl⟩
  rw [N.netRun k h, safeRun_event]
  exact hmem

/-! ## The first time something happens

Fairness reports *an* emission, and the block a vote signed is read off the state
the emitting step left. That reading needs the window, and the window is only
available while the action is outstanding — so the emission the argument reads
has to be the earliest one, not any one.
-/

/-- Anything that happens has a first time it happens. -/
theorem exists_least {P : Nat → Prop} : ∀ k n, n ≤ k → P n →
    ∃ j, P j ∧ ∀ i, i < j → ¬ P i
  | 0, n, hn, h => by
    have : n = 0 := Nat.le_zero.mp hn
    exact ⟨n, h, fun i hi => absurd (this ▸ hi) (Nat.not_lt_zero i)⟩
  | k + 1, n, hn, h => by
    by_cases hbelow : ∃ i, i < n ∧ P i
    · obtain ⟨i, hi, hPi⟩ := hbelow
      exact exists_least k i (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hi hn)) hPi
    · exact ⟨n, h, fun i hi hPi => hbelow ⟨i, hi, hPi⟩⟩

/-! ## What a transition keeps

The two kinds of step keep the same holdings for different reasons — a consensus
step by `StepSpec.contentRetained`, a collection by `GcSpec.keepsDecideAboveFloor`
and `GcSpec.keepsVoteAboveBar` — and the watermarks differ between the two halves
of the state. These are the statements every persistence proof below uses.
-/

/-- Above the floor, any transition keeps the decide path at `v`. -/
theorem retainsDecide_of_transition {cfg : Config} {leader : ViewNumber → Option PubKey}
    {node : PubKey} {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg v) : RetainsDecide s s' v := by
  cases ht with
  | step hs => exact (StepSpec.contentRetained hs v hfloor).decide
  | collect hg => exact GcSpec.keepsDecideAboveFloor hg v hfloor

/-- Above the floor and above the bar, any transition keeps the vote path at `v`. -/
theorem retainsVote_of_transition {cfg : Config} {leader : ViewNumber → Option PubKey}
    {node : PubKey} {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg v) (hbar : s'.barredView < v) : RetainsVote s s' v := by
  cases ht with
  | step hs => exact (StepSpec.contentRetained hs v hfloor).vote
  | collect hg => exact GcSpec.keepsVoteAboveBar hg v hbar

/--
A consensus step keeps the vote path above the floor, whatever the bar.

`StepSpec.contentRetained` is not keyed on the bar — only collection is — which
is what lets the step that *casts* a vote still be read for the block it voted
for. By then the mark is set and the window has closed.
-/
theorem retainsVote_of_step {cfg : Config} {node : PubKey}
    {s s' : NodeState} {input : Input} {output : List Output} {v : ViewNumber}
    {leader : ViewNumber → Option PubKey}
    (hs : StepSpec cfg leader node s input output s')
    (hfloor : s.aboveDecideFloor cfg v) : RetainsVote s s' v :=
  (StepSpec.contentRetained hs v hfloor).vote

/-- A consensus step keeps the decide path above the floor. -/
theorem retainsDecide_of_step {cfg : Config} {node : PubKey}
    {s s' : NodeState} {input : Input} {output : List Output} {v : ViewNumber}
    {leader : ViewNumber → Option PubKey}
    (hs : StepSpec cfg leader node s input output s')
    (hfloor : s.aboveDecideFloor cfg v) : RetainsDecide s s' v :=
  (StepSpec.contentRetained hs v hfloor).decide

/-! ## The marks a step does not set

One lemma per action, each the mark obligation in the direction progress needs:
a mark is set only by acting (`StepSpec.vote1Marked` and its companions), and
collection never invents one (`GcSpec.voted1Sound` and its companions). So a step
that does not take the action leaves it outstanding.
-/

theorem notVoted1_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hno : ∀ vote : Vote1, Output.send (.vote1 vote) ∈ e.outputs → vote.view ≠ v)
    (h : ¬ s.voted1Views v) : ¬ s'.voted1Views v := by
  intro hmark
  cases ht with
  | step hs =>
    obtain ⟨vote, hmem, hview⟩ := StepSpec.vote1Marked hs v h hmark
    exact hno vote hmem hview
  | collect hg => exact h (GcSpec.voted1Sound hg v hmark)

theorem notVoted2_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hno : ∀ vote : Vote2, Output.send (.vote2 vote) ∈ e.outputs → vote.view ≠ v)
    (h : ¬ s.voted2Views v) : ¬ s'.voted2Views v := by
  intro hmark
  cases ht with
  | step hs =>
    obtain ⟨vote, hmem, hview⟩ := StepSpec.vote2Marked hs v h hmark
    exact hno vote hmem hview
  | collect hg => exact h (GcSpec.voted2Sound hg v hmark)

theorem notDecided_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hno : ∀ blocks c1 c2, Output.decided blocks c1 c2 ∈ e.outputs →
      ∀ b ∈ blocks, b.viewNumber ≠ v)
    (h : ¬ s.decidedViews v) : ¬ s'.decidedViews v := by
  intro hmark
  cases ht with
  | step hs =>
    obtain ⟨blocks, c1, c2, b, hmem, hb, hbv⟩ := StepSpec.decidedMarked hs v h hmark
    exact hno blocks c1 c2 hmem b hb hbv
  | collect hg => exact h (GcSpec.decidedSound hg v hmark)

theorem notProposed_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hno : ∀ q : Proposal, Output.send (.proposal q) ∈ e.outputs → q.viewNumber ≠ v)
    (h : ¬ s.proposedViews v) : ¬ s'.proposedViews v := by
  intro hmark
  cases ht with
  | step hs =>
    obtain ⟨q, hmem, hview⟩ := StepSpec.proposedMarked hs v h hmark
    exact hno q hmem hview
  | collect hg => exact h (GcSpec.proposedSound hg v hmark)

/-! ## A vote1 stays owed

`Vote1Justification` is separated from the freshness mark on purpose: at the step
the vote is emitted the mark is set and `Vote1Enabled` is false, while the
justification still holds — and it is the justification that says which block the
vote signed.
-/

/-- What a vote1's justification needs is kept, inside the window. -/
theorem just1_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {p : Proposal}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg p.viewNumber)
    (hparent : p.parentCert.view ≠ ViewNumber.genesis →
      s.aboveDecideFloor cfg p.parentCert.view)
    (hbar : s'.barredView < p.viewNumber) (hlock : LockAllows s' p)
    (h : Vote1Justification s p) : Vote1Justification s' p := by
  have hv := retainsVote_of_transition ht hfloor hbar
  refine { proposalAdmitted := hv.admitted p h.proposalAdmitted
         , blockValid := h.blockValid
         , vidShare := ?_
         , safeToExtend := hlock.safe
         , parentLinked := ?_ }
  · cases hsh : s.vidShares p.viewNumber with
    | none => exact absurd h.vidShare (by rw [hsh]; simp)
    | some sh => rw [hv.vidShares sh hsh]; rfl
  · intro hne
    obtain ⟨parent, hpar, hhash, hrec⟩ := h.parentLinked hne
    have hd := retainsDecide_of_transition ht (hparent hne)
    exact ⟨parent, hd.proposals parent hpar, hhash, hd.blocksReconstructed _ hrec⟩

/-- A vote1 for `p` stays owed across a step that does not cast it. -/
theorem vote1Enabled_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {p : Proposal}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg p.viewNumber)
    (hparent : p.parentCert.view ≠ ViewNumber.genesis →
      s.aboveDecideFloor cfg p.parentCert.view)
    (hbar : s'.barredView < p.viewNumber) (htv : s'.timeoutView < p.viewNumber)
    (hlock : LockAllows s' p) (hfresh : ¬ s'.voted1Views p.viewNumber)
    (h : Vote1Enabled s p) : Vote1Enabled s' p := by
  obtain ⟨hjust, hvalidated, -, -, -, -⟩ := h
  have hv := retainsVote_of_transition ht hfloor hbar
  exact ⟨just1_step ht hfloor hparent hbar hlock hjust, hv.validated _ hvalidated, hfresh, htv,
    hbar, hlock.below⟩

/--
A vote1 owed when its window opens is owed at every state up to `bound`, as long
as it is not cast before then.
-/
theorem vote1Enabled_upTo {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal} {n bound : Nat}
    (hw : Vote1Window r p n)
    (hno : ∀ i, n ≤ i → i < bound → ∀ vote : Vote1,
      Output.send (.vote1 vote) ∈ (Run.event r i).outputs → vote.view ≠ p.viewNumber)
    (h : Vote1Enabled (Run.state r n) p) :
    ∀ m, n ≤ m → m ≤ bound → Vote1Enabled (Run.state r m) p := by
  intro m hm
  induction hm with
  | refl => intro _; exact h
  | @step m hnm ih =>
    intro hle
    have hmb : m < bound := Nat.lt_of_lt_of_le (Nat.lt_succ_self m) hle
    have hprev := ih (Nat.le_of_lt hmb)
    have hsucc : n ≤ m + 1 := Nat.le_succ_of_le hnm
    have hpend : Vote1Pending r p n m := fun i hi him => hno i hi (Nat.lt_trans him hmb)
    have hpend' : Vote1Pending r p n (m + 1) := fun i hi him => hno i hi (Nat.lt_of_lt_of_le him hle)
    have hfresh' : ¬ (Run.state r (m + 1)).voted1Views p.viewNumber :=
      notVoted1_step (Run.transition r m) (hno m hnm hmb) hprev.2.2.1
    exact vote1Enabled_step (Run.transition r m) (hw.floor m hnm hpend)
      (fun hne => hw.parentFloor hne m hnm hpend) (hw.bar (m + 1) hsucc hpend')
      (hw.timedOut (m + 1) hsucc hpend') (hw.lock (m + 1) hsucc hpend') hfresh' hprev

/-! ## A vote2 stays owed -/

/-- What a vote2's justification needs is kept, inside the window. -/
theorem just2_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {p : Proposal}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg p.viewNumber) (hbar : s'.barredView < p.viewNumber)
    (h : Vote2Justification s p) : Vote2Justification s' p := by
  have hv := retainsVote_of_transition ht hfloor hbar
  have hd := retainsDecide_of_transition ht hfloor
  obtain ⟨c1, hc1, hhash⟩ := h.certMatches
  exact { proposalAdmitted := hv.admitted p h.proposalAdmitted
        , certMatches := ⟨c1, hd.cert1s c1 hc1, hhash⟩
        , reconstructed := hd.blocksReconstructed _ h.reconstructed }

/-- A vote2 for `p` stays owed across a step that does not cast it. -/
theorem vote2Enabled_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {p : Proposal}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg p.viewNumber) (hbar : s'.barredView < p.viewNumber)
    (hfloor' : s'.aboveDecideFloor cfg p.viewNumber)
    (hskip : ¬ Vote1SkippedView s' p.viewNumber) (hcert2 : s'.cert2s p.viewNumber = none)
    (hdecided : ¬ s'.decidedViews p.viewNumber) (hfresh : ¬ s'.voted2Views p.viewNumber)
    (h : Vote2Enabled cfg s p) : Vote2Enabled cfg s' p :=
  ⟨just2_step ht hfloor hbar h.1, hskip, hfresh, hcert2, hdecided, hfloor', hbar⟩

/-- A vote2 owed when its window opens is owed up to `bound`, as long as it is not cast. -/
theorem vote2Enabled_upTo {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal} {n bound : Nat}
    (hw : Vote2Window r p n)
    (hno : ∀ i, n ≤ i → i < bound → ∀ vote : Vote2,
      Output.send (.vote2 vote) ∈ (Run.event r i).outputs → vote.view ≠ p.viewNumber)
    (h : Vote2Enabled cfg (Run.state r n) p) :
    ∀ m, n ≤ m → m ≤ bound → Vote2Enabled cfg (Run.state r m) p := by
  intro m hm
  induction hm with
  | refl => intro _; exact h
  | @step m hnm ih =>
    intro hle
    have hmb : m < bound := Nat.lt_of_lt_of_le (Nat.lt_succ_self m) hle
    have hprev := ih (Nat.le_of_lt hmb)
    have hsucc : n ≤ m + 1 := Nat.le_succ_of_le hnm
    have hpend : Vote2Pending r p n m := fun i hi him => hno i hi (Nat.lt_trans him hmb)
    have hpend' : Vote2Pending r p n (m + 1) := fun i hi him => hno i hi (Nat.lt_of_lt_of_le him hle)
    have hfresh' : ¬ (Run.state r (m + 1)).voted2Views p.viewNumber :=
      notVoted2_step (Run.transition r m) (hno m hnm hmb) hprev.2.2.1
    exact vote2Enabled_step (Run.transition r m) (hw.floor m hnm hpend)
      (hw.bar (m + 1) hsucc hpend') (hw.floor (m + 1) hsucc hpend')
      (hw.noSkip (m + 1) hsucc hpend') (hw.noCert2 (m + 1) hsucc hpend')
      (hw.notDecided (m + 1) hsucc hpend') hfresh' hprev

/-! ## A decide stays owed

Its window is the floor alone: `DecideEnabled` reads the decide path, and both
kinds of step keep all of that above the floor.
-/

/-- A decide for `v` stays owed across a step that does not deliver it. -/
theorem decideEnabled_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {v : ViewNumber}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg v) (hfloor' : s'.aboveDecideFloor cfg v)
    (hfresh : ¬ s'.decidedViews v)
    (h : DecideEnabled cfg s v) : DecideEnabled cfg s' v := by
  obtain ⟨-, -, hc1, c2, q, hc2, hq, hhash⟩ := h
  have hd := retainsDecide_of_transition ht hfloor
  refine ⟨hfresh, hfloor', ?_, c2, q, hd.cert2s c2 hc2, hd.proposals q hq, hhash⟩
  cases hx : s.cert1s v with
  | none => exact absurd hc1 (by rw [hx]; simp)
  | some c => rw [hd.cert1s c hx]; rfl

/-- A decide owed when its window opens is owed up to `bound`, as long as it is not delivered. -/
theorem decideEnabled_upTo {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {v : ViewNumber} {n bound : Nat}
    (hw : DecideWindow r v n)
    (hno : ∀ i, n ≤ i → i < bound → ∀ blocks c1 c2,
      Output.decided blocks c1 c2 ∈ (Run.event r i).outputs → ∀ b ∈ blocks, b.viewNumber ≠ v)
    (h : DecideEnabled cfg (Run.state r n) v) :
    ∀ m, n ≤ m → m ≤ bound → DecideEnabled cfg (Run.state r m) v := by
  intro m hm
  induction hm with
  | refl => intro _; exact h
  | @step m hnm ih =>
    intro hle
    have hmb : m < bound := Nat.lt_of_lt_of_le (Nat.lt_succ_self m) hle
    have hprev := ih (Nat.le_of_lt hmb)
    have hsucc : n ≤ m + 1 := Nat.le_succ_of_le hnm
    have hpend : DecidePending r v n m := fun i hi him => hno i hi (Nat.lt_trans him hmb)
    have hpend' : DecidePending r v n (m + 1) := fun i hi him => hno i hi (Nat.lt_of_lt_of_le him hle)
    have hfresh' : ¬ (Run.state r (m + 1)).decidedViews v :=
      notDecided_step (Run.transition r m) (hno m hnm hmb) hprev.1
    exact decideEnabled_step (Run.transition r m) (hw.floor m hnm hpend)
      (hw.floor (m + 1) hsucc hpend') hfresh' hprev

/-! ## A proposal stays owed -/

/-- The view a proposal extends is the one before it. -/
theorem parent_view_of_linked {p : Proposal} (h : p.parentCert.view + 1 = p.viewNumber) :
    p.viewNumber - 1 = p.parentCert.view := by
  rw [← h]
  show (⟨p.parentCert.view.toNat + 1 - 1⟩ : ViewNumber) = p.parentCert.view
  simp

/-- What a proposal's justification needs is kept, inside the window. -/
theorem justPropose_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {p : Proposal}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg p.viewNumber)
    (hparent : s.aboveDecideFloor cfg p.parentCert.view)
    (hbar : s'.barredView < p.viewNumber)
    (hlock : p.timeoutEvidence.isSome → s'.lockedCert = some p.parentCert)
    (h : ProposalJustification leader node s p) : ProposalJustification leader node s' p := by
  have hv := retainsVote_of_transition ht hfloor hbar
  have hd := retainsDecide_of_transition ht hparent
  refine { leader := h.leader, wellFormed := h.wellFormed, justified := ?_, headerBuilt := ?_ }
  · have hj := h.justified
    cases hte : p.timeoutEvidence with
    | some tc =>
      rw [hte] at hj
      exact ⟨hv.timeoutCerts tc hj.1, hlock (by rw [hte]; rfl)⟩
    | none =>
      rw [hte] at hj
      refine ⟨?_, hj.2⟩
      rw [parent_view_of_linked hj.2] at hj ⊢
      exact hd.cert1s _ hj.1
  · obtain ⟨parent, hpar, hhash, hhdr⟩ := h.headerBuilt
    exact ⟨parent, hd.proposals parent hpar, hhash, hv.headers _ _ hhdr⟩

/-- A proposal stays owed across a step that does not send it. -/
theorem proposeEnabled_step {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s s' : NodeState} {e : Event} {p : Proposal}
    (ht : Transition cfg (StepSpec cfg leader node) s e s')
    (hfloor : s.aboveDecideFloor cfg p.viewNumber)
    (hparent : s.aboveDecideFloor cfg p.parentCert.view)
    (hbar : s'.barredView < p.viewNumber) (htv : s'.timeoutView < p.viewNumber)
    (hlock : p.timeoutEvidence.isSome → s'.lockedCert = some p.parentCert)
    (hfresh : ¬ s'.proposedViews p.viewNumber)
    (h : ProposeEnabled leader node s p) : ProposeEnabled leader node s' p :=
  ⟨justPropose_step ht hfloor hparent hbar hlock h.1, hfresh, htv, hbar⟩

/-- A proposal owed when its window opens is owed up to `bound`, as long as it is not sent. -/
theorem proposeEnabled_upTo {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {r : Run cfg (StepSpec cfg leader node)} {p : Proposal} {n bound : Nat}
    (hw : ProposeWindow r p n)
    (hno : ∀ i, n ≤ i → i < bound → ∀ q : Proposal,
      Output.send (.proposal q) ∈ (Run.event r i).outputs → q.viewNumber ≠ p.viewNumber)
    (h : ProposeEnabled leader node (Run.state r n) p) :
    ∀ m, n ≤ m → m ≤ bound → ProposeEnabled leader node (Run.state r m) p := by
  intro m hm
  induction hm with
  | refl => intro _; exact h
  | @step m hnm ih =>
    intro hle
    have hmb : m < bound := Nat.lt_of_lt_of_le (Nat.lt_succ_self m) hle
    have hprev := ih (Nat.le_of_lt hmb)
    have hsucc : n ≤ m + 1 := Nat.le_succ_of_le hnm
    have hpend : ProposePending r p n m := fun i hi him => hno i hi (Nat.lt_trans him hmb)
    have hpend' : ProposePending r p n (m + 1) := fun i hi him => hno i hi (Nat.lt_of_lt_of_le him hle)
    have hfresh' : ¬ (Run.state r (m + 1)).proposedViews p.viewNumber :=
      notProposed_step (Run.transition r m) (hno m hnm hmb) hprev.2.1
    exact proposeEnabled_step (Run.transition r m) (hw.floor m hnm hpend)
      (hw.parentFloor m hnm hpend) (hw.bar (m + 1) hsucc hpend')
      (hw.timedOut (m + 1) hsucc hpend')
      (fun hte => hw.lock hte (m + 1) hsucc hpend') hfresh' hprev

end NewProtocol
