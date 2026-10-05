module

public import NewProtocolSpec.Properties
public import NewProtocolSpec.Proofs.Votes

/-!
# What a backed certificate says

A quorum of one epoch has an honest member, and what that member signed it signed
by the rules. So a backed `Cert1` names a well-formed block some honest node
received, a backed `Cert2` presupposes a backed `Cert1` over the same block, and
two certificates of one epoch at one view name one block. A certificate is at its
block's view or, for the last block of an epoch voted on again (`RevoteRequest`), a later
one.

`cert1_step` is where the signing rules meet: one step back from a certified block,
to its parent or to the certificate a re-vote voted on again, skips no committed
view of the epoch (`NoGapBelow`). It reads `SafeParent` and `SafeRevote`, the
vote2-before-timeout rule and the timeout vote's lock, through `vote1_safe` and
`timeout_lock_covers`, and `OpensEpochJustified` for the first block of an epoch.
-/

@[expose] public section

namespace NewProtocol

variable {cfg : Config} (tree : BlockTable)

/-- A backed `Cert1` has a signer honest in its epoch. -/
theorem cert1_signer {C : Committee} (N : Network cfg C) {c : Cert1} (h : Cert1Backed N.trace c) :
    ∃ k, ∃ he : C.honest c.data.epoch k, SentBy (N.trace k (.of he)) (.vote1 ⟨c.data, c.view, k⟩) := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect c.data.epoch q q hq hq
  exact ⟨k, hhon, hcast k hk hhon⟩

/--
A backed `Cert1` names a block an honest node received: its hash, epoch and
height are the block's, and its view is the block's or, after re-votes, a later
one. A later view occurs only for the last block of an epoch.

By induction on the view. An honest signer either voted on the proposal itself,
at its view, or answered a re-vote request, whose certificate is over the same
block at an earlier view.
-/
theorem cert1_origin {C : Committee} (N : Network cfg C) :
    ∀ n (c : Cert1), c.view.toNat ≤ n → Cert1Backed N.trace c →
      ∃ k, ∃ hk : C.Honest k, ∃ m sender p vid, (N.trace k hk m).input = .proposal sender p (some vid)
        ∧ ProposalWellFormed cfg p ∧ BlockValid p ∧ c.data = ⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩
        ∧ p.viewNumber ≤ c.view
        ∧ (p.viewNumber < c.view → IsLastBlock p.blockHeader.blockNumber cfg.epochHeight)
        ∧ (EntersEpoch cfg p → ∃ bc : Cert2, (Cert2Backed N.trace bc ∨ bc = cfg.anchorCert2) ∧ bc.view < p.viewNumber
          ∧ bc.data = p.parentCert.data.toVote2) := by
  intro n
  induction n using Nat.strongRecOn with
  | ind n ih =>
    intro c hle hc
    obtain ⟨k, he, hs⟩ := cert1_signer N hc
    obtain ⟨-, hcase⟩ := vote1_signed N hs he
    rcases hcase with ⟨m, sender, p, vid, hin, hwf, hval, hent, hview, hdata⟩ | ⟨m, sender, r, hin, hwf, hview, hdata⟩
    · refine ⟨k, .of he, m, sender, p, vid, hin, hwf, hval, hdata, ?_, fun hlt => ?_, hent⟩
      · show p.viewNumber.toNat ≤ c.view.toNat
        rw [show c.view = p.viewNumber from hview]; exact Nat.le_refl _
      · exact absurd (show c.view = p.viewNumber from hview) (fun he => Nat.lt_irrefl _ (he ▸ hlt))
    · have hrv : r.cert.view.toNat < c.view.toNat := by
        rw [show c.view = r.view from hview]; exact hwf.1
      obtain ⟨k', hk', m', sender', p, vid, hin', hwf', hval', hdata', hpv, -, hent⟩ :=
        ih r.cert.view.toNat (Nat.lt_of_lt_of_le hrv hle) r.cert (Nat.le_refl _)
          (N.revoteGenuine k (.of he) m sender r hin)
      have hcd : c.data = r.cert.data := hdata
      refine ⟨k', hk', m', sender', p, vid, hin', hwf', hval', hcd.trans hdata', ?_, fun _ => ?_, hent⟩
      · exact Nat.le_of_lt (Nat.lt_of_le_of_lt hpv hrv)
      · have hl := hwf.2.2
        rw [hdata'] at hl
        exact hl

/-- A backed `Cert1` names a block, and its epoch and height are that block's. -/
theorem cert1Backed_block {C : Committee} {N : Network cfg C} {c : Cert1}
    (h : Cert1Backed N.trace c) :
    ∃ b : Proposal, b.viewNumber ≤ c.view ∧ c.data.blockHash = blockHash b
      ∧ c.data.epoch = b.epoch ∧ c.data.blockNumber = b.blockHeader.blockNumber := by
  obtain ⟨-, -, -, -, b, -, -, -, -, hdata, hview, -, -⟩ := cert1_origin N _ c (Nat.le_refl _) h
  rw [hdata]
  exact ⟨b, hview, rfl, rfl, rfl⟩

/--
A backed `Cert1` is at a view after the anchor's.

The proposal it is over has a parent at an earlier view, the anchor's certificate
or a backed one, so by induction on the view no backed certificate is at the
anchor's view or before.
-/
theorem cert1_after_anchor {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg) :
    ∀ n (c : Cert1), c.view.toNat ≤ n → Cert1Backed N.trace c → cfg.anchorView < c.view := by
  intro n
  induction n using Nat.strongRecOn with
  | ind n ih =>
    intro c hle hc
    obtain ⟨k, hk, m, sender, p, vid, hin, hwf, -, -, hview, -⟩ := cert1_origin N _ c (Nat.le_refl _) hc
    have h1 : p.parentCert.view.toNat < p.viewNumber.toNat := hwf.parentEarlier
    have h2 : p.viewNumber.toNat ≤ c.view.toNat := hview
    show cfg.anchorView.toNat < c.view.toNat
    rcases N.parentGenuine k hk m sender p vid hin with ha | hb
    · have h3 : p.parentCert.view.toNat = cfg.anchorView.toNat := by
        rw [ha, hcfg.anchorCertView]
      omega
    · have h4 : cfg.anchorView.toNat < p.parentCert.view.toNat :=
        ih p.parentCert.view.toNat (by omega) p.parentCert (Nat.le_refl _) hb
      omega

/--
Everything a backed `Cert1` gives about the block it certifies: an honest node
received it, it is well formed, the tree holds it, its parent certificate is
backed or the anchor's, and if it opens an epoch a `Cert2` that is backed or the
anchor's is over its parent. The certificate is at the block's view, or at a later one if the block
is the last of its epoch.
-/
theorem cert1_proposal {C : Committee} (N : Network cfg C)
    (hres : Resolves cfg tree N) {c1 : Cert1} (h : Cert1Backed N.trace c1) :
    ∃ p : Proposal, p.viewNumber ≤ c1.view ∧ c1.data.blockHash = blockHash p
      ∧ c1.data.epoch = p.epoch
      ∧ ProposalWellFormed cfg p ∧ tree (blockHash p) = some p
      ∧ (Cert1Backed N.trace p.parentCert ∨ p.parentCert = cfg.anchorCert)
      ∧ (EntersEpoch cfg p → ∃ bc : Cert2, (Cert2Backed N.trace bc ∨ bc = cfg.anchorCert2)
          ∧ bc.view < p.viewNumber
          ∧ bc.data.blockHash = p.parentCert.data.blockHash
          ∧ bc.data.epoch = p.parentCert.data.epoch)
      ∧ c1.data.blockNumber = p.blockHeader.blockNumber
      ∧ (p.viewNumber < c1.view → IsLastBlock p.blockHeader.blockNumber cfg.epochHeight) := by
  obtain ⟨k, hk, m, sender, p, vid, hin, hwf, -, hdata, hview, hlast, hbound⟩ :=
    cert1_origin N _ c1 (Nat.le_refl _) h
  have hheld : ((N.trace k hk).history (m + 1)).HasProposal cfg p :=
    Or.inr (Or.inl ⟨sender, vid, hin ▸ Trace.received_self _ m⟩)
  refine ⟨p, hview, congrArg Vote1Data.blockHash hdata, congrArg Vote1Data.epoch hdata, hwf,
    hres k hk (m + 1) p hheld, ?_, fun hent => ?_, congrArg Vote1Data.blockNumber hdata, hlast⟩
  · exact (N.parentGenuine k hk m sender p vid hin).symm
  · obtain ⟨bc, hbc, hbv, hbd⟩ := hbound hent
    exact ⟨bc, hbc, hbv, by rw [hbd]; rfl, by rw [hbd]; rfl⟩

/-- Two backed `Cert1`s of one epoch at one view carry the same data. -/
theorem cert1_unique (cfg : Config) {C : Committee} (N : Network cfg C) {c c' : Cert1}
    (h : Cert1Backed N.trace c) (h' : Cert1Backed N.trace c')
    (he : c.data.epoch = c'.data.epoch) (hv : c.view = c'.view) :
    c.data.blockHash = c'.data.blockHash := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨q', hq', hcast'⟩ := h'
  obtain ⟨k, hk, hk', hhon⟩ := C.intersect c.data.epoch q q' hq (he ▸ hq')
  have hhon' : C.honest c'.data.epoch k := he ▸ hhon
  have hagree := vote1_agree N (hcast k hk hhon) (hcast' k hk' hhon') hhon he hv
  exact congrArg (fun v : Vote1 => v.data.blockHash) hagree

/-- A backed `Cert2` presupposes a backed `Cert1` over the same block, at the same view. -/
theorem cert2_implies_cert1 (cfg : Config) {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) {c2 : Cert2} (h : Cert2Backed N.trace c2) :
    ∃ c1 : Cert1, Cert1Backed N.trace c1 ∧ c1.view = c2.view
      ∧ c1.data.blockHash = c2.data.blockHash ∧ c1.data.epoch = c2.data.epoch := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect c2.data.epoch q q hq hq
  obtain ⟨-, hgen, c, hview, hdata, n, hc⟩ := vote2_signed N (hcast k hk hhon) hhon
  have hdata' : c2.data = c.data.toVote2 := hdata
  refine ⟨c, ?_, hview.symm, ?_, ?_⟩
  · rcases cert1_held_backed N hc with hanc | hb
    · exfalso
      have : c.view = cfg.anchorView := by rw [hanc]; exact hcfg.anchorCertView
      have hv : c2.view = cfg.anchorView := hview.trans this
      exact absurd (hv ▸ hgen) (Nat.lt_irrefl _)
    · exact hb
  · rw [hdata']; rfl
  · rw [hdata']; rfl

/-- The epoch never decreases from one block to the next. -/
theorem epochOf_le_succ (n : BlockNumber) (h : Nat) :
    (epochOf n h).toNat ≤ (epochOf (n + 1) h).toNat := by
  by_cases hh : h = 0
  · simp [epochOf_eq, hh]
  · by_cases hn : n.toNat = 0
    · rw [BlockNumber.ext (show n.toNat = (0 : BlockNumber).toNat from hn)]
      rw [show (0 : BlockNumber) + 1 = 1 from rfl, epochOf_one h hh]
      exact Nat.le_refl _
    · rw [epochOf_succ n h hh hn]
      split
      · show (epochOf n h).toNat ≤ (epochOf n h).toNat + 1
        exact Nat.le_succ _
      · exact Nat.le_refl _

/-- A backed `Cert1`, or the anchor's, names the epoch its height falls in. -/
theorem cert1_epoch_of_height {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    {c : Cert1} (hc : Cert1Backed N.trace c ∨ c = cfg.anchorCert) :
    c.data.epoch = epochOf c.data.blockNumber cfg.epochHeight := by
  rcases hc with hb | rfl
  · obtain ⟨-, -, -, -, b, -, -, hwf, -, hdata, -, -⟩ := cert1_origin N _ c (Nat.le_refl _) hb
    rw [hdata]; exact hwf.epoch
  · rw [hcfg.anchorCertEpoch, hcfg.anchorCertBlockNumber]

/-- A proposal is in its parent's epoch or a later one. -/
theorem parent_epoch_le {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    {p : Proposal} (hwf : ProposalWellFormed cfg p)
    (hpar : Cert1Backed N.trace p.parentCert ∨ p.parentCert = cfg.anchorCert) {e : EpochNumber}
    (he : e = p.epoch) : p.parentCert.data.epoch.toNat ≤ e.toNat := by
  rw [he, hwf.epoch, ← hwf.height, cert1_epoch_of_height N hcfg hpar]
  exact epochOf_le_succ _ _

/--
No `Cert2` of `c1`'s epoch lies strictly between `pc` and `c1`: every one before
`c1` is at `pc`'s view or earlier, or at the view of a backed certificate over
`pc`'s block that is still before `c1`.

What a vote1 behind `c1` checked of the certificate it builds on, `pc`, the parent
of a proposal or the certificate a re-vote request votes on again. The second case
is the last block of an epoch, whose re-votes certify it again at later views.
-/
def NoGapBelow {C : Committee} (N : Network cfg C) (c1 pc : Cert1) : Prop :=
  ∀ c : Cert2, Cert2Backed N.trace c → c.data.epoch = c1.data.epoch → c.view < c1.view →
    c.view ≤ pc.view
      ∨ ∃ T, Cert1Backed N.trace T ∧ T.data = pc.data ∧ T.data.epoch = c.data.epoch
        ∧ c.view ≤ T.view ∧ T.view < c1.view

/-- A backed `Cert2` is after genesis: its honest signers voted2 only there. -/
theorem cert2_after_anchor {C : Committee} (N : Network cfg C) {c : Cert2}
    (h : Cert2Backed N.trace c) : cfg.anchorView < c.view := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect c.data.epoch q q hq hq
  exact (vote2_signed N (hcast k hk hhon) hhon).2.1

/-- The run starts in the anchor's epoch or the next one. -/
theorem anchor_le_start : cfg.anchorCert.data.epoch.toNat ≤ cfg.startEpoch.toNat := by
  unfold Config.startEpoch
  split
  · exact Nat.le_succ _
  · exact Nat.le_refl _

/-- The block after the anchor is in the epoch the run starts in. -/
theorem epochOf_after_anchor (hcfg : ConfigCoherent cfg) :
    epochOf (cfg.anchorBlock.blockHeader.blockNumber + 1) cfg.epochHeight = cfg.startEpoch := by
  unfold Config.startEpoch
  rw [hcfg.anchorCertEpoch]
  by_cases hh : cfg.epochHeight = 0
  · have hnl : ¬ IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight :=
      fun h => (isLastBlock_iff.mp h).2.1 hh
    rw [ite_eq_right hnl]
    simp [epochOf_eq, hh]
  · by_cases hz : cfg.anchorBlock.blockHeader.blockNumber.toNat = 0
    · have hnl : ¬ IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight :=
        fun h => (isLastBlock_iff.mp h).1 hz
      rw [ite_eq_right hnl, BlockNumber.ext hz]
      exact epochOf_one _ hh
    · exact epochOf_succ _ _ hh hz

/--
A backed `Cert1` is of the epoch the run starts in or a later one.

Its proposal's parent is the anchor's certificate or a backed one at an earlier
view, a proposal on the anchor is in the epoch the run starts in
(`epochOf_after_anchor`), and a proposal is in its parent's epoch or a later one
(`parent_epoch_le`).
-/
theorem cert1_epoch_after_start {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg) :
    ∀ n (c : Cert1), c.view.toNat ≤ n → Cert1Backed N.trace c →
      cfg.startEpoch.toNat ≤ c.data.epoch.toNat := by
  intro n
  induction n using Nat.strongRecOn with
  | ind n ih =>
    intro c hle hc
    obtain ⟨k, hk, m, sender, p, vid, hin, hwf, -, hdata, hview, -⟩ := cert1_origin N _ c (Nat.le_refl _) hc
    have hce : c.data.epoch = p.epoch := by rw [hdata]
    have hpar := (N.parentGenuine k hk m sender p vid hin).symm
    have hpe := parent_epoch_le N hcfg hwf hpar hce
    rcases hpar with hb | ha
    · have h1 : p.parentCert.view.toNat < p.viewNumber.toNat := hwf.parentEarlier
      have h2 : p.viewNumber.toNat ≤ c.view.toNat := hview
      have := ih p.parentCert.view.toNat (by omega) p.parentCert (Nat.le_refl _) hb
      omega
    · rw [hce, hwf.epoch, ← hwf.height, ha, hcfg.anchorCertBlockNumber, epochOf_after_anchor hcfg]
      exact Nat.le_refl _

/--
The block a backed `Cert1` is over is at a view after the anchor's: it is a proposal
whose parent is the anchor's certificate or a backed one.
-/
theorem backed_block_after_anchor {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hcf : CollisionFree) {c : Cert1} (hc : Cert1Backed N.trace c) {b : Block}
    (hb : c.data.blockHash = blockHash b) : cfg.anchorView < b.viewNumber := by
  obtain ⟨k, hk, m, sender, p, vid, hin, hwf, -, hdata, -, -⟩ := cert1_origin N _ c (Nat.le_refl _) hc
  have hpb : p = b := hcf p b (by rw [← hb, hdata])
  subst hpb
  have h1 : p.parentCert.view.toNat < p.viewNumber.toNat := hwf.parentEarlier
  show cfg.anchorView.toNat < p.viewNumber.toNat
  rcases N.parentGenuine k hk m sender p vid hin with ha | hpb
  · have : p.parentCert.view.toNat = cfg.anchorView.toNat := by rw [ha, hcfg.anchorCertView]
    omega
  · have : cfg.anchorView.toNat < p.parentCert.view.toNat :=
      cert1_after_anchor N hcfg _ _ (Nat.le_refl _) hpb
    omega

/--
A checked timeout certificate of `c`'s epoch, for a view at or after `c`'s, whose
lock lets `pc` through, puts `c` no later than `pc`, or no later than the lock if
the lock is over `pc`'s block.

The quorum behind the timeout certificate and the one behind `c` share an honest
node. It voted2 at `c.view` before it timed out, so its lock is no earlier than
`c` in lock order (`timeout_lock_covers`), and the certificate's lock is no
earlier than its. `pc` is of `c`'s epoch or an earlier one, so a lock that lets it
through is of that same epoch, at `pc`'s view or before it, or over the same
block.
-/
theorem no_gap_of_timeout {C : Committee} (N : Network cfg C) {c : Cert2} {tc : TimeoutCert}
    {pc : Cert1} (hc2 : Cert2Backed N.trace c) (htc : TimeoutCertBacked N.trace tc)
    (hep : tc.data.epoch = c.data.epoch) (hv : c.view ≤ tc.view)
    (hallow : LockAllows tc.data.lock pc c.data.epoch) (hpce : pc.data.epoch.toNat ≤ c.data.epoch.toNat) :
    c.view ≤ pc.view ∨ (tc.data.lock.data = pc.data ∧ tc.data.lock.data.epoch = c.data.epoch
      ∧ c.view ≤ tc.data.lock.view) := by
  obtain ⟨qt, hqt, hcastt⟩ := htc
  obtain ⟨q2, hq2, hcast2⟩ := hc2
  obtain ⟨k, hkt, hk2, hhon⟩ := C.intersect _ qt q2 hqt (by rw [hep]; exact hq2)
  obtain ⟨tv, ⟨-, htvv, htve, htvl⟩, hst⟩ := hcastt k hkt hhon
  have hhon2 : C.honest c.data.epoch k := hep ▸ hhon
  have hcov := timeout_lock_covers N (hcast2 k hk2 hhon2) hst hhon2 (htve ▸ hhon) (by
    show c.view.toNat ≤ tv.view.toNat
    rw [show tv.view = tc.view from htvv]; exact hv)
  -- Read every comparison as numbers.
  have e1 : ∀ a b : EpochNumber, a < b → a.toNat < b.toNat := fun _ _ h => h
  have e2 : ∀ a b : EpochNumber, a = b → a.toNat = b.toNat := fun _ _ h => congrArg _ h
  simp only [LockLE, EpochViewLE, LockAllows] at htvl hallow hcov
  show c.view.toNat ≤ pc.view.toNat ∨ (tc.data.lock.data = pc.data
    ∧ tc.data.lock.data.epoch = c.data.epoch ∧ c.view.toNat ≤ tc.data.lock.view.toNat)
  have hcv : c.view.toNat ≤ tv.data.lock.view.toNat ∨ c.data.epoch.toNat < tv.data.lock.data.epoch.toNat := by
    rcases hcov with h | ⟨h1, h2⟩
    · exact Or.inr (e1 _ _ h)
    · exact Or.inl h2
  have hce : c.data.epoch.toNat ≤ tv.data.lock.data.epoch.toNat := by
    rcases hcov with h | ⟨h1, -⟩
    · exact Nat.le_of_lt (e1 _ _ h)
    · exact Nat.le_of_eq (e2 _ _ h1)
  rcases htvl with hlt | ⟨heq, hlv⟩ <;> rcases hallow with hlt' | ⟨heq', hcase⟩
  · have := e1 _ _ hlt; have := e1 _ _ hlt'; omega
  · have := e1 _ _ hlt; have := e2 _ _ heq'; omega
  · have := e2 _ _ heq; have := e1 _ _ hlt'; omega
  · have h3 := e2 _ _ heq
    have h4 := e2 _ _ heq'
    have hv' : c.view.toNat ≤ tv.data.lock.view.toNat := by
      rcases hcv with h | h
      · exact h
      · omega
    have hlv' : tv.data.lock.view.toNat ≤ tc.data.lock.view.toNat := hlv
    rcases hcase with h | h
    · exact Or.inl (Nat.le_trans hv' (Nat.le_trans hlv' h))
    · exact Or.inr ⟨h, EpochNumber.ext (by omega), Nat.le_trans hv' hlv'⟩

/--
The two cases of `no_gap_of_timeout`, as `NoGapBelow` asks for them: the lock is
backed, or it is the anchor's, and then `c` would be at the anchor's view or before.
-/
theorem noGap_of_evidence {C : Committee} (N : Network cfg C) {c1 pc : Cert1} {c : Cert2}
    {tc : TimeoutCert} (hc2 : Cert2Backed N.trace c) (htc : TimeoutCertBacked N.trace tc)
    (hchk : TimeoutLockChecked N.trace cfg tc) (hep : tc.data.epoch = c.data.epoch)
    (hv : c.view ≤ tc.view) (htv : tc.view.toNat + 1 = c1.view.toNat)
    (hallow : LockAllows tc.data.lock pc c.data.epoch) (hpce : pc.data.epoch.toNat ≤ c.data.epoch.toNat)
    (hcfg : ConfigCoherent cfg) :
    c.view ≤ pc.view
      ∨ ∃ T, Cert1Backed N.trace T ∧ T.data = pc.data ∧ T.data.epoch = c.data.epoch
        ∧ c.view ≤ T.view ∧ T.view < c1.view := by
  rcases no_gap_of_timeout N hc2 htc hep hv hallow hpce with h | ⟨hd, hde, hle⟩
  · exact Or.inl h
  · obtain ⟨hgen, hlv⟩ := hchk
    rcases hgen with hanc | hb
    · exfalso
      have h0 : c.view.toNat ≤ cfg.anchorView.toNat := by
        have : c.view.toNat ≤ tc.data.lock.view.toNat := hle
        rw [hanc, hcfg.anchorCertView] at this; exact this
      have : cfg.anchorView.toNat < c.view.toNat := cert2_after_anchor N hc2
      omega
    · refine Or.inr ⟨tc.data.lock, hb, hd, hde, hle, ?_⟩
      show tc.data.lock.view.toNat < c1.view.toNat
      have : tc.data.lock.view.toNat ≤ tc.view.toNat := hlv
      omega

/--
One step back from a backed `Cert1`, with no committed view skipped.

An honest signer of `c1` either voted on a proposal at `c1`'s view, and the step
is to its parent, or answered a re-vote request, and the step is to the
certificate it votes on again, over the same block at an earlier view. Either way
no `Cert2` of the epoch is skipped (`NoGapBelow`): without timeout evidence the
step covers no view, and with it the honest voter checked the evidence's lock
(`SafeParent`, `SafeRevote`). A proposal opening an epoch names its parent at the
parent's own view (`OpensEpochJustified`).
-/
theorem cert1_step {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hres : Resolves cfg tree N) {c1 : Cert1} (h : Cert1Backed N.trace c1) :
    (∃ p : Proposal, c1.view = p.viewNumber ∧ c1.data.blockHash = blockHash p
        ∧ c1.data.epoch = p.epoch
        ∧ ProposalWellFormed cfg p ∧ tree (blockHash p) = some p
        ∧ (Cert1Backed N.trace p.parentCert ∨ p.parentCert = cfg.anchorCert)
        ∧ (EntersEpoch cfg p → ∃ j, ∃ hj : C.Honest j, ∃ n q c2, ((N.trace j hj).history n).HasProposal cfg q
            ∧ q.viewNumber = p.parentCert.view ∧ p.parentCert.data.blockHash = blockHash q
            ∧ (((N.trace j hj).history n).HasCert2 c2 ∨ c2 = cfg.anchorCert2) ∧ c2.view < p.viewNumber
            ∧ c2.data = p.parentCert.data.toVote2)
        ∧ NoGapBelow N c1 p.parentCert)
      ∨ ∃ pc, Cert1Backed N.trace pc ∧ pc.data = c1.data ∧ pc.view < c1.view
        ∧ IsLastBlock pc.data.blockNumber cfg.epochHeight ∧ NoGapBelow N c1 pc := by
  obtain ⟨k, he, hs⟩ := cert1_signer N h
  have hk : C.Honest k := .of he
  rcases vote1_safe N hs he with ⟨m, sender, p, vid, hin, hwf, hsafe, ⟨n, hopen⟩, hview, hdata⟩
    | ⟨m, sender, r, hin, hwf, hsafe, hview, hdata⟩
  · have hheld : ((N.trace k hk).history (m + 1)).HasProposal cfg p :=
      Or.inr (Or.inl ⟨sender, vid, hin ▸ Trace.received_self _ m⟩)
    have hpe : c1.data.epoch = p.epoch := congrArg Vote1Data.epoch hdata
    refine Or.inl ⟨p, hview, congrArg Vote1Data.blockHash hdata, hpe,
      hwf, hres k hk (m + 1) p hheld, (N.parentGenuine k hk m sender p vid hin).symm,
      fun hent => ?_, fun c hc2 hep hlt => ?_⟩
    · obtain ⟨⟨q, hq, hqv, hqh⟩, c2, hc2, hc2v, hc2d⟩ := hopen hent
      exact ⟨k, hk, n, q, c2, hq, hqv, hqh, hc2, hc2v, hc2d⟩
    · have h1 : c.view.toNat < c1.view.toNat := hlt
      have h2 : c1.view.toNat = p.viewNumber.toNat := congrArg ViewNumber.toNat hview
      rcases hwf.covered with hnext | ⟨tc, hte, htv⟩
      · have : p.parentCert.view.toNat + 1 = p.viewNumber.toNat := congrArg ViewNumber.toNat hnext.2
        exact Or.inl (show c.view.toNat ≤ p.parentCert.view.toNat by omega)
      · obtain ⟨hep', hallow⟩ := hsafe tc hte
        have htv' : tc.view.toNat + 1 = p.viewNumber.toNat := congrArg ViewNumber.toNat htv
        obtain ⟨htcb, hchk⟩ := N.evidenceGenuine k hk m sender p vid tc hin hte
        refine noGap_of_evidence N hc2 htcb hchk (by rw [hep', hep, hpe])
          (show c.view.toNat ≤ tc.view.toNat by omega) (by omega) (by rw [hep, hpe]; exact hallow) ?_ hcfg
        exact parent_epoch_le N hcfg hwf (N.parentGenuine k hk m sender p vid hin).symm
          (hep.trans hpe)
  · have hcd : c1.data = r.cert.data := hdata
    refine Or.inr ⟨r.cert, N.revoteGenuine k hk m sender r hin, hcd.symm, ?_, ?_,
      fun c hc2 hep hlt => ?_⟩
    · rw [show c1.view = r.view from hview]; exact hwf.1
    · exact hwf.2.2
    · have h1 : c.view.toNat < c1.view.toNat := hlt
      have h2 : c1.view.toNat = r.view.toNat := congrArg ViewNumber.toNat hview
      rcases hwf.2.1 with hnext | ⟨tc, hte, htv⟩
      · have : r.cert.view.toNat + 1 = r.view.toNat := congrArg ViewNumber.toNat hnext.2
        exact Or.inl (show c.view.toNat ≤ r.cert.view.toNat by omega)
      · obtain ⟨hep', hallow⟩ := hsafe tc hte
        have htv' : tc.view.toNat + 1 = r.view.toNat := congrArg ViewNumber.toNat htv
        obtain ⟨htcb, hchk⟩ := N.revoteEvidenceGenuine k hk m sender r tc hin hte
        exact noGap_of_evidence N hc2 htcb hchk (by rw [hep', hep, hcd])
          (show c.view.toNat ≤ tc.view.toNat by omega) (by omega) (by rw [hep, hcd]; exact hallow)
          (by rw [hep, hcd]; exact Nat.le_refl _) hcfg

/-- A backed `Cert1` is not at the anchor's view (`cert1_after_anchor`). -/
theorem cert1_not_at_anchor {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    {c1 : Cert1} (h : Cert1Backed N.trace c1) : c1.view ≠ cfg.anchorView := fun hz => by
  have := cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) h
  rw [hz] at this
  exact Nat.lt_irrefl _ this

/-- The anchor is in the tree: every honest node holds it, and a backed certificate supplies one. -/
theorem anchor_in_tree {C : Committee} (N : Network cfg C) (hres : Resolves cfg tree N)
    {c1 : Cert1} (hc1 : Cert1Backed N.trace c1) :
    tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock := by
  obtain ⟨k, hk, -⟩ := cert1_signer N hc1
  exact hres k (.of hk) 0 cfg.anchorBlock (Or.inl rfl)

end NewProtocol
