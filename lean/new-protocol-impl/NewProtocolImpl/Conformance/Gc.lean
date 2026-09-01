module

public import NewProtocolImpl.Conformance

/-!
# Collection conforms

`GcConforms` and `GcPreservesWF`: the machine's pruning satisfies `GcSpec`
and keeps the representation invariant.

Collection only ever filters, so every fact `GcSpec` asks about what *survives*
is a fact about the filter's predicate, and every fact about what it may
*contain* is immediate. Two of the fields need more than that:

* `floorStable` — the floor is read off the highest decided view, so it stays
  put only because the cut on `decidedViews` is inclusive and that view is
  therefore kept. This is the reason `Impl.State.gc` cuts `decidedViews` at
  `floor ≤ v` where everything else on the decide path is cut at `floor < v`.
* the invariant's `decided` field — the same view is what keeps the decided set
  non-empty, and with it the machine's floor equal to the specification's.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {s : State}

/-! ## The two watermarks -/

/-- Where the bar lands. -/
def gcBar (s : State) : ViewNumber := max s.barredView (s.currentView - 1)

theorem gc_barredView : (s.gc cfg).barredView = gcBar s := rfl

theorem le_gcBar : s.barredView ≤ gcBar s := ViewNumber.le_max_left ..

/--
The bar only enters territory the node has left: the view it is in bounds it.

The bar it moves to is either the one it had — nothing to prove — or one below
the view the node is in, and moving at all means the node is past genesis.
-/
theorem gcBar_lt_currentView (h : gcBar s ≠ s.barredView) : gcBar s < s.currentView := by
  have h' : (gcBar s).toNat ≠ s.barredView.toNat := fun he => h (ViewNumber.ext he)
  have hm : (gcBar s).toNat = max s.barredView.toNat (s.currentView.toNat - 1) := by
    simp [gcBar]
  rw [ViewNumber.lt_def]
  omega

/-- The floor never rises above the highest decided view. -/
theorem floor_le_lastDecided : s.floor cfg ≤ s.lastDecided := Nat.sub_le ..

/-- The highest decided view survives collection. -/
theorem lastDecided_mem_gc (hwf : WF cfg s) : s.lastDecided ∈ (s.gc cfg).decidedViews := by
  show s.lastDecided ∈ s.decidedViews.filter fun v => decide (s.floor cfg ≤ v)
  exact mem_filter.mpr ⟨lastDecided_mem hwf.decided, decide_eq_true floor_le_lastDecided⟩

/-! ## The invariant -/

/-- Collection preserves the invariant. -/
theorem gc_wf (cfg : Config) (s : State) (hwf : WF cfg s) : WF cfg (s.gc cfg) where
  proposals v p h := hwf.proposals v p (get?_filter.mp h).1
  proposalsWellFormed v p h := hwf.proposalsWellFormed v p (get?_filter.mp h).1
  admitted v p h := by
    obtain ⟨hadm, hbar⟩ := get?_filter.mp h
    refine get?_filter.mpr ⟨hwf.admitted v p hadm, decide_eq_true ?_⟩
    exact Nat.lt_of_le_of_lt (ViewNumber.min_le_right ..) (of_decide_eq_true hbar)
  cert1s v c h := hwf.cert1s v c (get?_filter.mp h).1
  cert2s v c h := hwf.cert2s v c (get?_filter.mp h).1
  timeoutCerts v tc h := hwf.timeoutCerts v tc (get?_filter.mp h).1
  validated v p h := hwf.validated v p (get?_filter.mp h).1
  decided := Std.TreeSet.isSome_max?_of_mem (lastDecided_mem_gc hwf)
  branches v u h := by
    -- The vote mark survives above the new bar; at or below it, the second
    -- disjunct is what the bar itself gives.
    obtain ⟨hbr, -⟩ := get?_filter.mp h
    by_cases hbar : gcBar s < v
    · rcases hwf.branches v u hbr with hv | hle
      · exact Or.inl (mem_filter.mpr ⟨hv, decide_eq_true hbar⟩)
      · exact Or.inr (Nat.le_trans hle le_gcBar)
    · exact Or.inr (Nat.ge_of_not_lt hbar)

/-- The machine meets `GcPreservesWF`. -/
theorem gc_preservesWF (cfg : Config) : GcPreservesWF cfg := fun s hwf => gc_wf cfg s hwf

/-! ## The floor

Both directions of the floor's behaviour under collection, in the machine's
terms. The abstraction turns `aboveDecideFloor` into the pointwise condition of
`Impl.aboveFloor_iff`, so everything below is stated over the decided sets.
-/

/-- A view above the post-collection floor was above the pre-collection floor. -/
theorem gc_floorStable (hwf : WF cfg s) (v : ViewNumber)
    (h : (s.gc cfg).abstract.aboveDecideFloor cfg v) : s.abstract.aboveDecideFloor cfg v := by
  have hlast : s.lastDecided - cfg.decideBuffer < v := h s.lastDecided (lastDecided_mem_gc hwf)
  intro w hw
  exact Nat.lt_of_le_of_lt (sub_le_sub _ (le_lastDecided hw)) hlast

/-- The floor as the filters see it: `aboveDecideFloor` is `floor < v`. -/
theorem floor_lt_of_above (hwf : WF cfg s) {v : ViewNumber}
    (h : s.abstract.aboveDecideFloor cfg v) : s.floor cfg < v :=
  of_decide_eq_true ((aboveFloor_abstract hwf v).mpr h)

/-! ## The pruning rule -/

theorem gc_shrinks (s : State) : Shrinks s.abstract (s.gc cfg).abstract where
  proposals _ _ h := (get?_filter.mp h).1
  admitted _ _ h := (get?_filter.mp h).1
  vidShares _ _ h := (get?_filter.mp h).1
  validated _ _ h := (get?_filter.mp h).1
  blocksReconstructed _ _ h := (mem_filter.mp h).1
  headers _ _ _ h := (get?_filter.mp h).1
  cert1s _ _ h := (get?_filter.mp h).1
  cert2s _ _ h := (get?_filter.mp h).1
  timeoutCerts _ _ h := (get?_filter.mp h).1
  vote1Branches _ _ h := (get?_filter.mp h).1

/-- The machine meets `GcConforms`. -/
theorem gc_conforms (cfg : Config) : GcConforms cfg := by
  intro s hwf
  refine
    { shrinks := gc_shrinks s
      barredViewMono := le_gcBar
      barredViewJustified := ?_
      keepsDecideAboveFloor := ?_
      keepsVoteAboveBar := ?_
      floorStable := ?_
      decidedRetained := ?_
      decidedSound := ?_
      voted1Retained := ?_
      voted2Retained := ?_
      proposedRetained := ?_
      vote1BranchesRetained := ?_
      voted1Sound := ?_
      voted2Sound := ?_
      proposedSound := ?_
      lockSame := rfl
      currentViewSame := rfl
      timeoutViewSame := rfl }
  -- `barredViewJustified`
  · exact fun h => gcBar_lt_currentView (Ne.symm h)
  -- `keepsDecideAboveFloor`
  · intro v hv
    have hfl : s.floor cfg < v := floor_lt_of_above hwf hv
    exact
      { proposals := fun p hp => get?_filter.mpr ⟨hp, decide_eq_true
          (Nat.lt_of_le_of_lt (ViewNumber.min_le_left ..) hfl)⟩
        blocksReconstructed := fun pc hpc => mem_filter.mpr ⟨hpc, decide_eq_true hfl⟩
        cert1s := fun c hc => get?_filter.mpr ⟨hc, decide_eq_true hfl⟩
        cert2s := fun c hc => get?_filter.mpr ⟨hc, decide_eq_true hfl⟩ }
  -- `keepsVoteAboveBar`
  · intro v hv
    exact
      { admitted := fun p hp => get?_filter.mpr ⟨hp, decide_eq_true hv⟩
        vidShares := fun sh hsh => get?_filter.mpr ⟨hsh, decide_eq_true hv⟩
        validated := fun hb hhb => get?_filter.mpr ⟨hhb, decide_eq_true hv⟩
        headers := fun hb hd hhd => get?_filter.mpr ⟨hhd, decide_eq_true hv⟩
        timeoutCerts := fun tc htc => get?_filter.mpr ⟨htc, decide_eq_true hv⟩ }
  -- `floorStable`
  · exact fun v h => gc_floorStable hwf v h
  -- `decidedRetained`
  · intro v hv hd
    exact mem_filter.mpr ⟨hd, decide_eq_true (Nat.le_of_lt (floor_lt_of_above hwf hv))⟩
  -- `decidedSound`
  · exact fun v h => (mem_filter.mp h).1
  -- `voted1Retained` and `proposedRetained` are kept above the bar;
  -- `voted2Retained` above the floor, as `vote1BranchesRetained` is.
  · exact fun v hv h => mem_filter.mpr ⟨h, decide_eq_true hv⟩
  · exact fun v hv h => mem_filter.mpr ⟨h, decide_eq_true (floor_lt_of_above hwf hv)⟩
  · exact fun v hv h => mem_filter.mpr ⟨h, decide_eq_true hv⟩
  -- `vote1BranchesRetained`
  · exact fun v u hv h => get?_filter.mpr ⟨h, decide_eq_true (floor_lt_of_above hwf hv)⟩
  -- `voted1Sound`, `voted2Sound`, `proposedSound`
  · exact fun v h => (mem_filter.mp h).1
  · exact fun v h => (mem_filter.mp h).1
  · exact fun v h => (mem_filter.mp h).1

end Impl
end NewProtocol
