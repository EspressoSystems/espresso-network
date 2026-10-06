module

public import NewProtocolImpl.Conformance.Marks

/-!
# What an input owes, and what it emits

The obligations `StepSpec` attaches to an input rather than to an action: the
`Cert2` relay, the two view advances, the timeout vote, and the soundness side of
the three messages `Impl.handle` sends on its own.

`Impl.mem_ingestOut` is the reading in the other direction — a complete list of
what taking an input in can emit. Its main use is negative: no vote, proposal or
decide is ever among them, which is what lets the action obligations look at the
reaction pass alone.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {s : State} {i : Input}

/-! ## The cursors -/

/--
Where the view a step ends in comes from: a certificate for the previous view, a
timeout certificate over it, or the input that carried one.
-/
theorem ingest_currentViewJustified
    (h : s.currentView ≠ (ingest cfg node s i).currentView) :
    ∃ v, (ingest cfg node s i).currentView = v + 1
      ∧ ((∃ c, (ingest cfg node s i).cert1s.get? v = some c)
          ∨ (∃ tc, (ingest cfg node s i).timeoutCerts.get? (v + 1) = some tc)
          ∨ (∃ c, i = Input.advanceView c ∧ c.view = v)
          ∨ (∃ tc, i = Input.timeoutCertificate tc ∧ tc.view = v)) := by
  cases i with
  | advanceView c =>
    refine ⟨c.view, ?_, Or.inr (Or.inr (Or.inl ⟨c, rfl, rfl⟩))⟩
    have he : (ingest cfg node s (Input.advanceView c)).currentView
        = max s.currentView (c.view + 1) := by
      simp only [ingest, handle, apply_ite, ite_self]
    rw [he] at h ⊢
    exact ViewNumber.max_eq_right_of_ne (Ne.symm h)
  | timeoutCertificate tc =>
    refine ⟨tc.view, ?_, Or.inr (Or.inr (Or.inr ⟨tc, rfl, rfl⟩))⟩
    have he : (ingest cfg node s (Input.timeoutCertificate tc)).currentView
        = max s.currentView (tc.view + 1) := by
      simp only [ingest, handle, apply_ite, ite_self]
    rw [he] at h ⊢
    exact ViewNumber.max_eq_right_of_ne (Ne.symm h)
  | _ =>
    refine absurd ?_ h
    simp only [ingest, handle, apply_ite, ite_self]
    repeat' split
    all_goals rfl

/-! ## What is owed -/

theorem ingest_advanceOwed {c : Cert1} (hi : i = Input.advanceView c) :
    c.view + 1 ≤ (ingest cfg node s i).currentView := by
  subst hi
  have he : (ingest cfg node s (Input.advanceView c)).currentView
      = max s.currentView (c.view + 1) := by
    simp only [ingest, handle, apply_ite, ite_self]
  rw [he]
  exact ViewNumber.le_max_right ..

theorem ingest_timeoutCertAdvanceOwed {tc : TimeoutCert} (hi : i = Input.timeoutCertificate tc) :
    tc.view + 1 ≤ (ingest cfg node s i).currentView := by
  subst hi
  have he : (ingest cfg node s (Input.timeoutCertificate tc)).currentView
      = max s.currentView (tc.view + 1) := by
    simp only [ingest, handle, apply_ite, ite_self]
  rw [he]
  exact ViewNumber.le_max_right ..

/-- A `Cert2` seen for the first time is relayed: it has no other route to peers. -/
theorem ingest_cert2RelayOwed {c : Cert2} (hi : i = Input.certificate2 c)
    (hnone : s.cert2s.get? c.view = none) (hdec : c.view ∉ s.decidedViews)
    (hfl : s.aboveFloor cfg c.view = true) :
    Output.send (.cert2 c) ∈ ingestOut cfg node s i := by
  subst hi
  have hguard : s.aboveFloor cfg c.view = true ∧ ¬ s.cert2s.contains c.view = true := by
    refine ⟨hfl, fun hc => ?_⟩
    obtain ⟨x, hx⟩ := exists_get?_of_contains hc
    rw [hnone] at hx
    exact absurd hx (by simp)
  simp only [ingestOut, handle, if_pos hguard]
  rw [if_neg (fun hc => hdec (contains_iff_mem.mp hc))]
  exact List.mem_cons_self ..

/-- A timeout the node is entitled to answer is answered. -/
theorem ingest_timeoutVoteOwed {v : ViewNumber}
    (hi : (i = Input.timeout v ∧ v = s.currentView)
      ∨ (i = Input.timeoutOneHonest v ∧ s.currentView ≤ v)) :
    ∃ e, Output.send (.timeoutVote ⟨(), v, node⟩ e) ∈ ingestOut cfg node s i := by
  rcases hi with ⟨rfl, hv⟩ | ⟨rfl, hv⟩
  · refine ⟨s.catchupEvidence, ?_⟩
    simp only [ingestOut, handle, if_neg (show ¬ v ≠ s.currentView from fun hc => hc hv)]
    exact List.mem_cons_self ..
  · refine ⟨s.catchupEvidence, ?_⟩
    simp only [ingestOut, handle, if_neg (show ¬ v < s.currentView from Nat.not_lt.mpr hv)]
    exact List.mem_cons_self ..

/-! ## What may be emitted

Everything an input arm can emit, with the facts the obligations about those
messages ask for — and it is only three messages now that a node asks its own
subsystems for nothing. No vote, proposal or decide appears, which is the fact
the action obligations use; six of the nine inputs emit nothing at all.
-/

theorem mem_ingestOut {o : Output} (h : o ∈ ingestOut cfg node s i) :
    (∃ c, o = Output.send (.cert2 c) ∧ i = Input.certificate2 c)
      ∨ (∃ w e, o = Output.send (.timeoutVote ⟨(), w, node⟩ e)
          ∧ w ≤ (ingest cfg node s i).timeoutView
          ∧ ((i = Input.timeout w ∧ w = s.currentView)
              ∨ (i = Input.timeoutOneHonest w ∧ s.currentView ≤ w)))
      ∨ (∃ tc w, o = Output.send (.timeoutCert tc w) ∧ i = Input.timeoutCertificate tc
          ∧ w = tc.view + 1 ∧ w ≤ (ingest cfg node s i).currentView) := by
  cases i with
  | blockReconstructed v pc => exact absurd h (by simp [ingestOut, handle])
  | headerBuilt v pa hd =>
    refine absurd h ?_
    simp only [ingestOut, handle]
    repeat' split
    all_goals simp
  | blockValidated v hb =>
    refine absurd h ?_
    simp only [ingestOut, handle]
    repeat' split
    all_goals simp
  | certificate1 c =>
    refine absurd h ?_
    simp only [ingestOut, handle]
    repeat' split
    all_goals simp
  | advanceView c =>
    refine absurd h ?_
    simp only [ingestOut, handle]
    repeat' split
    all_goals simp
  | proposal sender p vid =>
    refine absurd h ?_
    simp only [ingestOut, handle]
    repeat' split
    all_goals simp
  | certificate2 c =>
    by_cases hg : s.aboveFloor cfg c.view = true ∧ ¬ s.cert2s.contains c.view = true
    · simp only [ingestOut, handle, if_pos hg] at h
      split at h
      · exact absurd h (by simp)
      · obtain rfl : o = Output.send (.cert2 c) := by simpa using h
        exact Or.inl ⟨c, rfl, rfl⟩
    · refine absurd h ?_
      simp only [ingestOut, handle, if_neg hg]
      simp
  | timeout v =>
    by_cases hg : v ≠ s.currentView
    · refine absurd h ?_
      simp only [ingestOut, handle, if_pos hg]
      simp
    · refine Or.inr (Or.inl ⟨v, s.catchupEvidence, ?_, ?_,
        Or.inl ⟨rfl, Decidable.of_not_not hg⟩⟩)
      · simp only [ingestOut, handle, if_neg hg] at h
        simpa using h
      · have he : (ingest cfg node s (Input.timeout v)).timeoutView
            = max s.timeoutView v := by simp only [ingest, handle, if_neg hg]
        rw [he]
        exact ViewNumber.le_max_right ..
  | timeoutOneHonest v =>
    by_cases hg : v < s.currentView
    · refine absurd h ?_
      simp only [ingestOut, handle, if_pos hg]
      simp
    · refine Or.inr (Or.inl ⟨v, s.catchupEvidence, ?_, ?_,
        Or.inr ⟨rfl, Nat.ge_of_not_lt hg⟩⟩)
      · simp only [ingestOut, handle, if_neg hg] at h
        simpa using h
      · have he : (ingest cfg node s (Input.timeoutOneHonest v)).timeoutView
            = max s.timeoutView v := by simp only [ingest, handle, if_neg hg]
        rw [he]
        exact ViewNumber.le_max_right ..
  | timeoutCertificate tc =>
    have hb : tc.view + 1 ≤ (ingest cfg node s (Input.timeoutCertificate tc)).currentView :=
      ingest_timeoutCertAdvanceOwed rfl
    by_cases hg : tc.view + 1 < s.currentView ∨ s.timeoutCerts.contains (tc.view + 1) = true
    · -- the certificate is already known, or the view already left: nothing is emitted
      refine absurd h ?_
      have hpair : handle cfg node (Input.timeoutCertificate tc) s
          = ({ s with currentView := max s.currentView (tc.view + 1) }, []) := by
        simp only [handle, if_pos hg]
        try rfl
      simp only [ingestOut, hpair]
      simp
    · refine Or.inr (Or.inr ⟨tc, tc.view + 1, ?_, rfl, rfl, hb⟩)
      have hpair : handle cfg node (Input.timeoutCertificate tc) s
          = ({ s with timeoutCerts := s.timeoutCerts.insert (tc.view + 1) tc,
                      currentView := max s.currentView (tc.view + 1) },
             [Output.send (.timeoutCert tc (tc.view + 1))]) := by
        simp only [handle, if_neg hg]
        try rfl
      simp only [ingestOut, hpair] at h
      simpa using h

end Impl
end NewProtocol
