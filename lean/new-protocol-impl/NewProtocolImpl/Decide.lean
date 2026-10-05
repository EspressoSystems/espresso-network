module

public import NewProtocolSpec.Lists
public import NewProtocolSpec.Proofs.Inputs

/-!
# Deciding what a node owes

`NewProtocol.Owed` quantifies over what a history received and sent. This module
states each obligation over the finite lists of `NewProtocolImpl.Lists`, proves
the two forms equivalent (`owedB_iff`), and so makes `Owed` decidable. The one
existential the lists do not bound directly, the proposal a leader could make, is
bounded by `proposalCandidates`: every proposal the node may make agrees with one
of them in everything but its identity, which no rule reads.

How `Owed` changes as a step's outputs grow is in `NewProtocolSpec.Proofs.Inputs`:
what it reads of the inputs does not change (`SameInputs`), and what it reads of
the outputs only ever rules obligations out (`owed_antitone`).
-/

@[expose] public section

namespace NewProtocolImpl

open NewProtocol NewProtocol.Lists History

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

/-! ## What a history sent, as lists -/

/-- The vote1s a history sent. -/
def vote1sSent (h : History) : List Vote1 := h.flatMap vote1sOf

/-- The vote2s a history sent. -/
def vote2sSent (h : History) : List Vote2 := h.flatMap vote2sOf

/-- The timeout votes a history sent. -/
def timeoutVotesSent (h : History) : List TimeoutVote := h.flatMap timeoutVotesOf

/-- The proposals a history sent. -/
def proposalsSent (h : History) : List Proposal := h.flatMap proposalsOf

/-- The re-vote requests a step sends. -/
def revotesOf (st : Step) : List RevoteRequest :=
  st.output.filterMap fun o => match o with
    | .send (.revote r) => some r
    | _ => none

/-- The re-vote requests a history sent. -/
def revotesSent (h : History) : List RevoteRequest := h.flatMap revotesOf

/-- The views of every block a history decided. -/
def decidedViews (h : History) : List ViewNumber :=
  h.flatMap fun st => (decidesOf st).flatMap fun d => d.1.map (·.viewNumber)

/-- The proposals a history received, with sender and share. -/
def receivedProposals (h : History) : List (PubKey × Proposal × VidShare) := h.flatMap proposalsIn

/-- The re-vote requests a step received, with sender. -/
def revotesIn (st : Step) : List (PubKey × RevoteRequest) :=
  match st.input with
  | .revote sender r => [(sender, r)]
  | _ => []

/-- The re-vote requests a history received, with sender. -/
def receivedRevotes (h : History) : List (PubKey × RevoteRequest) := h.flatMap revotesIn

/-- The headers built for the node, with the view and parent they were built for. -/
def headersBuilt (h : History) : List (ViewNumber × BlockHash × BlockHeader) :=
  h.filterMap fun st => match st.input with
    | .headerBuilt v parent hdr => some (v, parent, hdr)
    | _ => none

variable {cfg leader node}

theorem mem_timeoutVotesOf_iff {st : Step} {v : TimeoutVote} :
    v ∈ timeoutVotesOf st ↔ Output.send (.timeoutVote v) ∈ st.output := by
  refine ⟨fun hm => ?_, mem_timeoutVotesOf⟩
  obtain ⟨o, ho, he⟩ := List.mem_filterMap.mp hm
  match o, he with
  | .send (.timeoutVote _), rfl => exact ho

theorem mem_proposalsOf_iff {st : Step} {p : Proposal} :
    p ∈ proposalsOf st ↔ Output.send (.proposal p) ∈ st.output := by
  refine ⟨fun hm => ?_, mem_proposalsOf⟩
  obtain ⟨o, ho, he⟩ := List.mem_filterMap.mp hm
  match o, he with
  | .send (.proposal _), rfl => exact ho

theorem mem_revotesOf_iff {st : Step} {r : RevoteRequest} :
    r ∈ revotesOf st ↔ Output.send (.revote r) ∈ st.output := by
  unfold revotesOf
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨o, ho, he⟩
    match o, he with
    | .send (.revote _), rfl => exact ho
  · intro h; exact ⟨_, h, rfl⟩

theorem mem_vote1sSent {h : History} {v : Vote1} : v ∈ vote1sSent h ↔ h.Sent (.vote1 v) := by
  simp only [vote1sSent, List.mem_flatMap, mem_vote1sOf]; rfl

theorem mem_vote2sSent {h : History} {v : Vote2} : v ∈ vote2sSent h ↔ h.Sent (.vote2 v) := by
  simp only [vote2sSent, List.mem_flatMap, mem_vote2sOf]; rfl

theorem mem_timeoutVotesSent {h : History} {v : TimeoutVote} :
    v ∈ timeoutVotesSent h ↔ h.Sent (.timeoutVote v) := by
  simp only [timeoutVotesSent, List.mem_flatMap, mem_timeoutVotesOf_iff]; rfl

theorem mem_proposalsSent {h : History} {p : Proposal} :
    p ∈ proposalsSent h ↔ h.Sent (.proposal p) := by
  simp only [proposalsSent, List.mem_flatMap, mem_proposalsOf_iff]; rfl

theorem mem_revotesSent {h : History} {r : RevoteRequest} :
    r ∈ revotesSent h ↔ h.Sent (.revote r) := by
  simp only [revotesSent, List.mem_flatMap, mem_revotesOf_iff]; rfl

theorem mem_decidedViews {h : History} {w : ViewNumber} :
    w ∈ decidedViews h ↔ h.DecidedView w := by
  simp only [decidedViews, List.mem_flatMap, List.mem_map, DecidedView]
  constructor
  · rintro ⟨st, hst, ⟨blocks, c1, c2⟩, hd, b, hb, rfl⟩
    exact ⟨st, hst, blocks, c1, c2, b, mem_decidesOf.mp hd, hb, rfl⟩
  · rintro ⟨st, hst, blocks, c1, c2, b, hd, hb, rfl⟩
    exact ⟨st, hst, (blocks, c1, c2), mem_decidesOf.mpr hd, b, hb, rfl⟩

theorem mem_receivedProposals {h : History} {s : PubKey} {p : Proposal} {vid : VidShare} :
    (s, p, vid) ∈ receivedProposals h ↔ h.Received (.proposal s p (some vid)) := by
  simp only [receivedProposals, List.mem_flatMap]
  constructor
  · rintro ⟨st, hst, hx⟩
    exact ⟨st, hst, mem_proposalsIn hx⟩
  · rintro ⟨st, hst, hin⟩
    exact ⟨st, hst, by simp [proposalsIn, hin]⟩

theorem mem_receivedRevotes {h : History} {s : PubKey} {r : RevoteRequest} :
    (s, r) ∈ receivedRevotes h ↔ h.Received (.revote s r) := by
  simp only [receivedRevotes, List.mem_flatMap]
  constructor
  · rintro ⟨st, hst, hx⟩
    refine ⟨st, hst, ?_⟩
    unfold revotesIn at hx
    split at hx
    · next hin => simp at hx; obtain ⟨rfl, rfl⟩ := hx; exact hin
    · simp at hx
  · rintro ⟨st, hst, hin⟩
    exact ⟨st, hst, by simp [revotesIn, hin]⟩

theorem mem_headersBuilt {h : History} {v : ViewNumber} {parent : BlockHash} {hdr : BlockHeader} :
    (v, parent, hdr) ∈ headersBuilt h ↔ h.Received (.headerBuilt v parent hdr) := by
  constructor
  · intro hm
    obtain ⟨st, hst, he⟩ := List.mem_filterMap.mp hm
    revert he
    cases hin : st.input <;> intro he <;> simp at he
    obtain ⟨rfl, rfl, rfl⟩ := he
    exact ⟨st, hst, hin⟩
  · rintro ⟨st, hst, hin⟩
    exact List.mem_filterMap.mpr ⟨st, hst, by simp [hin]⟩

theorem mem_cert2sHeld {h : History} {c : Cert2} : c ∈ cert2sHeld h ↔ h.HasCert2 c := by
  refine ⟨hasCert2_of_mem, ?_⟩
  rintro (⟨st, hst, hin⟩ | ⟨c1, p, st, hst, hin⟩)
  all_goals exact List.mem_filterMap.mpr ⟨st, hst, by simp [hin]⟩

/-! ## The obligations, bounded -/

instance (p : Proposal) (vid : VidShare) : Decidable (ShareMatches p vid) :=
  inferInstanceAs (Decidable (_ ∧ _))

instance (a b : Cert1) : Decidable (LockLE a b) := inferInstanceAs (Decidable (_ ∨ _))

instance (l pc : Cert1) (e : EpochNumber) : Decidable (LockAllows l pc e) :=
  inferInstanceAs (Decidable (_ ∨ _))

instance (r : RevoteRequest) : Decidable (RevoteWellFormed cfg r) := by
  unfold RevoteWellFormed
  cases r.timeoutEvidence with
  | none => exact decidable_of_iff (r.cert.view < r.view ∧ r.cert.view + 1 = r.view
      ∧ IsLastBlock r.cert.data.blockNumber cfg.epochHeight) (by simp)
  | some tc => exact decidable_of_iff (r.cert.view < r.view
      ∧ tc.view + 1 = r.view
      ∧ IsLastBlock r.cert.data.blockNumber cfg.epochHeight) (by simp)

/-- `SafeParent`, as a check of the one timeout certificate a proposal can carry. -/
def SafeParentB (p : Proposal) : Prop :=
  match p.timeoutEvidence with
  | none => True
  | some tc => tc.data.epoch = p.epoch ∧ LockAllows tc.data.lock p.parentCert p.epoch

instance (p : Proposal) : Decidable (SafeParentB p) := by
  unfold SafeParentB; split <;> infer_instance

theorem safeParentB_iff {p : Proposal} : SafeParentB p ↔ SafeParent p := by
  unfold SafeParentB SafeParent
  cases p.timeoutEvidence with
  | none => exact ⟨fun _ _ h => (by cases h), fun _ => trivial⟩
  | some tc => exact ⟨fun h tc' ht => by cases ht; exact h, fun h => h tc rfl⟩

instance (p : Proposal) : Decidable (SafeParent p) := decidable_of_iff _ safeParentB_iff

/-- `SafeRevote`, as a check of the one timeout certificate a request can carry. -/
def SafeRevoteB (r : RevoteRequest) : Prop :=
  match r.timeoutEvidence with
  | none => True
  | some tc => tc.data.epoch = r.cert.data.epoch ∧ LockAllows tc.data.lock r.cert r.cert.data.epoch

instance (r : RevoteRequest) : Decidable (SafeRevoteB r) := by
  unfold SafeRevoteB; split <;> infer_instance

theorem safeRevoteB_iff {r : RevoteRequest} : SafeRevoteB r ↔ SafeRevote r := by
  unfold SafeRevoteB SafeRevote
  cases r.timeoutEvidence with
  | none => exact ⟨fun _ _ h => (by cases h), fun _ => trivial⟩
  | some tc => exact ⟨fun h tc' ht => by cases ht; exact h, fun h => h tc rfl⟩

instance (r : RevoteRequest) : Decidable (SafeRevote r) := decidable_of_iff _ safeRevoteB_iff

variable (cfg leader node)

/-- `NotBehind`, against the epoch the node is in. -/
def NotBehindB (h : History) (e : EpochNumber) : Prop := epochOfHistory cfg h ≤ e

/-- `CertJustified`, over lists. -/
def CertJustifiedB (h : History) (c : Cert1) : Option TimeoutCert → Prop
  | none => c ∈ buildables cfg h
  | some tc => c ∈ cert1sHeld cfg h ∧ ∃ m ∈ List.range (h.length + 1),
      tc ∈ timeoutCertsIn (h.upTo m)
      ∧ ((∃ l ∈ lockedOn cfg (h.upTo m), l.data = c.data)
        ∨ (IsLastBlock c.data.blockNumber cfg.epochHeight
          ∧ ∃ c2 ∈ cert2sHeld (h.upTo m), c2.data = c.data.toVote2))

/-- `OpensEpochJustified`, over lists. -/
def OpensB (h : History) (p : Proposal) : Prop :=
  EntersEpoch cfg p → (∃ parent ∈ proposalsHeld cfg h, parent.viewNumber = p.parentCert.view
    ∧ p.parentCert.data.blockHash = blockHash parent)
    ∧ ∃ c2 ∈ cert2sHeld h ++ [cfg.anchorCert2], c2.view < p.viewNumber ∧ c2.data = p.parentCert.data.toVote2

/-- `ProposalJustified`, over lists. -/
def ProposalJustifiedB (h : History) (p : Proposal) : Prop :=
  leader p.epoch p.viewNumber = some node ∧ ProposalWellFormed cfg p
    ∧ CertJustifiedB cfg h p.parentCert p.timeoutEvidence
    ∧ (∃ parent ∈ proposalsHeld cfg h, parent.viewNumber ≤ p.parentCert.view
        ∧ p.parentCert.data.blockHash = blockHash parent)
    ∧ OpensB cfg h p ∧ SafeParentB p ∧ NotBehindB cfg h p.epoch ∧ p.viewNumber ≤ viewOf cfg h
    ∧ h.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash p.blockHeader)

/-- `RevoteJustified`, over lists. -/
def RevoteJustifiedB (h : History) (r : RevoteRequest) : Prop :=
  leader r.cert.data.epoch r.view = some node ∧ RevoteWellFormed cfg r
    ∧ CertJustifiedB cfg h r.cert r.timeoutEvidence
    ∧ (r.timeoutEvidence = none → r.cert ∈ lockables cfg h) ∧ SafeRevoteB r
    ∧ NotBehindB cfg h r.cert.data.epoch ∧ r.view ≤ viewOf cfg h

/-- The timeout evidence worth trying: none, or any timeout certificate received. -/
def evidenceCandidates (h : History) : List (Option TimeoutCert) :=
  none :: (timeoutCertsIn h).map some

/-- The proposals for view `v`, one per header built for `v`, parent certificate held, and evidence in `evs`. -/
def proposalsWith (h : History) (v : ViewNumber) (evs : List (Option TimeoutCert)) : List Proposal :=
  ((headersBuilt h).filter fun x => decide (x.1 = v)).flatMap fun x =>
    (cert1sHeld cfg h).flatMap fun c =>
      evs.map fun te =>
        { blockHeader := x.2.2, viewNumber := v, epoch := epochOf x.2.2.blockNumber cfg.epochHeight,
          parentCert := c, timeoutEvidence := te, identity := ⟨0⟩ }

/--
The proposals worth trying for view `v`.

If the node may propose for `v`, one of these it may propose too
(`proposalCandidates_complete`).
-/
def proposalCandidates (h : History) (v : ViewNumber) : List Proposal :=
  proposalsWith cfg h v (evidenceCandidates h)

/-- The re-vote requests for view `v`, one per certificate held and evidence in `evs`. -/
def revotesWith (h : History) (v : ViewNumber) (evs : List (Option TimeoutCert)) : List RevoteRequest :=
  (cert1sHeld cfg h).flatMap fun c => evs.map fun te => ⟨c, v, te⟩

/-- The re-vote requests worth trying for view `v`. -/
def revoteCandidates (h : History) (v : ViewNumber) : List RevoteRequest :=
  revotesWith cfg h v (evidenceCandidates h)

/-- `History.AfterFloor`, over lists. -/
def AboveFloorB (h : History) (v : ViewNumber) : Prop :=
  cfg.anchorView < v ∧ ∀ w ∈ decidedViews h, w - cfg.decideBuffer < v

/-- `History.TimedOut`, over lists. -/
def TimedOutB (h : History) (v : ViewNumber) : Prop := ∃ vote ∈ timeoutVotesSent h, v ≤ vote.view

/-- `History.PastView`, over lists. -/
def PastViewB (h : History) (v : ViewNumber) : Prop :=
  TimedOutB h v ∨ ∃ tc ∈ timeoutCertsIn h, v ≤ tc.view

/-- `Owed`, over lists. -/
def OwedB (h : History) : Obligation → Prop
  | .vote1 p =>
      (∃ x ∈ receivedProposals h, x.2.1 = p ∧ leader p.epoch p.viewNumber = some x.1
        ∧ ShareMatches p x.2.2)
      ∧ ProposalWellFormed cfg p ∧ h.Received (.blockValidated p.viewNumber (blockHash p))
      ∧ (p.parentCert.view = cfg.anchorView ∨ EntersEpoch cfg p
          ∨ ∃ parent ∈ proposalsHeld cfg h, parent.viewNumber ≤ p.parentCert.view
              ∧ p.parentCert.data.blockHash = blockHash parent
              ∧ h.HasPayload cfg parent.viewNumber parent.payloadCommit)
      ∧ SafeParentB p ∧ OpensB cfg h p ∧ NotBehindB cfg h p.epoch
      ∧ ¬ TimedOutB h p.viewNumber
      ∧ (∀ vote ∈ vote1sSent h, vote.data.epoch = p.epoch → vote.view ≠ p.viewNumber)
      ∧ viewOf cfg h = p.viewNumber
  | .vote1Again r =>
      (∃ x ∈ receivedRevotes h, x.2 = r ∧ leader r.cert.data.epoch r.view = some x.1)
      ∧ RevoteWellFormed cfg r ∧ SafeRevoteB r
      ∧ (∃ b ∈ proposalsHeld cfg h, Certifies r.cert b ∧ h.HasPayload cfg b.viewNumber b.payloadCommit)
      ∧ NotBehindB cfg h r.cert.data.epoch ∧ ¬ TimedOutB h r.view
      ∧ (∀ vote ∈ vote1sSent h, vote.data.epoch = r.cert.data.epoch → vote.view ≠ r.view)
      ∧ viewOf cfg h = r.view
  | .vote2 c =>
      (c ∈ cert1sHeld cfg h ∧ ∃ b ∈ proposalsHeld cfg h, Certifies c b
        ∧ h.HasPayload cfg b.viewNumber b.payloadCommit)
      ∧ (∀ vote ∈ vote2sSent h, vote.data.epoch = c.data.epoch → vote.view ≠ c.view)
      ∧ (∀ c2 ∈ cert2sHeld h, c2.data.epoch = c.data.epoch → c2.view ≠ c.view)
      ∧ ¬ PastViewB h c.view ∧ AboveFloorB cfg h c.view
  | .decide c =>
      c ∈ cert2sHeld h ∧ ∃ b ∈ proposalsHeld cfg h, Commits c b
        ∧ (∃ c1 ∈ cert1sHeld cfg h, Certifies c1 b)
        ∧ b.viewNumber ∉ decidedViews h ∧ AboveFloorB cfg h b.viewNumber
  | .propose e v =>
      ((∃ p ∈ proposalCandidates cfg h v, p.epoch = e ∧ ProposalJustifiedB cfg leader node h p)
        ∨ ∃ r ∈ revoteCandidates cfg h v, r.cert.data.epoch = e ∧ RevoteJustifiedB cfg leader node h r)
      ∧ (∀ p ∈ proposalsSent h, p.viewNumber = v → p.epoch ≠ e)
      ∧ (∀ r ∈ revotesSent h, r.view = v → r.cert.data.epoch ≠ e)
      ∧ ¬ TimedOutB h v ∧ viewOf cfg h = v

instance (h : History) (e : EpochNumber) : Decidable (NotBehindB cfg h e) := by
  unfold NotBehindB; infer_instance

instance (h : History) (c : Cert1) (ev : Option TimeoutCert) : Decidable (CertJustifiedB cfg h c ev) := by
  cases ev <;> unfold CertJustifiedB <;> infer_instance

instance (h : History) (p : Proposal) : Decidable (OpensB cfg h p) := by
  unfold OpensB; infer_instance

instance (h : History) (p : Proposal) : Decidable (ProposalJustifiedB cfg leader node h p) := by
  unfold ProposalJustifiedB; infer_instance

instance (h : History) (r : RevoteRequest) : Decidable (RevoteJustifiedB cfg leader node h r) := by
  unfold RevoteJustifiedB; infer_instance

instance (h : History) (v : ViewNumber) : Decidable (AboveFloorB cfg h v) := by
  unfold AboveFloorB; infer_instance

instance (h : History) (v : ViewNumber) : Decidable (TimedOutB h v) := by
  unfold TimedOutB; infer_instance

instance (h : History) (v : ViewNumber) : Decidable (PastViewB h v) := by
  unfold PastViewB; infer_instance

instance decOwedB (h : History) : (o : Obligation) → Decidable (OwedB cfg leader node h o)
  | .vote1 _ => by unfold OwedB; infer_instance
  | .vote1Again _ => by unfold OwedB; infer_instance
  | .vote2 _ => by unfold OwedB; infer_instance
  | .decide _ => by unfold OwedB; infer_instance
  | .propose _ _ => by unfold OwedB; infer_instance

variable {cfg leader node}

/-! ## The bounded forms say the same -/

theorem aboveFloorB_iff {h : History} {v : ViewNumber} : AboveFloorB cfg h v ↔ h.AfterFloor cfg v := by
  simp only [AboveFloorB, AfterFloor, mem_decidedViews]

theorem timedOutB_iff {h : History} {v : ViewNumber} : TimedOutB h v ↔ h.TimedOut v := by
  simp only [TimedOutB, TimedOut, mem_timeoutVotesSent]

theorem inView_iff_viewOf {h : History} {v : ViewNumber} : h.InView cfg v ↔ viewOf cfg h = v :=
  ⟨viewOf_of_inView, fun e => e ▸ inView_viewOf cfg h⟩

theorem pastViewB_iff {h : History} {v : ViewNumber} : PastViewB h v ↔ h.PastView v := by
  simp only [PastViewB, PastView, timedOutB_iff, mem_timeoutCertsIn]

theorem notBehindB_iff {h : History} {e : EpochNumber} : NotBehindB cfg h e ↔ NotBehind cfg h e := by
  have hE := inEpoch_epochOfHistory cfg h
  constructor
  · intro hle e' he'
    have : e' = epochOfHistory cfg h :=
      EpochNumber.ext (Nat.le_antisymm (hE.2 e' he'.1) (he'.2 _ hE.1))
    rw [this]; exact hle
  · intro hnb; exact hnb _ hE

theorem reached_iff_viewOf {h : History} {v : ViewNumber} :
    (∃ u, h.ViewGround cfg u ∧ v ≤ u) ↔ v ≤ viewOf cfg h :=
  ⟨fun ⟨u, hu, hvu⟩ => Nat.le_trans hvu ((inView_viewOf cfg h).2 u hu),
    fun hle => ⟨viewOf cfg h, (inView_viewOf cfg h).1, hle⟩⟩

theorem take_length_le {h : History} {m : Nat} (hm : h.length ≤ m) : h.upTo m = h.upTo h.length := by
  simp only [History.upTo, List.take_of_length_le hm, List.take_length]

theorem certJustifiedB_iff {h : History} {c : Cert1} {ev : Option TimeoutCert} :
    CertJustifiedB cfg h c ev ↔ CertJustified cfg h c ev := by
  cases ev with
  | none => exact mem_buildables
  | some tc =>
    have hside : ∀ m, ((∃ l ∈ lockedOn cfg (h.upTo m), l.data = c.data)
        ∨ (IsLastBlock c.data.blockNumber cfg.epochHeight
          ∧ ∃ c2 ∈ cert2sHeld (h.upTo m), c2.data = c.data.toVote2))
        ↔ ((∃ l, (h.upTo m).LockedOn cfg l ∧ l.data = c.data)
          ∨ (IsLastBlock c.data.blockNumber cfg.epochHeight
            ∧ ∃ c2, (h.upTo m).HasCert2 c2 ∧ c2.data = c.data.toVote2)) := fun m => by
      simp only [mem_lockedOn, mem_cert2sHeld]
    simp only [CertJustifiedB, CertJustified, mem_cert1sHeld]
    apply and_congr Iff.rfl
    constructor
    · rintro ⟨m, _, htc, hs⟩
      exact ⟨m, mem_timeoutCertsIn.mp htc, (hside m).mp hs⟩
    · rintro ⟨m, htc, hs⟩
      by_cases hm : m ≤ h.length
      · exact ⟨m, List.mem_range.mpr (by omega), mem_timeoutCertsIn.mpr htc, (hside m).mpr hs⟩
      · have he := take_length_le (h := h) (m := m) (by omega)
        rw [he] at htc hs
        exact ⟨h.length, List.mem_range.mpr (by omega), mem_timeoutCertsIn.mpr htc, (hside _).mpr hs⟩

theorem opensB_iff {h : History} {p : Proposal} : OpensB cfg h p ↔ OpensEpochJustified cfg h p := by
  simp only [OpensB, OpensEpochJustified, mem_proposalsHeld, List.mem_append, mem_cert2sHeld,
    List.mem_singleton]

theorem proposalJustifiedB_iff {h : History} {p : Proposal} :
    ProposalJustifiedB cfg leader node h p ↔ ProposalJustified cfg leader node h p := by
  constructor
  · rintro ⟨hl, hw, hj, ⟨parent, hp, hv, hh⟩, hopen, hsafe, hcur, hreach, hb⟩
    exact ⟨⟨hl, hw, certJustifiedB_iff.mp hj, ⟨parent, mem_proposalsHeld.mp hp, hv, hh⟩,
      opensB_iff.mp hopen, safeParentB_iff.mp hsafe, notBehindB_iff.mp hcur,
      reached_iff_viewOf.mpr hreach⟩, hb⟩
  · rintro ⟨⟨hl, hw, hj, ⟨parent, hp, hv, hh⟩, hopen, hsafe, hcur, hreach⟩, hb⟩
    exact ⟨hl, hw, certJustifiedB_iff.mpr hj, ⟨parent, mem_proposalsHeld.mpr hp, hv, hh⟩,
      opensB_iff.mpr hopen, safeParentB_iff.mpr hsafe, notBehindB_iff.mpr hcur,
      reached_iff_viewOf.mp hreach, hb⟩

theorem revoteJustifiedB_iff {h : History} {r : RevoteRequest} :
    RevoteJustifiedB cfg leader node h r ↔ RevoteJustified cfg leader node h r := by
  constructor
  · rintro ⟨hl, hw, hj, hk, hs, hc, hreach⟩
    exact ⟨hl, hw, certJustifiedB_iff.mp hj, fun h => mem_lockables.mp (hk h), safeRevoteB_iff.mp hs,
      notBehindB_iff.mp hc, reached_iff_viewOf.mpr hreach⟩
  · rintro ⟨hl, hw, hj, hk, hs, hc, hreach⟩
    exact ⟨hl, hw, certJustifiedB_iff.mpr hj, fun h => mem_lockables.mpr (hk h), safeRevoteB_iff.mpr hs,
      notBehindB_iff.mpr hc, reached_iff_viewOf.mp hreach⟩


/-- A lock the node held at some point is a certificate it holds now. -/
theorem hasCert1_of_lockable_upTo {h : History} {m : Nat} {c : Cert1}
    (hl : (h.upTo m).Lockable cfg c) : h.HasCert1 cfg c := by
  have hr : ∀ i, (h.upTo m).Received i → h.Received i :=
    fun _ ⟨st, hst, hin⟩ => ⟨st, List.mem_of_mem_take hst, hin⟩
  rcases hl with rfl | ⟨hc, _⟩ | ⟨c2, p, hr', _⟩
  · exact Or.inl rfl
  · rcases hc with rfl | hc | ⟨c2, p, hc⟩
    · exact Or.inl rfl
    · exact Or.inr (Or.inl (hr _ hc))
    · exact Or.inr (Or.inr ⟨c2, p, hr _ hc⟩)
  · exact Or.inr (Or.inr ⟨c2, p, hr _ hr'⟩)

/-- The certificate a justified proposal or request builds on is one the node holds. -/
theorem hasCert1_of_certJustified {h : History} {c : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg h c ev) : h.HasCert1 cfg c := by
  cases ev with
  | none =>
    rcases (hj : h.Buildable cfg c) with rfl | ⟨hc, -⟩
    · exact Or.inl rfl
    · exact hc
  | some tc => exact hj.1

/-- The timeout evidence a justified proposal or request carries is among the candidates. -/
theorem evidence_mem_of_certJustified {h : History} {c : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg h c ev) : ev ∈ evidenceCandidates h := by
  cases ev with
  | none => exact List.mem_cons_self
  | some tc =>
    obtain ⟨-, m, ⟨st, hst, hin⟩, -⟩ := hj
    exact List.mem_cons_of_mem _ (List.mem_map.mpr
      ⟨tc, mem_timeoutCertsIn.mpr ⟨st, List.mem_of_mem_take hst, hin⟩, rfl⟩)

theorem mem_proposalsWith {h : History} {v : ViewNumber} {evs : List (Option TimeoutCert)}
    {p : Proposal} (hp : p ∈ proposalsWith cfg h v evs) :
    p.identity = ⟨0⟩ ∧ p.viewNumber = v ∧ p.timeoutEvidence ∈ evs := by
  simp only [proposalsWith, List.mem_flatMap, List.mem_map] at hp
  obtain ⟨_, _, _, _, te, hte, rfl⟩ := hp
  exact ⟨rfl, rfl, hte⟩

theorem viewNumber_of_mem_candidates {h : History} {v : ViewNumber} {p : Proposal}
    (hp : p ∈ proposalCandidates cfg h v) : p.viewNumber = v :=
  (mem_proposalsWith hp).2.1

/-- A candidate has identity zero, and any timeout evidence it carries the node received. -/
theorem mem_proposalCandidates {h : History} {v : ViewNumber} {p : Proposal}
    (hp : p ∈ proposalCandidates cfg h v) :
    p.identity = ⟨0⟩ ∧ p.viewNumber = v ∧ (p.timeoutEvidence = none
      ∨ ∃ tc, p.timeoutEvidence = some tc ∧ h.Received (.timeoutCertificate tc)) := by
  obtain ⟨hi, hv, hte⟩ := mem_proposalsWith hp
  rcases List.mem_cons.mp hte with hte | hte
  · exact ⟨hi, hv, Or.inl hte⟩
  · obtain ⟨tc, htc, he⟩ := List.mem_map.mp hte
    exact ⟨hi, hv, Or.inr ⟨tc, he.symm, mem_timeoutCertsIn.mp htc⟩⟩

theorem mem_revotesWith {h : History} {v : ViewNumber} {evs : List (Option TimeoutCert)}
    {r : RevoteRequest} (hr : r ∈ revotesWith cfg h v evs) :
    r.cert ∈ cert1sHeld cfg h ∧ r.view = v ∧ r.timeoutEvidence ∈ evs := by
  simp only [revotesWith, List.mem_flatMap, List.mem_map] at hr
  obtain ⟨c, hc, te, hte, rfl⟩ := hr
  exact ⟨hc, rfl, hte⟩

theorem view_of_mem_revoteCandidates {h : History} {v : ViewNumber} {r : RevoteRequest}
    (hr : r ∈ revoteCandidates cfg h v) : r.view = v :=
  (mem_revotesWith hr).2.1

/-- If the node may propose for `v`, it may propose one of the candidates, of the same epoch. -/
theorem proposalCandidates_complete {h : History} {p : Proposal}
    (hp : ProposalJustified cfg leader node h p) :
    ∃ q ∈ proposalCandidates cfg h p.viewNumber, q.epoch = p.epoch ∧ ProposalJustified cfg leader node h q := by
  obtain ⟨⟨hl, hw, hj, hpar, hopen, hsafe, hcur, hreach⟩, hb⟩ := hp
  refine ⟨{ p with identity := ⟨0⟩ }, ?_, rfl,
    ⟨hl, ⟨hw.parentEarlier, hw.covered, hw.epoch, hw.height⟩, hj, hpar, hopen, hsafe, hcur, hreach⟩, hb⟩
  have hmem : ∀ evs, p.timeoutEvidence ∈ evs →
      { p with identity := ⟨0⟩ } ∈ proposalsWith cfg h p.viewNumber evs := fun evs hte => by
    simp only [proposalsWith, List.mem_flatMap, List.mem_filter, List.mem_map, decide_eq_true_eq]
    refine ⟨(p.viewNumber, p.parentCert.data.blockHash, p.blockHeader),
      ⟨mem_headersBuilt.mpr hb, rfl⟩, p.parentCert, mem_cert1sHeld.mpr (hasCert1_of_certJustified hj),
      p.timeoutEvidence, hte, ?_⟩
    rw [← hw.epoch]
  exact hmem _ (evidence_mem_of_certJustified hj)

/-- If the node may ask for a re-vote for `v`, it may send one of the candidates. -/
theorem revoteCandidates_complete {h : History} {r : RevoteRequest}
    (hr : RevoteJustified cfg leader node h r) :
    r ∈ revoteCandidates cfg h r.view := by
  have hmem : ∀ evs, r.timeoutEvidence ∈ evs → r ∈ revotesWith cfg h r.view evs := fun evs hte => by
    simp only [revotesWith, List.mem_flatMap, List.mem_map]
    exact ⟨r.cert, mem_cert1sHeld.mpr (hasCert1_of_certJustified hr.justified), r.timeoutEvidence,
      hte, rfl⟩
  exact hmem _ (evidence_mem_of_certJustified hr.justified)

theorem owedVote1_iff {h : History} {p : Proposal} :
    OwedVote1 cfg leader h p ↔
      ((∃ sender vid, h.Received (.proposal sender p (some vid)) ∧ leader p.epoch p.viewNumber = some sender
        ∧ ShareMatches p vid)
      ∧ ProposalWellFormed cfg p ∧ h.Received (.blockValidated p.viewNumber (blockHash p))
      ∧ ParentReady cfg h p ∧ SafeParent p ∧ OpensEpochJustified cfg h p ∧ NotBehind cfg h p.epoch
      ∧ ¬ h.TimedOut p.viewNumber
      ∧ (∀ vote : Vote1, h.Sent (.vote1 vote) → vote.data.epoch = p.epoch → vote.view ≠ p.viewNumber)
      ∧ h.InView cfg p.viewNumber) :=
  ⟨fun ⟨a, b, c, d, e, f, g, i, j, k⟩ => ⟨a, b, c, d, e, f, g, i, j, k⟩,
    fun ⟨a, b, c, d, e, f, g, i, j, k⟩ => ⟨a, b, c, d, e, f, g, i, j, k⟩⟩

theorem owedVote1Again_iff {h : History} {r : RevoteRequest} :
    OwedVote1Again cfg leader h r ↔
      ((∃ sender, h.Received (.revote sender r) ∧ leader r.cert.data.epoch r.view = some sender)
      ∧ RevoteWellFormed cfg r ∧ SafeRevote r
      ∧ (∃ b, h.HasProposal cfg b ∧ Certifies r.cert b ∧ h.HasPayload cfg b.viewNumber b.payloadCommit)
      ∧ NotBehind cfg h r.cert.data.epoch ∧ ¬ h.TimedOut r.view
      ∧ (∀ vote : Vote1, h.Sent (.vote1 vote) → vote.data.epoch = r.cert.data.epoch → vote.view ≠ r.view)
      ∧ h.InView cfg r.view) :=
  ⟨fun ⟨a, b, c, d, e, f, g, i⟩ => ⟨a, b, c, d, e, f, g, i⟩,
    fun ⟨a, b, c, d, e, f, g, i⟩ => ⟨a, b, c, d, e, f, g, i⟩⟩

theorem owedVote2_iff {h : History} {c : Cert1} :
    OwedVote2 cfg h c ↔
      ((∃ b, h.HasCert1 cfg c ∧ h.HasProposal cfg b ∧ Certifies c b
          ∧ h.HasPayload cfg b.viewNumber b.payloadCommit)
        ∧ (∀ vote : Vote2, h.Sent (.vote2 vote) → vote.data.epoch = c.data.epoch → vote.view ≠ c.view)
        ∧ (∀ c2, h.HasCert2 c2 → c2.data.epoch = c.data.epoch → c2.view ≠ c.view)
        ∧ ¬ h.PastView c.view ∧ h.AfterFloor cfg c.view) :=
  ⟨fun ⟨a, b, c, d, e⟩ => ⟨a, b, c, d, e⟩, fun ⟨a, b, c, d, e⟩ => ⟨a, b, c, d, e⟩⟩

theorem owedDecide_iff {h : History} {c : Cert2} :
    OwedDecide cfg h c ↔
      ∃ b c1, h.HasCert2 c ∧ h.HasProposal cfg b ∧ Commits c b ∧ h.HasCert1 cfg c1
        ∧ Certifies c1 b ∧ ¬ h.DecidedView b.viewNumber ∧ h.AfterFloor cfg b.viewNumber :=
  ⟨fun ⟨a⟩ => a, fun a => ⟨a⟩⟩

theorem owedB_iff {h : History} {o : Obligation} :
    OwedB cfg leader node h o ↔ Owed cfg leader node h o := by
  cases o with
  | vote1 p =>
    simp only [OwedB, Owed, owedVote1_iff, ParentReady, safeParentB_iff, opensB_iff, notBehindB_iff, mem_proposalsHeld,
      mem_vote1sSent, timedOutB_iff, ← inView_iff_viewOf]
    apply and_congr _ Iff.rfl
    constructor
    · rintro ⟨⟨s, q, vid⟩, hx, rfl, hl, hs⟩
      exact ⟨s, vid, mem_receivedProposals.mp hx, hl, hs⟩
    · rintro ⟨s, vid, hr, hl, hs⟩
      exact ⟨(s, p, vid), mem_receivedProposals.mpr hr, rfl, hl, hs⟩
  | vote1Again r =>
    simp only [OwedB, Owed, owedVote1Again_iff, safeRevoteB_iff, notBehindB_iff, mem_proposalsHeld,
      mem_vote1sSent, timedOutB_iff, ← inView_iff_viewOf]
    apply and_congr _ Iff.rfl
    constructor
    · rintro ⟨⟨s, q⟩, hx, rfl, hl⟩
      exact ⟨s, mem_receivedRevotes.mp hx, hl⟩
    · rintro ⟨s, hr, hl⟩
      exact ⟨(s, r), mem_receivedRevotes.mpr hr, rfl, hl⟩
  | vote2 c =>
    simp only [OwedB, Owed, owedVote2_iff, mem_cert1sHeld, mem_proposalsHeld,
      mem_vote2sSent, mem_cert2sHeld, pastViewB_iff, aboveFloorB_iff]
    apply and_congr _ Iff.rfl
    constructor
    · rintro ⟨hc, b, hb, h1, h2⟩; exact ⟨b, hc, hb, h1, h2⟩
    · rintro ⟨b, hc, hb, h1, h2⟩; exact ⟨hc, b, hb, h1, h2⟩
  | decide c =>
    simp only [OwedB, Owed, owedDecide_iff, mem_cert2sHeld, mem_decidedViews, aboveFloorB_iff]
    constructor
    · rintro ⟨hc, b, hb, h1, ⟨c1, hc1, h2⟩, h3, h4⟩
      exact ⟨b, c1, hc, mem_proposalsHeld.mp hb, h1, mem_cert1sHeld.mp hc1, h2, h3, h4⟩
    · rintro ⟨b, c1, hc, hb, h1, hc1, h2, h3, h4⟩
      exact ⟨hc, b, mem_proposalsHeld.mpr hb, h1, ⟨c1, mem_cert1sHeld.mpr hc1, h2⟩, h3, h4⟩
  | propose e v =>
    have hj : ((∃ p ∈ proposalCandidates cfg h v, p.epoch = e ∧ ProposalJustifiedB cfg leader node h p)
        ∨ ∃ r ∈ revoteCandidates cfg h v, r.cert.data.epoch = e ∧ RevoteJustifiedB cfg leader node h r)
        ↔ ((∃ p, ProposalJustified cfg leader node h p ∧ p.viewNumber = v ∧ p.epoch = e)
          ∨ ∃ r, RevoteJustified cfg leader node h r ∧ r.view = v ∧ r.cert.data.epoch = e) := by
      constructor
      · rintro (⟨p, hp, he, hj⟩ | ⟨r, hr, he, hj⟩)
        · exact Or.inl ⟨p, proposalJustifiedB_iff.mp hj, viewNumber_of_mem_candidates hp, he⟩
        · exact Or.inr ⟨r, revoteJustifiedB_iff.mp hj, view_of_mem_revoteCandidates hr, he⟩
      · rintro (⟨p, hj, rfl, rfl⟩ | ⟨r, hj, rfl, rfl⟩)
        · obtain ⟨q, hq, hqe, hjq⟩ := proposalCandidates_complete hj
          exact Or.inl ⟨q, hq, hqe, proposalJustifiedB_iff.mpr hjq⟩
        · exact Or.inr ⟨r, revoteCandidates_complete hj, rfl, revoteJustifiedB_iff.mpr hj⟩
    have hn : ((∀ p ∈ proposalsSent h, p.viewNumber = v → p.epoch ≠ e)
        ∧ (∀ r ∈ revotesSent h, r.view = v → r.cert.data.epoch ≠ e)) ↔ ¬ ProposedIn h e v := by
      simp only [ProposedIn, mem_proposalsSent, mem_revotesSent, not_or, not_exists, not_and]
    show (_ ∧ _ ∧ _ ∧ ¬ TimedOutB h v ∧ viewOf cfg h = v) ↔ (_ ∧ ¬ ProposedIn h e v ∧ ¬ h.TimedOut v ∧ h.InView cfg v)
    rw [hj, ← hn, timedOutB_iff, inView_iff_viewOf, and_assoc]

instance (h : History) (o : Obligation) : Decidable (Owed cfg leader node h o) :=
  decidable_of_iff _ owedB_iff

/-! ## Growing the last step's outputs -/

section Grow

variable {pre : History} {i : Input} {out out' : List Output}

theorem sameInputs_grow : SameInputs (pre ++ [Step.mk i out]) (pre ++ [Step.mk i out']) := by
  simp [SameInputs]

theorem sent_grow (hsub : ∀ o ∈ out, o ∈ out') {m : Message} :
    (pre ++ [Step.mk i out]).Sent m → (pre ++ [Step.mk i out']).Sent m := by
  rintro ⟨st, hst, hm⟩
  rcases List.mem_append.mp hst with hst | hst
  · exact ⟨st, List.mem_append_left _ hst, hm⟩
  · rw [List.mem_singleton.mp hst] at hm
    exact ⟨_, List.mem_append_right _ List.mem_cons_self, hsub _ hm⟩

theorem decided_grow (hsub : ∀ o ∈ out, o ∈ out') {w : ViewNumber} :
    (pre ++ [Step.mk i out]).DecidedView w → (pre ++ [Step.mk i out']).DecidedView w := by
  rintro ⟨st, hst, blocks, c1, c2, b, hm, hb, hv⟩
  rcases List.mem_append.mp hst with hst | hst
  · exact ⟨st, List.mem_append_left _ hst, blocks, c1, c2, b, hm, hb, hv⟩
  · rw [List.mem_singleton.mp hst] at hm
    exact ⟨_, List.mem_append_right _ List.mem_cons_self, blocks, c1, c2, b, hsub _ hm, hb, hv⟩

theorem owed_grow (hsub : ∀ o ∈ out, o ∈ out') {o : Obligation}
    (ho : Owed cfg leader node (pre ++ [Step.mk i out']) o) : Owed cfg leader node (pre ++ [Step.mk i out]) o :=
  owed_antitone sameInputs_grow (fun _ => sent_grow hsub) (fun _ => decided_grow hsub) ho

end Grow

end NewProtocolImpl
