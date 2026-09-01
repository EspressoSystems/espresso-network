module

public import NewProtocolImpl.Conformance.Views

/-!
# Every step satisfies the specification

`NextConforms`: one theorem, sixty fields, and no new ideas — everything it
needs is proved elsewhere. What this file does is route each obligation to its
source and move the facts to the state the obligation talks about.

Three routings, one per group of fields:

* **content** — where it comes from, that an input's is taken, and what may not
  be lost. The input arm settles all of it
  (`NewProtocolImpl.Conformance.Handle`), and the reaction pass frames every
  content field, so the arm's facts *are* the step's.
* **actions** — a vote, proposal or decide in the output. Only the pass emits
  those, so the arm's outputs are dismissed with `Impl.mem_ingestOut` and the
  rest is `Impl.pass_vote1` and friends. The marks run the other way, through
  `NewProtocolImpl.Conformance.Marks`.
* **input-triggered** — the `Cert2` relay, the view advances and the timeout
  vote, all of which are the arm's (`NewProtocolImpl.Conformance.Inputs`).

The abstraction contributes nothing but the decide floor and the branch scan:
every other field of `NodeState` is the same lookup by definition, which is why
almost no step below mentions it.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}

/-! ## Two more readings of the abstraction -/

/-- The machine's branch scan is the specification's quantifier. -/
theorem vote1Skipped_iff {s : State} {v : ViewNumber} :
    s.vote1Skipped v = true ↔ Vote1SkippedView s.abstract v := by
  rw [vote1Skipped_eq_any]
  unfold Vote1SkippedView
  rw [List.any_eq_true]
  constructor
  · intro h
    obtain ⟨⟨w, u⟩, hmem, hcond⟩ := h
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hcond
    exact ⟨w, u, hcond.1, get?_of_mem_toList hmem, hcond.2⟩
  · intro h
    obtain ⟨w, u, hvw, hget, huv⟩ := h
    exact ⟨(w, u), mem_toList_of_get? hget, by simp [hvw, huv]⟩

/-- The floor the decide round is judged against is the floor of the state it began at. -/
theorem floorOf_decidedViews (cfg : Config) (s : State) :
    floorOf cfg s.decidedViews = s.floor cfg := rfl

theorem not_aboveDecideFloor {s : State} (hwf : WF cfg s) {v : ViewNumber}
    (h : v ≤ s.floor cfg) : ¬ s.abstract.aboveDecideFloor cfg v := by
  intro hc
  exact absurd (of_decide_eq_true (aboveFloor_of_abstract hwf hc)) (Nat.not_lt.mpr h)

/-! ## Lifting an output into the step -/

theorem mem_next_of_ingest {s : State} {i : Input} {o : Output}
    (h : o ∈ ingestOut cfg node s i) : o ∈ (next cfg leader node s i).2 := by
  rw [next_out]
  exact List.mem_append.mpr (Or.inl h)

theorem mem_next_of_pass {s : State} {i : Input} {o : Output}
    (h : o ∈ (seq (rounds cfg leader node (ingest cfg node s i))
      (ingest cfg node s i)).2) : o ∈ (next cfg leader node s i).2 := by
  rw [next_out]
  exact List.mem_append.mpr (Or.inr h)

/-! ## The theorem -/

/--
Every transition of the machine from a state satisfying the invariant satisfies
the step specification.
-/
theorem next_conforms (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey) :
    NextConforms cfg leader node := by
  intro s i henv hwf
  show StepSpec cfg leader node s.abstract i (next cfg leader node s i).2
    (next cfg leader node s i).1.abstract
  have hwft : WF cfg (ingest cfg node s i) := ingest_wf henv hwf
  rw [next_state]
  simp only [State.abstract]
  obtain ⟨hfr, hle, hwfu⟩ := st5_stage (cfg := cfg) (leader := leader) (node := node) hwft
  -- Only the reaction pass emits a vote, a proposal or a decide; the arm emits
  -- none of them.
  have hvote1 : ∀ {vt : Vote1}, Output.send (.vote1 vt) ∈ (next cfg leader node s i).2 →
      Output.send (.vote1 vt) ∈ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 := by
    intro vt h
    rcases mem_next_out h with h | h
    · rcases mem_ingestOut h with ⟨c, he, -⟩ | ⟨w, e, he, -, -⟩ | ⟨tc, w, he, -⟩ <;>
        exact absurd he (by simp)
    · exact h
  have hvote2 : ∀ {vt : Vote2}, Output.send (.vote2 vt) ∈ (next cfg leader node s i).2 →
      Output.send (.vote2 vt) ∈ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 := by
    intro vt h
    rcases mem_next_out h with h | h
    · rcases mem_ingestOut h with ⟨c, he, -⟩ | ⟨w, e, he, -, -⟩ | ⟨tc, w, he, -⟩ <;>
        exact absurd he (by simp)
    · exact h
  have hprop : ∀ {p : Proposal}, Output.send (.proposal p) ∈ (next cfg leader node s i).2 →
      Output.send (.proposal p) ∈ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 := by
    intro p h
    rcases mem_next_out h with h | h
    · rcases mem_ingestOut h with ⟨c, he, -⟩ | ⟨w, e, he, -, -⟩ | ⟨tc, w, he, -⟩ <;>
        exact absurd he (by simp)
    · exact h
  have hdec : ∀ {chain : List Block} {c1 : Cert1} {c2 : Cert2},
      Output.decided chain c1 c2 ∈ (next cfg leader node s i).2 →
      Output.decided chain c1 c2 ∈ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 := by
    intro chain c1 c2 h
    rcases mem_next_out h with h | h
    · rcases mem_ingestOut h with ⟨c, he, -⟩ | ⟨w, e, he, -, -⟩ | ⟨tc, w, he, -⟩ <;>
        exact absurd he (by simp)
    · exact h
  -- what kind of output the pass can emit at all
  have hpass_kind : ∀ {o : Output}, o ∈ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 →
      (∃ chain c1 c2, o = Output.decided chain c1 c2)
        ∨ (∃ c, o = Output.send (.cert1 c))
        ∨ (∃ w p share, o = Output.send (.vote1 ⟨⟨blockHash p⟩, w, node⟩)
            ∨ o = Output.send (.vidShare share))
        ∨ (∃ w p, o = Output.send (.vote2 ⟨⟨blockHash p⟩, w, node⟩))
        ∨ (∃ p, o = Output.send (.proposal p)) := by
    intro o h
    rw [pass_out] at h
    simp only [List.mem_append] at h
    rcases h with ((((h | h) | h) | h) | h)
    · exact Or.inl (dSeg_shape h)
    · exact Or.inr (Or.inl (advanceLock_shape h))
    · exact Or.inr (Or.inr (Or.inl (v1Seg_shape h)))
    · exact Or.inr (Or.inr (Or.inr (Or.inl (v2Seg_shape h))))
    · exact Or.inr (Or.inr (Or.inr (Or.inr (pSeg_shape h))))
  have hpass_no_tcert : ∀ {tc : TimeoutCert} {w : ViewNumber},
      Output.send (.timeoutCert tc w) ∉ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 := by
    intro tc w h
    rcases hpass_kind h with hk | hk | hk | hk | hk
    · obtain ⟨_, _, _, he⟩ := hk
      exact absurd he (by simp)
    · obtain ⟨_, he⟩ := hk
      exact absurd he (by simp)
    · obtain ⟨_, _, _, he | he⟩ := hk
      all_goals exact absurd he (by simp)
    · obtain ⟨_, _, he⟩ := hk
      exact absurd he (by simp)
    · obtain ⟨_, he⟩ := hk
      exact absurd he (by simp)
  have hpass_no_tvote : ∀ {vt : TimeoutVote} {e : Option CatchupEvidence},
      Output.send (.timeoutVote vt e) ∉ (seq (rounds cfg leader node
        (ingest cfg node s i)) (ingest cfg node s i)).2 := by
    intro vt e h
    rcases hpass_kind h with hk | hk | hk | hk | hk
    · obtain ⟨_, _, _, he⟩ := hk
      exact absurd he (by simp)
    · obtain ⟨_, he⟩ := hk
      exact absurd he (by simp)
    · obtain ⟨_, _, _, he | he⟩ := hk
      all_goals exact absurd he (by simp)
    · obtain ⟨_, _, he⟩ := hk
      exact absurd he (by simp)
    · obtain ⟨_, he⟩ := hk
      exact absurd he (by simp)
  -- the settled set the pass is given is the decided set the step began with
  have hsub : ∀ w, w ∈ s.decidedViews → w ∈ (ingest cfg node s i).decidedViews := by
    intro w hw
    rw [ingest_decidedViews]
    exact hw
  refine
    { proposalProvenance := ?proposalProvenance, admissionJustified := ?admissionJustified, vidShareProvenance := ?vidShareProvenance
      validatedProvenance := ?validatedProvenance, reconstructedProvenance := ?reconstructedProvenance, headerProvenance := ?headerProvenance
      cert1Provenance := ?cert1Provenance, cert2Provenance := ?cert2Provenance, timeoutCertProvenance := ?timeoutCertProvenance
      proposalIngested := ?proposalIngested, cert1Ingested := ?cert1Ingested, cert2Ingested := ?cert2Ingested, timeoutCertIngested := ?timeoutCertIngested
      blockValidatedIngested := ?blockValidatedIngested, reconstructedIngested := ?reconstructedIngested, headerIngested := ?headerIngested
      currentViewMono := ?currentViewMono, currentViewJustified := ?currentViewJustified, timeoutViewMono := ?timeoutViewMono
      timeoutViewJustified := ?timeoutViewJustified, barredViewUnchanged := ?barredViewUnchanged, vote1NotBarred := ?vote1NotBarred
      vote2NotBarred := ?vote2NotBarred, proposeNotBarred := ?proposeNotBarred, contentRetained := ?contentRetained, lockMono := ?lockMono
      decidedRetained := ?decidedRetained, voted1Retained := ?voted1Retained, vote1BranchesRetained := ?vote1BranchesRetained
      voted2Retained := ?voted2Retained, proposedRetained := ?proposedRetained
      vote1Once := ?vote1Once, vote1Bar := ?vote1Bar, vote1Justified := ?vote1Justified
      vote1CarriesShare := ?vote1CarriesShare, vote1Marked := ?vote1Marked, vote1Records := ?vote1Records, vote1BranchesSound := ?vote1BranchesSound
      vote2Once := ?vote2Once, vote2Justified := ?vote2Justified, vote2LockOrdered := ?vote2LockOrdered, vote2NotInSkippedView := ?vote2NotInSkippedView
      vote2NotAfterCert2 := ?vote2NotAfterCert2, vote2AboveFloor := ?vote2AboveFloor, vote2Marked := ?vote2Marked, lockJustified := ?lockJustified
      proposeOnce := ?proposeOnce, proposeBar := ?proposeBar, proposeJustified := ?proposeJustified, proposedMarked := ?proposedMarked
      decideJustified := ?decideJustified, decidedMarked := ?decidedMarked, cert2RelayOwed := ?cert2RelayOwed
      advanceOwed := ?advanceOwed, timeoutCertSound := ?timeoutCertSound, timeoutCertAdvanceOwed := ?timeoutCertAdvanceOwed
      timeoutVoteSound := ?timeoutVoteSound, timeoutVoteOwed := ?timeoutVoteOwed }
  all_goals try dsimp only
  -- **Provenance**
  case proposalProvenance =>
    intro v p h
    rw [hfr.proposals] at h
    exact ingest_proposalProvenance h
  case admissionJustified =>
    intro v p h
    rw [hfr.admitted] at h
    rcases ingest_admissionJustified h with hold | ⟨sender, vid, hi, hv, hb, hste, hwfp, hsm,
      hsh, hpp⟩
    · exact Or.inl hold
    · exact Or.inr ⟨sender, vid, hi, hv, hb, hste, hwfp, hsm,
        by rw [hfr.vidShares]; exact hsh, by rw [hfr.proposals]; exact hpp⟩
  case vidShareProvenance =>
    intro v sh h
    rw [hfr.vidShares] at h
    exact ingest_vidShareProvenance h
  case validatedProvenance =>
    intro v hb h
    rw [hfr.validated] at h
    exact ingest_validatedProvenance h
  case reconstructedProvenance =>
    intro v pc h
    rw [hfr.blocksReconstructed] at h
    exact ingest_reconstructedProvenance h
  case headerProvenance =>
    intro v hb hd h
    rw [hfr.headers] at h
    exact ingest_headerProvenance h
  case cert1Provenance =>
    intro v c h
    rw [hfr.cert1s] at h
    exact ingest_cert1Provenance h
  case cert2Provenance =>
    intro v c h
    rw [hfr.cert2s] at h
    exact ingest_cert2Provenance h
  case timeoutCertProvenance =>
    intro v tc h
    rw [hfr.timeoutCerts] at h
    exact ingest_timeoutCertProvenance h
  -- **Ingestion**
  case proposalIngested =>
    intro sender p vid hi hb h1 h2 h3 hste hwfp hsm
    obtain ⟨ha, hp, hv⟩ := ingest_proposalIngested hi hb h1 h2 h3 hste hwfp hsm
    exact ⟨by rw [hfr.admitted]; exact ha, by rw [hfr.proposals]; exact hp,
      by rw [hfr.vidShares]; exact hv⟩
  case cert1Ingested =>
    intro c hi hfl hw
    rw [hfr.cert1s]
    exact ingest_cert1Ingested hi (aboveFloor_of_abstract hwf hfl) hw
  case cert2Ingested =>
    intro c hi hfl hw
    rw [hfr.cert2s]
    exact ingest_cert2Ingested hi (aboveFloor_of_abstract hwf hfl) hw
  case timeoutCertIngested =>
    intro tc hi hcv hw
    rw [hfr.timeoutCerts]
    exact ingest_timeoutCertIngested hi hcv hw
  case blockValidatedIngested =>
    intro v hb hi hw
    rw [hfr.validated]
    exact ingest_blockValidatedIngested hi hw
  case reconstructedIngested =>
    intro v pc hi
    rw [hfr.blocksReconstructed]
    exact ingest_reconstructedIngested hi
  case headerIngested =>
    intro v pa hb hi hw
    rw [hfr.headers]
    exact ingest_headerIngested hi hw
  -- **The cursors**
  case currentViewMono =>
    exact Nat.le_trans ingest_currentViewMono hle.currentView
  case currentViewJustified =>
    intro hne
    rcases pass_currentView (cfg := cfg) (leader := leader) (node := node) hwft with hsame | ⟨w, c, hcur, hc1⟩
    · rw [hsame] at hne ⊢
      obtain ⟨v, hv, hev⟩ := ingest_currentViewJustified hne
      refine ⟨v, hv, ?_⟩
      rcases hev with ⟨c, hc⟩ | ⟨tc, htc⟩ | hin | hin
      · exact Or.inl ⟨c, by rw [hfr.cert1s]; exact hc⟩
      · exact Or.inr (Or.inl ⟨tc, by rw [hfr.timeoutCerts]; exact htc⟩)
      · exact Or.inr (Or.inr (Or.inl hin))
      · exact Or.inr (Or.inr (Or.inr hin))
    · exact ⟨w, hcur, Or.inl ⟨c, hc1⟩⟩
  case timeoutViewMono =>
    rw [hfr.timeoutView]
    exact ingest_timeoutViewMono
  case timeoutViewJustified =>
    intro hne
    rw [hfr.timeoutView] at hne ⊢
    exact ingest_timeoutViewJustified hne
  -- **Abandoned views**
  case barredViewUnchanged =>
    rw [hfr.barredView]
    exact ingest_barredView
  case vote1NotBarred =>
    intro vt h
    obtain ⟨p, share, -, -, -, -, -, -, -, -, -, hbar, -, -⟩ := pass_vote1 hwft (hvote1 h)
    rw [hfr.barredView]
    exact hbar
  case vote2NotBarred =>
    intro vt h
    obtain ⟨p, c, -, -, -, -, -, -, -, -, hbar, -, -, -⟩ := pass_vote2 hwft (hvote2 h)
    rw [hfr.barredView]
    exact hbar
  case proposeNotBarred =>
    intro p h
    obtain ⟨-, -, -, -, -, -, -, hbar⟩ := pass_propose hwft (hprop h)
    rw [hfr.barredView]
    exact hbar
  -- **What the state may not lose**
  case contentRetained =>
    intro v hfl
    exact
      { decide :=
          { proposals := fun p hp => by
              rw [hfr.proposals]; exact ingest_proposals_retained hp
            blocksReconstructed := fun pc hpc => by
              rw [hfr.blocksReconstructed]; exact ingest_reconstructed_retained hpc
            cert1s := fun c hc => by rw [hfr.cert1s]; exact ingest_cert1s_retained hc
            cert2s := fun c hc => by rw [hfr.cert2s]; exact ingest_cert2s_retained hc }
        vote :=
          { admitted := fun p hp => by
              rw [hfr.admitted]; exact ingest_admitted_retained hp
            vidShares := fun sh hsh => by
              rw [hfr.vidShares]; exact ingest_vidShares_retained hsh
            validated := fun hb hhb => by
              rw [hfr.validated]; exact ingest_validated_retained hhb
            headers := fun hb hd' hhd => by
              rw [hfr.headers]; exact ingest_header_retained hhd
            timeoutCerts := fun tc htc => by
              rw [hfr.timeoutCerts]; exact ingest_timeoutCerts_retained htc } }
  case lockMono =>
    intro lock hlock
    exact hle.lock lock (by rw [ingest_lockedCert]; exact hlock)
  case decidedRetained =>
    exact fun v h => hle.decided v (by rw [ingest_decidedViews]; exact h)
  case voted1Retained =>
    exact fun v h => hle.voted1 v (by rw [ingest_voted1Views]; exact h)
  case vote1BranchesRetained =>
    exact fun v u h => hle.branches v u (by rw [ingest_vote1Branches]; exact h)
  case voted2Retained =>
    exact fun v h => hle.voted2 v (by rw [ingest_voted2Views]; exact h)
  case proposedRetained =>
    exact fun v h => hle.proposed v (by rw [ingest_proposedViews]; exact h)
  -- **Vote1**
  case vote1Once =>
    intro vt h
    obtain ⟨p, share, -, -, -, -, -, -, hnv, hmark, -, -, -, -⟩ := pass_vote1 hwft (hvote1 h)
    exact ⟨by rw [ingest_voted1Views] at hnv; exact hnv, hmark⟩
  case vote1Bar =>
    intro vt h
    obtain ⟨p, share, -, -, -, -, -, -, -, -, hto, -, -⟩ := pass_vote1 hwft (hvote1 h)
    rw [hfr.timeoutView]
    exact hto
  case vote1Justified =>
    intro vt h
    obtain ⟨p, share, hvt, hadm, hsv, hsh, hlinked, hste, -, -, -, -, -, -⟩ :=
      pass_vote1 hwft (hvote1 h)
    have hpv : p.viewNumber = vt.view := hwft.proposals _ _ (hwft.admitted _ _ hadm)
    refine ⟨p, ?_, hpv.symm, by rw [hvt], by rw [hvt]⟩
    exact
      { proposalAdmitted := by rw [hpv, hfr.admitted]; exact hadm
        blockValid := hwft.validated _ p hsv
        vidShare := by
          rw [hpv, hfr.vidShares]
          exact Option.isSome_iff_exists.mpr ⟨share, hsh⟩
        safeToExtend := safeToExtend_iff.mp hste
        parentLinked := parentLinked_spec
          (by rw [parentLinked_congr hfr.proposals hfr.blocksReconstructed]; exact hlinked) }
  case vote1CarriesShare =>
    intro vt h
    obtain ⟨p, share, -, -, -, hsh, -, -, -, -, -, -, -, hout⟩ :=
      pass_vote1 hwft (hvote1 h)
    rw [← hfr.vidShares] at hsh
    exact ⟨share, hsh, mem_next_of_pass hout⟩
  case vote1Marked =>
    intro v hno hyes
    obtain ⟨vt, hout, hvv⟩ := pass_vote1Marked hwft (by rw [ingest_voted1Views]; exact hno) hyes
    exact ⟨vt, mem_next_of_pass hout, hvv⟩
  case vote1Records =>
    intro vt h
    obtain ⟨p, share, -, hadm, -, -, -, -, -, -, -, -, hbr, -⟩ := pass_vote1 hwft (hvote1 h)
    exact ⟨p, by rw [hfr.admitted]; exact hadm, hbr⟩
  case vote1BranchesSound =>
    intro v u hnone hsome
    obtain ⟨vt, hout, hvv⟩ :=
      pass_branchesSound hwft (by rw [ingest_vote1Branches]; exact hnone) hsome
    exact ⟨vt, mem_next_of_pass hout, hvv⟩
  -- **Vote2**
  case vote2Once =>
    intro vt h
    obtain ⟨p, c, -, -, -, -, -, hnv, hmark, -, -, -, -, -⟩ := pass_vote2 hwft (hvote2 h)
    exact ⟨by rw [ingest_voted2Views] at hnv; exact hnv, hmark⟩
  case vote2Justified =>
    intro vt h
    obtain ⟨p, c, hvt, hadm, hc1, hbh, hrec, -, -, -, -, -, -, -⟩ :=
      pass_vote2 hwft (hvote2 h)
    have hpv : p.viewNumber = vt.view := hwft.proposals _ _ (hwft.admitted _ _ hadm)
    refine ⟨p, ?_, hpv.symm, by rw [hvt], by rw [hvt]⟩
    exact
      { proposalAdmitted := by rw [hpv, hfr.admitted]; exact hadm
        certMatches := ⟨c, by rw [hpv, hfr.cert1s]; exact hc1, hbh⟩
        reconstructed := by rw [hpv, hfr.blocksReconstructed]; exact hrec }
  case vote2LockOrdered =>
    intro vt h
    obtain ⟨p, c, -, -, -, -, -, -, -, -, -, -, -, hlock⟩ := pass_vote2 hwft (hvote2 h)
    exact hlock
  case vote2NotInSkippedView =>
    intro vt h
    obtain ⟨p, c, -, -, -, -, -, -, -, -, -, -, haround, -⟩ := pass_vote2 hwft (hvote2 h)
    intro hc
    exact absurd (vote1Skipped_iff.mpr hc) (by rw [haround]; simp)
  case vote2NotAfterCert2 =>
    intro vt h
    obtain ⟨p, c, -, -, -, -, -, -, -, hc2, -, -, -, -⟩ := pass_vote2 hwft (hvote2 h)
    cases hs : s.cert2s.get? vt.view with
    | none => rfl
    | some c' => exact absurd (ingest_cert2s_retained hs) (by rw [hc2]; simp)
  case vote2AboveFloor =>
    intro vt h
    obtain ⟨p, c, -, -, -, -, -, -, -, -, -, hfl, -, -⟩ := pass_vote2 hwft (hvote2 h)
    exact abstract_aboveDecideFloor hwfu hfl
  case vote2Marked =>
    intro v hno hyes
    obtain ⟨vt, hout, hvv⟩ := pass_vote2Marked hwft (by rw [ingest_voted2Views]; exact hno) hyes
    exact ⟨vt, mem_next_of_pass hout, hvv⟩
  -- **The lock**
  case lockJustified =>
    intro lock hlock
    rcases pass_lock (cfg := cfg) (leader := leader) (node := node) hwft with hsame | ⟨c, hc, hold, hc1, p, hadm, hbh, hrec⟩
    · exact Or.inl (by rw [← ingest_lockedCert (cfg := cfg) (node := node)
        (s := s) (i := i), ← hsame]; exact hlock)
    · obtain rfl : lock = c := by rw [hc] at hlock; simpa using hlock.symm
      refine Or.inr ⟨?_, by rw [hfr.cert1s]; exact hc1, p, ?_, hbh, ?_⟩
      · intro old hold'
        exact hold old (by rw [ingest_lockedCert]; exact hold')
      · rw [hfr.proposals]; exact hwft.admitted _ _ hadm
      · rw [hfr.blocksReconstructed]; exact hrec
  -- **Proposals**
  case proposeOnce =>
    intro p h
    obtain ⟨-, -, -, -, hnp, hmark, -, -⟩ := pass_propose hwft (hprop h)
    exact ⟨by rw [ingest_proposedViews] at hnp; exact hnp, hmark⟩
  case proposeBar =>
    intro p h
    obtain ⟨-, -, -, -, -, -, hto, -⟩ := pass_propose hwft (hprop h)
    rw [hfr.timeoutView]
    exact hto
  case proposeJustified =>
    intro p h
    obtain ⟨hlead, hwfp, hjust, hheaders, -, -, -, -⟩ := pass_propose hwft (hprop h)
    obtain ⟨parent, hpar, hmatch, hhd⟩ := hheaders
    refine { leader := hlead, wellFormed := hwfp, justified := ?_,
             headerBuilt := ⟨parent, ?_, hmatch, ?_⟩ }
    · unfold ParentCertJustified
      cases hte : p.timeoutEvidence with
      | some tc =>
        rw [hte] at hjust
        exact ⟨by rw [hfr.timeoutCerts]; exact hjust.1, hjust.2⟩
      | none =>
        rw [hte] at hjust
        exact ⟨by rw [hfr.cert1s]; exact hjust.1, hjust.2⟩
    · rw [hfr.proposals]; exact hpar
    · rw [hfr.headers]; exact hhd
  case proposedMarked =>
    intro v hno hyes
    obtain ⟨p, hout, hpv⟩ := pass_proposedMarked hwft (by rw [ingest_proposedViews]; exact hno) hyes
    exact ⟨p, mem_next_of_pass hout, hpv⟩
  -- **Deciding**
  case decideJustified =>
    intro chain c1 c2 h
    obtain ⟨head, rest, hchain, hc2v, hbh, hc2, hc1, hlinked, hlast, hblocks⟩ :=
      pass_decide hwft (hdec h)
    -- The decide was judged against the state the pass began at, which is the
    -- state the input arm left: it has decided no more than the step ends with,
    -- and no more than the step began with either.
    have hsw : ∀ d, d ∈ s.decidedViews → d ∈ (ingest cfg node s i).decidedViews :=
      fun d hd => by rw [ingest_decidedViews]; exact hd
    refine ⟨⟨head, rest, hchain, hc2v, hbh, by rw [hfr.cert2s]; exact hc2,
      by rw [hfr.cert1s]; exact hc1⟩, hlinked, ?_, ?_⟩
    · intro last hl
      rcases hlast last hl with hset | hfl | hnh
      · exact Or.inl (hle.decided _ hset)
      · exact Or.inr (Or.inl (not_aboveDecideFloor_of_le hwft hle.decided hfl))
      · exact Or.inr (Or.inr fun ⟨q, hq, hbq⟩ => hnh ⟨q, by rw [← hfr.proposals]; exact hq, hbq⟩)
    · intro b hb
      obtain ⟨hfl, hnd, hmark, hheld⟩ := hblocks b hb
      exact ⟨aboveDecideFloor_of_floor_lt hsw hfl,
        fun hd => hnd (hsw _ hd), hmark, by rw [hfr.proposals]; exact hheld⟩
  case decidedMarked =>
    intro v hno hyes
    obtain ⟨chain, c1, c2, b, hout, hb, hbv⟩ :=
      pass_decidedMarked hwft (by rw [ingest_decidedViews]; exact hno) hyes
    exact ⟨chain, c1, c2, b, mem_next_of_pass hout, hb, hbv⟩
  -- **Certificate relay**
  case cert2RelayOwed =>
    intro c hi hnone hnd hfl
    exact mem_next_of_ingest (ingest_cert2RelayOwed hi hnone hnd (aboveFloor_of_abstract hwf hfl))
  -- **View changes**
  case advanceOwed =>
    intro c hi
    exact Nat.le_trans (ingest_advanceOwed hi) hle.currentView
  case timeoutCertSound =>
    intro tc v h
    rcases mem_next_out h with h | h
    · rcases mem_ingestOut h with ⟨c, he, -⟩ | ⟨w, e, he, -, -⟩ | ⟨tc', w, he, hi, hv, hb⟩
      · exact absurd he (by simp)
      · exact absurd he (by simp)
      · obtain ⟨rfl, rfl⟩ : tc = tc' ∧ v = w := by simpa [and_comm] using he
        exact ⟨hi, hv, Nat.le_trans hb hle.currentView⟩
    · exact absurd h hpass_no_tcert
  case timeoutCertAdvanceOwed =>
    intro tc hi
    exact Nat.le_trans (ingest_timeoutCertAdvanceOwed hi) hle.currentView
  -- **Timeouts**
  case timeoutVoteSound =>
    intro vt e h
    rcases mem_next_out h with h | h
    · rcases mem_ingestOut h with ⟨c, he, -⟩ | ⟨w, e', he, hb, hin⟩ | ⟨tc, w, he, -⟩
      · exact absurd he (by simp)
      · obtain ⟨rfl, rfl⟩ : vt = ⟨(), w, node⟩ ∧ e = e' := by simpa using he
        exact ⟨rfl, by rw [hfr.timeoutView]; exact hb, hin⟩
      · exact absurd he (by simp)
    · exact absurd h hpass_no_tvote
  case timeoutVoteOwed =>
    intro v hi
    obtain ⟨e, hout⟩ := ingest_timeoutVoteOwed hi
    exact ⟨e, mem_next_of_ingest hout⟩

end Impl
end NewProtocol
