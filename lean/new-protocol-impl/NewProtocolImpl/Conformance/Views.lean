module

public import NewProtocolImpl.Conformance.Step

/-!
# The view a pass ends in

The last piece the assembly needs: the reaction pass either leaves the view alone
or enters one past a certificate it holds, which is what
`StepSpec.currentViewJustified` asks of the half of the step the pass owns. Only
`Impl.advanceLock` moves the view, and it moves it to one past the certificate
it locked on.

Also here: the congruence that lets a parent link checked at one state be read at
another that holds the same proposals and payloads.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber} {t : State}

/-! ## Which rounds leave the view alone -/

theorem tryDecide_currentView (s : State) :
    (tryDecide cfg settled v s).1.currentView = s.currentView := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote1_currentView (s : State) :
    (tryVote1 node v s).1.currentView = s.currentView := by
  unfold tryVote1
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote2_currentView (s : State) :
    (tryVote2 cfg node v s).1.currentView = s.currentView := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryPropose_currentView (s : State) :
    (tryPropose leader node v s).1.currentView = s.currentView := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals rfl

/-! ## The view at the end of a pass -/

theorem st1_currentView : (st1 cfg t).currentView = t.currentView :=
  seq_proj State.currentView (fun f hf t' => by
      obtain ⟨v, -, rfl⟩ :=
        List.mem_map.mp (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hf)
      exact tryDecide_currentView t') t

theorem st5_currentView :
    (st5 cfg leader node t).currentView = (st2 cfg t).currentView := by
  rw [st5, seq_proj State.currentView (fun f hf t' => by
        obtain ⟨k, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (fun k => tryPropose leader node k.1) t.headers.keys from hf)
        exact tryPropose_currentView t'),
    st4, seq_proj State.currentView (fun f hf t' => by
        obtain ⟨w, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (tryVote2 cfg node) t.admitted.keys from hf)
        exact tryVote2_currentView t'),
    st3, seq_proj State.currentView (fun f hf t' => by
        obtain ⟨w, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (tryVote1 node) t.admitted.keys from hf)
        exact tryVote1_currentView t')]

/--
A pass leaves the view where it was, or enters one past a certificate it holds.

The lock advance is the only round that moves the view, and the certificate it
locked on is in the state the step ends with.
-/
theorem pass_currentView (hwf : WF t) :
    (st5 cfg leader node t).currentView = t.currentView
      ∨ ∃ w c, (st5 cfg leader node t).currentView = w + 1
          ∧ (st5 cfg leader node t).cert1s.get? w = some c := by
  rcases advanceLock_cases (r := advanceLock (st1 cfg t)) rfl with heq | ⟨c, hbest, -, heq⟩
  · refine Or.inl ?_
    rw [st5_currentView, st2, heq, st1_currentView]
  · -- the lock advance fired: the view is one past the certificate it locked on
    obtain ⟨w, hl⟩ := bestLock_spec hbest
    obtain ⟨hc1, -⟩ := lockable_spec hl
    have hcw : c.view = w := (st1_stage hwf).2.2.cert1s _ _ hc1
    have hcur : (st5 cfg leader node t).currentView
        = max (st1 cfg t).currentView (c.view + 1) := by
      rw [st5_currentView, st2, heq]
    by_cases hlt : (st1 cfg t).currentView ≤ c.view + 1
    · refine Or.inr ⟨c.view, c, ?_, ?_⟩
      · rw [hcur, ViewNumber.max_eq_right_of_le hlt]
      · rw [hcw, (st5_stage hwf).1.cert1s, ← (st1_stage hwf).1.cert1s]
        exact hc1
    · -- the view was already past it, so nothing moved
      refine Or.inl ?_
      rw [hcur, show max (st1 cfg t).currentView (c.view + 1)
          = (st1 cfg t).currentView from if_neg hlt, st1_currentView]

/-! ## Reading a parent link elsewhere -/

theorem parentLinked_congr {a b : State} (hp : b.proposals = a.proposals)
    (hr : b.blocksReconstructed = a.blocksReconstructed) (p : Proposal) :
    b.parentLinked p = a.parentLinked p := by
  unfold State.parentLinked State.reconstructed
  rw [hp, hr]

end Impl
end NewProtocol
