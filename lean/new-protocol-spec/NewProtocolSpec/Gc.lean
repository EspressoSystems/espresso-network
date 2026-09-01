module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step

/-!
# Garbage collection

`GcSpec`, the rule for the kind of transition that prunes rather than acts.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
The content of `s'` is contained in that of `s`: collection may drop
entries, never invent or alter them.
-/
structure Shrinks (s s' : NodeState) : Prop where
  proposals : ∀ v p, s'.proposals v = some p → s.proposals v = some p
  admitted : ∀ v p, s'.admitted v = some p → s.admitted v = some p
  vidShares : ∀ v sh, s'.vidShares v = some sh → s.vidShares v = some sh
  validated : ∀ v h, s'.validated v = some h → s.validated v = some h
  blocksReconstructed : ∀ v pc, s'.blocksReconstructed v pc → s.blocksReconstructed v pc
  headers : ∀ v h hd, s'.headers v h = some hd → s.headers v h = some hd
  cert1s : ∀ v c, s'.cert1s v = some c → s.cert1s v = some c
  cert2s : ∀ v c, s'.cert2s v = some c → s.cert2s v = some c
  timeoutCerts : ∀ v tc, s'.timeoutCerts v = some tc → s.timeoutCerts v = some tc
  vote1Branches : ∀ v u, s'.vote1Branches v = some u → s.vote1Branches v = some u

/--
Garbage collection: a silent transition that prunes state a node can no
longer act on.

Two watermarks, because two parts of the state go stale at different times —
this is the whole content of the rule, and merging them is wrong in both
directions:

* the **decide floor** bounds the decide path (`RetainsDecide`). A `Cert2`
  for an old view may still arrive, so a block and its certificates must
  outlive the view they belong to.
* `NodeState.barredView` bounds the vote path (`RetainsVote`) and the
  vote and proposal marks. Once a view is abandoned nothing can be voted or
  proposed there (`SafetySpec.vote1NotBarred` and friends), so neither the state
  a vote would read nor the record of having voted is load-bearing any more.

The bar may only advance below the view the node is in: entering a view is
itself evidence that the previous ones are settled
(`StepSpec.currentViewJustified`), and a node that could bar arbitrarily far
ahead could abandon views it ought to be voting in.

`decidedViews` follows the decide floor rather than the bar, since re-deciding
is what forgetting it would risk, and that is already impossible below the
floor. The lock and the cursors are preserved exactly: a collected node must
not forget what it is locked on, nor where it is.

Collection emits nothing, so none of the `StepSpec` obligations about outputs
apply.
-/
structure GcSpec (s s' : NodeState) : Prop where
  /-- Content may only be dropped, never invented or altered. -/
  shrinks : Shrinks s s'

  /-- The bar only rises, and only into territory the node has already left. -/
  barredViewMono : s.barredView ≤ s'.barredView

  /-- The bar only moves onto a view already left behind. -/
  barredViewJustified : s.barredView ≠ s'.barredView → s'.barredView < s.currentView

  /-- Above the decide floor, everything a decide needs is kept. -/
  keepsDecideAboveFloor : ∀ v, s.aboveDecideFloor cfg v → RetainsDecide s s' v

  /-- Above the bar, everything a vote needs is kept. -/
  keepsVoteAboveBar : ∀ v, s'.barredView < v → RetainsVote s s' v

  /--
  The decide floor does not fall.

  It is derived from the decided views, so dropping the newest of them would
  move it *down* and bring abandoned views back into scope. Everything that
  reasons about the floor assumes it only ever rises.
  -/
  floorStable : ∀ v, s'.aboveDecideFloor cfg v → s.aboveDecideFloor cfg v

  /-- A decide, and the record of one, survives as long as it could still happen. -/
  decidedRetained : ∀ v, s.aboveDecideFloor cfg v → s.decidedViews v → s'.decidedViews v

  /-- Collection never invents a decide. -/
  decidedSound : ∀ v, s'.decidedViews v → s.decidedViews v

  /-- A vote or proposal is only forgotten where it can never be repeated. -/
  voted1Retained : ∀ v, s'.barredView < v → s.voted1Views v → s'.voted1Views v

  /--
  Likewise a vote2 mark, but down to the decide floor rather than the bar.

  A vote2 is barred below the floor on its own account
  (`SafetySpec.vote2AboveFloor`), so what this mark protects are the views
  *between* the floor and the bar: views a node has abandoned for proposing and
  for vote1, and may still commit on a late `Cert1`. Keyed this way, an
  implementation needs no bar of its own to conform here — the floor is enough,
  and it is the watermark `vote1BranchesRetained` already uses.
  -/
  voted2Retained : ∀ v, s.aboveDecideFloor cfg v → s.voted2Views v → s'.voted2Views v

  /-- Likewise a proposal mark. -/
  proposedRetained : ∀ v, s'.barredView < v → s.proposedViews v → s'.proposedViews v

  /--
  The branch a vote1 endorsed outlives the vote mark itself.

  Its watermark is the decide floor, not `NodeState.barredView`: the vote2
  vote this bars may arrive long after the view it belongs to was abandoned, so
  forgetting the record at the same point as the vote mark would reopen the very
  gap `Vote1SkippedView` closes.
  -/
  vote1BranchesRetained : ∀ v u, s.aboveDecideFloor cfg v →
    s.vote1Branches v = some u → s'.vote1Branches v = some u

  /-- Collection never invents a vote it did not cast. -/
  voted1Sound : ∀ v, s'.voted1Views v → s.voted1Views v

  /-- Nor a vote2 it did not cast. -/
  voted2Sound : ∀ v, s'.voted2Views v → s.voted2Views v

  /-- Nor a proposal it did not make. -/
  proposedSound : ∀ v, s'.proposedViews v → s.proposedViews v

  /-- Collection never touches the lock. -/
  lockSame : s'.lockedCert = s.lockedCert

  /-- Nor the current view. -/
  currentViewSame : s'.currentView = s.currentView

  /-- Nor the epoch it takes itself to be in. -/
  currentEpochSame : s'.currentEpoch = s.currentEpoch

  /-- Nor the timeout bar. -/
  timeoutViewSame : s'.timeoutView = s.timeoutView

end NewProtocol
