module

public import NewProtocolSpec.Lists
public import NewProtocolImpl.Decide

/-!
# Checking a recorded trace against the signing rules

The rules of `NewProtocol.SafeHistory` quantify over what a history received and
sent, so on a recorded trace, a finite list of steps, they can be decided. This
module states each rule in a bounded form that Lean can decide, and proves that a
trace passing every one obeys `SafeHistory` (`checkSafe_sound`).

One premise cannot be checked on a trace: that a block a node voted for is valid.
Consensus does not interpret blocks, so the check asks instead that the node was
told the block is valid before voting, and the soundness theorem takes that such
reports are truthful (`ValidityTruthful`).

This checks a trace against the specification directly, independently of any
machine. A trace can disagree with the reference machine and still pass, when the
two make different permitted choices.
-/

@[expose] public section

namespace NewProtocolDiff

open NewProtocol NewProtocol.Lists History

/-! ## The rules in bounded form -/

variable (cfg : Config) (node : PubKey) (h : History)

/-- `SafeHistory.vote1Justified`, with the validity report in place of validity. -/
def CheckVote1 : Prop :=
  ∀ i, (hi : i < h.length) → ∀ v ∈ vote1sOf h[i], v.signer = node ∧
    ((∃ st ∈ h.take (i + 1), ∃ x ∈ proposalsIn st, ProposalWellFormed cfg x.2.1 ∧ SafeParent x.2.1
        ∧ NewProtocolImpl.OpensB cfg (h.take (i + 1)) x.2.1 ∧ Vote1For v x.2.1
        ∧ ∃ st' ∈ h.take (i + 1), st'.input = .blockValidated x.2.1.viewNumber (blockHash x.2.1))
      ∨ ∃ x ∈ NewProtocolImpl.receivedRevotes (h.take (i + 1)), RevoteWellFormed cfg x.2
        ∧ SafeRevote x.2 ∧ Vote1Again v x.2)

/-- `SafeHistory.vote1Once`. -/
def CheckVote1Once : Prop :=
  ∀ st ∈ h, ∀ st' ∈ h, ∀ v ∈ vote1sOf st, ∀ v' ∈ vote1sOf st',
    v.data.epoch = v'.data.epoch → v.view = v'.view → v = v'

/-- `SafeHistory.vote2Justified`. -/
def CheckVote2 : Prop :=
  ∀ i, (hi : i < h.length) → ∀ v ∈ vote2sOf h[i], v.signer = node ∧ cfg.anchorView < v.view
    ∧ ∃ c ∈ cert1sHeld cfg (h.take (i + 1)), ∃ b ∈ proposalsHeld cfg (h.take (i + 1)), Certifies c b
      ∧ History.HasPayload cfg (h.take (i + 1)) b.viewNumber b.payloadCommit
      ∧ v.view = c.view ∧ v.data = c.data.toVote2

/-- `SafeHistory.vote2Once`. -/
def CheckVote2Once : Prop :=
  ∀ st ∈ h, ∀ st' ∈ h, ∀ v ∈ vote2sOf st, ∀ v' ∈ vote2sOf st',
    v.data.epoch = v'.data.epoch → v.view = v'.view → v = v'

/-- `SafeHistory.vote2BeforeTimeout`. -/
def CheckVote2BeforeTimeout : Prop :=
  ∀ i, (hi : i < h.length) → ∀ v ∈ vote2sOf h[i], ∀ st ∈ h.take (i + 1), ∀ tv ∈ timeoutVotesOf st,
    ¬ v.view ≤ tv.view

/-- `SafeHistory.timeoutLock`. -/
def CheckTimeoutLock : Prop :=
  ∀ i, (hi : i < h.length) → ∀ tv ∈ timeoutVotesOf h[i], ∀ st ∈ h.take i, ∀ v2 ∈ vote2sOf st,
    v2.data.epoch < tv.data.lock.data.epoch
      ∨ (v2.data.epoch = tv.data.lock.data.epoch ∧ v2.view ≤ tv.data.lock.view)

/-- `SafeHistory.decideJustified`. -/
def CheckDecide : Prop :=
  ∀ i, (hi : i < h.length) → ∀ d ∈ decidesOf h[i],
    (∃ head ∈ d.1.take 1, d.2.2 ∈ cert2sHeld (h.take (i + 1)) ∧ Commits d.2.2 head
      ∧ d.2.1 ∈ cert1sHeld cfg (h.take (i + 1)) ∧ Certifies d.2.1 head)
      ∧ ChainLinked d.1
      ∧ ∀ b ∈ d.1, b ∈ proposalsHeld cfg (h.take (i + 1)) ∧ cfg.anchorView < b.viewNumber

/-- The blocks an output delivers, if it is a decide. -/
def decidedBlocks : Output → List (List Block)
  | .decided blocks _ _ => [blocks]
  | _ => []

/-- `SafeHistory.decideOnce`. -/
def CheckDecideOnce : Prop :=
  ∀ i, (hi : i < h.length) → ∀ j, (hj : j < h[i].output.length) → ∀ blocks ∈ decidedBlocks h[i].output[j],
    (blocks.map (·.viewNumber)).Nodup
      ∧ ∀ b ∈ blocks, b.viewNumber ∉
        NewProtocolImpl.decidedViews (h.take i ++ [Step.mk h[i].input (h[i].output.take j)])

instance : Decidable (CheckVote1 cfg node h) := by unfold CheckVote1; infer_instance
instance : Decidable (CheckVote1Once h) := by unfold CheckVote1Once; infer_instance
instance : Decidable (CheckVote2 cfg node h) := by unfold CheckVote2; infer_instance
instance : Decidable (CheckVote2Once h) := by unfold CheckVote2Once; infer_instance
instance : Decidable (CheckVote2BeforeTimeout h) := by unfold CheckVote2BeforeTimeout; infer_instance
instance : Decidable (CheckTimeoutLock h) := by unfold CheckTimeoutLock; infer_instance
-- The decide rule's conjuncts outgrow the default instance size.
set_option synthInstance.maxSize 1024 in
instance : Decidable (CheckDecide cfg h) := by unfold CheckDecide; infer_instance
instance : Decidable (CheckDecideOnce h) := by unfold CheckDecideOnce; infer_instance

/-- Each signing rule, by name, and whether the trace obeys it. -/
def checkRules : List (String × Bool) :=
  [ ("vote1Justified", decide (CheckVote1 cfg node h)),
    ("vote1Once", decide (CheckVote1Once h)),
    ("vote2Justified", decide (CheckVote2 cfg node h)),
    ("vote2Once", decide (CheckVote2Once h)),
    ("vote2BeforeTimeout", decide (CheckVote2BeforeTimeout h)),
    ("timeoutLock", decide (CheckTimeoutLock h)),
    ("decideJustified", decide (CheckDecide cfg h)),
    ("decideOnce", decide (CheckDecideOnce h)) ]

/-- The trace obeys every signing rule. -/
def checkSafe : Bool := (checkRules cfg node h).all (·.2)

/-! ## Soundness -/

variable {cfg node h}

/--
**A trace that passes the check obeys the signing rules**, given that the validity
reports it received were truthful.
-/
theorem checkSafe_sound (hc : checkSafe cfg node h = true) (hvalid : ValidityTruthful h)
    (P : EpochNumber → Prop) : SafeHistory cfg node P h := by
  simp only [checkSafe, checkRules, List.all_cons, List.all_nil, Bool.and_true,
    Bool.and_eq_true, decide_eq_true_eq] at hc
  obtain ⟨h1, h1o, h2, h2o, hbt, hlock, hdec, honce⟩ := hc
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rintro n vote ⟨st, hst, hmem⟩ _
    obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hst
    obtain ⟨hsig, ⟨st', hst', x, hx, hwf, hsafe, hop, hfor, st'', hst'', hval⟩ | ⟨x, hx, hw, hs, hag⟩⟩ :=
      h1 n hn vote (mem_vote1sOf.mpr hmem)
    · have hin := mem_proposalsIn hx
      refine ⟨hsig, Or.inl ⟨x.1, x.2.1, x.2.2, ⟨st', hst', hin⟩, hwf, ?_, hsafe,
        NewProtocolImpl.opensB_iff.mp hop, hfor⟩⟩
      exact hvalid _ _ ⟨st'', List.mem_of_mem_take hst'', hval⟩ x.2.1 rfl
    · exact ⟨hsig, Or.inr ⟨x.1, x.2, NewProtocolImpl.mem_receivedRevotes.mp hx, hw, hs, hag⟩⟩
  · intro v v' hs hs' _ he hv
    obtain ⟨st, hst, hm⟩ := hs
    obtain ⟨st', hst', hm'⟩ := hs'
    exact h1o st hst st' hst' v (mem_vote1sOf.mpr hm) v' (mem_vote1sOf.mpr hm') he hv
  · rintro n vote ⟨st, hst, hmem⟩ _
    obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hst
    obtain ⟨hsig, hgen, c, hc, b, hb, hcert, hpay, hview, hdata⟩ :=
      h2 n hn vote (mem_vote2sOf.mpr hmem)
    exact ⟨hsig, hgen, c, b, hasCert1_of_mem hc, hasProposal_of_mem hb, hcert, hpay, hview, hdata⟩
  · intro v v' hs hs' _ he hv
    obtain ⟨st, hst, hm⟩ := hs
    obtain ⟨st', hst', hm'⟩ := hs'
    exact h2o st hst st' hst' v (mem_vote2sOf.mpr hm) v' (mem_vote2sOf.mpr hm') he hv
  · rintro n vote ⟨st, hst, hmem⟩ _ tv ⟨st', hst', htv⟩ _
    obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hst
    exact Nat.lt_of_not_le fun hle =>
      hbt n hn vote (mem_vote2sOf.mpr hmem) st' hst' tv (mem_timeoutVotesOf htv) hle
  · rintro n vote ⟨st, hst, hmem⟩ _ v2 ⟨st', hst', hv2⟩ _
    obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hst
    exact hlock n hn vote (mem_timeoutVotesOf hmem) st' hst' v2 (mem_vote2sOf.mpr hv2)
  · intro n st blocks c1 c2 hst hmem _
    obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hst
    obtain ⟨⟨head, hhead, hc2, hcommit, hc1, hcert⟩, hlinked, hall⟩ :=
      hdec n hn (blocks, c1, c2) (mem_decidesOf.mpr hmem)
    obtain ⟨rest, rfl⟩ : ∃ rest, blocks = head :: rest := by
      cases blocks with
      | nil => simp at hhead
      | cons b rest => simp at hhead; subst hhead; exact ⟨rest, rfl⟩
    exact ⟨head, rest, rfl, hasCert2_of_mem hc2, hcommit, mem_cert1sHeld.mp hc1, hcert, hlinked,
      fun b hb => ⟨hasProposal_of_mem (hall b hb).1, (hall b hb).2⟩⟩

  · intro n st j blocks c1 c2 hst hj _
    obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hst
    obtain ⟨hjl, hje⟩ := List.getElem?_eq_some_iff.mp hj
    obtain ⟨hnd, hall⟩ := honce n hn j hjl blocks (by rw [hje]; exact List.mem_singleton_self _)
    exact ⟨hnd, fun b hb hd => hall b hb (NewProtocolImpl.mem_decidedViews.mpr hd)⟩

end NewProtocolDiff
