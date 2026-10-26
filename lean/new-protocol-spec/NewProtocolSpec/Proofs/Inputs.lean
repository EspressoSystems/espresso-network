module

public import NewProtocolSpec.Rules

/-!
# What reads only the inputs, and what the outputs only rule out

Two histories with the same inputs hold the same certificates and blocks, can lock
on the same certificates, and are in the same view and epoch (`SameInputs`). What
`Owed` reads of the outputs only ever rules an obligation out (`owed_antitone`).
A node's history with what it signed for other epochs left out
(`History.restrict`) has the same inputs, and for a node honest in every epoch it
is the whole history (`owedIn_all`).
-/

@[expose] public section

namespace NewProtocol

open History

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {node : PubKey}

/-! ## What reads only the inputs -/

/-- Two histories received the same inputs, in the same order. -/
def SameInputs (h1 h2 : History) : Prop := h1.map (·.input) = h2.map (·.input)

section Transfer

variable {h1 h2 : History} (he : SameInputs h1 h2)
include he

theorem SameInputs.received (i : Input) : h1.Received i ↔ h2.Received i := by
  have hm : ∀ h : History, h.Received i ↔ i ∈ h.map (·.input) := fun _ => List.mem_map.symm
  rw [hm, hm, he]

theorem SameInputs.upTo (m : Nat) : SameInputs (h1.upTo m) (h2.upTo m) := by
  unfold SameInputs History.upTo
  rw [List.map_take, List.map_take, he]

theorem SameInputs.hasCert1 (c : Cert1) : h1.HasCert1 cfg c ↔ h2.HasCert1 cfg c := by
  simp only [HasCert1, he.received]

theorem SameInputs.hasCert2 (c : Cert2) : h1.HasCert2 c ↔ h2.HasCert2 c := by
  simp only [HasCert2, he.received]

theorem SameInputs.hasProposal (b : Block) : h1.HasProposal cfg b ↔ h2.HasProposal cfg b := by
  simp only [HasProposal, he.received]

theorem SameInputs.hasPayload (v : ViewNumber) (pc : PayloadCommit) :
    h1.HasPayload cfg v pc ↔ h2.HasPayload cfg v pc := by
  simp only [HasPayload, he.received]

theorem SameInputs.lockable (c : Cert1) : h1.Lockable cfg c ↔ h2.Lockable cfg c := by
  simp only [Lockable, TookEpochChange, he.received, he.hasCert1, he.hasProposal, he.hasPayload]

theorem SameInputs.buildable (c : Cert1) : h1.Buildable cfg c ↔ h2.Buildable cfg c := by
  simp only [Buildable, he.hasCert1, he.hasProposal]

theorem SameInputs.lockedOn (c : Cert1) : h1.LockedOn cfg c ↔ h2.LockedOn cfg c := by
  simp only [LockedOn, he.lockable]

theorem SameInputs.epochGround (e : EpochNumber) : h1.EpochGround cfg e ↔ h2.EpochGround cfg e := by
  simp only [EpochGround, TookEpochChange, he.hasCert1, he.received]

theorem SameInputs.inEpoch (e : EpochNumber) : h1.InEpoch cfg e ↔ h2.InEpoch cfg e := by
  simp only [InEpoch, he.epochGround]

theorem SameInputs.notBehind (e : EpochNumber) : NotBehind cfg h1 e ↔ NotBehind cfg h2 e := by
  simp only [NotBehind, he.inEpoch]

theorem SameInputs.viewGround (v : ViewNumber) : h1.ViewGround cfg v ↔ h2.ViewGround cfg v := by
  simp only [ViewGround, TookEpochChange, he.hasCert1, he.received]

theorem SameInputs.inView (v : ViewNumber) : h1.InView cfg v ↔ h2.InView cfg v := by
  simp only [InView, he.viewGround]

theorem SameInputs.certJustified (c : Cert1) (ev : Option TimeoutCert) :
    CertJustified cfg h1 c ev ↔ CertJustified cfg h2 c ev := by
  cases ev with
  | none => exact he.buildable c
  | some tc =>
    simp only [CertJustified, he.hasCert1, (he.upTo _).received, (he.upTo _).lockedOn,
      (he.upTo _).hasCert2]

theorem SameInputs.parentReady (p : Proposal) : ParentReady cfg h1 p ↔ ParentReady cfg h2 p := by
  simp only [ParentReady, he.hasProposal, he.hasPayload]

theorem SameInputs.opens (p : Proposal) :
    OpensEpochJustified cfg h1 p ↔ OpensEpochJustified cfg h2 p := by
  simp only [OpensEpochJustified, he.hasProposal, he.hasCert2]

theorem SameInputs.proposalReady (p : Proposal) :
    ProposalReady cfg leader node h1 p ↔ ProposalReady cfg leader node h2 p := by
  have hj : ParentJustified cfg h1 p ↔ ParentJustified cfg h2 p := he.certJustified _ _
  constructor
  · rintro ⟨hl, hw, hjust, ⟨parent, hp, hv, hh⟩, hopen, hsafe, hcur, ⟨u, hu, hvu⟩⟩
    exact ⟨hl, hw, hj.mp hjust, ⟨parent, (he.hasProposal _).mp hp, hv, hh⟩, (he.opens p).mp hopen, hsafe,
      (he.notBehind _).mp hcur, ⟨u, (he.viewGround _).mp hu, hvu⟩⟩
  · rintro ⟨hl, hw, hjust, ⟨parent, hp, hv, hh⟩, hopen, hsafe, hcur, ⟨u, hu, hvu⟩⟩
    exact ⟨hl, hw, hj.mpr hjust, ⟨parent, (he.hasProposal _).mpr hp, hv, hh⟩, (he.opens p).mpr hopen, hsafe,
      (he.notBehind _).mpr hcur, ⟨u, (he.viewGround _).mpr hu, hvu⟩⟩

theorem SameInputs.proposalJustified (p : Proposal) :
    ProposalJustified cfg leader node h1 p ↔ ProposalJustified cfg leader node h2 p :=
  ⟨fun ⟨hr, hb⟩ => ⟨(he.proposalReady p).mp hr, (he.received _).mp hb⟩,
    fun ⟨hr, hb⟩ => ⟨(he.proposalReady p).mpr hr, (he.received _).mpr hb⟩⟩

theorem SameInputs.revoteJustified (r : RevoteRequest) :
    RevoteJustified cfg leader node h1 r ↔ RevoteJustified cfg leader node h2 r := by
  constructor
  · rintro ⟨hl, hw, hj, hk, hs, hc, ⟨u, hu, hvu⟩⟩
    exact ⟨hl, hw, (he.certJustified _ _).mp hj, fun h => (he.lockable _).mp (hk h), hs,
      (he.notBehind _).mp hc, ⟨u, (he.viewGround _).mp hu, hvu⟩⟩
  · rintro ⟨hl, hw, hj, hk, hs, hc, ⟨u, hu, hvu⟩⟩
    exact ⟨hl, hw, (he.certJustified _ _).mpr hj, fun h => (he.lockable _).mpr (hk h), hs,
      (he.notBehind _).mpr hc, ⟨u, (he.viewGround _).mpr hu, hvu⟩⟩

end Transfer

/-! ## More output only rules obligations out -/

/--
What `Owed` reads of the outputs only ever rules an obligation out, and what it
reads of the inputs does not change: so a history with the same inputs and more
sent and decided owes no more.
-/
theorem owed_antitone {h1 h2 : History} (he : SameInputs h1 h2)
    (hs : ∀ m, h1.Sent m → h2.Sent m) {o : Obligation}
    (hd : ∀ w, h1.DecidedView w → h2.DecidedView w)
    (ho : Owed cfg leader node h2 o) : Owed cfg leader node h1 o := by
  have hfl : ∀ v, h2.AfterFloor cfg v → h1.AfterFloor cfg v :=
    fun _ ⟨hg, hw⟩ => ⟨hg, fun w hdw => hw w (hd w hdw)⟩
  have hto : ∀ v, ¬ h2.TimedOut v → ¬ h1.TimedOut v :=
    fun _ hn ⟨vote, hsv, hle⟩ => hn ⟨vote, hs _ hsv, hle⟩
  have hpi : ∀ e v, ¬ ProposedIn h2 e v → ¬ ProposedIn h1 e v := by
    rintro _ _ hn (⟨p, hsp, hv⟩ | ⟨r, hsr, hv⟩)
    · exact hn (Or.inl ⟨p, hs _ hsp, hv⟩)
    · exact hn (Or.inr ⟨r, hs _ hsr, hv⟩)
  cases o with
  | vote1 p =>
    obtain ⟨⟨s, vid, hr, hl, hsh⟩, hw, hval, hpr, hsk, hop, hnb, ht, honce, hv⟩ := ho
    refine ⟨⟨s, vid, (he.received _).mpr hr, hl, hsh⟩, hw, (he.received _).mpr hval, ?_,
      hsk, (he.opens _).mpr hop, (he.notBehind _).mpr hnb, hto _ ht,
      fun vote hsv => honce vote (hs _ hsv), (he.inView _).mpr hv⟩
    rcases hpr with hg | hen | ⟨parent, hb, hv, hh, hpay⟩
    · exact Or.inl hg
    · exact Or.inr (Or.inl hen)
    · exact Or.inr (Or.inr ⟨parent, (he.hasProposal _).mpr hb, hv, hh, (he.hasPayload _ _).mpr hpay⟩)
  | vote1Again r =>
    obtain ⟨⟨s, hr, hl⟩, hw, hsk, ⟨b, hb, hcert, hpay⟩, hnb, ht, honce, hv⟩ := ho
    exact ⟨⟨s, (he.received _).mpr hr, hl⟩, hw, hsk,
      ⟨b, (he.hasProposal _).mpr hb, hcert, (he.hasPayload _ _).mpr hpay⟩, (he.notBehind _).mpr hnb,
      hto _ ht, fun vote hsv => honce vote (hs _ hsv), (he.inView _).mpr hv⟩
  | vote2 c =>
    obtain ⟨⟨b, hc, hb, hcert, hpay⟩, honce, hc2, hpast, hfloor⟩ := ho
    refine ⟨⟨b, (he.hasCert1 _).mpr hc, (he.hasProposal _).mpr hb, hcert, (he.hasPayload _ _).mpr hpay⟩,
      fun vote hsv => honce vote (hs _ hsv), fun c2 h => hc2 c2 ((he.hasCert2 _).mp h),
      fun hp => hpast ?_, hfl _ hfloor⟩
    rcases hp with ⟨vote, hsv, hle⟩ | ⟨tc, htc, hle⟩
    · exact Or.inl ⟨vote, hs _ hsv, hle⟩
    · exact Or.inr ⟨tc, (he.received _).mp htc, hle⟩
  | decide c =>
    obtain ⟨b, c1, hc, hb, hcm, hc1, hcert, hdec, hfloor⟩ := ho
    exact ⟨b, c1, (he.hasCert2 _).mpr hc, (he.hasProposal _).mpr hb, hcm, (he.hasCert1 _).mpr hc1,
      hcert, fun h => hdec (hd _ h), hfl _ hfloor⟩
  | propose e v =>
    obtain ⟨hj, honce, ht, hvw⟩ := ho
    refine ⟨?_, hpi _ _ honce, hto _ ht, (he.inView _).mpr hvw⟩
    rcases hj with ⟨p, hj, hv⟩ | ⟨r, hj, hv⟩
    · exact Or.inl ⟨p, (he.proposalJustified p).mpr hj, hv⟩
    · exact Or.inr ⟨r, (he.revoteJustified r).mpr hj, hv⟩

/-! ## The history for the epochs a node is honest in -/

section Restrict

open Classical

variable {P : EpochNumber → Prop} {h : History}

theorem sameInputs_restrict : SameInputs (h.restrict P) h := by
  simp [SameInputs, History.restrict, List.map_map, Function.comp_def]

theorem restrict_sent {m : Message} : (h.restrict P).Sent m ↔ h.Sent m ∧ (Output.send m).SignedIn P := by
  constructor
  · rintro ⟨st', hst', hm⟩
    obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hst'
    obtain ⟨hm, hp⟩ := List.mem_filter.mp hm
    exact ⟨⟨st, hst, hm⟩, of_decide_eq_true hp⟩
  · rintro ⟨⟨st, hst, hm⟩, hp⟩
    exact ⟨_, List.mem_map.mpr ⟨st, hst, rfl⟩, List.mem_filter.mpr ⟨hm, decide_eq_true hp⟩⟩

theorem restrict_decidedView {v : ViewNumber} :
    (h.restrict P).DecidedView v ↔ ∃ st ∈ h, ∃ blocks c1 c2 b, Output.decided blocks c1 c2 ∈ st.output
      ∧ P c2.data.epoch ∧ b ∈ blocks ∧ b.viewNumber = v := by
  constructor
  · rintro ⟨st', hst', blocks, c1, c2, b, hm, hb, hv⟩
    obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hst'
    obtain ⟨hm, hp⟩ := List.mem_filter.mp hm
    exact ⟨st, hst, blocks, c1, c2, b, hm, of_decide_eq_true hp, hb, hv⟩
  · rintro ⟨st, hst, blocks, c1, c2, b, hm, hp, hb, hv⟩
    exact ⟨_, List.mem_map.mpr ⟨st, hst, rfl⟩, blocks, c1, c2, b,
      List.mem_filter.mpr ⟨hm, decide_eq_true hp⟩, hb, hv⟩

theorem restrict_upTo (m : Nat) : (h.restrict P).upTo m = (h.upTo m).restrict P := by
  simp only [History.upTo, History.restrict, List.map_take]

/-- A history whose every output is for the counted epochs is its own restriction. -/
theorem restrict_eq_self (hs : ∀ st ∈ h, ∀ o ∈ st.output, o.SignedIn P) : h.restrict P = h := by
  unfold History.restrict
  conv => rhs; rw [← List.map_id h]
  refine List.map_congr_left fun st hst => ?_
  rw [List.filter_eq_self.mpr fun o ho => decide_eq_true (hs st hst o ho)]
  rfl

/-- With every epoch counted, nothing is left out. -/
theorem restrict_all (hP : ∀ e, P e) : h.restrict P = h :=
  restrict_eq_self fun _ _ o _ => by
    cases o with
    | send m => cases m <;> simp only [Output.SignedIn] <;> first | trivial | exact hP _
    | decided => exact hP _

/-- A node honest in every epoch owes what `Owed` says. -/
theorem owedIn_all (hP : ∀ e, P e) {o : Obligation} :
    OwedIn cfg leader node P h o ↔ Owed cfg leader node h o := by
  unfold OwedIn
  rw [restrict_all hP]
  exact ⟨fun h => h.2, fun ho => ⟨hP _, ho⟩⟩

end Restrict

end NewProtocol
