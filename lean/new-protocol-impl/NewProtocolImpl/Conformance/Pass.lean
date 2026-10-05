module

public import NewProtocolImpl.Conformance.Cases

/-!
# A pass, taken apart

The reaction pass of `Impl.next` is five segments in a fixed order, and the
order is what several obligations turn on. This file names the states between
the segments (`Impl.st1` … `Impl.st5`) and says what each segment can no
longer change:

* the lock is settled at `Impl.st2` and stays — so the lock a vote1
  tested is the lock the step is judged against (`Vote1Justification.safeToExtend`), and
  the lock a vote2 needs is one the step already holds
  (`vote2LockOrdered`);
* the decided views are settled at `Impl.st1` — so the floor a vote2
  tested is the floor at the end (`vote2AboveFloor`);
* the branch records are settled at `Impl.st3` — so the record a vote2
  tested against is the whole record at the end (`vote2NotInSkippedView`);
* content is settled before the pass begins, by `Frame`.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)
variable (t : State)

/-! ## The segments -/

/-- The decide round: one attempt per view holding a `Cert2`. -/
def dSeg : List StepFn := t.cert2s.keys.map (tryDecide cfg t.decidedViews)

/-- The vote1 round: one attempt per admitted proposal. -/
def v1Seg : List StepFn := t.admitted.keys.map (tryVote1 node)

/-- The vote2 round. -/
def v2Seg : List StepFn := t.admitted.keys.map (tryVote2 cfg node)

/-- The proposing round: one attempt per view a header is headers for. -/
def pSeg : List StepFn := t.headers.keys.map fun k => tryPropose leader node k.1

theorem rounds_eq : rounds cfg leader node t
    = dSeg cfg t ++ [advanceLock] ++ v1Seg node t ++ v2Seg cfg node t
      ++ pSeg leader node t := rfl

/-! ## The states between them -/

/-- After the decides. -/
def st1 : State := (seq (dSeg cfg t) t).1

/-- After the lock advance. -/
def st2 : State := (advanceLock (st1 cfg t)).1

/-- After the vote1s. -/
def st3 : State := (seq (v1Seg node t) (st2 cfg t)).1

/-- After the vote2s. -/
def st4 : State := (seq (v2Seg cfg node t) (st3 cfg node t)).1

/-- After the proposals: the state the step ends in. -/
def st5 : State := (seq (pSeg leader node t) (st4 cfg node t)).1

theorem pass_state :
    (seq (rounds cfg leader node t) t).1 = st5 cfg leader node t := by
  rw [rounds_eq]
  simp only [seq_append, seq, List.append_nil, st1, st2, st3, st4, st5]

theorem pass_out :
    (seq (rounds cfg leader node t) t).2
      = (seq (dSeg cfg t) t).2 ++ (advanceLock (st1 cfg t)).2
        ++ (seq (v1Seg node t) (st2 cfg t)).2
        ++ (seq (v2Seg cfg node t) (st3 cfg node t)).2
        ++ (seq (pSeg leader node t) (st4 cfg node t)).2 := by
  rw [rounds_eq]
  simp only [seq_append, seq, List.append_nil, List.append_assoc, st1, st2, st3, st4]

/-! ## Every segment grows the state -/

theorem dSeg_grows : Grows (seq (dSeg cfg t)) := by
  refine seq_grows fun f hf => ?_
  obtain ⟨v, -, rfl⟩ :=
    List.mem_map.mp (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hf)
  exact tryDecide_grows

theorem v1Seg_grows : Grows (seq (v1Seg node t)) := by
  refine seq_grows fun f hf => ?_
  obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
  exact tryVote1_grows

theorem v2Seg_grows : Grows (seq (v2Seg cfg node t)) := by
  refine seq_grows fun f hf => ?_
  obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
  exact tryVote2_grows

theorem pSeg_grows : Grows (seq (pSeg leader node t)) := by
  refine seq_grows fun f hf => ?_
  obtain ⟨k, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (fun k => tryPropose leader node k.1) t.headers.keys from hf)
  exact tryPropose_grows

/-! ## The stages

Each state the pass passes through frames the content of the one it started
from, grows its marks, and keeps the invariant. Chaining these is how a fact
checked at one round transfers to the state the step ends in.
-/

section Stages

variable {cfg leader node t}

theorem st1_stage (hwf : WF t) :
    Frame t (st1 cfg t) ∧ Le t (st1 cfg t) ∧ WF (st1 cfg t) :=
  dSeg_grows cfg t t hwf

theorem st2_stage (hwf : WF t) :
    Frame t (st2 cfg t) ∧ Le t (st2 cfg t) ∧ WF (st2 cfg t) := by
  obtain ⟨hf, hle, hw⟩ := st1_stage hwf
  obtain ⟨hf', hle', hw'⟩ := advanceLock_grows (st1 cfg t) hw
  exact ⟨hf.trans hf', hle.trans hle', hw'⟩

theorem st3_stage (hwf : WF t) :
    Frame t (st3 cfg node t) ∧ Le t (st3 cfg node t)
      ∧ WF (st3 cfg node t) := by
  obtain ⟨hf, hle, hw⟩ := st2_stage hwf
  obtain ⟨hf', hle', hw'⟩ := v1Seg_grows node t (st2 cfg t) hw
  exact ⟨hf.trans hf', hle.trans hle', hw'⟩

theorem st4_stage (hwf : WF t) :
    Frame t (st4 cfg node t) ∧ Le t (st4 cfg node t)
      ∧ WF (st4 cfg node t) := by
  obtain ⟨hf, hle, hw⟩ := st3_stage hwf
  obtain ⟨hf', hle', hw'⟩ := v2Seg_grows cfg node t (st3 cfg node t) hw
  exact ⟨hf.trans hf', hle.trans hle', hw'⟩

theorem st5_stage (hwf : WF t) :
    Frame t (st5 cfg leader node t) ∧ Le t (st5 cfg leader node t)
      ∧ WF (st5 cfg leader node t) := by
  obtain ⟨hf, hle, hw⟩ := st4_stage hwf
  obtain ⟨hf', hle', hw'⟩ := pSeg_grows leader node t (st4 cfg node t) hw
  exact ⟨hf.trans hf', hle.trans hle', hw'⟩

/-- From the vote2s to the end: what the proposals round leaves. -/
theorem st4_to_st5 (hwf : WF t) :
    Frame (st4 cfg node t) (st5 cfg leader node t)
      ∧ Le (st4 cfg node t) (st5 cfg leader node t) := by
  obtain ⟨hf, hle, -⟩ := pSeg_grows leader node t (st4 cfg node t) (st4_stage hwf).2.2
  exact ⟨hf, hle⟩

/-- From the vote1s to the end. -/
theorem st3_to_st5 (hwf : WF t) :
    Le (st3 cfg node t) (st5 cfg leader node t) :=
  ((v2Seg_grows cfg node t (st3 cfg node t) (st3_stage hwf).2.2).2.1).trans
    (st4_to_st5 hwf).2

/-- From the decides to the end. -/
theorem st1_to_st5 (hwf : WF t) :
    Le (st1 cfg t) (st5 cfg leader node t) := by
  have h12 := advanceLock_grows (st1 cfg t) (st1_stage hwf).2.2
  have h23 := v1Seg_grows node t (st2 cfg t) (st2_stage hwf).2.2
  exact (h12.2.1.trans h23.2.1).trans (st3_to_st5 hwf)

end Stages

/-! ## What is settled when -/

section Settled

variable {cfg leader node t}

/-- The lock does not move after the lock advance. -/
theorem st5_lock :
    (st5 cfg leader node t).lockedCert = (st2 cfg t).lockedCert := by
  rw [st5, seq_proj State.lockedCert (fun f hf t' => by
        obtain ⟨k, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (fun k => tryPropose leader node k.1) t.headers.keys from hf)
        exact tryPropose_lock t'),
    st4, seq_proj State.lockedCert (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
        exact tryVote2_lock t'),
    st3, seq_proj State.lockedCert (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
        exact tryVote1_lock t')]

/-- Nothing is decided after the decide round. -/
theorem st5_decided :
    (st5 cfg leader node t).decidedViews = (st1 cfg t).decidedViews := by
  rw [st5, seq_proj State.decidedViews (fun f hf t' => by
        obtain ⟨k, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (fun k => tryPropose leader node k.1) t.headers.keys from hf)
        exact tryPropose_decided t'),
    st4, seq_proj State.decidedViews (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
        exact tryVote2_decided t'),
    st3, seq_proj State.decidedViews (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
        exact tryVote1_decided t'),
    st2, advanceLock_decided]

/-- The lock is settled at `st2`: the vote1s do not move it. -/
theorem st3_lock : (st3 cfg node t).lockedCert = (st2 cfg t).lockedCert := by
  rw [st3, seq_proj State.lockedCert (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
        exact tryVote1_lock t')]

/-- Nothing is decided between the decide round and the vote2s. -/
theorem st3_decided :
    (st3 cfg node t).decidedViews = (st1 cfg t).decidedViews := by
  rw [st3, seq_proj State.decidedViews (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
        exact tryVote1_decided t'), st2, advanceLock_decided]

/-- The lock is still the one the lock advance left, when the proposals are made. -/
theorem st4_lock : (st4 cfg node t).lockedCert = (st2 cfg t).lockedCert := by
  rw [st4, seq_proj State.lockedCert (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
        exact tryVote2_lock t'), st3_lock]

/-- No branch is recorded after the vote1 round. -/
theorem st5_branches :
    (st5 cfg leader node t).vote1Branches = (st3 cfg node t).vote1Branches := by
  rw [st5, seq_proj State.vote1Branches (fun f hf t' => by
        obtain ⟨k, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (fun k => tryPropose leader node k.1) t.headers.keys from hf)
        exact tryPropose_branches t'),
    st4, seq_proj State.vote1Branches (fun f hf t' => by
        obtain ⟨v, -, rfl⟩ := List.mem_map.mp (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
        exact tryVote2_branches t')]

end Settled

end Impl
end NewProtocol
