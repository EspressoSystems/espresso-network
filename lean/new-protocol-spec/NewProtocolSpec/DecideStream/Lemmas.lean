module

public import NewProtocolSpec.DecideStream.Defs

/-!
# Working lemmas for the decide stream invariant

Kernel-checked scaffolding: the invariant holds initially
(`decideInv_initial`), collection preserves it (`decideInv_gc`) and so does a
consensus step (`decideInv_step`). An audit can skip the file.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
A fresh node claims nothing: its only decided view is the anchor's, which sits
at or below the floor.
-/
theorem genesis_not_above {s : NodeState} (h : s.decidedViews ViewNumber.genesis) :
    ¬ s.aboveDecideFloor cfg ViewNumber.genesis := fun hfl => by
  have := hfl _ h
  exact absurd this
    (by show ¬ (ViewNumber.genesis.toNat - cfg.decideBuffer < ViewNumber.genesis.toNat)
        exact Nat.not_lt_zero _)

theorem decideInv_initial : DecideInv cfg (NodeState.initial cfg) where
  held v hfl hd := by
    simp only [NodeState.initial] at hd
    subst hd
    exact absurd hfl (genesis_not_above cfg rfl)

/--
The floor only rises when the decided views only grow, so a view above it
afterwards was above it before.
-/
theorem aboveDecideFloor_of_grows {s s' : NodeState} {v : ViewNumber}
    (hret : ∀ w, s.decidedViews w → s'.decidedViews w)
    (h : s'.aboveDecideFloor cfg v) : s.aboveDecideFloor cfg v :=
  fun w hw => h w (hret w hw)

/-- Collection preserves the invariant: it drops nothing above the floor. -/
theorem decideInv_gc {s s' : NodeState} (h : DecideInv cfg s) (hg : GcSpec cfg s s') :
    DecideInv cfg s' where
  held v hfl hd := by
    have hfl' := GcSpec.floorStable hg _ hfl
    obtain ⟨b, hb⟩ :=
      Option.isSome_iff_exists.mp (DecideInv.held h v hfl' (GcSpec.decidedSound hg v hd))
    exact Option.isSome_of_eq_some (((GcSpec.keepsDecideAboveFloor hg) v hfl').proposals b hb)

/-- A consensus step preserves the invariant. -/
theorem decideInv_step {s s' : NodeState} {input : Input} {output : List Output}
    (h : DecideInv cfg s) (hs : StepSpec cfg leader node s input output s') :
    DecideInv cfg s' := by
  have hmono : ∀ {v}, s'.aboveDecideFloor cfg v → s.aboveDecideFloor cfg v :=
    fun hfl => aboveDecideFloor_of_grows cfg (SafetySpec.decidedRetained (StepSpec.toSafetySpec hs)) hfl
  -- A view decided before the step keeps the block it decided.
  have hkeep : ∀ v, s'.aboveDecideFloor cfg v → s.decidedViews v →
      ∀ b, s.proposals v = some b → s'.proposals v = some b := by
    intro v hfl hd b hb
    exact ((StepSpec.contentRetained hs v (hmono hfl)).decide).proposals b hb
  refine ⟨?_⟩
  intro v hfl hd
  by_cases hold : s.decidedViews v
  · obtain ⟨b, hb⟩ := Option.isSome_iff_exists.mp (DecideInv.held h v (hmono hfl) hold)
    exact Option.isSome_of_eq_some (hkeep v hfl hold b hb)
  · obtain ⟨blocks, c1, c2, bb, hout, hmem, hbv⟩ := StepSpec.decidedMarked hs v hold hd
    obtain ⟨-, -, -, hall⟩ := StepSpec.decideJustified hs blocks c1 c2 hout
    exact Option.isSome_of_eq_some (hbv ▸ (hall bb hmem).2.2.2)

end NewProtocol
