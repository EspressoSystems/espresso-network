module

public import NewProtocolImpl.Conformance.Owed

/-!
# The machine actually acts

`Impl.conforms` discharges progress by eagerness — nothing is ever left owed —
and an argument of that shape deserves a check that it is not protecting an inert
machine. A node that admitted nothing and stored nothing would also leave nothing
owed. What rules that out is the ingestion half of `StepSpec`, proved in
`NewProtocolImpl.Conformance.Sound`; what is checked here is the other side of
the same coin: where an action is due once the input is recorded, the step *takes*
it.

These are ordinary consequences of what is already proved, not further
obligations. They are the reading of `Impl.next_settles` that means something: it
says nothing is owed when a step ends, and each theorem below says *why* — because
the step acted, and not because the opportunity quietly evaporated.

Each therefore names its escape hatches, and they are exactly the state a later
round of the same pass may move: the lock, the decided views and the branch
records. There is nothing else, because everything else an action reads is frozen
for the whole pass.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {s : State} {i : Input} {a b : State} {p : Proposal}

/-! ## Carrying a justification across a pass

Content is framed for the whole pass, so each justification transfers on its own;
the two that also read the lock take the lock's fate as a hypothesis.
-/

theorem vote1Justification_congr (hfr : Frame a b) (hlock : b.lockedCert = a.lockedCert)
    (hj : Vote1Justification a.abstract p) : Vote1Justification b.abstract p := by
  obtain ⟨hadm, hvalid, hvid, hsafe, hpar⟩ := hj
  refine ⟨?_, hvalid, ?_, show SafeToExtend b.lockedCert p by rw [hlock]; exact hsafe, ?_⟩
  · show b.admitted.get? p.viewNumber = some p
    rw [hfr.admitted]; exact hadm
  · show (b.vidShares.get? p.viewNumber).isSome = true
    rw [hfr.vidShares]; exact hvid
  · intro hne
    obtain ⟨parent, hp, hbh, hrec⟩ := hpar hne
    refine ⟨parent, ?_, hbh, ?_⟩
    · show b.proposals.get? p.parentCert.view = some parent
      rw [hfr.proposals]; exact hp
    · show (p.parentCert.view, parent.payloadCommit) ∈ b.blocksReconstructed
      rw [hfr.blocksReconstructed]; exact hrec

theorem vote2Justification_congr (hfr : Frame a b)
    (hj : Vote2Justification a.abstract p) : Vote2Justification b.abstract p := by
  obtain ⟨hadm, ⟨c1, hc1, hc1h⟩, hrec⟩ := hj
  refine ⟨?_, ⟨c1, ?_, hc1h⟩, ?_⟩
  · show b.admitted.get? p.viewNumber = some p
    rw [hfr.admitted]; exact hadm
  · show b.cert1s.get? p.viewNumber = some c1
    rw [hfr.cert1s]; exact hc1
  · show (p.viewNumber, p.payloadCommit) ∈ b.blocksReconstructed
    rw [hfr.blocksReconstructed]; exact hrec

theorem proposalJustification_congr (hfr : Frame a b) (hlock : b.lockedCert = a.lockedCert)
    (hj : ProposalJustification leader node a.abstract p) :
    ProposalJustification leader node b.abstract p := by
  obtain ⟨hlead, hwfp, hjust, parent, hpar, hbh, hhdr⟩ := hj
  refine ⟨hlead, hwfp, ?_, parent, ?_, hbh, ?_⟩
  · revert hjust
    unfold ParentCertJustified
    cases p.timeoutEvidence with
    | some tc =>
      refine fun h => ⟨?_, show b.lockedCert = some p.parentCert by rw [hlock]; exact h.2⟩
      show b.timeoutCerts.get? p.viewNumber = some tc
      rw [hfr.timeoutCerts]; exact h.1
    | none =>
      refine fun h => ⟨?_, h.2⟩
      show b.cert1s.get? (p.viewNumber - 1) = some p.parentCert
      rw [hfr.cert1s]; exact h.1
  · show b.proposals.get? p.parentCert.view = some parent
    rw [hfr.proposals]; exact hpar
  · show b.headers.get? (p.viewNumber, blockHash parent) = some p.blockHeader
    rw [hfr.headers]; exact hhdr

theorem timeoutView_lt (hfr : Frame a b) {v : ViewNumber}
    (h : a.abstract.timeoutView < v) : b.abstract.timeoutView < v := by
  show b.timeoutView < v
  rw [hfr.timeoutView]; exact h

theorem barredView_lt (hfr : Frame a b) {v : ViewNumber}
    (h : a.abstract.barredView < v) : b.abstract.barredView < v := by
  show b.barredView < v
  rw [hfr.barredView]; exact h

/-! ## What a step does with what it is owed -/

/--
A vote1 due once the input is recorded is one the step casts — unless the
step's own lock advance moved the lock, which is the one thing that can withdraw
the opportunity: `Vote1Justification.safeToExtend` is re-read against the lock
the step leaves behind.
-/
theorem vote1_acted (henv : ValidityReported i) (hwf : WF s)
    (hen : Vote1Enabled (ingest cfg node s i).abstract p)
    (hfresh : p.viewNumber ∉ (ingest cfg node s i).voted1Views) :
    (∃ vt : Vote1, Output.send (.vote1 vt) ∈ (next cfg leader node s i).2
        ∧ vt.view = p.viewNumber)
      ∨ (next cfg leader node s i).1.lockedCert ≠ (ingest cfg node s i).lockedCert := by
  have hwft := ingest_wf (cfg := cfg) (node := node) (s := s) (i := i) henv hwf
  by_cases hm : p.viewNumber ∈ (st5 cfg leader node (ingest cfg node s i)).voted1Views
  · obtain ⟨vt, hout, hview⟩ := pass_vote1Marked hwft hfresh hm
    exact Or.inl ⟨vt, mem_next_of_pass hout, hview⟩
  · refine Or.inr ?_
    rw [next_state]
    intro hlock
    obtain ⟨hj, hver, -, htv, hbar, hlk⟩ := hen
    have hfr := (st5_stage (cfg := cfg) (leader := leader) (node := node) hwft).1
    exact pass_vote1_settled hwft p ⟨vote1Justification_congr hfr hlock hj,
      show (st5 cfg leader node (ingest cfg node s i)).validated.get? p.viewNumber
          = some (blockHash p) by rw [hfr.validated]; exact hver,
      hm, timeoutView_lt hfr htv, barredView_lt hfr hbar,
      fun lock hl => hlk lock (show (ingest cfg node s i).lockedCert = some lock by
        rw [← hlock]; exact hl)⟩

/--
A vote2 due once the input is recorded is one the step casts — unless the
same pass settled the view another way: decided it, raised the floor past it, or
cast a vote1 that skipped it.
-/
theorem vote2_acted (henv : ValidityReported i) (hwf : WF s)
    (hen : Vote2Enabled cfg (ingest cfg node s i).abstract p)
    (hfresh : p.viewNumber ∉ (ingest cfg node s i).voted2Views) :
    (∃ vt : Vote2, Output.send (.vote2 vt) ∈ (next cfg leader node s i).2
        ∧ vt.view = p.viewNumber)
      ∨ Vote1SkippedView (next cfg leader node s i).1.abstract p.viewNumber
      ∨ (next cfg leader node s i).1.abstract.decidedViews p.viewNumber
      ∨ ¬ (next cfg leader node s i).1.abstract.aboveDecideFloor cfg p.viewNumber := by
  have hwft := ingest_wf (cfg := cfg) (node := node) (s := s) (i := i) henv hwf
  by_cases hm : p.viewNumber ∈ (st5 cfg leader node (ingest cfg node s i)).voted2Views
  · obtain ⟨vt, hout, hview⟩ := pass_vote2Marked hwft hfresh hm
    exact Or.inl ⟨vt, mem_next_of_pass hout, hview⟩
  rw [next_state]
  by_cases haround : Vote1SkippedView
      (st5 cfg leader node (ingest cfg node s i)).abstract p.viewNumber
  · exact Or.inr (Or.inl haround)
  by_cases hdec : p.viewNumber ∈ (st5 cfg leader node (ingest cfg node s i)).decidedViews
  · exact Or.inr (Or.inr (Or.inl hdec))
  by_cases hab : (st5 cfg leader node (ingest cfg node s i)).abstract.aboveDecideFloor
      cfg p.viewNumber
  · -- nothing was withdrawn, so the vote would still be owed: impossible
    obtain ⟨hj, -, -, hc2, -, -, hbar⟩ := hen
    have hfr := (st5_stage (cfg := cfg) (leader := leader) (node := node) hwft).1
    refine (pass_vote2_settled (cfg := cfg) (leader := leader) (node := node) hwft p ?_).elim
    refine ⟨vote2Justification_congr hfr hj, haround, hm, ?_, hdec, hab, barredView_lt hfr hbar⟩
    show (st5 cfg leader node (ingest cfg node s i)).cert2s.get? p.viewNumber = none
    rw [hfr.cert2s]; exact hc2
  · exact Or.inr (Or.inr (Or.inr hab))

/--
A decide due once the input is recorded is one the step emits — unless the same
pass raised the floor past the view, which is the one way a decide stops being
owed without happening.
-/
theorem decide_acted (henv : ValidityReported i) (hwf : WF s) {v : ViewNumber}
    (hen : DecideEnabled cfg (ingest cfg node s i).abstract v) :
    (∃ chain c1 c2 b, Output.decided chain c1 c2 ∈ (next cfg leader node s i).2
        ∧ b ∈ chain ∧ b.viewNumber = v)
      ∨ ¬ (next cfg leader node s i).1.abstract.aboveDecideFloor cfg v := by
  have hwft := ingest_wf (cfg := cfg) (node := node) (s := s) (i := i) henv hwf
  obtain ⟨hnd, -, hc1, c2, q, hc2, hq, hbh⟩ := hen
  by_cases hm : v ∈ (st5 cfg leader node (ingest cfg node s i)).decidedViews
  · obtain ⟨chain, c1, c2', b, hout, hb, hbv⟩ := pass_decidedMarked hwft hnd hm
    exact Or.inl ⟨chain, c1, c2', b, mem_next_of_pass hout, hb, hbv⟩
  rw [next_state]
  by_cases hab : (st5 cfg leader node (ingest cfg node s i)).abstract.aboveDecideFloor cfg v
  · -- nothing was withdrawn, so the decide would still be owed: impossible
    have hfr := (st5_stage (cfg := cfg) (leader := leader) (node := node) hwft).1
    refine (pass_decide_settled (cfg := cfg) (leader := leader) (node := node) hwft v ?_).elim
    refine ⟨hm, hab, ?_, c2, q, ?_, ?_, hbh⟩
    · show ((st5 cfg leader node (ingest cfg node s i)).cert1s.get? v).isSome = true
      rw [hfr.cert1s]; exact hc1
    · show (st5 cfg leader node (ingest cfg node s i)).cert2s.get? v = some c2
      rw [hfr.cert2s]; exact hc2
    · show (st5 cfg leader node (ingest cfg node s i)).proposals.get? v = some q
      rw [hfr.proposals]; exact hq
  · exact Or.inr hab

/--
A proposal due once the input is recorded is one the step makes — unless the
step's own lock advance moved the lock, which after a timeout is what the
proposal's parent certificate has to be.
-/
theorem propose_acted (henv : ValidityReported i) (hwf : WF s)
    (hen : ProposeEnabled leader node (ingest cfg node s i).abstract p)
    (hfresh : p.viewNumber ∉ (ingest cfg node s i).proposedViews) :
    (∃ q : Proposal, Output.send (.proposal q) ∈ (next cfg leader node s i).2
        ∧ q.viewNumber = p.viewNumber)
      ∨ (next cfg leader node s i).1.lockedCert ≠ (ingest cfg node s i).lockedCert := by
  have hwft := ingest_wf (cfg := cfg) (node := node) (s := s) (i := i) henv hwf
  by_cases hm : p.viewNumber ∈ (st5 cfg leader node (ingest cfg node s i)).proposedViews
  · obtain ⟨q, hout, hview⟩ := pass_proposedMarked hwft hfresh hm
    exact Or.inl ⟨q, mem_next_of_pass hout, hview⟩
  · refine Or.inr ?_
    rw [next_state]
    intro hlock
    obtain ⟨hj, -, htv, hbar⟩ := hen
    have hfr := (st5_stage (cfg := cfg) (leader := leader) (node := node) hwft).1
    exact pass_propose_settled hwft p ⟨proposalJustification_congr hfr hlock hj, hm,
      timeoutView_lt hfr htv, barredView_lt hfr hbar⟩

end Impl
end NewProtocol
