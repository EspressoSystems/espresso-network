module

public import NewProtocolImpl.Conformance

/-!
# Taking the input in

What `Impl.handle` does to each field of the state, and what it emits.

The obligations of `StepSpec` that constrain *content* — where it comes from,
that an input's is taken, and what may not be lost — are settled here, because
the reaction pass that follows adds no content. The obligations that constrain
*actions* need only one thing from this file: that `handle` emits no vote, no
proposal and no decide.

Each proof is a case analysis over the fourteen inputs. The arms that write
nothing are closed by the frame lemmas; the arms that write are guarded, and
the guard is exactly the hypothesis the matching obligation carries.
-/

@[expose] public section

set_option linter.unusedSectionVars false
set_option linter.unusedSimpArgs false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {s : State} {i : Input}

/--
The state after taking `i` in.

No leader schedule: recording an input does not depend on who leads, now that
no arm of `Impl.handle` asks the node's own subsystems for anything. Only the
reaction pass reads `leader`, in order to propose.
-/
abbrev ingest (cfg : Config) (node : PubKey) (s : State) (i : Input) : State :=
  (handle cfg node i s).1

/-- What taking `i` in emits. -/
abbrev ingestOut (cfg : Config) (node : PubKey) (s : State) (i : Input) : List Output :=
  (handle cfg node i s).2

/-! ## Frames

The marks, the lock and the bar are the reaction pass's business; taking an
input in leaves them alone.
-/

theorem ingest_lockedCert : (ingest cfg node s i).lockedCert = s.lockedCert := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

theorem ingest_barredView : (ingest cfg node s i).barredView = s.barredView := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

/--
Only the epoch change moves the epoch, and it moves it past the epoch its
certificates finished.
-/
theorem ingest_currentEpoch :
    (ingest cfg node s i).currentEpoch = s.currentEpoch
      ∨ ∃ c1 c2 p, i = Input.epochChange c1 c2 p
          ∧ s.acceptsEpochChange cfg c1 c2 p = true
          ∧ (ingest cfg node s i).currentEpoch = c2.data.epoch + 1 := by
  cases i with
  | epochChange c1 c2 p =>
    simp only [ingest, handle, apply_ite, ite_self]
    by_cases hg : s.acceptsEpochChange cfg c1 c2 p = true
    · refine Or.inr ⟨c1, c2, p, rfl, hg, ?_⟩
      simp only [ingest, handle, apply_ite, ite_self, if_pos hg]
      repeat' split
      all_goals rfl
    · simp only [if_neg hg]
      exact Or.inl trivial
  | _ => exact Or.inl (by simp only [ingest, handle]; repeat' (first | split | rfl))

theorem ingest_decidedViews : (ingest cfg node s i).decidedViews = s.decidedViews := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

theorem ingest_voted1Views : (ingest cfg node s i).voted1Views = s.voted1Views := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

theorem ingest_voted2Views : (ingest cfg node s i).voted2Views = s.voted2Views := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

theorem ingest_proposedViews : (ingest cfg node s i).proposedViews = s.proposedViews := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

theorem ingest_vote1Branches : (ingest cfg node s i).vote1Branches = s.vote1Branches := by
  cases i <;> simp only [ingest, handle] <;> repeat' (first | split | rfl)

/-- The floor is fixed by the decided views, so taking an input in leaves it where it was. -/
theorem ingest_floor : (ingest cfg node s i).floor cfg = s.floor cfg := by
  unfold State.floor State.lastDecided
  rw [ingest_decidedViews]

theorem ingest_aboveFloor (v : ViewNumber) :
    (ingest cfg node s i).aboveFloor cfg v = s.aboveFloor cfg v := by
  unfold State.aboveFloor
  rw [ingest_floor]

/-! ## The rules, decided

The machine's `Bool` forms of the specification's admission rules are those
rules. Everything below reads a guard through these, so the state the machine
writes is the state the specification licenses.
-/

theorem writable_iff {α : Type} [DecidableEq α] {o : Option α} {x : α} :
    writable o x = true ↔ Writable o x := by
  simp [writable, Writable]

theorem wellFormed_iff {p : Proposal} : wellFormed cfg p = true ↔ ProposalWellFormed cfg p := by
  cases hte : p.timeoutEvidence <;> simp [wellFormed, ProposalWellFormed, hte, and_assoc]

theorem shareMatches_iff {p : Proposal} {vid : VidShare} :
    shareMatches p vid = true ↔ ShareMatches p vid := by
  simp [shareMatches, ShareMatches, Proposal.payloadCommit]

theorem safeToExtend_iff {l : Option Cert1} {p : Proposal} :
    safeToExtend l p = true ↔ SafeToExtend l p := by
  cases l with
  | none => simp [safeToExtend, SafeToExtend]
  | some lock =>
    simp only [safeToExtend, SafeToExtend]
    split <;> simp

theorem epochChangeWellFormed_iff {c1 : Cert1} {c2 : Cert2} {p : Proposal} :
    epochChangeWellFormed cfg c1 c2 p = true ↔ EpochChangeWellFormed cfg c1 c2 p := by
  simp only [epochChangeWellFormed, EpochChangeWellFormed, Bool.and_eq_true, beq_iff_eq,
    decide_eq_true_eq, wellFormed_iff, and_assoc]

/-- What taking an epoch change amounts to: the specification's rule, clause by clause. -/
theorem acceptsEpochChange_iff {s : State} {c1 : Cert1} {c2 : Cert2} {p : Proposal} :
    s.acceptsEpochChange cfg c1 c2 p = true ↔
      EpochChangeWellFormed cfg c1 c2 p
        ∧ s.currentEpoch ≤ c2.data.epoch
        ∧ ∀ lock, s.lockedCert = some lock → lock.view ≤ c1.view := by
  cases hl : s.lockedCert with
  | none =>
    simp only [State.acceptsEpochChange, Bool.and_eq_true, decide_eq_true_eq,
      epochChangeWellFormed_iff, hl, and_assoc]
    simp
  | some lock =>
    simp only [State.acceptsEpochChange, Bool.and_eq_true, decide_eq_true_eq,
      epochChangeWellFormed_iff, hl, and_assoc]
    simp

/--
The epoch only rises: the one arm that writes it declines an epoch change for
an epoch already entered, so what it writes is past what was held.
-/
theorem ingest_currentEpochMono : s.currentEpoch ≤ (ingest cfg node s i).currentEpoch := by
  rcases ingest_currentEpoch (cfg := cfg) (node := node) (s := s) (i := i) with
    he | ⟨c1, c2, q, hi, hg, he⟩
  · exact he ▸ Nat.le_refl _
  · exact he ▸ Nat.le_trans (acceptsEpochChange_iff.mp hg).2.1 (Nat.le_succ _)

/-- The three slot conditions an epoch change writes under. -/
theorem epochChange_writable {s : State} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
        && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) p) = true) :
    Writable (s.cert1s.get? c1.view) c1 ∧ Writable (s.cert2s.get? c2.view) c2
      ∧ Writable (s.proposals.get? c2.view) p := by
  simp only [Bool.and_eq_true, writable_iff] at hw
  exact ⟨hw.1.1.2, hw.1.2, hw.2⟩

/-- What admitting an arriving proposal amounts to: the specification's rule, clause by clause. -/
theorem admits_iff {p : Proposal} {vid : VidShare} :
    s.admits cfg p vid = true ↔
      s.barredView < p.viewNumber
        ∧ Writable (s.admitted.get? p.viewNumber) p
        ∧ Writable (s.proposals.get? p.viewNumber) p
        ∧ Writable (s.vidShares.get? p.viewNumber) vid
        ∧ ProposalWellFormed cfg p ∧ ShareMatches p vid ∧ SafeToExtend s.lockedCert p := by
  simp only [State.admits, Bool.and_eq_true, decide_eq_true_eq, writable_iff, wellFormed_iff,
    shareMatches_iff, safeToExtend_iff, and_assoc]

/-! ## Provenance

Nothing enters the state except through the input just taken. Every arm either
leaves a field alone — the frames above, and the projections that reduce — or
writes what the input carried.
-/

theorem ingest_proposalProvenance {v : ViewNumber} {p : Proposal}
    (h : (ingest cfg node s i).proposals.get? v = some p) :
    s.proposals.get? v = some p
      ∨ ((∃ sender vid, i = Input.proposal sender p vid)
          ∧ p.viewNumber = v ∧ ProposalWellFormed cfg p)
      ∨ (∃ c1 c2, i = Input.epochChange c1 c2 p
          ∧ p.viewNumber = v ∧ EpochChangeWellFormed cfg c1 c2 p) := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw] at h
        rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
        · have hwf := (acceptsEpochChange_iff.mp hg).1
          exact Or.inr (Or.inr ⟨c1, c2, rfl, hwf.2.2.2.2.1.trans hwf.1, hwf⟩)
        · exact Or.inl h
      · rw [ingest, handle, if_pos hg, if_neg hw] at h; exact Or.inl h
    · rw [ingest, handle, if_neg hg] at h; exact Or.inl h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self] at h
    repeat' split at h
    all_goals
      first
      | exact Or.inl h
      | exact Or.inl (get?_of_get?_erase h)
      | (rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
         · rename_i hg
           exact Or.inr (Or.inl ⟨⟨_, _, rfl⟩, rfl, (admits_iff.mp hg).2.2.2.2.1⟩)
         · exact Or.inl h)

theorem ingest_admissionJustified {v : ViewNumber} {p : Proposal}
    (h : (ingest cfg node s i).admitted.get? v = some p) :
    s.admitted.get? v = some p
      ∨ ∃ sender vid, i = Input.proposal sender p vid
          ∧ p.viewNumber = v
          ∧ s.barredView < v
          ∧ SafeToExtend s.lockedCert p
          ∧ ProposalWellFormed cfg p
          ∧ ShareMatches p vid
          ∧ (ingest cfg node s i).vidShares.get? v = some vid
          ∧ (ingest cfg node s i).proposals.get? v = some p := by
  cases i with
  | proposal sender q vid =>
    simp only [ingest, handle, apply_ite, ite_self] at h ⊢
    by_cases hg : s.admits cfg q vid = true
    · simp only [if_pos hg] at h ⊢
      rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
      · obtain ⟨hb, -, -, -, hwf, hsm, hste⟩ := admits_iff.mp hg
        exact Or.inr ⟨sender, vid, rfl, rfl, hb, hste, hwf, hsm,
          get?_insert_self, get?_insert_self⟩
      · exact Or.inl h
    · simp only [if_neg hg] at h
      exact Or.inl h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self] at h
    repeat' split at h
    all_goals first | exact Or.inl h | exact Or.inl (get?_of_get?_erase h)

theorem ingest_vidShareProvenance {v : ViewNumber} {sh : VidShare}
    (h : (ingest cfg node s i).vidShares.get? v = some sh) :
    s.vidShares.get? v = some sh
      ∨ ∃ sender p, i = Input.proposal sender p sh ∧ p.viewNumber = v := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] at h <;> repeat' split at h
  all_goals
    first
    | exact Or.inl h
    | exact Or.inl (get?_of_get?_erase h)
    | (rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
       · exact Or.inr ⟨_, _, rfl, rfl⟩
       · exact Or.inl h)

theorem ingest_validatedProvenance {v : ViewNumber} {hb : BlockHash}
    (h : (ingest cfg node s i).validated.get? v = some hb) :
    s.validated.get? v = some hb ∨ i = Input.blockValidated v hb := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] at h <;> repeat' split at h
  all_goals
    first
    | exact Or.inl h
    | (rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
       · exact Or.inr rfl
       · exact Or.inl h)

theorem ingest_reconstructedProvenance {v : ViewNumber} {pc : PayloadCommit}
    (h : (v, pc) ∈ (ingest cfg node s i).blocksReconstructed) :
    (v, pc) ∈ s.blocksReconstructed ∨ i = Input.blockReconstructed v pc := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] at h <;> repeat' split at h
  all_goals
    first
    | exact Or.inl h
    | (rcases mem_insert.mp h with he | h
       · obtain ⟨rfl, rfl⟩ : v = _ ∧ pc = _ := by simpa using he
         exact Or.inr rfl
       · exact Or.inl h)

theorem ingest_headerProvenance {v : ViewNumber} {hb : BlockHash} {hd : BlockHeader}
    (h : (ingest cfg node s i).headers.get? (v, hb) = some hd) :
    s.headers.get? (v, hb) = some hd ∨ i = Input.headerBuilt v hb hd := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] at h <;> repeat' split at h
  all_goals
    first
    | exact Or.inl h
    | (rcases get?_insert_cases h with ⟨he, rfl⟩ | h
       · obtain ⟨rfl, rfl⟩ : v = _ ∧ hb = _ := by simpa using he
         exact Or.inr rfl
       · exact Or.inl h)

theorem ingest_cert1Provenance {v : ViewNumber} {c : Cert1}
    (h : (ingest cfg node s i).cert1s.get? v = some c) :
    s.cert1s.get? v = some c
      ∨ ((i = Input.certificate1 c ∨ i = Input.advanceView c) ∧ c.view = v)
      ∨ (∃ c2 p, i = Input.epochChange c c2 p ∧ c.view = v) := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw] at h
        rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
        · exact Or.inr (Or.inr ⟨c2, q, rfl, rfl⟩)
        · exact Or.inl h
      · rw [ingest, handle, if_pos hg, if_neg hw] at h; exact Or.inl h
    · rw [ingest, handle, if_neg hg] at h; exact Or.inl h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self] at h
    repeat' split at h
    all_goals
      first
      | exact Or.inl h
      | (rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
         · first
           | exact Or.inr (Or.inl ⟨Or.inl rfl, rfl⟩)
           | exact Or.inr (Or.inl ⟨Or.inr rfl, rfl⟩)
         · exact Or.inl h)

theorem ingest_cert2Provenance {v : ViewNumber} {c : Cert2}
    (h : (ingest cfg node s i).cert2s.get? v = some c) :
    s.cert2s.get? v = some c
      ∨ (i = Input.certificate2 c ∧ c.view = v)
      ∨ (∃ c1 p, i = Input.epochChange c1 c p ∧ c.view = v) := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw] at h
        rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
        · exact Or.inr (Or.inr ⟨c1, q, rfl, rfl⟩)
        · exact Or.inl h
      · rw [ingest, handle, if_pos hg, if_neg hw] at h; exact Or.inl h
    · rw [ingest, handle, if_neg hg] at h; exact Or.inl h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self] at h
    repeat' split at h
    all_goals
      first
      | exact Or.inl h
      | (rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
         · exact Or.inr (Or.inl ⟨rfl, rfl⟩)
         · exact Or.inl h)

theorem ingest_timeoutCertProvenance {v : ViewNumber} {tc : TimeoutCert}
    (h : (ingest cfg node s i).timeoutCerts.get? v = some tc) :
    s.timeoutCerts.get? v = some tc ∨ (i = Input.timeoutCertificate tc ∧ v = tc.view + 1) := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] at h <;> repeat' split at h
  all_goals
    first
    | exact Or.inl h
    | (rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
       · exact Or.inr ⟨rfl, rfl⟩
       · exact Or.inl h)

/-! ## Ingestion

The other direction: what an input carries is taken. Each guard of `handle` is
the conjunction of the conditions the matching obligation assumes, so under
those the arm writes — and where a guard also tests that the slot is free, the
obligation's `Writable` says the value already there is the one being written.
-/

theorem ingest_proposalIngested {sender : PubKey} {p : Proposal} {vid : VidShare}
    (hi : i = Input.proposal sender p vid) (hb : s.barredView < p.viewNumber)
    (h1 : Writable (s.admitted.get? p.viewNumber) p)
    (h2 : Writable (s.proposals.get? p.viewNumber) p)
    (h3 : Writable (s.vidShares.get? p.viewNumber) vid)
    (h4 : SafeToExtend s.lockedCert p) (h5 : ProposalWellFormed cfg p) (h6 : ShareMatches p vid) :
    (ingest cfg node s i).admitted.get? p.viewNumber = some p
      ∧ (ingest cfg node s i).proposals.get? p.viewNumber = some p
      ∧ (ingest cfg node s i).vidShares.get? p.viewNumber = some vid := by
  subst hi
  have hg : s.admits cfg p vid = true := admits_iff.mpr ⟨hb, h1, h2, h3, h5, h6, h4⟩
  simp only [ingest, handle, apply_ite, ite_self, if_pos hg]
  exact ⟨get?_insert_self, get?_insert_self, get?_insert_self⟩

theorem ingest_epochChangeIngested {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : i = Input.epochChange c1 c2 p) (hacc : s.acceptsEpochChange cfg c1 c2 p = true)
    (hfl : s.aboveFloor cfg c2.view = true)
    (hw1 : Writable (s.cert1s.get? c1.view) c1) (hw2 : Writable (s.cert2s.get? c2.view) c2)
    (hw3 : Writable (s.proposals.get? c2.view) p) :
    (ingest cfg node s i).cert1s.get? c1.view = some c1
      ∧ (ingest cfg node s i).cert2s.get? c2.view = some c2
      ∧ (ingest cfg node s i).proposals.get? c2.view = some p := by
  subst hi
  have hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
      && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) p) = true := by
    simp only [Bool.and_eq_true]
    exact ⟨⟨⟨hfl, writable_iff.mpr hw1⟩, writable_iff.mpr hw2⟩, writable_iff.mpr hw3⟩
  rw [ingest, handle, if_pos hacc, if_pos hw]
  exact ⟨get?_insert_self, get?_insert_self, get?_insert_self⟩

theorem ingest_epochChangeOwed {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : i = Input.epochChange c1 c2 p) (hacc : s.acceptsEpochChange cfg c1 c2 p = true) :
    c2.view + 1 ≤ (ingest cfg node s i).currentView
      ∧ (ingest cfg node s i).currentEpoch = c2.data.epoch + 1 := by
  subst hi
  have hv : (ingest cfg node s (Input.epochChange c1 c2 p)).currentView
      = max s.currentView (c2.view + 1) := by
    rw [ingest, handle, if_pos hacc]; repeat' split
    all_goals rfl
  have he : (ingest cfg node s (Input.epochChange c1 c2 p)).currentEpoch
      = c2.data.epoch + 1 := by
    rw [ingest, handle, if_pos hacc]; repeat' split
    all_goals rfl
  exact ⟨hv ▸ ViewNumber.le_max_right .., he⟩

theorem ingest_cert1Ingested {c : Cert1}
    (hi : i = Input.certificate1 c ∨ i = Input.advanceView c)
    (hfl : s.aboveFloor cfg c.view = true) (hw : Writable (s.cert1s.get? c.view) c) :
    (ingest cfg node s i).cert1s.get? c.view = some c := by
  have hfull : ¬ (s.aboveFloor cfg c.view = true ∧ ¬ s.cert1s.contains c.view = true) →
      s.cert1s.get? c.view = some c := by
    intro hg
    have hc : s.cert1s.contains c.view = true := by
      by_cases hn : s.cert1s.contains c.view = true
      · exact hn
      · exact absurd ⟨hfl, hn⟩ hg
    obtain ⟨x, hx⟩ := exists_get?_of_contains hc
    rcases hw with hnone | hsome
    · exact absurd (hnone.symm.trans hx) (by simp)
    · exact hsome
  rcases hi with rfl | rfl
  all_goals
    simp only [ingest, handle, apply_ite, ite_self]
    by_cases hg : s.aboveFloor cfg c.view = true ∧ ¬ s.cert1s.contains c.view = true
    · rw [if_pos hg]; exact get?_insert_self
    · rw [if_neg hg]; exact hfull hg

theorem ingest_cert2Ingested {c : Cert2} (hi : i = Input.certificate2 c)
    (hfl : s.aboveFloor cfg c.view = true) (hw : Writable (s.cert2s.get? c.view) c) :
    (ingest cfg node s i).cert2s.get? c.view = some c := by
  subst hi
  simp only [ingest, handle, apply_ite, ite_self]
  by_cases hg : s.aboveFloor cfg c.view = true ∧ ¬ s.cert2s.contains c.view = true
  · rw [if_pos hg]; exact get?_insert_self
  · rw [if_neg hg]
    have hc : s.cert2s.contains c.view = true := by
      by_cases hn : s.cert2s.contains c.view = true
      · exact hn
      · exact absurd ⟨hfl, hn⟩ hg
    obtain ⟨x, hx⟩ := exists_get?_of_contains hc
    rcases hw with hnone | hsome
    · exact absurd (hnone.symm.trans hx) (by simp)
    · exact hsome

theorem ingest_timeoutCertIngested {tc : TimeoutCert} (hi : i = Input.timeoutCertificate tc)
    (hcv : s.currentView ≤ tc.view + 1) (hw : Writable (s.timeoutCerts.get? (tc.view + 1)) tc) :
    (ingest cfg node s i).timeoutCerts.get? (tc.view + 1) = some tc := by
  subst hi
  simp only [ingest, handle, apply_ite, ite_self]
  by_cases hg : tc.view + 1 < s.currentView ∨ s.timeoutCerts.contains (tc.view + 1) = true
  · rw [if_pos hg]
    have hc : s.timeoutCerts.contains (tc.view + 1) = true :=
      hg.resolve_left (Nat.not_lt.mpr hcv)
    obtain ⟨x, hx⟩ := exists_get?_of_contains hc
    rcases hw with hnone | hsome
    · exact absurd (hnone.symm.trans hx) (by simp)
    · exact hsome
  · rw [if_neg hg]
    exact get?_insert_self

theorem ingest_blockValidatedIngested {v : ViewNumber} {hb : BlockHash}
    (hi : i = Input.blockValidated v hb) (hw : Writable (s.validated.get? v) hb) :
    (ingest cfg node s i).validated.get? v = some hb := by
  subst hi
  simp only [ingest, handle, apply_ite, ite_self]
  by_cases hg : s.validated.contains v = true
  · rw [if_pos hg]
    obtain ⟨x, hx⟩ := exists_get?_of_contains hg
    rcases hw with hnone | hsome
    · exact absurd (hnone.symm.trans hx) (by simp)
    · exact hsome
  · rw [if_neg hg]; exact get?_insert_self

theorem ingest_reconstructedIngested {v : ViewNumber} {pc : PayloadCommit}
    (hi : i = Input.blockReconstructed v pc) :
    (v, pc) ∈ (ingest cfg node s i).blocksReconstructed := by
  subst hi; exact mem_insert_self

theorem ingest_headerIngested {v : ViewNumber} {parent : BlockHash} {hd : BlockHeader}
    (hi : i = Input.headerBuilt v parent hd) (hw : Writable (s.headers.get? (v, parent)) hd) :
    (ingest cfg node s i).headers.get? (v, parent) = some hd := by
  subst hi
  simp only [ingest, handle, apply_ite, ite_self]
  by_cases hg : s.headers.contains (v, parent) = true
  · rw [if_pos hg]
    obtain ⟨x, hx⟩ := exists_get?_of_contains hg
    rcases hw with hnone | hsome
    · exact absurd (hnone.symm.trans hx) (by simp)
    · exact hsome
  · rw [if_neg hg]; exact get?_insert_self

/-! ## Retention

What the state holds it keeps: no arm of `Impl.handle` erases anything, so
every lemma below is unconditional. That is the whole content of
`StepSpec.contentRetained` for this machine.
-/

theorem ingest_proposals_retained {v : ViewNumber} {p : Proposal} (h : s.proposals.get? v = some p) :
    (ingest cfg node s i).proposals.get? v = some p := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw]
        exact get?_insert_of_writable (epochChange_writable hw).2.2 h
      · rw [ingest, handle, if_pos hg, if_neg hw]; exact h
    · rw [ingest, handle, if_neg hg]; exact h
  | proposal sender q vid =>
    simp only [ingest, handle, apply_ite, ite_self]
    by_cases hg : s.admits cfg q vid = true
    · rw [if_pos hg]; exact get?_insert_of_writable (admits_iff.mp hg).2.2.1 h
    · rw [if_neg hg]; exact h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self]
    repeat' split
    all_goals exact h
theorem ingest_admitted_retained {v : ViewNumber} {p : Proposal} (h : s.admitted.get? v = some p) :
    (ingest cfg node s i).admitted.get? v = some p := by
  cases i with
  | proposal sender q vid =>
    simp only [ingest, handle, apply_ite, ite_self]
    by_cases hg : s.admits cfg q vid = true
    · rw [if_pos hg]; exact get?_insert_of_writable (admits_iff.mp hg).2.1 h
    · rw [if_neg hg]; exact h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self]
    repeat' split
    all_goals exact h
theorem ingest_vidShares_retained {v : ViewNumber} {sh : VidShare} (h : s.vidShares.get? v = some sh) :
    (ingest cfg node s i).vidShares.get? v = some sh := by
  cases i with
  | proposal sender q vid =>
    simp only [ingest, handle, apply_ite, ite_self]
    by_cases hg : s.admits cfg q vid = true
    · rw [if_pos hg]; exact get?_insert_of_writable (admits_iff.mp hg).2.2.2.1 h
    · rw [if_neg hg]; exact h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self]
    repeat' split
    all_goals exact h
theorem ingest_cert1s_retained {v : ViewNumber} {c : Cert1} (h : s.cert1s.get? v = some c) :
    (ingest cfg node s i).cert1s.get? v = some c := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw]
        exact get?_insert_of_writable (epochChange_writable hw).1 h
      · rw [ingest, handle, if_pos hg, if_neg hw]; exact h
    · rw [ingest, handle, if_neg hg]; exact h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self]
    repeat' split
    all_goals
      first
      | exact h
      | (rename_i hg
         exact get?_insert_of_ne (fun he => hg.2 (he ▸ contains_of_get? h)) h)

theorem ingest_cert2s_retained {v : ViewNumber} {c : Cert2} (h : s.cert2s.get? v = some c) :
    (ingest cfg node s i).cert2s.get? v = some c := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw]
        exact get?_insert_of_writable (epochChange_writable hw).2.1 h
      · rw [ingest, handle, if_pos hg, if_neg hw]; exact h
    · rw [ingest, handle, if_neg hg]; exact h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self]
    repeat' split
    all_goals
      first
      | exact h
      | (rename_i hg
         exact get?_insert_of_ne (fun he => hg.2 (he ▸ contains_of_get? h)) h)

theorem ingest_timeoutCerts_retained {v : ViewNumber} {tc : TimeoutCert}
    (h : s.timeoutCerts.get? v = some tc) :
    (ingest cfg node s i).timeoutCerts.get? v = some tc := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] <;> repeat' split
  all_goals
    first
    | exact h
    | (rename_i hg
       exact get?_insert_of_ne (fun he => hg (Or.inr (he ▸ contains_of_get? h))) h)

theorem ingest_validated_retained {v : ViewNumber} {hb : BlockHash}
    (h : s.validated.get? v = some hb) :
    (ingest cfg node s i).validated.get? v = some hb := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] <;> repeat' split
  all_goals
    first
    | exact h
    | (rename_i hg
       exact get?_insert_of_ne (fun he => hg (he ▸ contains_of_get? h)) h)

theorem ingest_header_retained {k : ViewNumber × BlockHash} {hd : BlockHeader}
    (h : s.headers.get? k = some hd) :
    (ingest cfg node s i).headers.get? k = some hd := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] <;> repeat' split
  all_goals
    first
    | exact h
    | (rename_i hg
       exact get?_insert_of_ne (fun he => hg (he ▸ contains_of_get? h)) h)

theorem ingest_reconstructed_retained {v : ViewNumber} {pc : PayloadCommit}
    (h : (v, pc) ∈ s.blocksReconstructed) :
    (v, pc) ∈ (ingest cfg node s i).blocksReconstructed := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] <;> repeat' split
  all_goals first | exact h | exact mem_insert_of_mem h

/-! ## The invariant

Taking an input in keeps the representation invariant. The keying comes from the
provenance lemmas — an entry is old, and keyed, or it is the input's own value,
filed under the view it names. Only the pairing of `admitted` with `proposals`
needs its own argument, because the arm that erases a proposal erases its
admission with it.
-/

theorem ingest_admitted_proposals {v : ViewNumber} {p : Proposal} (hwf : WF cfg s)
    (h : (ingest cfg node s i).admitted.get? v = some p) :
    (ingest cfg node s i).proposals.get? v = some p := by
  cases i with
  | epochChange c1 c2 q =>
    by_cases hg : s.acceptsEpochChange cfg c1 c2 q = true
    · by_cases hw : (s.aboveFloor cfg c2.view && writable (s.cert1s.get? c1.view) c1
          && writable (s.cert2s.get? c2.view) c2 && writable (s.proposals.get? c2.view) q) = true
      · rw [ingest, handle, if_pos hg, if_pos hw] at h ⊢
        exact get?_insert_of_writable (epochChange_writable hw).2.2 (hwf.admitted _ _ h)
      · rw [ingest, handle, if_pos hg, if_neg hw] at h ⊢; exact hwf.admitted _ _ h
    · rw [ingest, handle, if_neg hg] at h ⊢; exact hwf.admitted _ _ h
  | proposal sender q vid =>
    simp only [ingest, handle, apply_ite, ite_self] at h ⊢
    by_cases hg : s.admits cfg q vid = true
    · rw [if_pos hg] at h ⊢
      rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
      · exact get?_insert_self
      · exact get?_insert_of_writable (admits_iff.mp hg).2.2.1 (hwf.admitted _ _ h)
    · rw [if_neg hg] at h ⊢
      exact hwf.admitted _ _ h
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self] at h ⊢
    repeat' split
    all_goals exact hwf.admitted _ _ h

/--
Taking an input in preserves the representation invariant.

The validity field is the one that needs something from outside: the only arm
that writes the table is `Input.blockValidated`, and `ValidityReported` is
precisely the promise that what it writes is true.
-/
theorem ingest_wf (henv : ValidityReported i) (hwf : WF cfg s) : WF cfg (ingest cfg node s i) where
  proposals v p h := by
    rcases ingest_proposalProvenance h with hold | ⟨-, hv, -⟩ | ⟨-, -, -, hv, -⟩
    · exact hwf.proposals v p hold
    · exact hv
    · exact hv
  proposalsWellFormed v p h hne := by
    rcases ingest_proposalProvenance h with hold | ⟨-, -, hwfp⟩ | ⟨d1, d2, -, -, hwf2⟩
    · exact hwf.proposalsWellFormed v p hold hne
    · exact wellFormed_iff.mpr hwfp
    · exact wellFormed_iff.mpr hwf2.2.2.2.2.2.1
  admitted v p h := ingest_admitted_proposals hwf h
  cert1s v c h := by
    rcases ingest_cert1Provenance h with hold | ⟨-, hv⟩ | ⟨-, -, -, hv⟩
    · exact hwf.cert1s v c hold
    · exact hv
    · exact hv
  cert2s v c h := by
    rcases ingest_cert2Provenance h with hold | ⟨-, hv⟩ | ⟨-, -, -, hv⟩
    · exact hwf.cert2s v c hold
    · exact hv
    · exact hv
  timeoutCerts v tc h := by
    rcases ingest_timeoutCertProvenance h with hold | ⟨-, hv⟩
    · exact hwf.timeoutCerts v tc hold
    · exact hv.symm
  validated v p h := by
    rcases ingest_validatedProvenance h with hold | hin
    · exact hwf.validated v p hold
    · exact henv v (blockHash p) hin p rfl
  decided := by rw [ingest_decidedViews]; exact hwf.decided
  branches v u h := by
    rw [ingest_vote1Branches] at h
    rw [ingest_voted1Views, ingest_barredView]
    exact hwf.branches v u h

/-! ## The cursors -/

theorem ingest_currentViewMono : s.currentView ≤ (ingest cfg node s i).currentView := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] <;> repeat' split
  all_goals first | exact Nat.le_refl _ | exact ViewNumber.le_max_left ..

theorem ingest_timeoutViewMono : s.timeoutView ≤ (ingest cfg node s i).timeoutView := by
  cases i <;> simp only [ingest, handle, apply_ite, ite_self] <;> repeat' split
  all_goals first | exact Nat.le_refl _ | exact ViewNumber.le_max_left ..

/-- The bar only rises to a view whose own timer fired. -/
theorem ingest_timeoutViewJustified
    (h : s.timeoutView ≠ (ingest cfg node s i).timeoutView) :
    i = Input.timeout (ingest cfg node s i).timeoutView
      ∨ i = Input.timeoutOneHonest (ingest cfg node s i).timeoutView := by
  cases i with
  | timeout v =>
    have hv : (ingest cfg node s (Input.timeout v)).timeoutView = v := by
      by_cases hg : v ≠ s.currentView
      · exact absurd (show s.timeoutView = _ by simp only [ingest, handle, if_pos hg]) h
      · have he : (ingest cfg node s (Input.timeout v)).timeoutView
            = max s.timeoutView v := by simp only [ingest, handle, if_neg hg]
        rw [he] at h ⊢
        exact ViewNumber.max_eq_right_of_ne (Ne.symm h)
    exact Or.inl (by rw [hv])
  | timeoutOneHonest v =>
    have hv : (ingest cfg node s (Input.timeoutOneHonest v)).timeoutView = v := by
      by_cases hg : v < s.currentView
      · exact absurd (show s.timeoutView = _ by simp only [ingest, handle, if_pos hg]) h
      · have he : (ingest cfg node s (Input.timeoutOneHonest v)).timeoutView
            = max s.timeoutView v := by simp only [ingest, handle, if_neg hg]
        rw [he] at h ⊢
        exact ViewNumber.max_eq_right_of_ne (Ne.symm h)
    exact Or.inr (by rw [hv])
  | _ =>
    simp only [ingest, handle, apply_ite, ite_self] at h
    repeat' split at h
    all_goals exact absurd rfl h

end Impl
end NewProtocol
