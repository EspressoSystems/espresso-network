module

public import NewProtocolImpl.Conformance.Shape

/-!
# The vote2

Everything `StepSpec` asks about an emitted vote2, established at the
state the step ends in.

This is the obligation with the most reach, and the one the round order exists
for. Three of its clauses are about fields a *later* round could in principle
have changed:

* `vote2LockOrdered` wants the lock to have reached the vote's view — and the
  lock stopped moving at `Impl.st2`, before the votes;
* `vote2AboveFloor` wants the view above the floor of the state the step ends
  in — and the decided views stopped growing at `Impl.st1`;
* `vote2NotInSkippedView` wants no branch record to skip the view — and the records
  stopped growing at `Impl.st3`, where this round starts.

So all three can be read off the state the vote was cast at, which is what
`Impl.v2Frozen` carries through `Impl.mem_seq`.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {t : State}

/-- The three fields the vote2 round leaves alone. -/
def v2Frozen (s : State) :
    Option Cert1 × TreeSet ViewNumber × TreeMap ViewNumber ViewNumber :=
  (s.lockedCert, s.decidedViews, s.vote1Branches)

theorem v2Seg_frozen (f : StepFn) (hf : f ∈ v2Seg cfg node t) (u : State) :
    v2Frozen (f u).1 = v2Frozen u := by
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
  unfold v2Frozen
  rw [tryVote2_lock, tryVote2_decided, tryVote2_branches]

theorem v2Seg_grows' (f : StepFn) (hf : f ∈ v2Seg cfg node t) : Grows cfg f := by
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
  exact tryVote2_grows

/-! ## Two fields read through a state that agrees on their input -/

theorem aboveFloor_congr {a b : State} (h : a.decidedViews = b.decidedViews) (cfg : Config)
    (v : ViewNumber) : a.aboveFloor cfg v = b.aboveFloor cfg v := by
  unfold State.aboveFloor State.floor State.lastDecided
  rw [h]

theorem vote1Skipped_congr {a b : State} (h : a.vote1Branches = b.vote1Branches)
    (v : ViewNumber) : a.vote1Skipped v = b.vote1Skipped v := by
  unfold State.vote1Skipped
  rw [h]

/-! ## The obligation -/

/--
A vote2 in a pass's output: the proposal it signs, and every fact the
specification asks of it — content at the state the pass began from, marks and
the lock at the state it ends in.
-/
theorem pass_vote2 (hwf : WF cfg t) {vt : Vote2}
    (h : Output.send (.vote2 vt) ∈ (seq (rounds cfg leader node t) t).2) :
    ∃ p c, vt = ⟨⟨blockHash p, p.epoch⟩, vt.view, node⟩
      ∧ t.admitted.get? vt.view = some p
      ∧ t.cert1s.get? vt.view = some c ∧ c.data.blockHash = blockHash p
      ∧ c.data.epoch = p.epoch
      ∧ (vt.view, p.payloadCommit) ∈ t.blocksReconstructed
      ∧ vt.view ∉ t.voted2Views
      ∧ vt.view ∈ (st5 cfg leader node t).voted2Views
      ∧ t.cert2s.get? vt.view = none
      ∧ t.barredView < vt.view
      ∧ (st5 cfg leader node t).aboveFloor cfg vt.view = true
      ∧ (st5 cfg leader node t).vote1Skipped vt.view = false
      ∧ ∃ l, (st5 cfg leader node t).lockedCert = some l ∧ vt.view ≤ l.view := by
  rw [pass_out] at h
  simp only [List.mem_append] at h
  -- Only the vote2 round emits a vote2.
  rcases h with ((((h | h) | h) | h) | h)
  · obtain ⟨chain, c1, c2, he⟩ := dSeg_shape h
    exact absurd he (by simp)
  · obtain ⟨c, he⟩ := advanceLock_shape h
    exact absurd he (by simp)
  · obtain ⟨w, q, share, hc⟩ := v1Seg_shape h
    rcases hc with he | he <;> exact absurd he (by simp)
  · -- the vote2 round
    obtain ⟨f, hf, u, hwfu, hfr, hle, hfrozen, ho, -, hle', -⟩ :=
      mem_seq v2Frozen (fun f hf => v2Seg_grows' f hf) (fun f hf u => v2Seg_frozen f hf u)
        (st3_stage hwf).2.2 h
    obtain ⟨v, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
    rcases tryVote2_cases (r := tryVote2 cfg node v u) rfl with heq |
      ⟨p, c, hadm, hlockable, hbar, hvoted, hc2, -, hfl, haround, hlockb, heq⟩
    · rw [heq] at ho; exact absurd ho (by simp)
    · rw [heq] at ho
      obtain rfl : vt = ⟨⟨blockHash p, p.epoch⟩, v, node⟩ := by simpa using ho
      -- the pass began at `t`, so compose the frames and the growth
      have hft : Frame t u := (st3_stage hwf).1.trans hfr
      have hlet : Le t u := (st3_stage hwf).2.1.trans hle
      -- the lock, the decided views and the branch records are those of `st3`
      have hlockU : u.lockedCert = (st2 cfg t).lockedCert := by
        rw [show u.lockedCert = (v2Frozen u).1 from rfl, hfrozen]
        exact st3_lock
      have hdecU : u.decidedViews = (st1 cfg t).decidedViews := by
        rw [show u.decidedViews = (v2Frozen u).2.1 from rfl, hfrozen]
        exact st3_decided
      have hbrU : u.vote1Branches = (st3 cfg node t).vote1Branches := by
        rw [show u.vote1Branches = (v2Frozen u).2.2 from rfl, hfrozen]
        rfl
      obtain ⟨hc1, q, hadm', hbh, hep, hrec⟩ := lockable_spec hlockable
      -- `lockable` reads the held proposal, the vote the admitted one; `WF.admitted`
      -- says the two agree wherever both are present.
      obtain rfl : p = q := by
        rw [hwfu.admitted v p hadm] at hadm'
        exact Option.some.injEq .. ▸ hadm'
      obtain ⟨l, hl, hlv⟩ := le_of_lockBelow_false hlockb
      refine ⟨p, c, rfl, ?_, ?_, hbh, hep, ?_, ?_, ?_, ?_, ?_, ?_, ?_, l, ?_, hlv⟩
      -- content, moved back to the start of the pass
      · rw [← hft.admitted]; exact hadm
      · rw [← hft.cert1s]; exact hc1
      · rw [← hft.blocksReconstructed]; exact hrec
      -- freshness, moved back
      · exact fun hc => hvoted (hlet.voted2 _ hc)
      -- the mark, moved forward
      · refine (st4_to_st5 hwf).2.voted2 _ (hle'.voted2 _ ?_)
        rw [heq]
        exact mem_insert_self
      · rw [← hft.cert2s]; exact hc2
      · rw [← hft.barredView]; exact hbar
      -- the floor at the end of the step is the floor this round saw
      · rw [aboveFloor_congr (st5_decided.trans hdecU.symm) cfg v]; exact hfl
      -- so are the branch records
      · rw [vote1Skipped_congr (st5_branches.trans hbrU.symm) v]; exact haround
      · rw [st5_lock, ← hlockU]; exact hl
  · obtain ⟨q, he⟩ := pSeg_shape h
    exact absurd he (by simp)

end Impl
end NewProtocol
