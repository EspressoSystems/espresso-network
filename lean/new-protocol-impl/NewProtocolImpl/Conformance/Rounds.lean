module

public import NewProtocolImpl.Conformance.Handle

/-!
# The reaction rounds

What the five rounds of `Impl.rounds` do to the state, in two parts.

* `Impl.Frame` — everything they leave alone: all the content, and the two
  bars. Content is the input arms' business (`NewProtocolImpl.Conformance.Handle`),
  so a justification checked against the state a round starts from still holds
  at the end of the step, and every provenance and retention obligation follows
  from the arms alone.
* `Impl.Le` — what they may only grow: the marks, the cursors and the lock's
  view. This is what the retention obligations for the marks need, and it is
  closed under `seq`, which is how a whole pass inherits it.

The initial state's invariant is here too, since it is the same kind of fact:
what the machine starts with.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

/-! ## Frames -/

/-- What a round leaves alone: every field the input arms write, and both bars. -/
structure Frame (s t : State) : Prop where
  proposals : t.proposals = s.proposals
  admitted : t.admitted = s.admitted
  vidShares : t.vidShares = s.vidShares
  validated : t.validated = s.validated
  blocksReconstructed : t.blocksReconstructed = s.blocksReconstructed
  headers : t.headers = s.headers
  cert1s : t.cert1s = s.cert1s
  cert2s : t.cert2s = s.cert2s
  timeoutCerts : t.timeoutCerts = s.timeoutCerts
  barredView : t.barredView = s.barredView
  timeoutView : t.timeoutView = s.timeoutView

namespace Frame

protected theorem refl (s : State) : Frame s s :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

protected theorem trans {s t u : State} (h₁ : Frame s t) (h₂ : Frame t u) : Frame s u where
  proposals := h₂.proposals.trans h₁.proposals
  admitted := h₂.admitted.trans h₁.admitted
  vidShares := h₂.vidShares.trans h₁.vidShares
  validated := h₂.validated.trans h₁.validated
  blocksReconstructed := h₂.blocksReconstructed.trans h₁.blocksReconstructed
  headers := h₂.headers.trans h₁.headers
  cert1s := h₂.cert1s.trans h₁.cert1s
  cert2s := h₂.cert2s.trans h₁.cert2s
  timeoutCerts := h₂.timeoutCerts.trans h₁.timeoutCerts
  barredView := h₂.barredView.trans h₁.barredView
  timeoutView := h₂.timeoutView.trans h₁.timeoutView

/-- A framed state has the same floor, since the decided views are all the floor reads. -/
theorem lockable {s t : State} (h : Frame s t) (hd : t.blocksReconstructed = s.blocksReconstructed)
    (v : ViewNumber) : t.lockable v = s.lockable v := by
  unfold State.lockable State.reconstructed
  rw [h.cert1s, h.admitted, hd]

end Frame

/-! ## Growth -/

/-- What a round may only grow: the marks, the cursors and the lock's view. -/
structure Le (s t : State) : Prop where
  decided : ∀ v, v ∈ s.decidedViews → v ∈ t.decidedViews
  voted1 : ∀ v, v ∈ s.voted1Views → v ∈ t.voted1Views
  voted2 : ∀ v, v ∈ s.voted2Views → v ∈ t.voted2Views
  proposed : ∀ v, v ∈ s.proposedViews → v ∈ t.proposedViews
  branches : ∀ v u, s.vote1Branches.get? v = some u → t.vote1Branches.get? v = some u
  currentView : s.currentView ≤ t.currentView
  timeoutView : s.timeoutView ≤ t.timeoutView
  lock : ∀ l, s.lockedCert = some l → ∃ l', t.lockedCert = some l' ∧ l.view ≤ l'.view

namespace Le

protected theorem refl (s : State) : Le s s where
  decided _ h := h
  voted1 _ h := h
  voted2 _ h := h
  proposed _ h := h
  branches _ _ h := h
  currentView := Nat.le_refl _
  timeoutView := Nat.le_refl _
  lock l h := ⟨l, h, Nat.le_refl _⟩

protected theorem trans {s t u : State} (h₁ : Le s t) (h₂ : Le t u) : Le s u where
  decided v h := h₂.decided v (h₁.decided v h)
  voted1 v h := h₂.voted1 v (h₁.voted1 v h)
  voted2 v h := h₂.voted2 v (h₁.voted2 v h)
  proposed v h := h₂.proposed v (h₁.proposed v h)
  branches v u h := h₂.branches v u (h₁.branches v u h)
  currentView := Nat.le_trans h₁.currentView h₂.currentView
  timeoutView := Nat.le_trans h₁.timeoutView h₂.timeoutView
  lock l h :=
    have ⟨l', ht, hle⟩ := h₁.lock l h
    have ⟨l'', hu, hle'⟩ := h₂.lock l' ht
    ⟨l'', hu, Nat.le_trans hle hle'⟩

end Le

/-- The decide fold only grows the decided set. -/
theorem mem_decideFold {l : List Block} {d : TreeSet ViewNumber} {v : ViewNumber}
    (h : v ∈ d) : v ∈ l.foldl (fun d b => d.insert b.viewNumber) d := by
  induction l generalizing d with
  | nil => exact h
  | cons b l ih => exact ih (mem_insert_of_mem h)

/-! ## The invariant travels with a frame

Only the keyed maps and the decided set matter to `Impl.WF`, so a round
preserves it as soon as it frames the maps and does not shrink the decided set.
-/

theorem WF.of_frame {s t : State} (h : WF cfg s) (hf : Frame s t) (hle : Le s t)
    (hb : ∀ v u, t.vote1Branches.get? v = some u →
      s.vote1Branches.get? v = some u ∨ v ∈ t.voted1Views) : WF cfg t where
  proposals := hf.proposals ▸ h.proposals
  proposalsWellFormed := hf.proposals ▸ h.proposalsWellFormed
  admitted := hf.proposals ▸ hf.admitted ▸ h.admitted
  cert1s := hf.cert1s ▸ h.cert1s
  cert2s := hf.cert2s ▸ h.cert2s
  timeoutCerts := hf.timeoutCerts ▸ h.timeoutCerts
  validated := hf.validated ▸ h.validated
  decided :=
    Std.TreeSet.isSome_max?_of_mem (hle.decided _ (lastDecided_mem h.decided))
  branches v u hv := by
    rcases hb v u hv with hold | hnew
    · rcases h.branches v u hold with hvoted | hbar
      · exact Or.inl (hle.voted1 v hvoted)
      · exact Or.inr (hf.barredView ▸ hbar)
    · exact Or.inl hnew

/-! ## The rounds, one by one -/

section Rounds

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber} (s : State)

theorem tryDecide_frame : Frame s (tryDecide cfg settled v s).1 := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

theorem tryDecide_le : Le s (tryDecide cfg settled v s).1 := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals
    first
    | exact Le.refl _
    | exact ⟨fun _ h => mem_decideFold h, fun _ h => h, fun _ h => h, fun _ h => h,
        fun _ _ h => h, Nat.le_refl _, Nat.le_refl _, fun l h => ⟨l, h, Nat.le_refl _⟩⟩

theorem advanceLock_frame : Frame s (advanceLock s).1 := by
  unfold advanceLock
  repeat' (first | split | dsimp only)
  all_goals exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

theorem advanceLock_le : Le s (advanceLock s).1 := by
  unfold advanceLock
  split
  case h_2 => exact Le.refl _
  rename_i c hc
  split
  case isFalse => exact Le.refl _
  case isTrue hb =>
    refine ⟨fun _ h => h, fun _ h => h, fun _ h => h, fun _ h => h, fun _ _ h => h,
      ViewNumber.le_max_left .., Nat.le_refl _, fun l hl => ⟨c, rfl, ?_⟩⟩
    unfold State.lockBelow at hb
    rw [hl] at hb
    exact Nat.le_of_lt (of_decide_eq_true hb)

theorem tryVote1_frame : Frame s (tryVote1 node v s).1 := by
  unfold tryVote1
  repeat' (first | split | dsimp only)
  all_goals exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

/--
The vote1 round grows the state.

The one field that is not a plain insertion into a growing set is the branch
record, which must not overwrite one already there. `WF.branches` is what rules
that out: a record at this view would mean a vote cast here, or a view
abandoned, and the round's own guard excludes both.
-/
theorem tryVote1_le (hwf : WF cfg s) : Le s (tryVote1 node v s).1 := by
  unfold tryVote1
  split
  case isTrue => exact Le.refl _
  case isFalse hguard =>
    simp only [not_or] at hguard
    obtain ⟨hto, hbar, hvoted⟩ := hguard
    have hfree : s.vote1Branches.get? v = none := by
      cases hb : s.vote1Branches.get? v with
      | none => rfl
      | some u =>
        rcases hwf.branches v u hb with hv | hle
        · exact absurd (contains_iff_mem.mpr hv) (by simpa using hvoted)
        · exact absurd hle (by simpa using hbar)
    repeat' (first | split | dsimp only)
    all_goals
      first
      | exact Le.refl _
      | exact ⟨fun _ h => h, fun _ h => mem_insert_of_mem h, fun _ h => h, fun _ h => h,
          fun _ _ h => get?_insert_of_writable (Or.inl hfree) h,
          Nat.le_refl _, Nat.le_refl _, fun l h => ⟨l, h, Nat.le_refl _⟩⟩

theorem tryVote2_frame : Frame s (tryVote2 cfg node v s).1 := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

theorem tryVote2_le : Le s (tryVote2 cfg node v s).1 := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals
    first
    | exact Le.refl _
    | exact ⟨fun _ h => h, fun _ h => h, fun _ h => mem_insert_of_mem h, fun _ h => h,
        fun _ _ h => h, Nat.le_refl _, Nat.le_refl _, fun l h => ⟨l, h, Nat.le_refl _⟩⟩

theorem tryPropose_frame : Frame s (tryPropose cfg leader node v s).1 := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

theorem tryPropose_le : Le s (tryPropose cfg leader node v s).1 := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals
    first
    | exact Le.refl _
    | exact ⟨fun _ h => h, fun _ h => h, fun _ h => h, fun _ h => mem_insert_of_mem h,
        fun _ _ h => h, Nat.le_refl _, Nat.le_refl _, fun l h => ⟨l, h, Nat.le_refl _⟩⟩

/-! ### The branch record

The one mark that is a map rather than a set, so retention is a lookup rather
than a membership. Four of the five rounds leave it alone; the vote1
writes exactly where it marks the vote.
-/

theorem tryDecide_branches :
    (tryDecide cfg settled v s).1.vote1Branches = s.vote1Branches := by
  unfold tryDecide
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem advanceLock_branches : (advanceLock s).1.vote1Branches = s.vote1Branches := by
  unfold advanceLock
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote2_branches :
    (tryVote2 cfg node v s).1.vote1Branches = s.vote1Branches := by
  unfold tryVote2
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryPropose_branches :
    (tryPropose cfg leader node v s).1.vote1Branches = s.vote1Branches := by
  unfold tryPropose
  repeat' (first | split | dsimp only)
  all_goals rfl

theorem tryVote1_branches {w : ViewNumber} {u : ViewNumber}
    (h : (tryVote1 node v s).1.vote1Branches.get? w = some u) :
    s.vote1Branches.get? w = some u ∨ w ∈ (tryVote1 node v s).1.voted1Views := by
  unfold tryVote1 at h ⊢
  by_cases hg : v ≤ s.timeoutView ∨ v ≤ s.barredView ∨ s.voted1Views.contains v = true
  · rw [if_pos hg] at h ⊢; exact Or.inl h
  · rw [if_neg hg] at h ⊢
    cases hp : s.admitted.get? v with
    | none => simp only [hp] at h ⊢; exact Or.inl h
    | some p =>
      cases hs : s.vidShares.get? v with
      | none => simp only [hp, hs] at h ⊢; exact Or.inl h
      | some share =>
        simp only [hp, hs] at h ⊢
        by_cases hj : s.validated.get? v = some (blockHash p) ∧ s.parentLinked p = true
            ∧ safeToExtend s.lockedCert p = true
        · rw [if_pos hj] at h ⊢
          rcases get?_insert_cases h with ⟨rfl, rfl⟩ | h
          · exact Or.inr mem_insert_self
          · exact Or.inl h
        · rw [if_neg hj] at h ⊢; exact Or.inl h

end Rounds

/-! ## Sequencing -/

/--
A round: it frames the content, grows the marks, and keeps the invariant.

The invariant travels along because every round needs it — the vote1 to
know its record slot is free, and the others to read the floor.
-/
def Grows (cfg : Config) (f : StepFn) : Prop :=
  ∀ s, WF cfg s → Frame s (f s).1 ∧ Le s (f s).1 ∧ WF cfg (f s).1

section Instances

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
variable {settled : TreeSet ViewNumber} {v : ViewNumber}

theorem tryDecide_grows : Grows cfg (tryDecide cfg settled v) := fun s hwf =>
  ⟨tryDecide_frame s, tryDecide_le s,
   WF.of_frame hwf (tryDecide_frame s) (tryDecide_le s)
     (fun _ _ h => Or.inl (tryDecide_branches s ▸ h))⟩

theorem advanceLock_grows : Grows cfg advanceLock := fun s hwf =>
  ⟨advanceLock_frame s, advanceLock_le s,
   WF.of_frame hwf (advanceLock_frame s) (advanceLock_le s)
     (fun _ _ h => Or.inl (advanceLock_branches s ▸ h))⟩

theorem tryVote1_grows : Grows cfg (tryVote1 node v) := fun s hwf =>
  ⟨tryVote1_frame s, tryVote1_le s hwf,
   WF.of_frame hwf (tryVote1_frame s) (tryVote1_le s hwf) (fun _ _ h => tryVote1_branches s h)⟩

theorem tryVote2_grows : Grows cfg (tryVote2 cfg node v) := fun s hwf =>
  ⟨tryVote2_frame s, tryVote2_le s,
   WF.of_frame hwf (tryVote2_frame s) (tryVote2_le s)
     (fun _ _ h => Or.inl (tryVote2_branches s ▸ h))⟩

theorem tryPropose_grows : Grows cfg (tryPropose cfg leader node v) := fun s hwf =>
  ⟨tryPropose_frame s, tryPropose_le s,
   WF.of_frame hwf (tryPropose_frame s) (tryPropose_le s)
     (fun _ _ h => Or.inl (tryPropose_branches s ▸ h))⟩

end Instances

theorem seq_grows {fs : List StepFn} (h : ∀ f ∈ fs, Grows cfg f) : Grows cfg (seq fs) := by
  induction fs with
  | nil => exact fun s hwf => ⟨Frame.refl s, Le.refl s, hwf⟩
  | cons f fs ih =>
    intro s hwf
    have hf := h f (List.mem_cons_self ..) s hwf
    have hrest := ih (fun g hg => h g (List.mem_cons_of_mem _ hg)) (f s).1 hf.2.2
    exact ⟨hf.1.trans hrest.1, hf.2.1.trans hrest.2.1, hrest.2.2⟩

/-- Every round of a reaction pass frames the content and grows the marks. -/
theorem rounds_grows (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)
    (t : State) : Grows cfg (seq (rounds cfg leader node t)) := by
  refine seq_grows fun f hf => ?_
  simp only [rounds, List.mem_append, List.mem_map, List.mem_singleton] at hf
  rcases hf with (((⟨v, -, rfl⟩ | rfl) | ⟨v, -, rfl⟩) | ⟨v, -, rfl⟩) | ⟨k, -, rfl⟩
  · exact tryDecide_grows
  · exact advanceLock_grows
  · exact tryVote1_grows
  · exact tryVote2_grows
  · exact tryPropose_grows

/-- The whole step keeps the invariant: `NextPreservesWF`. -/
theorem next_preservesWF (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey) :
    NextPreservesWF cfg leader node := by
  intro s input henv hwf
  exact (rounds_grows cfg leader node _ _ (ingest_wf henv hwf)).2.2

/-! ## The initial state -/

/--
The initial state satisfies the invariant.

Only the anchor is present, and `ConfigCoherent` places it at genesis —
where `WF.proposalsWellFormed` does not ask for well-formedness, the anchor
having no parent to point back to.
-/
theorem initial_wf (cfg : Config) (h : ConfigCoherent cfg) : WF cfg (initial cfg) where
  proposals v p hv := by
    rw [initial, get?_insert] at hv
    split at hv
    · rename_i he
      cases hv
      exact h.anchorBlockView.trans he.symm
    · exact nomatch hv
  proposalsWellFormed v p hv hne := by
    rw [initial, get?_insert] at hv
    split at hv
    · exact absurd ‹v = ViewNumber.genesis› hne
    · exact nomatch hv
  admitted v p hv := by
    rw [initial] at hv
    exact nomatch hv
  cert1s v c hv := by
    rw [initial, get?_insert] at hv
    split at hv
    · rename_i he
      cases hv
      exact h.anchorCertView.trans he.symm
    · exact nomatch hv
  cert2s v c hv := by
    rw [initial] at hv
    exact nomatch hv
  timeoutCerts v tc hv := by
    rw [initial] at hv
    exact nomatch hv
  validated v p hv := by
    rw [initial] at hv
    exact nomatch hv
  decided := Std.TreeSet.isSome_max?_of_mem (show ViewNumber.genesis ∈ _ from mem_insert_self)
  branches v u hv := by
    rw [initial] at hv
    exact nomatch hv

end Impl
end NewProtocol
