module

public import NewProtocolImpl.Conformance.Seq

/-!
# What each round leaves alone

`Impl.Frame` covers the fields *no* round writes. Three more fields are written
by one round each, and left alone by the other four; which four differs, and that
difference is the whole content of the round order:

* the **lock** moves only in `Impl.advanceLock`, so the lock a vote1
  tested is the lock the step is judged against (`Vote1Justification.safeToExtend`), and
  the lock a vote2 needs is one the step already holds
  (`vote2LockOrdered`);
* the **decided views** grow only in `Impl.tryDecide`, so the floor a vote2
  vote tested is the floor at the end of the step (`vote2AboveFloor`);
* the **branch records** grow only in `Impl.tryVote1`, which is why they are in
  `NewProtocolImpl.Conformance.Rounds` rather than here, so the records a
  vote2 tested against are all the records there are (`vote2NotInSkippedView`).
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber}

/-! ## The lock -/

theorem tryDecide_lock (s : State) :
    (tryDecide cfg settled v s).1.lockedCert = s.lockedCert := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote1_lock (s : State) : (tryVote1 node v s).1.lockedCert = s.lockedCert := by
  unfold tryVote1
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote2_lock (s : State) : (tryVote2 cfg node v s).1.lockedCert = s.lockedCert := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryPropose_lock (s : State) :
    (tryPropose cfg leader node v s).1.lockedCert = s.lockedCert := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals rfl

/-! ## The decided views -/

theorem advanceLock_decided (s : State) : (advanceLock s).1.decidedViews = s.decidedViews := by
  unfold advanceLock
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote1_decided (s : State) : (tryVote1 node v s).1.decidedViews = s.decidedViews := by
  unfold tryVote1
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote2_decided (s : State) :
    (tryVote2 cfg node v s).1.decidedViews = s.decidedViews := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryPropose_decided (s : State) :
    (tryPropose cfg leader node v s).1.decidedViews = s.decidedViews := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals rfl

/-! ## The voted1 marks -/

theorem tryDecide_voted1 (s : State) :
    (tryDecide cfg settled v s).1.voted1Views = s.voted1Views := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem advanceLock_voted1 (s : State) :
    (advanceLock s).1.voted1Views = s.voted1Views := by
  unfold advanceLock
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote2_voted1 (s : State) :
    (tryVote2 cfg node v s).1.voted1Views = s.voted1Views := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryPropose_voted1 (s : State) :
    (tryPropose cfg leader node v s).1.voted1Views = s.voted1Views := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals rfl
/-! ## The voted2 marks -/

theorem tryDecide_voted2 (s : State) :
    (tryDecide cfg settled v s).1.voted2Views = s.voted2Views := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem advanceLock_voted2 (s : State) :
    (advanceLock s).1.voted2Views = s.voted2Views := by
  unfold advanceLock
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote1_voted2 (s : State) :
    (tryVote1 node v s).1.voted2Views = s.voted2Views := by
  unfold tryVote1
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryPropose_voted2 (s : State) :
    (tryPropose cfg leader node v s).1.voted2Views = s.voted2Views := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals rfl
/-! ## The proposed marks -/

theorem tryDecide_proposed (s : State) :
    (tryDecide cfg settled v s).1.proposedViews = s.proposedViews := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem advanceLock_proposed (s : State) :
    (advanceLock s).1.proposedViews = s.proposedViews := by
  unfold advanceLock
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote1_proposed (s : State) :
    (tryVote1 node v s).1.proposedViews = s.proposedViews := by
  unfold tryVote1
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote2_proposed (s : State) :
    (tryVote2 cfg node v s).1.proposedViews = s.proposedViews := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals rfl

end Impl
end NewProtocol
