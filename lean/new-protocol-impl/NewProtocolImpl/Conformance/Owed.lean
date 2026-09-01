module

public import NewProtocolImpl.Conformance.Guards
public import NewProtocolImpl.Conformance.Progress

/-!
# A step leaves nothing owed

`NextSettles`, the last obligation, and the reason the machine's progress is
the eagerness argument: nothing is enabled when a step ends, so `WeaklyFair`'s
antecedent is never satisfied.

One lemma per action, each of the same shape. An action owed at the end of the
pass was owed at the state its own round ran at — because what the action reads
is either frozen for the whole pass (`Frame`) or settled by an earlier round —
and there its guard is exactly the specification's clauses, so the round *fired*.
But firing sets the mark that the action's freshness clause denies. So nothing
was owed.

What makes the rounds line up this way is their order, and each round depends on
the one before it:

* the **lock advance** runs before both votes, so the lock a vote tests is the
  lock the end of the pass is judged against — and it has reached every view the
  state licenses, which is what a vote2 needs
  (`Impl.advanceLock_reached`);
* the **vote1s** run before the vote2s, so the branch records a
  vote2 tests against are all of them;
* the **decides** run first, so nothing later moves the floor.
-/

@[expose] public section

set_option linter.unusedSectionVars false

open Std (TreeMap TreeSet)

namespace NewProtocol
namespace Impl

variable {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey} {t : State}

/-! ## One attempt at an owed action fires -/

/--
A decide owed at `a` is one the attempt at `u` takes.

Every guard is contradicted by a clause of `DecideEnabled`, read in the
direction the safety half did not need: the certificates and the block are there
because `a` holds them, and the view is fresh because it is undecided at `u`.
Nothing is asked of the chain the attempt would deliver — whatever the walk
reaches is deliverable, which is why an owed decide can always be taken now.
-/
theorem tryDecide_fires {a u : State} (hwfa : WF cfg a) (hfr : Frame a u)
    {v : ViewNumber} (hfresh : v ∉ u.decidedViews) (hen : DecideEnabled cfg a.abstract v) :
    v ∈ (tryDecide cfg a.decidedViews v u).1.decidedViews := by
  obtain ⟨-, habove, hc1s, c2, p, hc2, hp, hbh⟩ := hen
  obtain ⟨c1, hc1⟩ := Option.isSome_iff_exists.mp hc1s
  have hfl : floorOf cfg a.decidedViews < v := by
    rw [floorOf_decidedViews]
    exact of_decide_eq_true (aboveFloor_of_abstract hwfa habove)
  have hpv : p.viewNumber = v := hwfa.proposals _ _ hp
  unfold tryDecide
  rw [if_neg (by
    rintro (hc | hc)
    · exact hfresh (contains_iff_mem.mp hc)
    · exact hc hfl)]
  rw [show u.cert2s.get? v = some c2 by rw [hfr.cert2s]; exact hc2,
    show u.cert1s.get? v = some c1 by rw [hfr.cert1s]; exact hc1,
    show u.proposals.get? v = some p by rw [hfr.proposals]; exact hp]
  dsimp only
  rw [if_pos hbh]
  exact hpv ▸ mem_decideFold_of_mem (List.mem_cons_self ..)

/-- A vote1 owed at `a` is one the attempt at `u` casts. -/
theorem tryVote1_fires {a u : State} {p : Proposal} (hfr : Frame a u)
    (hlock : u.lockedCert = a.lockedCert) (hfresh : p.viewNumber ∉ u.voted1Views)
    (hen : Vote1Enabled a.abstract p) :
    p.viewNumber ∈ (tryVote1 node p.viewNumber u).1.voted1Views := by
  obtain ⟨⟨hadm, -, hvid, hsafe, hpar⟩, hver, hnv, htv, hbar, -⟩ := hen
  obtain ⟨share, hshare⟩ := Option.isSome_iff_exists.mp hvid
  have htv' : a.timeoutView < p.viewNumber := htv
  have hbar' : a.barredView < p.viewNumber := hbar
  unfold tryVote1
  rw [if_neg (by
    rintro (hc | hc | hc)
    · exact absurd (hfr.timeoutView ▸ hc) (Nat.not_le.mpr htv')
    · exact absurd (hfr.barredView ▸ hc) (Nat.not_le.mpr hbar')
    · exact hfresh (contains_iff_mem.mp hc))]
  rw [show u.admitted.get? p.viewNumber = some p by rw [hfr.admitted]; exact hadm,
    show u.vidShares.get? p.viewNumber = some share by rw [hfr.vidShares]; exact hshare]
  dsimp only
  rw [if_pos ⟨by rw [hfr.validated]; exact hver,
    parentLinked_of_spec (by
      intro hne
      obtain ⟨parent, hp, hbh, hrec⟩ := hpar hne
      exact ⟨parent, by rw [hfr.proposals]; exact hp, hbh,
        by rw [hfr.blocksReconstructed]; exact hrec⟩),
    safeToExtend_iff.mpr (by rw [hlock]; exact hsafe)⟩]
  exact mem_insert_self

/-- A vote2 owed at `a` is one the attempt at `u` casts. -/
theorem tryVote2_fires {a u : State} {p : Proposal} (hfr : Frame a u)
    (hbranches : u.vote1Branches = a.vote1Branches)
    (hreached : ∃ l, u.lockedCert = some l ∧ p.viewNumber ≤ l.view)
    (habove : u.aboveFloor cfg p.viewNumber = true)
    (hfresh : p.viewNumber ∉ u.voted2Views) (hnd : p.viewNumber ∉ u.decidedViews)
    (hen : Vote2Enabled cfg a.abstract p) :
    p.viewNumber ∈ (tryVote2 cfg node p.viewNumber u).1.voted2Views := by
  obtain ⟨⟨hadm, ⟨c1, hc1, hc1h⟩, hrec⟩, haround, hnv, hc2, hdec, hab, hbar⟩ := hen
  obtain ⟨l, hl, hlv⟩ := hreached
  have hbar' : a.barredView < p.viewNumber := hbar
  unfold tryVote2
  rw [if_neg (by
    rintro (hc | hc | hc | hc | hc | hc | hc)
    · exact absurd (hfr.barredView ▸ hc) (Nat.not_le.mpr hbar')
    · exact hfresh (contains_iff_mem.mp hc)
    · rw [show u.cert2s.get? p.viewNumber = none by rw [hfr.cert2s]; exact hc2] at hc
      exact absurd hc (by simp)
    · exact hnd (contains_iff_mem.mp hc)
    · exact hc habove
    · exact haround (vote1Skipped_iff.mp ((vote1Skipped_congr hbranches p.viewNumber) ▸ hc))
    · unfold State.lockBelow at hc
      rw [hl] at hc
      exact absurd (of_decide_eq_true hc) (Nat.not_lt.mpr hlv))]
  rw [show u.lockable p.viewNumber = some c1 from
    lockable_of_spec (by rw [hfr.cert1s]; exact hc1) (by rw [hfr.admitted]; exact hadm)
      hc1h (by rw [hfr.blocksReconstructed]; exact hrec)]
  dsimp only
  rw [show u.admitted.get? p.viewNumber = some p by rw [hfr.admitted]; exact hadm]
  exact mem_insert_self

/-- The proposal search finds a candidate whenever one is justified. -/
theorem candidate_isSome {u : State} {p : Proposal}
    (hj : ProposalJustification cfg leader node u.abstract p) :
    ((u.timeoutCandidate cfg p.viewNumber).or (u.normalCandidate cfg p.viewNumber)).isSome := by
  obtain ⟨hlead, hwfp, hjust, parent, hpar, hbh, hhdr⟩ := hj
  have hpar' : u.proposals[p.parentCert.view]? = some parent := hpar
  have hhdr' : u.headers[(p.viewNumber, blockHash parent)]? = some p.blockHeader := hhdr
  have hlt : p.parentCert.view < p.viewNumber := hwfp.1
  have hmatch : parentMatches p.parentCert parent = true := by
    unfold parentMatches
    rw [Bool.or_eq_true]
    by_cases hg : p.parentCert.view = ViewNumber.genesis
    · exact Or.inl (by simp [hg])
    · exact Or.inr (by simp [(hbh hg).symm])
  unfold ParentCertJustified at hjust
  cases hte : p.timeoutEvidence with
  | some tc =>
    rw [hte] at hjust
    have htc' : u.timeoutCerts[p.viewNumber]? = some tc := hjust.1
    have hlk' : u.lockedCert = some p.parentCert := hjust.2
    have hto : (u.timeoutCandidate cfg p.viewNumber).isSome = true := by
      simp [State.timeoutCandidate, htc', hlk', hlt, hpar', hmatch, hhdr']
    rcases hq : u.timeoutCandidate cfg p.viewNumber with _ | q
    · exact absurd hto (by rw [hq]; simp)
    · simp
  | none =>
    rw [hte] at hjust
    have hc1' : u.cert1s[p.viewNumber - 1]? = some p.parentCert := hjust.1
    have hno : (u.normalCandidate cfg p.viewNumber).isSome = true := by
      simp [State.normalCandidate, hc1', hjust.2, hpar', hmatch, hhdr']
    rcases hq : u.timeoutCandidate cfg p.viewNumber with _ | q
    · simpa using hno
    · simp

/-- A proposal owed at `a` is one the attempt at `u` makes. -/
theorem tryPropose_fires {a u : State} {p : Proposal} (hfr : Frame a u)
    (hlock : u.lockedCert = a.lockedCert) (hfresh : p.viewNumber ∉ u.proposedViews)
    (hen : ProposeEnabled cfg leader node a.abstract p) :
    p.viewNumber ∈ (tryPropose cfg leader node p.viewNumber u).1.proposedViews := by
  obtain ⟨hj, hnp, htv, hbar⟩ := hen
  have htv' : a.timeoutView < p.viewNumber := htv
  have hbar' : a.barredView < p.viewNumber := hbar
  obtain ⟨hlead, hwfp, hjust, parent, hpar, hbh, hhdr⟩ := hj
  obtain ⟨q, hq⟩ := Option.isSome_iff_exists.mp (candidate_isSome (leader := leader) (node := node)
    (u := u) (p := p) (by
      refine ⟨hlead, hwfp, ?_, parent,
        show u.proposals.get? p.parentCert.view = some parent by rw [hfr.proposals]; exact hpar,
        hbh,
        show u.headers.get? (p.viewNumber, blockHash parent) = some p.blockHeader by
          rw [hfr.headers]; exact hhdr⟩
      revert hjust
      unfold ParentCertJustified
      cases p.timeoutEvidence with
      | some tc =>
        exact fun h => ⟨show u.timeoutCerts.get? p.viewNumber = some tc by
            rw [hfr.timeoutCerts]; exact h.1,
          show u.lockedCert = some p.parentCert by rw [hlock]; exact h.2⟩
      | none =>
        exact fun h => ⟨show u.cert1s.get? (p.viewNumber - 1) = some p.parentCert by
          rw [hfr.cert1s]; exact h.1, h.2⟩))
  unfold tryPropose
  rw [if_neg (by
    rintro (hc | hc | hc | hc)
    · exact absurd (hfr.timeoutView ▸ hc) (Nat.not_le.mpr htv')
    · exact absurd (hfr.barredView ▸ hc) (Nat.not_le.mpr hbar')
    · exact hfresh (contains_iff_mem.mp hc)
    · exact hc hlead), hq]
  exact mem_insert_self

/-! ## Nothing is owed when the pass ends -/

/-- The vote1s leave none owed. -/
theorem pass_vote1_settled (hwf : WF cfg t) (p : Proposal) : ¬ Vote1Enabled (st5 cfg leader node t).abstract p := by
  intro hen
  have hfr5 := (st5_stage hwf (cfg := cfg) (leader := leader) (node := node)).1
  have hadm : t.admitted.get? p.viewNumber = some p := by
    rw [← hfr5.admitted]; exact hen.1.proposalAdmitted
  obtain ⟨u, hwfu, hfru, -, hlocku, hfr', hle'⟩ :=
    seq_at State.lockedCert (fs := v1Seg node t)
      (fun f hf => by obtain ⟨w, -, rfl⟩ := List.mem_map.mp hf; exact tryVote1_grows)
      (fun f hf w => by obtain ⟨x, -, rfl⟩ := List.mem_map.mp hf; exact tryVote1_lock w)
      (st2_stage hwf).2.2 (tryVote1 node p.viewNumber)
      (List.mem_map.mpr ⟨p.viewNumber, mem_keys_of_get? hadm, rfl⟩)
  have hlefire : Le (tryVote1 node p.viewNumber u).1 (st5 cfg leader node t) :=
    hle'.trans (st3_to_st5 hwf)
  have hleu5 : Le u (st5 cfg leader node t) := (tryVote1_le u hwfu).trans hlefire
  exact hen.2.2.1 (hlefire.voted1 _ (tryVote1_fires
    (Frame.swap hfr5 ((st2_stage hwf).1.trans hfru)) (hlocku.trans st5_lock.symm)
    (fun hc => hen.2.2.1 (hleu5.voted1 _ hc)) hen))

/-- The vote2s leave none owed. -/
theorem pass_vote2_settled (hwf : WF cfg t) (p : Proposal) :
    ¬ Vote2Enabled cfg (st5 cfg leader node t).abstract p := by
  intro hen
  obtain ⟨⟨hadm, ⟨c1, hc1, hc1h⟩, hrec⟩, haround, hnv, hc2, hdec, hab, hbar⟩ := hen
  have hfr5 := (st5_stage hwf (cfg := cfg) (leader := leader) (node := node)).1
  have hwf5 := (st5_stage hwf (cfg := cfg) (leader := leader) (node := node)).2.2
  have hadmt : t.admitted.get? p.viewNumber = some p := by rw [← hfr5.admitted]; exact hadm
  obtain ⟨u, hwfu, hfru, -, hproj, hfr', hle'⟩ :=
    seq_at (fun s => (s.lockedCert, s.vote1Branches)) (fs := v2Seg cfg node t)
      (fun f hf => by obtain ⟨w, -, rfl⟩ := List.mem_map.mp hf; exact tryVote2_grows)
      (fun f hf w => by
        obtain ⟨x, -, rfl⟩ := List.mem_map.mp hf
        rw [tryVote2_lock, tryVote2_branches])
      (st3_stage hwf).2.2 (tryVote2 cfg node p.viewNumber)
      (List.mem_map.mpr ⟨p.viewNumber, mem_keys_of_get? hadmt, rfl⟩)
  have hlefire : Le (tryVote2 cfg node p.viewNumber u).1 (st5 cfg leader node t) :=
    hle'.trans (st4_to_st5 hwf).2
  have hleu5 : Le u (st5 cfg leader node t) := (tryVote2_le u).trans hlefire
  have hfru5 : Frame (st5 cfg leader node t) u :=
    Frame.swap hfr5 (((st3_stage hwf).1).trans hfru)
  -- the lock has reached this view, because the advance saw the same licence
  have hlockable : (st1 cfg t).lockable p.viewNumber = some c1 := by
    have hfr1 := (st1_stage hwf (cfg := cfg)).1
    exact lockable_of_spec (by rw [hfr1.cert1s, ← hfr5.cert1s]; exact hc1)
      (by rw [hfr1.admitted, ← hfr5.admitted]; exact hadm) hc1h
      (by rw [hfr1.blocksReconstructed, ← hfr5.blocksReconstructed]; exact hrec)
  have hc1v : c1.view = p.viewNumber := by
    have hfr1 := (st1_stage hwf (cfg := cfg)).1
    exact (st1_stage hwf (cfg := cfg)).2.2.cert1s _ _ (by
      rw [hfr1.cert1s, ← hfr5.cert1s]; exact hc1)
  obtain ⟨l, hl, hlv⟩ := advanceLock_reached (s := st1 cfg t)
    (mem_keys_of_get? (show (st1 cfg t).admitted.get? p.viewNumber = some p by
      rw [(st1_stage hwf (cfg := cfg)).1.admitted, ← hfr5.admitted]; exact hadm)) hlockable
  -- and the lock does not move again
  have hlu : u.lockedCert = some l := by
    have h1 : u.lockedCert = (st3 cfg node t).lockedCert := congrArg Prod.fst hproj
    rw [h1, st3_lock, st2]
    exact hl
  have habove : u.aboveFloor cfg p.viewNumber = true := by
    refine decide_eq_true (Nat.lt_of_le_of_lt ?_
      (of_decide_eq_true (aboveFloor_of_abstract hwf5 hab)))
    exact sub_le_sub _ (le_lastDecided (hleu5.decided _ (lastDecided_mem hwfu.decided)))
  -- the branch records are all in place before this round
  have hbr3 : u.vote1Branches = (st3 cfg node t).vote1Branches := congrArg Prod.snd hproj
  have hbr : u.vote1Branches = (st5 cfg leader node t).vote1Branches := by
    rw [hbr3, ← st5_branches]
  exact hnv (hlefire.voted2 _ (tryVote2_fires hfru5 hbr ⟨l, hlu, hc1v ▸ hlv⟩ habove
    (fun hc => hnv (hleu5.voted2 _ hc)) (fun hc => hdec (hleu5.decided _ hc))
    ⟨⟨hadm, ⟨c1, hc1, hc1h⟩, hrec⟩, haround, hnv, hc2, hdec, hab, hbar⟩))

/-- The proposals leave none owed. -/
theorem pass_propose_settled (hwf : WF cfg t) (p : Proposal) :
    ¬ ProposeEnabled cfg leader node (st5 cfg leader node t).abstract p := by
  intro hen
  obtain ⟨hj, hnp, htv, hbar⟩ := hen
  obtain ⟨hlead, hwfp, hjust, parent, hpar, hbh, hhdr⟩ := hj
  have hfr5 := (st5_stage hwf (cfg := cfg) (leader := leader) (node := node)).1
  have hhdrt : t.headers.get? (p.viewNumber, blockHash parent) = some p.blockHeader := by
    rw [← hfr5.headers]; exact hhdr
  obtain ⟨u, hwfu, hfru, -, hlocku, hfr', hle'⟩ :=
    seq_at State.lockedCert (fs := pSeg cfg leader node t)
      (fun f hf => by obtain ⟨k, -, rfl⟩ := List.mem_map.mp hf; exact tryPropose_grows)
      (fun f hf w => by obtain ⟨k, -, rfl⟩ := List.mem_map.mp hf; exact tryPropose_lock w)
      (st4_stage hwf).2.2 (tryPropose cfg leader node p.viewNumber)
      (List.mem_map.mpr ⟨(p.viewNumber, blockHash parent), mem_keys_of_get? hhdrt, rfl⟩)
  have hlefire : Le (tryPropose cfg leader node p.viewNumber u).1 (st5 cfg leader node t) := hle'
  have hleu5 : Le u (st5 cfg leader node t) := (tryPropose_le u).trans hlefire
  exact hnp (hlefire.proposed _ (tryPropose_fires
    (Frame.swap hfr5 ((st4_stage hwf).1.trans hfru))
    (hlocku.trans ((st4_lock).trans st5_lock.symm))
    (fun hc => hnp (hleu5.proposed _ hc))
    ⟨⟨hlead, hwfp, hjust, parent, hpar, hbh, hhdr⟩, hnp, htv, hbar⟩))

/--
The decides leave none owed.

The transfer here is not to the state the round ran at but all the way back to
the state the pass began at, which is also the watermark the round was judged
against — so the attempt this finds is the very attempt the round made.
-/
theorem pass_decide_settled (hwf : WF cfg t) (v : ViewNumber) :
    ¬ DecideEnabled cfg (st5 cfg leader node t).abstract v := by
  intro hen
  obtain ⟨hfr5, hle5, -⟩ := st5_stage hwf (cfg := cfg) (leader := leader) (node := node)
  have hent : DecideEnabled cfg t.abstract v := decideEnabled_congr hfr5 hle5.decided hen
  obtain ⟨c2, -, hc2, -, -⟩ := hent.2.2.2
  obtain ⟨u, -, hfru, -, -, -, hle'⟩ :=
    seq_at (fun _ => ()) (fs := dSeg cfg t)
      (fun f hf => by
        obtain ⟨w, -, rfl⟩ := List.mem_map.mp
          (show f ∈ List.map (tryDecide cfg t.decidedViews) t.cert2s.keys from hf)
        exact tryDecide_grows)
      (fun _ _ _ => rfl) hwf (tryDecide cfg t.decidedViews v)
      (List.mem_map.mpr ⟨v, mem_keys_of_get? hc2, rfl⟩)
  have hend : ∀ w, w ∈ (tryDecide cfg t.decidedViews v u).1.decidedViews →
      w ∈ (st5 cfg leader node t).decidedViews :=
    fun w hw => (st1_to_st5 hwf).decided w (hle'.decided w hw)
  exact hen.1 (hend v (tryDecide_fires hwf hfru
    (fun hc => hen.1 (hend v ((tryDecide_le u).decided v hc))) hent))

/-- The machine meets `NextSettles`. -/
theorem next_settles (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey) :
    NextSettles cfg leader node := by
  intro s input henv hwf
  rw [next_state]
  exact
    { vote1 := pass_vote1_settled (ingest_wf henv hwf)
      vote2 := pass_vote2_settled (ingest_wf henv hwf)
      decide := pass_decide_settled (ingest_wf henv hwf)
      propose := pass_propose_settled (ingest_wf henv hwf) }

/-- **The machine conforms.** -/
theorem conforms (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey) :
    ProtocolConforms cfg leader node :=
  protocol_conforms cfg leader node (next_settles cfg leader node)

end Impl
end NewProtocol
