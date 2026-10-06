module

public import NewProtocolImpl.Conformance.Vote2

/-!
# The vote1

Everything `StepSpec` asks about an emitted vote1.

What the lock-before-votes order buys: `Vote1Justification.safeToExtend`
is checked against the lock *as it stands at the end of the step* — and the lock
stopped moving at `Impl.st2`, before this round began, so the test this round
made is the test it is judged by.

The other two of note are the pair that keeps a vote from being forgotten: the
vote travels with the VID share that lets peers reconstruct the block
(`vote1CarriesShare`, which needs the round's *other* output to reach the pass —
the last clause of `Impl.mem_seq`), and it records the branch it endorsed
(`vote1Records`), which survives to the end of the step because the vote2s
and the proposals add no records of their own.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {t : State}

theorem v1Seg_frozen (f : StepFn) (hf : f ∈ v1Seg node t) (u : State) :
    (f u).1.lockedCert = u.lockedCert := by
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
  exact tryVote1_lock u

theorem v1Seg_grows' (f : StepFn) (hf : f ∈ v1Seg node t) : Grows f := by
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
  exact tryVote1_grows

/--
A vote1 in a pass's output: the proposal it signs, the share it travels
with, and every fact the specification asks of it.
-/
theorem pass_vote1 (hwf : WF t) {vt : Vote1}
    (h : Output.send (.vote1 vt) ∈ (seq (rounds cfg leader node t) t).2) :
    ∃ p share, vt = ⟨⟨blockHash p⟩, vt.view, node⟩
      ∧ t.admitted.get? vt.view = some p
      ∧ t.validated.get? vt.view = some (blockHash p)
      ∧ t.vidShares.get? vt.view = some share
      ∧ t.parentLinked p = true
      ∧ safeToExtend (st5 cfg leader node t).lockedCert p = true
      ∧ vt.view ∉ t.voted1Views
      ∧ vt.view ∈ (st5 cfg leader node t).voted1Views
      ∧ t.timeoutView < vt.view ∧ t.barredView < vt.view
      ∧ (st5 cfg leader node t).vote1Branches.get? vt.view = some p.parentCert.view
      ∧ Output.send (.vidShare share) ∈ (seq (rounds cfg leader node t) t).2 := by
  have hout := pass_out (cfg := cfg) (leader := leader) (node := node) (t := t)
  rw [hout] at h
  simp only [List.mem_append] at h
  -- Only the vote1 round emits a vote1.
  rcases h with ((((h | h) | h) | h) | h)
  · obtain ⟨chain, c1, c2, he⟩ := dSeg_shape h
    exact absurd he (by simp)
  · obtain ⟨c, he⟩ := advanceLock_shape h
    exact absurd he (by simp)
  · -- the vote1 round
    obtain ⟨f, hf, u, hwfu, hfr, hle, hlockU', ho, -, hle', hall⟩ :=
      mem_seq State.lockedCert (fun f hf => v1Seg_grows' f hf) (fun f hf u => v1Seg_frozen f hf u)
        (st2_stage hwf).2.2 h
    obtain ⟨v, -, rfl⟩ :=
      List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
    rcases tryVote1_cases (r := tryVote1 node v u) rfl with heq |
      ⟨p, share, hadm, hshare, hto, hbar, hvoted, hsv, hlinked, hste, heq⟩
    · rw [heq] at ho; exact absurd ho (by simp)
    · have hvt : vt = ⟨⟨blockHash p⟩, v, node⟩ := by rw [heq] at ho; simpa using ho
      subst hvt
      have hft : Frame t u := (st2_stage hwf).1.trans hfr
      have hlet : Le t u := (st2_stage hwf).2.1.trans hle
      -- the lock at the end of the step is the lock this round tested
      have hlockU : u.lockedCert = (st5 cfg leader node t).lockedCert := by
        rw [st5_lock]; exact hlockU'
      refine ⟨p, share, rfl, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
      -- content, moved back to the start of the pass
      · rw [← hft.admitted]; exact hadm
      · rw [← hft.validated]; exact hsv
      · rw [← hft.vidShares]; exact hshare
      · rw [show t.parentLinked p = u.parentLinked p by
            unfold State.parentLinked State.reconstructed
            rw [hft.proposals, hft.blocksReconstructed]]
        exact hlinked
      · rw [← hlockU]; exact hste
      -- freshness, moved back; the mark, moved forward
      · exact fun hc => hvoted (hlet.voted1 _ hc)
      · refine (st3_to_st5 hwf).voted1 _ (hle'.voted1 _ ?_)
        rw [heq]
        exact mem_insert_self
      · rw [← hft.timeoutView]; exact hto
      · rw [← hft.barredView]; exact hbar
      -- the branch record, moved forward
      · refine (st3_to_st5 hwf).branches _ _ (hle'.branches _ _ ?_)
        rw [heq]
        exact get?_insert_self
      -- the share travels with the vote
      · rw [hout]
        refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inl
          (List.mem_append.mpr (Or.inr ?_)))))
        refine hall _ ?_
        rw [heq]
        exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
  · obtain ⟨w, q, he⟩ := v2Seg_shape h
    exact absurd he (by simp)
  · obtain ⟨q, he⟩ := pSeg_shape h
    exact absurd he (by simp)

end Impl
end NewProtocol
