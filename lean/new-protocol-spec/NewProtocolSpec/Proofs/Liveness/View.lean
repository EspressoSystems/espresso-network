module

public import NewProtocolSpec.Proofs.Liveness.Stable
public import NewProtocolSpec.Proofs.Decide

/-!
# A view with an honest leader, in a stable epoch

In a stable epoch `E` (`Liveness.Stable`), take a view `w` whose leader for `E` is
honest in `E` and that no honest node reached, nor the view before it, by the time the
epoch became stable. The argument is the single-epoch one, for that epoch:

* Every honest vote1 at `w` or later is for epoch `E`, and every certificate
  there is of `E` (`Liveness.late_vote1`, `Liveness.late_cert_epoch`).
* Every honest node reaches `w` within `2Δ` of the first, and stays in it while
  nobody holds a certificate at `w` or later (`Liveness.reach_spread`,
  `Liveness.view_is_w`).
* The leader acts in `E`: it proposes on its lock, opens `E` on the last block of
  the epoch before, or asks for a re-vote of `E`'s last block
  (`Liveness.leader_acts`).
* The members vote1, the members of `E` lock on the certificate, the members
  vote2, and every node honest in `E`, a member or not, gets the block and decides `w` or
  a later view (`Liveness.late_decide`). A re-vote cannot get that far: its
  `Cert2` would commit `E`'s last block, which a stable epoch rules out.
-/

@[expose] public section

namespace NewProtocol
namespace Liveness

open History Lists

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}
variable {N : TimedNetwork cfg leader C}

/-- A step after `T` is at or after the cut at `T`. -/
theorem cut_le_of_lt (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n) {k : PubKey}
    {hk : C.Honest k} {T i : Nat} (hT : T < N.time k hk i) : cut N hu k hk T ≤ i :=
  least_le (hu k hk T) hT

/-- No honest node reached view `u` by time `t0`. -/
def Late (N : TimedNetwork cfg leader C) (t0 : Nat) (u : ViewNumber) : Prop := ¬ ReachedBy N t0 u

theorem late_mono {t0 : Nat} {u u' : ViewNumber} (hl : Late N t0 u) (hle : u ≤ u') : Late N t0 u' :=
  fun ⟨k, hk, n, hn, v, hv, hv'⟩ => hl ⟨k, hk, n, hn, v, hv, Nat.le_trans hle hv'⟩

/-- A vote1 of an epoch its node is honest in is cast in a view the node has reached. -/
theorem vote1_reached {k : PubKey} {hk : C.Honest k} {i : Nat} {v : Vote1}
    (hi : Output.send (.vote1 v) ∈ (N.trace k hk i).output) (he : C.honest v.data.epoch k) :
    Reached N k hk (i + 1) v.view := by
  obtain ⟨⟨u, hu, hle⟩, -⟩ := (N.protocol k hk (i + 1)).vote1Leader i v ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self i)), hi⟩ he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hu
  exact ⟨u, hu, hle⟩

/-- A vote1 at a late view is cast after `t0`. -/
theorem late_vote_time {t0 : Nat} {k : PubKey} {hk : C.Honest k} {i : Nat} {v : Vote1}
    (hi : Output.send (.vote1 v) ∈ (N.trace k hk i).output) (he : C.honest v.data.epoch k)
    (hl : Late N t0 v.view) : t0 < N.time k hk i :=
  Nat.lt_of_not_le fun hle => hl ⟨k, hk, i + 1,
    fun _ hj => Nat.le_trans (time_mono N k hk (Nat.le_of_lt_succ hj)) hle, vote1_reached hi he⟩

section Late

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ) {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
include hcfg hcf hs hst

omit hcfg hcf in
/-- A history from a cut at or after `t0`'s, of a node honest in `E` or later, is in epoch `E`. -/
theorem late_inEpoch {k : PubKey} {hk : C.Honest k} (hkE : C.HonestFrom E k) {i : Nat}
    (hT : t0 < N.time k hk i) : ((N.trace k hk).history (i + 1)).InEpoch cfg E :=
  stable_inEpoch hst hs.timeUnbounded hkE
    (Nat.le_succ_of_le (cut_le_of_lt hs.timeUnbounded hT))

omit hcfg hcf in
/-- A node not honest in `E` or later signs nothing after `t0` it needs to be in its epoch for. -/
theorem late_retired {k : PubKey} {hk : C.Honest k} (hkE : ¬ C.HonestFrom E k) {i : Nat}
    (hT : t0 < N.time k hk i) {e : EpochNumber} (he : C.honest e k) :
    ¬ NotBehind cfg ((N.trace k hk).history (i + 1)) e :=
  stable_retired hst hs.timeUnbounded hkE (Nat.le_succ_of_le (cut_le_of_lt hs.timeUnbounded hT)) he

/--
A proposal an honest node in `E` may vote for, or send, is of epoch `E`.

It is of `E` or later, since the node is in `E`. A later epoch would need a
certified parent of a later epoch, which a stable epoch has none of, or would open
`E + 1` behind a `Cert2` over `E`'s last block the voter holds, which a stable
epoch rules out.
-/
theorem proposal_epoch_of {k : PubKey} {hk : C.Honest k} {n : Nat} {p : Proposal}
    (hE : ((N.trace k hk).history n).InEpoch cfg E) (hwf : ProposalWellFormed cfg p)
    (hpar : Cert1Backed N.trace p.parentCert ∨ p.parentCert = cfg.anchorCert)
    (hopen : OpensEpochJustified cfg ((N.trace k hk).history n) p)
    (hnb : NotBehind cfg ((N.trace k hk).history n) p.epoch) : p.epoch = E := by
  have hge : E.toNat ≤ p.epoch.toNat := hnb E hE
  have hpce : p.parentCert.data.epoch.toNat ≤ E.toNat := by
    rcases hpar with hb | ha
    · exact stable_no_later hcfg hcf hs hst hb
    · rw [ha]; exact stable_anchor hst
  have hpe : p.epoch.toNat ≤ p.parentCert.data.epoch.toNat + 1 := by
    rw [hwf.epoch, ← hwf.height, cert1_epoch_of_height N.toNetwork hcfg hpar]
    by_cases hh : cfg.epochHeight = 0
    · simp [epochOf_eq, hh]
    · by_cases hz : p.parentCert.data.blockNumber.toNat = 0
      · rw [BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = (0 : BlockNumber).toNat from hz)]
        rw [show (0 : BlockNumber) + 1 = 1 from rfl, epochOf_one _ hh]; omega
      · rw [epochOf_succ _ _ hh hz]
        split
        · exact Nat.le_refl _
        · exact Nat.le_succ _
  refine EpochNumber.ext (Nat.le_antisymm ?_ hge)
  by_cases hsame : p.epoch.toNat ≤ p.parentCert.data.epoch.toNat
  · omega
  -- The proposal opens an epoch: its parent is the last block of the one before.
  have hent : EntersEpoch cfg p := by
    have hh : cfg.epochHeight ≠ 0 := by
      intro hh
      have : p.epoch.toNat = 0 := by rw [hwf.epoch]; simp [epochOf_eq, hh]
      omega
    by_cases hz : p.parentCert.data.blockNumber.toNat = 0
    · exfalso
      have h1 : p.epoch = epochOf 0 cfg.epochHeight := by
        rw [hwf.epoch, ← hwf.height,
          BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = (0 : BlockNumber).toNat from hz)]
        exact epochOf_one _ hh
      have h2 : p.parentCert.data.epoch = epochOf 0 cfg.epochHeight := by
        rw [cert1_epoch_of_height N.toNetwork hcfg hpar,
          BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = (0 : BlockNumber).toNat from hz)]
      rw [h1, ← h2] at hsame
      exact hsame (Nat.le_refl _)
    · by_cases hlast : IsLastBlock p.parentCert.data.blockNumber cfg.epochHeight
      · show IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
        rw [← hwf.height]; simpa using hlast
      · exfalso
        apply hsame
        rw [hwf.epoch, ← hwf.height, epochOf_succ _ _ hh hz, ite_eq_right hlast,
          cert1_epoch_of_height N.toNetwork hcfg hpar]
        exact Nat.le_refl _
  have hlast0 : IsLastBlock p.parentCert.data.blockNumber cfg.epochHeight := by
    have h := hent
    unfold EntersEpoch at h
    rwa [← hwf.height, BlockNumber.add_sub_cancel] at h
  by_cases hpe' : p.parentCert.data.epoch = E
  · -- Opening `E + 1`: the voter holds `E`'s last block and a `Cert2` over it.
    exfalso
    obtain ⟨⟨q, hq, hqv, hqh⟩, c2, hc2, -, hc2d⟩ := hopen hent
    rcases hc2 with hc2 | hc2a
    case inr =>
      -- The anchor's `Cert2`: the parent is the anchor, which then ends the epoch
      -- before the one the run starts in, so it is not `E`.
      have hh : cfg.anchorCert.data.blockHash = p.parentCert.data.blockHash := by
        have := congrArg Vote2Data.blockHash hc2d
        rw [hc2a] at this
        exact this
      rcases hpar with hb | ha
      · have := backed_block_after_anchor N.toNetwork hcfg hcf hb (by rw [← hh, hcfg.anchorCertBlock])
        exact Nat.lt_irrefl _ this
      · have hlastA : IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight := by
          rw [← hcfg.anchorCertBlockNumber, ← ha]; exact hlast0
        have hs0 : cfg.startEpoch.toNat ≤ E.toNat := stable_start hst
        have hst1 : cfg.startEpoch = cfg.anchorCert.data.epoch + 1 := by
          unfold Config.startEpoch; rw [ite_eq_left hlastA]
        rw [hst1, ← ha, hpe'] at hs0
        exact absurd hs0 (by show ¬ E.toNat + 1 ≤ E.toNat; omega)
    rcases hpar with hb | ha
    · obtain ⟨b, -, hbh, hbe, hbn⟩ := cert1Backed_block hb
      have hbq : b = q := hcf b q (by rw [← hbh, hqh])
      subst hbq
      have hlast : IsLastBlock b.blockHeader.blockNumber cfg.epochHeight := by
        rw [← hbn]; exact hlast0
      have hcq : Commits c2 b := by
        refine ⟨?_, ?_⟩
        · obtain ⟨cc, hccb, hccv, hcch, -⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg
            (cert2_held_backed N.toNetwork hc2)
          obtain ⟨b', hb'v, hb'h, -, -⟩ := cert1Backed_block hccb
          have hb'b : b' = b := hcf b' b (by rw [← hb'h, hcch, hc2d]; exact hbh)
          rw [← hccv, ← hb'b]; exact hb'v
        · rw [hc2d]
          show (⟨p.parentCert.data.blockHash, p.parentCert.data.epoch, p.parentCert.data.blockNumber⟩
            : Vote2Data) = _
          rw [hbh, hbe, hbn]
      exact stable_no_commit hs hst hq hc2 hcq hlast (by rw [← hbe]; exact hpe')
    · -- The parent is the anchor, which ends `E`: the voter holds it with a `Cert2`.
      have hqa : q = cfg.anchorBlock := hcf q _ (by rw [← hqh, ha, hcfg.anchorCertBlock])
      subst hqa
      have hlast : IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight := by
        rw [← hcfg.anchorCertBlockNumber, ← ha]; exact hlast0
      have hcq : Commits c2 cfg.anchorBlock := by
        refine ⟨Nat.le_of_lt (cert2_after_anchor N.toNetwork (cert2_held_backed N.toNetwork hc2)), ?_⟩
        rw [hc2d, ha]
        show (⟨cfg.anchorCert.data.blockHash, cfg.anchorCert.data.epoch, cfg.anchorCert.data.blockNumber⟩
          : Vote2Data) = _
        rw [hcfg.anchorCertBlock, hcfg.anchorCertBlockNumber, hcfg.anchorBlockEpoch]
      exact stable_no_commit hs hst hq hc2 hcq hlast (by rw [hcfg.anchorBlockEpoch, ← ha]; exact hpe')
  · have : p.parentCert.data.epoch.toNat ≠ E.toNat := fun h => hpe' (EpochNumber.ext h)
    omega

/-- A proposal an honest node votes for, at a late view, is of epoch `E`. -/
theorem late_proposal_epoch {k : PubKey} {hk : C.Honest k} (hkE : C.HonestFrom E k) {i : Nat} {v : Vote1}
    (hi : Output.send (.vote1 v) ∈ (N.trace k hk i).output) (he : C.honest v.data.epoch k)
    (hl : Late N t0 v.view) {sender : PubKey} {p : Proposal} {vid : VidShare}
    (hrec : ((N.trace k hk).history (i + 1)).Received (.proposal sender p (some vid)))
    (hwf : ProposalWellFormed cfg p)
    (hopen : OpensEpochJustified cfg ((N.trace k hk).history (i + 1)) p)
    (hnb : NotBehind cfg ((N.trace k hk).history (i + 1)) p.epoch) : p.epoch = E := by
  obtain ⟨j, -, hj⟩ := (Trace.received_history _).mp hrec
  exact proposal_epoch_of hcfg hcf hs hst (late_inEpoch hs hst hkE (late_vote_time hi he hl)) hwf
    (N.parentGenuine k hk j sender p vid hj).symm hopen hnb

/--
Every honest vote1 at a late view is for epoch `E`: on a proposal of `E` its
leader of the view sent, or answering a re-vote request of `E`'s last block its
leader sent.
-/
theorem late_vote1 {k : PubKey} {hk : C.Honest k} {i : Nat} {v : Vote1}
    (hi : Output.send (.vote1 v) ∈ (N.trace k hk i).output) (he : C.honest v.data.epoch k)
    (hl : Late N t0 v.view) :
    (∃ sender p vid, ((N.trace k hk).history (i + 1)).Received (.proposal sender p (some vid))
        ∧ leader E v.view = some sender ∧ Vote1For v p ∧ p.epoch = E)
      ∨ ∃ sender r, ((N.trace k hk).history (i + 1)).Received (.revote sender r)
        ∧ leader E v.view = some sender ∧ Vote1Again v r ∧ r.cert.data.epoch = E := by
  have hget := Trace.history_getElem? (N.trace k hk) (Nat.lt_succ_self i)
  obtain ⟨-, hcase⟩ := (N.protocol k hk (i + 1)).vote1Leader i v ⟨_, hget, hi⟩ he
  obtain ⟨-, hjust⟩ := (N.protocol k hk (i + 1)).vote1Justified i v ⟨_, hget, hi⟩ he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hcase hjust
  -- A retired voter is past the vote's epoch.
  have hkE : C.HonestFrom E k := by
    refine Classical.byContradiction fun hkE => late_retired hs hst hkE (late_vote_time hi he hl) he ?_
    rcases hcase with ⟨_, p, _, _, _, hfor, hnb⟩ | ⟨_, r, _, _, hfor, hnb⟩
    · rw [show v.data.epoch = p.epoch from congrArg Vote1Data.epoch hfor.2]; exact hnb
    · rw [show v.data.epoch = r.cert.data.epoch from congrArg Vote1Data.epoch hfor.2]; exact hnb
  have hE := late_inEpoch hs hst hkE (late_vote_time hi he hl)
  rcases hcase with ⟨sender, p, vid, hrec, hlead, hfor, hnb⟩ | ⟨sender, r, hrec, hlead, hfor, hnb⟩
  · -- The vote1 is for this proposal, which the signing rules checked.
    rcases hjust with ⟨s0, p0, vid0, hrec0, hwf0, -, -, hopen0, hfor0⟩ | ⟨s0, r0, hrec0, hwf0, -, hfor0⟩
    · have hpp : p = p0 := hcf p p0 (congrArg Vote1Data.blockHash (hfor.2.symm.trans hfor0.2))
      subst hpp
      have hpe := late_proposal_epoch hcfg hcf hs hst hkE hi he hl hrec0 hwf0 hopen0 hnb
      refine Or.inl ⟨sender, p, vid, hrec, ?_, hfor, hpe⟩
      rw [← hpe, show v.view = p.viewNumber from hfor.1]; exact hlead
    · -- A re-vote naming this block would be at a later view than the block's own.
      exfalso
      obtain ⟨j, -, hj⟩ := (Trace.received_history _).mp hrec0
      obtain ⟨b, hbv, hbh, -, -⟩ := cert1Backed_block (N.revoteGenuine k hk j s0 r0 hj)
      have hd : r0.cert.data.blockHash = blockHash p :=
        congrArg Vote1Data.blockHash (hfor0.2.symm.trans hfor.2)
      have hbp : b = p := hcf b p (by rw [← hbh, hd])
      subst hbp
      have h1 : b.viewNumber.toNat ≤ r0.cert.view.toNat := hbv
      have h2 : r0.cert.view.toNat < r0.view.toNat := hwf0.1
      have h3 : v.view = b.viewNumber := hfor.1
      have h4 : v.view = r0.view := hfor0.1
      have : b.viewNumber.toNat = r0.view.toNat := by rw [← h3, h4]
      omega
  · have hrb : Cert1Backed N.trace r.cert := by
      obtain ⟨j, -, hj⟩ := (Trace.received_history _).mp hrec
      exact N.revoteGenuine k hk j sender r hj
    have hre : r.cert.data.epoch = E :=
      EpochNumber.ext (Nat.le_antisymm (stable_no_later hcfg hcf hs hst hrb) (hnb E hE))
    exact Or.inr ⟨sender, r, hrec, by rw [← hre, show v.view = r.view from hfor.1]; exact hlead,
      hfor, hre⟩

/-- A backed `Cert1` at a late view is of epoch `E`. -/
theorem late_cert_epoch {c : Cert1} (hc : Cert1Backed N.trace c) (hl : Late N t0 c.view) :
    c.data.epoch = E := by
  obtain ⟨k, he, i, hi⟩ := cert1_signer N.toNetwork hc
  rcases late_vote1 hcfg hcf hs hst hi he hl with ⟨_, p, _, _, _, hfor, hpe⟩ | ⟨_, r, _, _, hfor, hre⟩
  · rw [show c.data = _ from hfor.2]; exact hpe
  · rw [show c.data = _ from hfor.2]; exact hre

omit hs hst hcfg hcf in
/-- A backed `Cert1` carries the data of the block it names. -/
theorem cert1_data_eq {c : Cert1} (hc : Cert1Backed N.trace c) :
    ∃ b : Block, b.viewNumber ≤ c.view ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩ := by
  obtain ⟨b, hv, hh, he, hn⟩ := cert1Backed_block hc
  refine ⟨b, hv, ?_⟩
  obtain ⟨⟨h1, e1, n1⟩, v1⟩ := c
  simp only at hh he hn ⊢
  rw [hh, he, hn]

/-- Two backed `Cert1`s at one late view carry the same data. -/
theorem late_cert_data {c c' : Cert1} (hc : Cert1Backed N.trace c) (hc' : Cert1Backed N.trace c')
    (hv : c.view = c'.view) (hl : Late N t0 c.view) : c.data = c'.data := by
  have he : c.data.epoch = c'.data.epoch := by
    rw [late_cert_epoch hcfg hcf hs hst hc hl, late_cert_epoch hcfg hcf hs hst hc' (hv ▸ hl)]
  have hh := cert1_unique cfg N.toNetwork hc hc' he hv
  obtain ⟨b, -, hb⟩ := cert1_data_eq hc
  obtain ⟨b', -, hb'⟩ := cert1_data_eq hc'
  have hbb : b = b' := hcf b b' (by
    have h1 := congrArg Vote1Data.blockHash hb
    have h2 := congrArg Vote1Data.blockHash hb'
    simp only at h1 h2
    rw [← h1, ← h2, hh])
  rw [hb, hb', hbb]

/--
A block an honest node holds a `Cert2` over, at a late view, is not the last of its
epoch, and the `Cert2` is at the block's own view.

Only the last block of an epoch is certified at a later view than its own, by a
re-vote, and the last block of `E` is never held with a `Cert2` in a stable epoch.
-/
theorem late_commit {j : PubKey} {hj : C.Honest j} {n : Nat} {q : Block} {c2 : Cert2}
    (hq : ((N.trace j hj).history n).HasProposal cfg q) (hc2 : ((N.trace j hj).history n).HasCert2 c2)
    (hcq : Commits c2 q) (hl : Late N t0 c2.view) :
    q.viewNumber = c2.view ∧ ¬ IsLastBlock q.blockHeader.blockNumber cfg.epochHeight := by
  obtain ⟨cc, hccb, hccv, hcch, -⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg (cert2_held_backed N.toNetwork hc2)
  have hcce := late_cert_epoch hcfg hcf hs hst hccb (by rw [hccv]; exact hl)
  obtain ⟨-, -, -, -, b0, -, -, -, -, hd0, hv0, hlast0, -⟩ := cert1_origin N.toNetwork _ cc (Nat.le_refl _) hccb
  have hb0q : b0 = q := hcf b0 q (by
    have h1 := congrArg Vote1Data.blockHash hd0
    have h2 := congrArg Vote2Data.blockHash hcq.2
    simp only at h1 h2
    rw [← h1, hcch, h2])
  subst hb0q
  have hnl : ¬ IsLastBlock b0.blockHeader.blockNumber cfg.epochHeight := fun hlast =>
    stable_no_commit hs hst hq hc2 hcq hlast (by
      have h1 := congrArg Vote1Data.epoch hd0
      simp only at h1
      rw [← h1, hcce])
  refine ⟨?_, hnl⟩
  rw [← hccv]
  exact ViewNumber.le_antisymm hv0 (Nat.le_of_not_lt fun hlt => hnl (hlast0 hlt))

end Late

/-- The first time a late view is reached is after `t0`. -/
theorem late_firstReach {t0 : Nat} {u : ViewNumber} (hlu : Late N t0 u) (hex : ∃ T, ReachedBy N T u) :
    t0 < firstReach N hex := by
  refine Nat.lt_of_not_le fun hle => hlu ?_
  obtain ⟨j, hj, n', hn', hr⟩ := least_spec hex
  exact ⟨j, hj, n', fun i hi => Nat.le_trans (hn' i hi) hle, hr⟩

/-! ## Every honest node reaches `w`, and stays there -/

section Spread

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ) {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
include hcfg hcf hs hst

omit hcf hst hcfg in
/-- An epoch change a node took reaches every node honest in the epoch that ended or later, with the same `Cert2`. -/
theorem tookEpochChange_spread {j : PubKey} {hj : C.Honest j} {n : Nat} {c1 : Cert1} {c2 : Cert2}
    {p : Proposal} (htook : ((N.trace j hj).history (n + 1)).TookEpochChange cfg c1 c2 p)
    (k : PubKey) (hk : C.Honest k) (hkf : C.HonestFrom c2.data.epoch k) :
    N.By k hk (max (N.time j hj n) GST + Δ) fun h => ∃ c1', h.TookEpochChange cfg c1' c2 p := by
  obtain ⟨hrec, hwf⟩ := htook
  have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
  exact hs.epochChange c2 p hcq hwf.last j hj n
    ⟨Or.inr (Or.inr ⟨c1, c2, hrec⟩), Or.inr ⟨c1, p, hrec⟩⟩ k hk hkf

/-- Every certificate an honest node can lock on is the anchor's or backed, so of `E` or earlier. -/
theorem lockable_epoch_le {h : History} (hh : ∃ j, ∃ hj : C.Honest j, ∃ n, h = (N.trace j hj).history n)
    {c : Cert1} (hc : h.Lockable cfg c) : c.data.epoch.toNat ≤ E.toNat := by
  obtain ⟨j, hj, n, rfl⟩ := hh
  rcases cert1_held_backed N.toNetwork (hasCert1_of_lockable hc) with ha | hb
  · rw [ha]; exact stable_anchor hst
  · exact stable_no_later hcfg hcf hs hst hb

omit hcfg hcf hst in
/--
Within `Δ` of an honest node holding a certificate, after GST, every member of its
epoch can lock on it.
-/
theorem lock_of_held {j : PubKey} {hj : C.Honest j} {k : PubKey} {hk : C.Honest k} {T : Nat}
    (hgst : GST ≤ T) {c : Cert1} (hm : C.members c.data.epoch k) (hkf : C.HonestFrom c.data.epoch k)
    (hc : ((N.trace j hj).history (cut N hs.timeUnbounded j hj T)).HasCert1 cfg c) :
    ((N.trace k hk).history (cut N hs.timeUnbounded k hk (T + Δ))).Lockable cfg c := by
  by_cases ha : c = cfg.anchorCert
  · exact Or.inl ha
  have h0 : cut N hs.timeUnbounded j hj T ≠ 0 := fun h => ha (hasCert1_nil (h ▸ hc))
  obtain ⟨n', hn', htn⟩ := cut_pred N hs.timeUnbounded h0
  rw [hn'] at hc
  have := Nat.max_le.mpr ⟨htn, hgst⟩
  exact by_cut N hs.timeUnbounded (fun a b hab h => lockable_grows (received_grows _ hab) h)
    (by_later (by omega) (hs.lockSpread c j hj n' hc k hk hm hkf))

omit hcfg hcf hs hst in
theorem inEpoch_unique {h : History} {e e' : EpochNumber} (he : h.InEpoch cfg e)
    (he' : h.InEpoch cfg e') : e = e' :=
  EpochNumber.ext (Nat.le_antisymm (he'.2 e he.1) (he.2 e' he'.1))

omit hcfg hcf hs in
/-- From `t0` on, epoch `E` is no earlier than the epoch of any node honest in `E` or later. -/
theorem stable_notBehind (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n) {j : PubKey}
    {hj : C.Honest j} (hjE : C.HonestFrom E j) {n : Nat} (hn : cut N hu j hj t0 ≤ n) :
    NotBehind cfg ((N.trace j hj).history n) E :=
  fun e' he' => by rw [inEpoch_unique he' (stable_inEpoch hst hu hjE hn)]; exact Nat.le_refl _

omit hcfg hcf hs hst in
/-- A history is behind no epoch it is in. -/
theorem notBehind_of_inEpoch {h : History} {e : EpochNumber} (he : h.InEpoch cfg e) : NotBehind cfg h e :=
  fun e' he' => by rw [inEpoch_unique he' he]; exact Nat.le_refl _

omit hcf in
/-- A timeout certificate an honest node receives for a late view is of epoch `E`. -/
theorem late_tc_epoch {k : PubKey} {hk : C.Honest k} {n : Nat} {tc : TimeoutCert}
    (hin : (N.trace k hk n).input = .timeoutCertificate tc) (htw : cfg.anchorView.toNat + 2 ≤ tc.view.toNat)
    (hl : Late N t0 tc.view) : tc.data.epoch = E := by
  have hu := hs.timeUnbounded
  have hex : ∃ T, ReachedBy N T tc.view := ⟨N.time k hk n, k, hk, n + 1,
    fun i hi => time_mono N k hk (Nat.le_of_lt_succ hi),
    tc.view + 1, Or.inr (Or.inl ⟨tc, hin ▸ Trace.received_self _ n, rfl⟩),
    Nat.le_succ _⟩
  obtain ⟨q, hq, hvotes⟩ := N.timeoutCertCausal k hk n tc hin
  obtain ⟨k', hqk, -, hk'e⟩ := C.intersect _ q q hq hq
  obtain ⟨m, vote, ⟨-, hvv, hve, -⟩, hm, -⟩ := hvotes k' hqk hk'e
  have hk' : C.Honest k' := .of hk'e
  have hvh : C.honest vote.data.epoch k' := by rw [hve]; exact hk'e
  have hlate := timeout_late hcfg hs htw hex hvh hm (by rw [hvv]; exact Nat.le_refl _)
  have ht0 : t0 < firstReach N hex := Nat.lt_of_not_le fun hle => hl
    (let ⟨j, hj, n', hn', hr⟩ := least_spec hex; ⟨j, hj, n', fun i hi => Nat.le_trans (hn' i hi) hle, hr⟩)
  obtain ⟨-, hep, -⟩ := (N.protocol k' hk' (m + 1)).timeoutJustified m (N.trace k' hk' m) vote
    (Trace.history_getElem? _ (Nat.lt_succ_self m)) hm hvh
  rw [Trace.history_upTo _ (Nat.le_succ m)] at hep
  have hcut : cut N hu k' hk' t0 ≤ m := cut_le_of_lt hu (show t0 < N.time k' hk' m by omega)
  -- A retired signer would be past the epoch it names.
  have hk'E : C.HonestFrom E k' := Classical.byContradiction fun hk'E =>
    stable_retired hst hu hk'E hcut hvh (notBehind_of_inEpoch hep)
  have hEm := stable_inEpoch hst hu hk'E hcut
  rw [← hve]
  exact inEpoch_unique hep hEm

variable {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w) (hl : Late N t0 (w - 1))
include hw hl

omit hcf hs hst hl in
/-- A node that reaches `w` reaches it in some step: no history starts there. -/
theorem reach_step {m : PubKey} {hm : C.Honest m} {n : Nat} (hr : Reached N m hm n w) :
    ∃ s, s < n ∧ ¬ Reached N m hm s w ∧ Reached N m hm (s + 1) w := by
  have hex' : ∃ i, Reached N m hm i w := ⟨n, hr⟩
  have hl0 := least_spec hex'
  have hle : least _ hex' ≤ n := least_le hex' hr
  have h0 : least _ hex' ≠ 0 := fun h => not_reached_nil hcfg hw (h ▸ hl0)
  obtain ⟨s, hs'⟩ : ∃ s, least _ hex' = s + 1 := ⟨least _ hex' - 1, by omega⟩
  exact ⟨s, by omega, least_min hex' (by omega), hs' ▸ hl0⟩

omit hw hl in
/--
What first takes an honest node to `w` or later: a certificate or an epoch change,
which reaches every node honest in `E` or later within `Δ`, or a timeout
certificate it is handed in that step.
-/
theorem first_ground {m : PubKey} {hm : C.Honest m} {s : Nat} (hr : Reached N m hm (s + 1) w)
    (hnot : ¬ Reached N m hm s w) :
    (∀ k (hk : C.Honest k), C.HonestFrom E k →
        N.By k hk (max (N.time m hm s) GST + Δ) fun h => ∃ u, h.ViewGround cfg u ∧ w ≤ u)
      ∨ ∃ tc, (N.trace m hm s).input = .timeoutCertificate tc ∧ w ≤ tc.view + 1 := by
  obtain ⟨v, hv, hwv⟩ := hr
  rcases hv with ⟨c, hc, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, htook, rfl⟩
  · left
    intro k hk hkE
    have hce : c.data.epoch.toNat ≤ E.toNat := by
      rcases cert1_held_backed N.toNetwork hc with ha | hb
      · rw [ha]; exact stable_anchor hst
      · exact stable_no_later hcfg hcf hs hst hb
    obtain ⟨n2, hn2, hc2⟩ := hs.certSpread c m hm s hc k hk (hkE.mono hce)
    exact ⟨n2, hn2, c.view + 1, Or.inl ⟨c, hc2, rfl⟩, hwv⟩
  · right
    obtain ⟨i, hi, hin⟩ := (Trace.received_history _).mp htc
    refine ⟨tc, ?_, hwv⟩
    rcases Nat.lt_succ_iff_lt_or_eq.mp hi with hlt | rfl
    · exact absurd ⟨tc.view + 1, Or.inr (Or.inl ⟨tc, (Trace.received_history _).mpr ⟨i, hlt, hin⟩, rfl⟩),
        hwv⟩ hnot
    · exact hin
  · left
    intro k hk hkE
    have hc2e : c2.data.epoch.toNat + 1 ≤ E.toNat :=
      hst.bound m hm (s + 1) _ (Or.inr (Or.inl ⟨c1, c2, p, htook, rfl⟩))
    obtain ⟨n2, hn2, c1', htook'⟩ := tookEpochChange_spread hs htook k hk (hkE.mono (by omega))
    exact ⟨n2, hn2, c2.view + 1, Or.inr (Or.inr ⟨c1', c2, p, htook', rfl⟩), hwv⟩

omit hcfg hcf hw hl in
/--
A timeout certificate that first takes an honest node to `w`, handed to it without
a timeout vote of its own for the certificate's view and epoch, takes every node
honest in `E` or later to `w` within `Δ`.
-/
theorem forward_push {m : PubKey} {hm : C.Honest m} {s : Nat} {tc : TimeoutCert}
    (hin : (N.trace m hm s).input = .timeoutCertificate tc) (hnot : ¬ Reached N m hm s w)
    (hwt : w ≤ tc.view + 1)
    (hnv : ¬ ∃ vote : TimeoutVote, ((N.trace m hm).history (s + 1)).Sent (.timeoutVote vote)
      ∧ vote.view = tc.view ∧ vote.data.epoch = tc.data.epoch)
    (k : PubKey) (hk : C.Honest k) (hkE : C.HonestFrom E k) :
    N.By k hk (max (N.time m hm s) GST + Δ) fun h => ∃ u, h.ViewGround cfg u ∧ w ≤ u := by
  have htce : tc.data.epoch.toNat ≤ E.toNat :=
    hst.bound m hm (s + 1) _ (Or.inr (Or.inr (Or.inl ⟨tc, hin ▸ Trace.received_self _ s, rfl⟩)))
  have hwt' : w.toNat ≤ tc.view.toNat + 1 := hwt
  have hcap : ∀ u, ((N.trace m hm).history s).ViewGround cfg u → u.toNat ≤ tc.view.toNat := fun u hu => by
    have h2 : ¬ w.toNat ≤ u.toNat := fun h => hnot ⟨u, hu, h⟩
    omega
  obtain ⟨n2, hn2, u, hu, hlt⟩ := hs.timeoutCertForward tc m hm s hin hcap hnv k hk (hkE.mono htce)
  have hlt' : tc.view.toNat < u.toNat := hlt
  exact ⟨n2, hn2, u, hu, show w.toNat ≤ u.toNat by omega⟩

/--
Within `2Δ` of the first node reaching `w`, every node honest in `E` or later has.

What took the first node there is a certificate or an epoch change, which reach
every node within `Δ`, or a timeout certificate for the view before `w`, of `E`.
Its honest signers' timeout votes give every node the one-honest indication within
`Δ`, and a member of `E` still in an earlier view answers with its own vote. If
every member of `E` has voted by then, the votes form a certificate everywhere
within another `Δ`. A member that has not voted was taken to `w` already: by a
certificate or an epoch change, or by a timeout certificate it was handed without
having voted, which it sends on (`Synchrony.timeoutCertForward`).
-/
theorem reach_spread (hgst : GST ≤ firstReach N hex) (hΔτ : Δ < τ) (k : PubKey) (hk : C.Honest k)
    (hkE : C.HonestFrom E k) :
    Reached N k hk (cut N hs.timeUnbounded k hk (firstReach N hex + Δ + Δ)) w := by
  have hu := hs.timeUnbounded
  suffices hall : ∀ k (hk : C.Honest k), C.HonestFrom E k →
      N.By k hk (firstReach N hex + Δ + Δ) fun h => ∃ u, h.ViewGround cfg u ∧ w ≤ u by
    exact by_cut N hu (fun _ _ hab ⟨u, hg, hle⟩ => ⟨u, viewGround_grows (received_grows _ hab) hg, hle⟩)
      (hall k hk hkE)
  intro k hk hkE
  -- `w - 1` is late, so later than the first view.
  have hw1 : cfg.anchorView.toNat + 2 ≤ (w - 1).toNat := by
    refine Nat.le_of_not_lt fun hlt => hl ⟨k, hk, 0, fun _ hi => absurd hi (Nat.not_lt_zero _),
      cfg.anchorCert.view + 1, Or.inl ⟨cfg.anchorCert, Or.inl rfl, rfl⟩, ?_⟩
    show (w - 1).toNat ≤ cfg.anchorCert.view.toNat + 1
    rw [hcfg.anchorCertView]; omega
  obtain ⟨j, hj, n, hn, hr⟩ := least_spec hex
  obtain ⟨s, hsn, hnot, hrs⟩ := reach_step hcfg hw hr
  have hts : N.time j hj s ≤ firstReach N hex := hn s hsn
  rcases first_ground hcfg hcf hs hst hrs hnot with hpush | ⟨tc, hin, hwt⟩
  · exact by_later (by omega) (hpush k hk hkE)
  -- A timeout certificate for `w` or later comes only after `τ` (`Liveness.timeoutCert_late`).
  have hview : ∀ {m : PubKey} {hm : C.Honest m} {i : Nat} {tc' : TimeoutCert},
      (N.trace m hm i).input = .timeoutCertificate tc' → N.time m hm i ≤ firstReach N hex + Δ →
      w ≤ tc'.view + 1 → tc'.view = w - 1 := fun {m} {hm} {i} {tc'} hin' hti hwt' => by
    have hwt'' : w.toNat ≤ tc'.view.toNat + 1 := hwt'
    refine ViewNumber.ext (show tc'.view.toNat = w.toNat - 1 from ?_)
    refine Nat.le_antisymm ?_ (by omega)
    refine Nat.le_of_not_lt fun hlt => ?_
    have := timeoutCert_late hcfg hs hw hex hin' (show w.toNat ≤ tc'.view.toNat by omega)
    omega
  have htw1 : tc.view = w - 1 := hview hin (by omega) hwt
  have hlate : Late N t0 tc.view := htw1 ▸ hl
  have htE : tc.data.epoch = E := late_tc_epoch hcfg hs hst hin (htw1 ▸ hw1) hlate
  have hex1 : ∃ T, ReachedBy N T tc.view := by
    obtain ⟨v, hv, hwv⟩ := hrs
    exact ⟨N.time j hj s, j, hj, s + 1, fun i hi => time_mono N j hj (Nat.le_of_lt_succ hi), v, hv,
      by rw [htw1]; show w.toNat - 1 ≤ v.toNat; have : w.toNat ≤ v.toNat := hwv; omega⟩
  have hF1 : t0 < firstReach N hex1 := late_firstReach hlate hex1
  have hmax : max (firstReach N hex) GST = firstReach N hex := Nat.max_eq_left hgst
  by_cases hvoted : ∀ m (hmE : C.members E m ∧ C.honest E m), ∃ L,
      N.SentByTime m (.of hmE.2) (firstReach N hex + Δ) (.timeoutVote ⟨⟨E, L⟩, tc.view, m⟩)
  · -- Every member of `E` voted: their votes form the certificate everywhere.
    obtain ⟨n2, hn2, tc', -, htv', hrec⟩ := hs.timeoutCert E _ tc.view (firstReach N hex + Δ)
      (N.honestQuorum E) (fun m ⟨hm, hmh⟩ => ⟨hmh, hvoted m ⟨hm, hmh⟩⟩) k hk hkE
    refine by_later (by omega) ⟨n2, hn2, tc'.view + 1, Or.inr (Or.inl ⟨tc', hrec, rfl⟩), ?_⟩
    show w.toNat ≤ tc'.view.toNat + 1
    rw [htv', htw1]; show w.toNat ≤ w.toNat - 1 + 1; omega
  obtain ⟨m, hmE, hmv⟩ : ∃ m, ∃ hmE : C.members E m ∧ C.honest E m, ¬ ∃ L,
      N.SentByTime m (.of hmE.2) (firstReach N hex + Δ) (.timeoutVote ⟨⟨E, L⟩, tc.view, m⟩) :=
    Classical.byContradiction fun hne => hvoted fun m hmE =>
      Classical.byContradiction fun h => hne ⟨m, hmE, h⟩
  have hm : C.Honest m := .of hmE.2
  -- A vote of `m` for the certificate's view and epoch would be one it was not to have.
  have hnv : ∀ s2, N.time m hm s2 ≤ firstReach N hex + Δ → ¬ ∃ vote : TimeoutVote,
      ((N.trace m hm).history (s2 + 1)).Sent (.timeoutVote vote)
        ∧ vote.view = tc.view ∧ vote.data.epoch = tc.data.epoch := by
    rintro s2 hts2 ⟨vote, hsent, hvv, hve⟩
    obtain ⟨i, hi, hiout⟩ := (Trace.sent_history _).mp hsent
    have hveE : vote.data.epoch = E := hve.trans htE
    obtain ⟨hsig, -⟩ := (N.protocol m hm (i + 1)).timeoutJustified i _ vote
      (Trace.history_getElem? _ (Nat.lt_succ_self i)) hiout (by rw [hveE]; exact hmE.2)
    obtain ⟨⟨ve, vl⟩, vv, vs⟩ := vote
    simp only at hsig hvv hveE
    rw [hsig, hvv, hveE] at hiout
    exact hmv ⟨vl, i + 1, fun i' hi' => Nat.le_trans (time_mono N m hm (by omega)) hts2,
      (Trace.sent_history _).mpr ⟨i, Nat.lt_succ_self _, hiout⟩⟩
  -- `m` at `w` by `F + Δ` without such a vote: what took it there reaches everyone.
  have hpushm : ∀ n3, (∀ i, i < n3 → N.time m hm i ≤ firstReach N hex + Δ) → Reached N m hm n3 w →
      N.By k hk (firstReach N hex + Δ + Δ) fun h => ∃ u, h.ViewGround cfg u ∧ w ≤ u := by
    intro n3 hn3 hr3
    obtain ⟨s2, hs2n, hnot2, hr2⟩ := reach_step hcfg hw hr3
    have hts2 := hn3 s2 hs2n
    rcases first_ground hcfg hcf hs hst hr2 hnot2 with hp | ⟨tc2, hin2, hwt2⟩
    · exact by_later (by omega) (hp k hk hkE)
    · have htw2 : tc2.view = tc.view := (hview hin2 hts2 hwt2).trans htw1.symm
      have htE2 : tc2.data.epoch = E := late_tc_epoch hcfg hs hst hin2 (by rw [htw2, htw1]; exact hw1)
        (htw2 ▸ hlate)
      exact by_later (by omega) (forward_push hs hst hin2 hnot2 hwt2
        (by rw [htw2, htE2, ← htE]; exact hnv s2 hts2) k hk hkE)
  -- The honest signers' votes reach `m` as the indication, unless it is past the view.
  obtain ⟨q, hq, hvotes⟩ := N.timeoutCertCausal j hj s tc hin
  obtain ⟨n2, hn2, hP⟩ := hs.timeoutOneHonest E q tc.view (firstReach N hex) (htE ▸ hq)
    (fun k' hqk hk' => by
      obtain ⟨m', vote, ⟨hsig, hvv, hve, -⟩, hout, htm⟩ := hvotes k' hqk (htE ▸ hk')
      refine ⟨vote.data.lock, m' + 1, fun i hi => Nat.le_trans (time_mono N k' _ (Nat.le_of_lt_succ hi))
        (by omega), (Trace.sent_history _).mpr ⟨m', Nat.lt_succ_self _, ?_⟩⟩
      obtain ⟨⟨ve, vl⟩, vv, vs⟩ := vote
      simp only at hsig hvv hve
      rw [htE] at hve
      rw [hsig, hvv, hve] at hout
      exact hout)
    m hm (.of hmE.2)
  rw [hmax] at hn2
  -- A timeout vote for the view, of an epoch `m` is honest in, is of `E`: it comes after `t0`.
  have hvoteE : ∀ i (vote : TimeoutVote), Output.send (.timeoutVote vote) ∈ (N.trace m hm i).output →
      vote.view = tc.view → C.honest vote.data.epoch m →
      vote = ⟨⟨E, vote.data.lock⟩, tc.view, m⟩ := fun i vote hiout hvv hve => by
    have hlate' := timeout_late hcfg hs (htw1 ▸ hw1) hex1 hve hiout (by rw [hvv]; exact Nat.le_refl _)
    obtain ⟨hsig, hep, -⟩ := (N.protocol m hm (i + 1)).timeoutJustified i _ vote
      (Trace.history_getElem? _ (Nat.lt_succ_self i)) hiout hve
    rw [Trace.history_upTo _ (Nat.le_succ i)] at hep
    have hinE' : ((N.trace m hm).history i).InEpoch cfg E :=
      stable_inEpoch hst hu (.of hmE.2) (cut_le_of_lt hu (show t0 < N.time m hm i by omega))
    have hveE : vote.data.epoch = E := inEpoch_unique hep hinE'
    obtain ⟨⟨ve, vl⟩, vv, vs⟩ := vote
    simp only at hsig hvv hveE
    rw [hsig, hvv, hveE]
  rcases hP with hrec | ⟨u, hgu, hlt⟩ | ⟨vote, hsent, hvv, hve⟩
  · obtain ⟨s3, hs3, hin3⟩ := (Trace.received_history _).mp hrec
    have hts3 : N.time m hm s3 ≤ firstReach N hex + Δ := hn2 s3 hs3
    by_cases hR : Reached N m hm s3 w
    · exact hpushm s3 (fun i hi => Nat.le_trans (time_mono N m hm (Nat.le_of_lt hi)) hts3) hR
    -- Still before `w`, `m` answers the indication with a vote of `E`.
    exfalso
    obtain ⟨k', e, hk', m', vote, hve, hout, hvv, htm⟩ := N.oneHonestCausal m hm s3 tc.view hin3
    have hlate' := timeout_late hcfg hs (htw1 ▸ hw1) hex1 (hk := .of hk') (by rw [hve]; exact hk') hout
      (by rw [hvv]; exact Nat.le_refl _)
    have ht0 : t0 < N.time m hm s3 := by omega
    have hinE : ((N.trace m hm).history s3).InEpoch cfg E :=
      stable_inEpoch hst hu (.of hmE.2) (cut_le_of_lt hu ht0)
    have hinV := inView_viewOf (cfg := cfg) (h := (N.trace m hm).history s3)
    have hvle : viewOf cfg ((N.trace m hm).history s3) ≤ tc.view := by
      have h2 : ¬ w.toNat ≤ (viewOf cfg ((N.trace m hm).history s3)).toNat := fun h => hR ⟨_, hinV.1, h⟩
      rw [htw1]; show (viewOf cfg ((N.trace m hm).history s3)).toNat ≤ w.toNat - 1; omega
    obtain ⟨e', L, hep, hout'⟩ := (N.protocol m hm (s3 + 1)).timeoutAnswered s3 _ tc.view
      (Trace.history_getElem? _ (Nat.lt_succ_self s3))
      (Or.inr ⟨hin3, by rw [Trace.history_upTo _ (Nat.le_succ s3)]; exact ⟨_, hinV, hvle⟩⟩)
      (fun e'' he'' => by
        rw [Trace.history_upTo _ (Nat.le_succ s3)] at he''
        rw [inEpoch_unique he'' hinE]; exact ⟨hmE.2, hmE.1⟩)
    rw [Trace.history_upTo _ (Nat.le_succ s3)] at hep
    obtain rfl : e' = E := inEpoch_unique hep hinE
    exact hmv ⟨L, s3 + 1, fun i hi => Nat.le_trans (time_mono N m hm (Nat.le_of_lt_succ hi)) hts3,
      (Trace.sent_history _).mpr ⟨s3, Nat.lt_succ_self _, hout'⟩⟩
  · exact hpushm n2 hn2 ⟨u, hgu, by
      have h1 : tc.view.toNat < u.toNat := hlt
      have h2 : tc.view.toNat = w.toNat - 1 := congrArg ViewNumber.toNat htw1
      show w.toNat ≤ u.toNat; omega⟩
  · exfalso
    obtain ⟨i, hi, hiout⟩ := (Trace.sent_history _).mp hsent
    have heq := hvoteE i vote hiout hvv hve
    rw [heq] at hiout
    exact hmv ⟨vote.data.lock, i + 1, fun i' hi' => Nat.le_trans (time_mono N m hm (by omega)) (hn2 i hi),
      (Trace.sent_history _).mpr ⟨i, Nat.lt_succ_self _, hiout⟩⟩

/--
If no honest node holds a certificate at `w` or later by `T`, with `T` before anyone
may give `w` up, then from `Δ` after the first node reached `w` until `T` every
honest node is in `w` exactly.

An epoch change into a later view than `w` would need a `Cert2` at `w` or later
over an epoch's last block, which a stable epoch rules out at a late view
(`Liveness.late_commit`).
-/
theorem view_is_w {T : Nat} (hT2 : firstReach N hex + Δ + Δ ≤ T) (hT : T < firstReach N hex + τ)
    (hno : ∀ j (hj : C.Honest j) c,
      ((N.trace j hj).history (cut N hs.timeUnbounded j hj T)).HasCert1 cfg c → c.view < w)
    (k : PubKey) (hk : C.Honest k) (hkE : C.HonestFrom E k) {n : Nat}
    (hlo : cut N hs.timeUnbounded k hk (firstReach N hex + Δ + Δ) ≤ n)
    (hhi : n ≤ cut N hs.timeUnbounded k hk T) :
    ((N.trace k hk).history n).InView cfg w := by
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst
    (Nat.le_of_lt (late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex))
  have hreach := reached_mono hlo (reach_spread hcfg hcf hs hst hw hex hl hgst (by omega) k hk hkE)
  have hle : ∀ v, ((N.trace k hk).history n).ViewGround cfg v → v ≤ w := by
    intro v hv
    rcases hv with ⟨c, hc, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, htook, rfl⟩
    · have := hno k hk c (hasCert1_grows (received_grows _ hhi) hc)
      show c.view.toNat + 1 ≤ w.toNat
      have : c.view.toNat < w.toNat := this
      omega
    · obtain ⟨i, hi, hin⟩ := (Trace.received_history _).mp htc
      have hti : N.time k hk i ≤ T := cut_before N hs.timeUnbounded (Nat.lt_of_lt_of_le hi hhi)
      show tc.view.toNat + 1 ≤ w.toNat
      by_cases hlt : tc.view.toNat < w.toNat
      · omega
      · have := timeoutCert_late hcfg hs hw hex hin (show w.toNat ≤ tc.view.toNat by omega)
        omega
    · show c2.view.toNat + 1 ≤ w.toNat
      refine Nat.le_of_not_lt fun hlt => ?_
      obtain ⟨hrec, hwf⟩ := htook
      have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
      have hc2l : Late N t0 c2.view := late_mono hl (by show w.toNat - 1 ≤ c2.view.toNat; omega)
      exact (late_commit hcfg hcf hs hst (Or.inr (Or.inr ⟨c1, c2, hrec⟩))
        (Or.inr ⟨c1, p, hrec⟩) hcq hc2l).2 hwf.last
  obtain ⟨v, hv, hwv⟩ := hreach
  have : v = w := ViewNumber.le_antisymm (hle v hv) hwv
  subst this
  exact ⟨hv, hle⟩

end Spread

/-! ## The leader acts -/

/--
What the honest leader of a late view holds that lets it propose, for the proof:
the node could propose for view `v` on `parent`: it leads `v` in the epoch of the
block after `parent`, and holds a parent certificate `pc` over `parent` it may
build on: one it can lock on for the view before, or, after a timeout, one over
the block of its lock, when the timeout certificate's lock lets it through and the
certificate is of the block's epoch.
-/
def ReadyToPropose (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey)
    (node : PubKey) (h : History) (v : ViewNumber) (pc : Cert1) (parent : Block) : Prop :=
  leader (epochOf (pc.data.blockNumber + 1) cfg.epochHeight) v = some node
    ∧ h.HasProposal cfg parent ∧ pc.view = parent.viewNumber
    ∧ pc.data.blockHash = blockHash parent
    ∧ ((h.Lockable cfg pc ∧ pc.view + 1 = v)
      ∨ ∃ tc, h.Received (.timeoutCertificate tc) ∧ tc.view + 1 = v ∧ h.HasCert1 cfg pc
          ∧ ((∃ l, h.LockedOn cfg l ∧ l.data = pc.data)
            ∨ (IsLastBlock pc.data.blockNumber cfg.epochHeight
              ∧ ∃ c2, h.HasCert2 c2 ∧ c2.data = pc.data.toVote2))
          ∧ tc.data.epoch = epochOf (pc.data.blockNumber + 1) cfg.epochHeight
          ∧ TimeoutLockAllows tc pc (epochOf (pc.data.blockNumber + 1) cfg.epochHeight))

section Leader

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ) {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
include hcfg hcf hs hst

omit hcfg hcf hs hst in
/-- A block is in the epoch its parent's height leads to. -/
theorem epochOf_one' (h : Nat) : epochOf 1 h = epochOf 0 h := by
  by_cases hh : h = 0
  · simp [epochOf_eq, hh]
  · exact epochOf_one h hh

omit hcfg hcf hs hst in
/-- The block after a block that is not its epoch's last is in the same epoch. -/
theorem epochOf_next_same {n : BlockNumber} (hnl : ¬ IsLastBlock n cfg.epochHeight) :
    epochOf (n + 1) cfg.epochHeight = epochOf n cfg.epochHeight := by
  by_cases hh : cfg.epochHeight = 0
  · simp [epochOf_eq, hh]
  · by_cases hz : n.toNat = 0
    · rw [BlockNumber.ext (show n.toNat = (0 : BlockNumber).toNat from hz)]
      show epochOf 1 _ = _
      exact epochOf_one' _
    · rw [epochOf_succ _ _ hh hz, ite_eq_right hnl]

variable {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hl : Late N t0 (w - 1))
include hw hl

omit hcf hs hst hw hl in
/--
A certificate an honest node could lock on, of an epoch before the one the run starts
in, is the anchor's: every other is backed, or comes with a backed `Cert2`, of that
epoch or a later one.
-/
theorem lockable_before_start {j : PubKey} {hj : C.Honest j} {n : Nat} {c : Cert1}
    (hl : ((N.trace j hj).history n).Lockable cfg c) (hlt : c.data.epoch.toNat < cfg.startEpoch.toNat) :
    c = cfg.anchorCert := by
  rcases hl with ha | ⟨hc, -⟩ | ⟨c2, p, hrec, hwf⟩
  · exact ha
  · rcases cert1_held_backed N.toNetwork hc with ha | hb
    · exact ha
    · exfalso
      have := cert1_epoch_after_start N.toNetwork hcfg _ _ (Nat.le_refl _) hb
      omega
  · exfalso
    obtain ⟨i, -, hin⟩ := (Trace.received_history _).mp hrec
    have hc2b : Cert2Backed N.trace c2 :=
      N.cert2Genuine j hj i c2 (by rw [hin]; rfl)
    obtain ⟨cc, hccb, -, -, hcce⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg hc2b
    have h1 := cert1_epoch_after_start N.toNetwork hcfg _ _ (Nat.le_refl _) hccb
    have h2 : c.data.epoch = c2.data.epoch := congrArg Vote2Data.epoch hwf.sameData
    rw [hcce, ← h2] at h1
    omega

/--
The leader of `w`, in `w` and able to lock only on views before it, can act on its
lock: propose on it, ask for a re-vote of `E`'s last block, or open `E` on the
last block of the epoch before.

Which of the three is fixed by its lock `L`. Of `E` and not the last block: a
proposal of `E`. Of `E` and the last block: a re-vote. Of an earlier epoch: then
its solid grounds for `E` are an epoch change into `E`, over the epoch's last
block it opens `E` on. A timeout certificate that put it in `w` is of `E`, and
its lock is no later than `L`, so it lets the parent through.
-/
theorem leader_ready {l : PubKey} {hl' : C.Honest l} (hlE : C.HonestFrom E l) (hlead : leader E w = some l)
    {n1 n2 : Nat}
    (h12 : n1 ≤ n2) (ht0 : cut N hs.timeUnbounded l hl' t0 ≤ n1)
    (hin1 : ((N.trace l hl').history n1).InView cfg w)
    (hin2 : ((N.trace l hl').history n2).InView cfg w)
    (hno2 : ∀ L, ((N.trace l hl').history n2).Lockable cfg L → L.view < w)
    (hlockE : ∀ c, ((N.trace l hl').history n1).HasCert1 cfg c → c.data.epoch = E →
      ((N.trace l hl').history n2).Lockable cfg c)
    (htc : ∀ tc, ((N.trace l hl').history n1).Received (.timeoutCertificate tc) → tc.view + 1 = w →
      tc.data.epoch = E ∧ (tc.data.lock.data.epoch.toNat < E.toNat
        ∨ ((N.trace l hl').history n2).Lockable cfg tc.data.lock)) :
    (∃ pc parent, ReadyToPropose cfg leader l ((N.trace l hl').history n2) w pc parent ∧ pc.view < w
        ∧ (IsLastBlock pc.data.blockNumber cfg.epochHeight →
          ∃ c2, (((N.trace l hl').history n2).HasCert2 c2 ∨ c2 = cfg.anchorCert2) ∧ c2.view < w
            ∧ c2.data = pc.data.toVote2)
        ∧ epochOf (pc.data.blockNumber + 1) cfg.epochHeight = E)
      ∨ ∃ r, RevoteJustified cfg leader l ((N.trace l hl').history n2) r ∧ r.view = w
        ∧ r.cert.data.epoch = E := by
  have hu := hs.timeUnbounded
  have hgr := received_grows (N.trace l hl') h12
  obtain ⟨L, hL⟩ := exists_lockedOn (cfg := cfg) ((N.trace l hl').history n2)
  have hLw := hno2 L hL.1
  have hLe := lockable_epoch_le hcfg hcf hs hst ⟨l, hl', n2, rfl⟩ hL.1
  have hwN : w.toNat - 1 + 1 = w.toNat := by omega
  -- How the leader came to be in `w`: a certificate for the view before, or a timeout certificate.
  have hentry : (∃ c, ((N.trace l hl').history n1).HasCert1 cfg c ∧ c.view + 1 = w)
      ∨ ∃ tc, ((N.trace l hl').history n1).Received (.timeoutCertificate tc) ∧ tc.view + 1 = w := by
    rcases hin1.1 with ⟨c, hc, hwc⟩ | ⟨tc, htc0, hwt⟩ | ⟨c1, c2, p, htook, hw2⟩
    · exact Or.inl ⟨c, hc, hwc.symm⟩
    · exact Or.inr ⟨tc, htc0, hwt.symm⟩
    · exfalso
      obtain ⟨hrec, hwf⟩ := htook
      have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
      have hc2l : Late N t0 c2.view := late_mono hl (by
        have : w.toNat = c2.view.toNat + 1 := congrArg ViewNumber.toNat hw2
        show w.toNat - 1 ≤ c2.view.toNat; omega)
      exact (late_commit hcfg hcf hs hst (Or.inr (Or.inr ⟨c1, c2, hrec⟩))
        (Or.inr ⟨c1, p, hrec⟩) hcq hc2l).2 hwf.last
  -- A certificate that put the leader in `w` is of `E`, so the lock is of `E`, for the view before.
  have hentry_c : ∀ c, ((N.trace l hl').history n1).HasCert1 cfg c → c.view + 1 = w →
      L.data.epoch = E ∧ L.view + 1 = w := fun c hc hcw => by
    have hcv : c.view.toNat + 1 = w.toNat := congrArg ViewNumber.toNat hcw
    have hgen : cfg.anchorView < c.view := by show cfg.anchorView.toNat < c.view.toNat; omega
    have hcb := held_backed hcfg hc hgen
    have hce := late_cert_epoch hcfg hcf hs hst hcb (late_mono hl (by show w.toNat - 1 ≤ c.view.toNat; omega))
    have hLc := hL.2 c (hlockE c hc hce)
    have hLw' : L.view.toNat < w.toNat := hLw
    rcases hLc with hlt | ⟨heq, hle⟩
    · exfalso
      have : c.data.epoch.toNat < L.data.epoch.toNat := hlt
      rw [hce] at this; omega
    · refine ⟨heq.symm.trans hce, ViewNumber.ext ?_⟩
      have : c.view.toNat ≤ L.view.toNat := hle
      show L.view.toNat + 1 = w.toNat; omega
  -- The timeout path's facts, for a parent `pc` the lock lets through.
  have htcpath : ∀ tc, ((N.trace l hl').history n1).Received (.timeoutCertificate tc) → tc.view + 1 = w →
      tc.data.epoch = E ∧ (tc.data.lock.data.epoch.toNat < E.toNat ∨ LockLE tc.data.lock L) := fun tc htc0 hwt => by
    obtain ⟨hte, htl⟩ := htc tc htc0 hwt
    exact ⟨hte, htl.imp_right (hL.2 _)⟩
  have hsolid := stable_solid hst hu hlE ht0
  have hnb := stable_notBehind hst hu hlE (Nat.le_trans ht0 h12)
  by_cases hLE : L.data.epoch = E
  · -- The lock is of `E`. Its block's proposal is held, or it is the anchor's.
    have hLb : L = cfg.anchorCert ∨ ∃ b, ((N.trace l hl').history n2).HasProposal cfg b ∧ Certifies L b := by
      rcases hL.1 with hanc | ⟨-, b, hb, hcert, -⟩ | ⟨c2', p', hrec', hwf'⟩
      · exact Or.inl hanc
      · exact Or.inr ⟨b, hb, hcert⟩
      · exfalso
        have hcq : Commits c2' p' := ⟨hwf'.1, by rw [← hwf'.sameData, hwf'.cert1Data]; rfl⟩
        refine stable_no_commit hs hst (Or.inr (Or.inr ⟨L, c2', hrec'⟩))
          (Or.inr ⟨L, p', hrec'⟩) hcq hwf'.last ?_
        have := congrArg Vote1Data.epoch hwf'.cert1Data
        simp only at this
        rw [← this, hLE]
    -- The block and its view.
    obtain ⟨b, hb, hbv, hbh, hbe, hbn, hbwf⟩ : ∃ b, ((N.trace l hl').history n2).HasProposal cfg b
        ∧ b.viewNumber ≤ L.view ∧ L.data.blockHash = blockHash b
        ∧ L.data.epoch = epochOf b.blockHeader.blockNumber cfg.epochHeight
        ∧ L.data.blockNumber = b.blockHeader.blockNumber
        ∧ (b.viewNumber < L.view → IsLastBlock b.blockHeader.blockNumber cfg.epochHeight) := by
      rcases hLb with hanc | ⟨b, hb, hcert⟩
      · refine ⟨cfg.anchorBlock, Or.inl rfl, ?_, ?_, ?_, ?_, fun hlt => ?_⟩
        · rw [hanc, hcfg.anchorCertView]; exact Nat.le_refl _
        · rw [hanc]; exact hcfg.anchorCertBlock
        · rw [hanc, hcfg.anchorCertEpoch]
        · rw [hanc, hcfg.anchorCertBlockNumber]
        · exfalso; rw [hanc, hcfg.anchorCertView] at hlt
          exact Nat.lt_irrefl _ hlt
      · rcases cert1_held_backed N.toNetwork (hasCert1_of_lockable hL.1) with hanc | hLbk
        · -- The anchor's certificate, over a held block, which is then at genesis too.
          refine ⟨b, hb, hcert.1, congrArg Vote1Data.blockHash hcert.2, ?_, congrArg Vote1Data.blockNumber hcert.2,
            fun hlt => ?_⟩
          · rw [hanc, hcfg.anchorCertEpoch]
            have := congrArg Vote1Data.blockNumber hcert.2
            simp only at this
            rw [← this, hanc, hcfg.anchorCertBlockNumber]
          · exfalso
            have hba : b = cfg.anchorBlock := hcf b _ (by
              have := congrArg Vote1Data.blockHash hcert.2
              simp only at this
              rw [← this, hanc, hcfg.anchorCertBlock])
            rw [hba, hanc, hcfg.anchorCertView] at hlt
            exact Nat.lt_irrefl _ hlt
        · obtain ⟨-, -, -, -, b0, -, -, hwf0, -, hd0, hv0, hlast0, -⟩ :=
            cert1_origin N.toNetwork _ L (Nat.le_refl _) hLbk
          have hb0 : b0 = b := hcf b0 b (by
            have h1 := congrArg Vote1Data.blockHash hd0
            have h2 := congrArg Vote1Data.blockHash hcert.2
            simp only at h1 h2
            rw [← h1, h2])
          subst hb0
          refine ⟨b0, hb, hv0, congrArg Vote1Data.blockHash hd0, ?_, congrArg Vote1Data.blockNumber hd0, hlast0⟩
          have := congrArg Vote1Data.epoch hd0
          simp only at this
          rw [this, hwf0.epoch]
    have hpath : ∀ (P : Prop), (L.view + 1 = w → P) →
        (∀ tc, ((N.trace l hl').history n1).Received (.timeoutCertificate tc) → tc.view + 1 = w →
          tc.data.epoch = E ∧ (tc.data.lock.data.epoch.toNat < E.toNat ∨ LockLE tc.data.lock L) → P) → P :=
        fun P h1 h2 => by
      rcases hentry with ⟨c, hc, hcw⟩ | ⟨tc, htc0, hwt⟩
      · exact h1 (hentry_c c hc hcw).2
      · exact h2 tc htc0 hwt (htcpath tc htc0 hwt)
    have hallow : ∀ tc : TimeoutCert, (tc.data.lock.data.epoch.toNat < E.toNat ∨ LockLE tc.data.lock L) →
        TimeoutLockAllows tc L E := by
      intro tc hle
      rcases hle with hlt | hlt | ⟨heq, hv⟩
      · exact Or.inl hlt
      · exact Or.inl (hLE ▸ hlt)
      · exact Or.inr ⟨heq, Or.inl hv⟩
    by_cases hlast : IsLastBlock b.blockHeader.blockNumber cfg.epochHeight
    · -- The last block of `E`: a re-vote.
      right
      have hlastL : IsLastBlock L.data.blockNumber cfg.epochHeight := by rw [hbn]; exact hlast
      have hleadL : leader L.data.epoch w = some l := by rw [hLE]; exact hlead
      have hfull : ∀ m, ((N.trace l hl').history n2).upTo m = (N.trace l hl').history (min m n2) :=
        fun m => upTo_history _
      refine hpath _ (fun hnext => ⟨⟨L, w, none⟩, ⟨hleadL, ⟨hLw, Or.inl ⟨rfl, hnext⟩, hlastL⟩,
        buildable_of_lockable hL.1, fun _ => hL.1, (fun _ h => by cases h), by show NotBehind cfg _ L.data.epoch; rw [hLE]; exact hnb,
        ⟨w, hin2.1, Nat.le_refl _⟩⟩, rfl, hLE⟩)
        (fun tc htc0 hwt ⟨hte, htl⟩ => ?_)
      refine ⟨⟨L, w, some tc⟩, ⟨hleadL, ⟨hLw, Or.inr ⟨tc, rfl, hwt⟩, hlastL⟩, ?_,
        (fun h => by cases h), ?_, by show NotBehind cfg _ L.data.epoch; rw [hLE]; exact hnb, ⟨w, hin2.1, Nat.le_refl _⟩⟩, rfl, hLE⟩
      · refine ⟨hasCert1_of_lockable hL.1, n2, ?_, Or.inl ⟨L, ?_, rfl⟩⟩
        · rw [hfull, Nat.min_self]; exact hgr _ htc0
        · rw [hfull, Nat.min_self]; exact hL
      · intro tc' h
        cases h
        exact ⟨by rw [hte, hLE], by show TimeoutLockAllows tc L L.data.epoch; rw [hLE]; exact hallow tc htl⟩
    · -- Not the last block: a proposal on the lock, in `E`.
      left
      have hbvL : L.view = b.viewNumber := ViewNumber.le_antisymm
        (Nat.le_of_not_lt fun hlt => hlast (hbwf hlt)) hbv
      have hnext : epochOf (L.data.blockNumber + 1) cfg.epochHeight = E := by
        rw [hbn, epochOf_next_same hlast, ← hbe, hLE]
      refine ⟨L, b, ⟨by rw [hnext]; exact hlead, hb, hbvL, hbh, ?_⟩, hLw,
        fun hl2 => absurd (hbn ▸ hl2) hlast, hnext⟩
      exact hpath _ (fun hnx => Or.inl ⟨hL.1, hnx⟩)
        (fun tc htc0 hwt ⟨hte, htl⟩ => Or.inr ⟨tc, hgr _ htc0, hwt, hasCert1_of_lockable hL.1,
          Or.inl ⟨L, hL, rfl⟩, by rw [hnext, hte], by rw [hnext]; exact hallow tc htl⟩)
  · -- The lock is of an earlier epoch: the leader opens `E` on the epoch change it took.
    left
    have hLlt : L.data.epoch.toNat < E.toNat := by
      have : L.data.epoch.toNat ≠ E.toNat := fun h => hLE (EpochNumber.ext h)
      omega
    -- No certificate of `E` is lockable, so the leader came by a timeout certificate.
    obtain ⟨tc, htc0, hwt⟩ : ∃ tc, ((N.trace l hl').history n1).Received (.timeoutCertificate tc)
        ∧ tc.view + 1 = w := by
      rcases hentry with ⟨c, hc, hcw⟩ | h
      · exact absurd (hentry_c c hc hcw).1 hLE
      · exact h
    obtain ⟨hte, htl⟩ := htcpath tc htc0 hwt
    rcases hsolid with hanc | ⟨c1, c2, p, ⟨hrec1, hwf⟩, hEc⟩ | ⟨c, hc, -, hce⟩
    · -- The run started in `E`, so the lock is the anchor's, and the anchor ended the
      -- epoch before: the leader opens `E` on the anchor, behind its `Cert2`.
      have hLa : L = cfg.anchorCert :=
        lockable_before_start hcfg hL.1 (by rw [← hanc]; exact hLlt)
      have hlastA : IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight := by
        by_cases h : IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight
        · exact h
        · exfalso
          have : cfg.startEpoch = cfg.anchorCert.data.epoch := by
            unfold Config.startEpoch; rw [ite_eq_right h]
          rw [hLa, hanc, this] at hLlt
          exact Nat.lt_irrefl _ hLlt
      have hnext : epochOf (cfg.anchorCert.data.blockNumber + 1) cfg.epochHeight = E := by
        rw [hcfg.anchorCertBlockNumber, epochOf_after_anchor hcfg, hanc]
      have hav : cfg.anchorCert.view < w := by
        show cfg.anchorCert.view.toNat < w.toNat
        rw [hcfg.anchorCertView]; omega
      refine ⟨cfg.anchorCert, cfg.anchorBlock, ⟨by rw [hnext]; exact hlead, Or.inl rfl,
        hcfg.anchorCertView, hcfg.anchorCertBlock,
        Or.inr ⟨tc, hgr _ htc0, hwt, Or.inl rfl, Or.inl ⟨L, hL, by rw [hLa]⟩, by rw [hnext, hte], ?_⟩⟩,
        hav, fun _ => ⟨cfg.anchorCert2, Or.inr rfl, hav, rfl⟩, hnext⟩
      rw [hnext]
      left
      rcases htl with h | h | ⟨h, -⟩
      · exact h
      · exact Nat.lt_trans h hLlt
      · rw [h]; exact hLlt
    · have hrec := hgr _ hrec1
      have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
      have hlast := hwf.last
      have hpe : p.epoch = epochOf p.blockHeader.blockNumber cfg.epochHeight := hwf.wellFormed.epoch
      have hc2e : c2.data.epoch = p.epoch := congrArg Vote2Data.epoch hcq.2
      have hc1n : c1.data.blockNumber = p.blockHeader.blockNumber := congrArg Vote1Data.blockNumber hwf.cert1Data
      have hnext : epochOf (c1.data.blockNumber + 1) cfg.epochHeight = E := by
        rw [hc1n, epochOf_succ _ _ hlast.2.1 (isLastBlock_iff.mp hlast).1, ite_eq_left hlast, ← hpe, ← hc2e, hEc]
      have hc2w : c2.view < w := by
        have := hin2.2 _ (Or.inr (Or.inr ⟨c1, c2, p, ⟨hrec, hwf⟩, rfl⟩))
        show c2.view.toNat < w.toNat
        have : c2.view.toNat + 1 ≤ w.toNat := this
        omega
      refine ⟨c1, p, ⟨by rw [hnext]; exact hlead, Or.inr (Or.inr ⟨c1, c2, hrec⟩),
        hwf.cert1View.symm, congrArg Vote1Data.blockHash hwf.cert1Data,
        Or.inr ⟨tc, hgr _ htc0, hwt, Or.inr (Or.inr ⟨c2, p, hrec⟩),
          Or.inr ⟨hc1n ▸ hlast, c2, Or.inr ⟨c1, p, hrec⟩, hwf.sameData.symm⟩, by rw [hnext, hte], ?_⟩⟩, ?_,
        fun _ => ⟨c2, Or.inl (Or.inr ⟨c1, p, hrec⟩), hc2w, hwf.sameData.symm⟩, hnext⟩
      · rw [hnext]
        left
        rcases htl with h | h | ⟨h, -⟩
        · exact h
        · exact Nat.lt_trans h hLlt
        · rw [h]; exact hLlt
      · show c1.view.toNat < w.toNat
        have : c1.view.toNat ≤ c2.view.toNat := by
          rw [← hwf.cert1View]; exact hwf.1
        have : c2.view.toNat < w.toNat := hc2w
        omega
    · exfalso
      rcases hL.2 c (hlockE c hc hce.symm) with h | ⟨h, -⟩
      · have : c.data.epoch.toNat < L.data.epoch.toNat := h
        rw [← hce] at this; omega
      · rw [← h, ← hce] at hLlt; exact Nat.lt_irrefl _ hLlt

/-! ## The leader's proposal or re-vote request -/

omit hcfg hcf hs hst hw hl in
/-- A node that could propose for `v` is ready to propose for `v` with any header at the right height. -/
theorem ready_of_tuple {r : Trace} {m n : Nat} (hmn : m ≤ n) {l : PubKey} {v : ViewNumber}
    {pc : Cert1} {parent : Block} (hready : ReadyToPropose cfg leader l (r.history m) v pc parent)
    (hlt : pc.view < v)
    (hbd : IsLastBlock pc.data.blockNumber cfg.epochHeight →
      ∃ c2, ((r.history m).HasCert2 c2 ∨ c2 = cfg.anchorCert2) ∧ c2.view < v ∧ c2.data = pc.data.toVote2)
    {hdr : BlockHeader} (hnum : hdr.blockNumber = pc.data.blockNumber + 1)
    (hcur : NotBehind cfg (r.history n) (epochOf (pc.data.blockNumber + 1) cfg.epochHeight))
    (hreach : ∃ u, (r.history n).ViewGround cfg u ∧ v ≤ u) :
    ∃ ev, ProposalReady cfg leader l (r.history n)
      ⟨hdr, v, epochOf hdr.blockNumber cfg.epochHeight, pc, ev, ⟨0⟩⟩ := by
  obtain ⟨hlead, hb, hpv, hph, hpath⟩ := hready
  have hgr := received_grows r hmn
  have hlead' : leader (epochOf hdr.blockNumber cfg.epochHeight) v = some l := by rw [hnum]; exact hlead
  have hcur' : NotBehind cfg (r.history n) (epochOf hdr.blockNumber cfg.epochHeight) := by
    rw [hnum]; exact hcur
  have hent : ∀ ev, EntersEpoch cfg ⟨hdr, v, epochOf hdr.blockNumber cfg.epochHeight, pc, ev, ⟨0⟩⟩ →
      IsLastBlock pc.data.blockNumber cfg.epochHeight := fun ev he => by
    unfold EntersEpoch at he
    simp only at he
    rwa [hnum, BlockNumber.add_sub_cancel] at he
  have hboundary : ∀ ev, EntersEpoch cfg ⟨hdr, v, epochOf hdr.blockNumber cfg.epochHeight, pc, ev, ⟨0⟩⟩ →
      ∃ c2, ((r.history n).HasCert2 c2 ∨ c2 = cfg.anchorCert2) ∧ c2.view < v ∧ c2.data = pc.data.toVote2 :=
    fun ev he => by
      obtain ⟨c2, hc2, h1, h2⟩ := hbd (hent ev he)
      exact ⟨c2, hc2.imp_left (hasCert2_grows hgr), h1, h2⟩
  have hbuilt : ∃ parent', (r.history n).HasProposal cfg parent' ∧ parent'.viewNumber ≤ pc.view
      ∧ pc.data.blockHash = blockHash parent' :=
    ⟨parent, hasProposal_grows hgr hb, Nat.le_of_eq (congrArg ViewNumber.toNat hpv.symm), hph⟩
  have hopen : ∀ ev, OpensEpochJustified cfg (r.history n)
      ⟨hdr, v, epochOf hdr.blockNumber cfg.epochHeight, pc, ev, ⟨0⟩⟩ := fun ev he =>
    ⟨⟨parent, hasProposal_grows hgr hb, hpv.symm, hph⟩, hboundary ev he⟩
  rcases hpath with ⟨hlock, hnext⟩ | ⟨tc, htc, htv, hc1, hjust, hte, hallow⟩
  · exact ⟨none, hlead', ⟨hlt, Or.inl ⟨rfl, hnext⟩, rfl, hnum.symm⟩,
      buildable_of_lockable (lockable_grows hgr hlock),
      hbuilt, hopen none, (fun _ h => by cases h), hcur', hreach⟩
  · refine ⟨some tc, hlead', ⟨hlt, Or.inr ⟨tc, rfl, htv⟩, rfl, hnum.symm⟩, ?_, hbuilt, hopen _, ?_,
      hcur', hreach⟩
    · show CertJustified cfg (r.history n) pc (some tc)
      refine ⟨hasCert1_grows hgr hc1, m, ?_, ?_⟩
      · rw [Trace.history_upTo r hmn]; exact htc
      · rw [Trace.history_upTo r hmn]; exact hjust
    · intro tc' h
      cases h
      exact ⟨by rw [hte, hnum], by show TimeoutLockAllows _ _ (epochOf hdr.blockNumber cfg.epochHeight); rw [hnum]; exact hallow⟩

omit hcfg hcf hs hst hw hl in
/-- A view whose number is one less than `w`'s is `w - 1`. -/
theorem view_pred {v : ViewNumber} (hv : v + 1 = w) : v = w - 1 :=
  ViewNumber.ext (by
    have : v.toNat + 1 = w.toNat := congrArg ViewNumber.toNat hv
    show v.toNat = w.toNat - 1; omega)

omit hw in
/--
The honest leader of `w` proposes, or asks for a re-vote, for `w` within `4Δ + δ`
of the first node reaching `w`: it is in `w` by `2Δ`, can lock on what put it there
by `3Δ`, being a member of `E`, and has a header by `4Δ`.
-/
theorem leader_acts {δ : Nat} (hp : Prompt N δ) (hw3 : cfg.anchorView.toNat + 3 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    {T : Nat} (hT1 : firstReach N hex + Δ + 3 * Δ + δ ≤ T) (hT : T < firstReach N hex + τ)
    (hno : ∀ j (hj : C.Honest j) c,
      ((N.trace j hj).history (cut N hs.timeUnbounded j hj T)).HasCert1 cfg c → c.view < w)
    {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    (hmem : C.members E l) :
    ∃ j, ((∃ p, Output.send (.proposal p) ∈ (N.trace l (.of hlE) j).output ∧ p.viewNumber = w ∧ C.honest p.epoch l)
        ∨ ∃ r, Output.send (.revote r) ∈ (N.trace l (.of hlE) j).output ∧ r.view = w ∧ C.honest r.cert.data.epoch l)
      ∧ N.time l (.of hlE) j ≤ firstReach N hex + Δ + 3 * Δ + δ := by
  have hw : cfg.anchorView.toNat + 2 ≤ w.toNat := by omega
  have hl' : C.Honest l := .of hlE
  have hu := hs.timeUnbounded
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  have hc12 : cut N hu l hl' (firstReach N hex + Δ + Δ) ≤ cut N hu l hl' (firstReach N hex + Δ + 2 * Δ) :=
    cut_mono N hu (by omega)
  have hc23 : cut N hu l hl' (firstReach N hex + Δ + 2 * Δ) ≤ cut N hu l hl' (firstReach N hex + Δ + 3 * Δ) :=
    cut_mono N hu (by omega)
  have hc2T : cut N hu l hl' (firstReach N hex + Δ + 2 * Δ) ≤ cut N hu l hl' T := cut_mono N hu (by omega)
  have hin1 := view_is_w hcfg hcf hs hst hw hex hl (by omega) hT hno l hl' (.of hlE) (Nat.le_refl _)
    (Nat.le_trans hc12 hc2T)
  have hin2 := view_is_w hcfg hcf hs hst hw hex hl (by omega) hT hno l hl' (.of hlE) hc12 hc2T
  have hn2 : cut N hu l hl' (firstReach N hex + Δ + 2 * Δ) ≠ 0 := fun h => by
    have hr := reach_spread hcfg hcf hs hst hw hex hl hgst (by omega) l hl' (.of hlE)
    rw [show cut N hu l hl' (firstReach N hex + Δ + Δ) = 0 by omega] at hr
    exact not_reached_nil hcfg hw hr
  have htc : ∀ tc, ((N.trace l hl').history (cut N hu l hl' (firstReach N hex + Δ + Δ))).Received
      (.timeoutCertificate tc) → tc.view + 1 = w →
      tc.data.epoch = E ∧ (tc.data.lock.data.epoch.toNat < E.toNat
        ∨ ((N.trace l hl').history (cut N hu l hl' (firstReach N hex + Δ + 2 * Δ))).Lockable
          cfg tc.data.lock) := fun tc hr hwt => by
    obtain ⟨i, hi, hin⟩ := (Trace.received_history _).mp hr
    have hti : N.time l hl' i ≤ firstReach N hex + Δ + Δ := cut_before N hu hi
    have htv : tc.view = w - 1 := view_pred hwt
    have hte := late_tc_epoch hcfg hs hst hin (by rw [htv]; show cfg.anchorView.toNat + 2 ≤ w.toNat - 1; omega) (htv ▸ hl)
    refine ⟨hte, ?_⟩
    -- A lock of `E` the leader, a member of `E`, can lock on; one of an earlier epoch does not matter.
    have hle : tc.data.lock.data.epoch.toNat ≤ E.toNat := by
      rcases (N.timeoutLockGenuine l hl' i tc hin).1 with ha | hb
      · rw [ha]; exact stable_anchor hst
      · exact stable_no_later hcfg hcf hs hst hb
    by_cases hlt : tc.data.lock.data.epoch.toNat < E.toNat
    · exact Or.inl hlt
    have hlkE : tc.data.lock.data.epoch = E := EpochNumber.ext (by omega)
    have := Nat.max_le.mpr ⟨hti, show GST ≤ firstReach N hex + Δ + Δ by omega⟩
    exact Or.inr (by_cut N hu (fun a b hab h => lockable_grows (received_grows _ hab) h)
      (by_later (by omega) (hs.timeoutLockSpread tc l hl' i
        ((Trace.received_history _).mpr ⟨i, Nat.lt_succ_self _, hin⟩) l hl' (hlkE ▸ hmem)
        (.of (by rw [hte]; exact hlE)))))
  have ht01 : cut N hu l hl' t0 ≤ cut N hu l hl' (firstReach N hex + Δ + Δ) := cut_mono N hu (by omega)
  have ht0 : cut N hu l hl' t0 ≤ cut N hu l hl' (firstReach N hex + Δ + 2 * Δ) := cut_mono N hu (by omega)
  -- What the leader may send, by `4Δ`.
  have hnbE : ∀ n, cut N hu l hl' (firstReach N hex + Δ + 2 * Δ) ≤ n →
      NotBehind cfg ((N.trace l hl').history n) E := fun n hn =>
    stable_notBehind hst hu (.of hlE) (Nat.le_trans ht0 hn)
  have hact : (∃ p, ProposalJustified cfg leader l ((N.trace l hl').history
        (cut N hu l hl' (firstReach N hex + Δ + 3 * Δ))) p ∧ p.viewNumber = w ∧ p.epoch = E)
      ∨ ∃ r, RevoteJustified cfg leader l ((N.trace l hl').history
        (cut N hu l hl' (firstReach N hex + Δ + 3 * Δ))) r ∧ r.view = w ∧ r.cert.data.epoch = E := by
    rcases leader_ready hcfg hcf hs hst hw hl (.of hlE) hlead hc12 ht01 hin1 hin2
        (fun L hL => hno l hl' L (hasCert1_of_lockable (lockable_grows (received_grows _ hc2T) hL)))
        (fun c hc hce => by
          have := lock_of_held hs (j := l) (hj := hl') (k := l) (hk := hl')
            (T := firstReach N hex + Δ + Δ) (by omega) (hce ▸ hmem) (.of (by rw [hce]; exact hlE)) hc
          rwa [show firstReach N hex + Δ + Δ + Δ = firstReach N hex + Δ + 2 * Δ by omega] at this) htc with
      ⟨pc, parent, hrd, hlt, hbd, hpe⟩ | ⟨r, hr, hrv, hre⟩
    · left
      obtain ⟨n2', hn2', ht2⟩ := cut_pred N hu hn2
      -- Ready but for a header, the leader is handed one.
      obtain ⟨ev0, hready0⟩ := ready_of_tuple (Nat.le_refl _) hrd hlt hbd
        (hdr := ⟨⟨0⟩, pc.data.blockNumber + 1⟩) rfl (by rw [hpe]; exact hnbE _ (Nat.le_refl _))
        ⟨w, hin2.1, Nat.le_refl _⟩
      have hlE' : C.honest (epochOf (pc.data.blockNumber + 1) cfg.epochHeight) l := by rw [hpe]; exact hlE
      have hhdr := by_later (show max (N.time l hl' n2') GST + Δ ≤ firstReach N hex + Δ + 3 * Δ by
        have := Nat.max_le.mpr ⟨ht2, show GST ≤ firstReach N hex + Δ + 2 * Δ by omega⟩; omega)
        (hs.header l hl' n2' _ hlE' (hn2' ▸ hin2) (hn2' ▸ hready0))
      obtain ⟨hdr, hnum, hrec⟩ := by_cut N hu
        (fun a b hab ⟨hdr, h1, h2⟩ => ⟨hdr, h1, received_grows _ hab _ h2⟩) hhdr
      have hin3 := view_is_w hcfg hcf hs hst hw hex hl (by omega) hT hno l hl' (.of hlE) (Nat.le_trans hc12 hc23)
        (cut_mono N hu (by omega))
      obtain ⟨ev, hready⟩ := ready_of_tuple hc23 hrd hlt hbd (hdr := hdr) hnum
        (by rw [hpe]; exact hnbE _ hc23) ⟨w, hin3.1, Nat.le_refl _⟩
      exact ⟨_, ⟨hready, hrec⟩, rfl, by show epochOf hdr.blockNumber _ = E; rw [hnum, hpe]⟩
    · exact Or.inr ⟨r, revoteJustified_grows _ hc23 hr (by rw [hre]; exact hnbE _ hc23), hrv, hre⟩
  obtain ⟨n3', hn3', ht3⟩ := cut_pred N hu (k := l) (hk := hl') (T := firstReach N hex + Δ + 3 * Δ)
    (fun h => hn2 (by omega))
  refine Classical.byContradiction fun hneg => not_owed_throughout N hp (k := l) (hk := hl') (n := n3')
    (B := firstReach N hex + Δ + 3 * Δ + δ) (o := .propose E w) (by omega) fun m hm htm => ?_
  have hle : cut N hu l hl' (firstReach N hex + Δ + 3 * Δ) ≤ m + 1 := by omega
  have hmT : m + 1 ≤ cut N hu l hl' T :=
    le_cut N hu fun i hi => Nat.le_trans (time_mono N l hl' (Nat.le_of_lt_succ hi)) (by omega)
  have hsi := sameInputs_restrict (P := (C.honest · l)) (h := (N.trace l hl').history (m + 1))
  refine ⟨hlE, ?_, fun hprop => ?_, fun ht => ?_, (hsi.inView _).mpr ?_⟩
  · rcases hact with ⟨p, hpj, hpv, hpe⟩ | ⟨r, hrj, hrv, hre⟩
    · exact Or.inl ⟨p, (hsi.proposalJustified p).mpr
        (proposalJustified_grows _ hle hpj (by rw [hpe]; exact hnbE _ (by omega))), hpv, hpe⟩
    · exact Or.inr ⟨r, (hsi.revoteJustified r).mpr
        (revoteJustified_grows _ hle hrj (by rw [hre]; exact hnbE _ (by omega))), hrv, hre⟩
  · rcases hprop with ⟨p', hs', hv', -⟩ | ⟨r', hs', hv', -⟩
    · obtain ⟨hs', he'⟩ := restrict_sent.mp hs'
      obtain ⟨j, hj, hjm⟩ := (Trace.sent_history _).mp hs'
      exact hneg ⟨j, Or.inl ⟨p', hjm, hv', he'⟩, Nat.le_trans (time_mono N l hl' (Nat.le_of_lt_succ hj)) htm⟩
    · obtain ⟨hs', he'⟩ := restrict_sent.mp hs'
      obtain ⟨j, hj, hjm⟩ := (Trace.sent_history _).mp hs'
      exact hneg ⟨j, Or.inr ⟨r', hjm, hv', he'⟩, Nat.le_trans (time_mono N l hl' (Nat.le_of_lt_succ hj)) htm⟩
  · refine not_past hcfg hs hw hex (n := m + 1) (fun i hi => ?_) (show w ≤ w from Nat.le_refl w.toNat)
      (Or.inl ht)
    have := time_mono N l hl' (Nat.le_of_lt_succ hi)
    omega
  · exact view_is_w hcfg hcf hs hst hw hex hl (by omega) hT hno l hl' (.of hlE) (Nat.le_trans (Nat.le_trans hc12 hc23) hle)
      hmT

end Leader

/-! ## The members vote1 -/

section Members

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ) {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
  {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hl : Late N t0 (w - 1))
include hcfg hcf hs hst hw hl

omit hcfg hcf hs hst hw hl in
/-- An honest node's proposal is one it may make, once it has taken the step that sends it. -/
theorem sent_justified {l : PubKey} {hl' : C.Honest l} {jp : Nat} {pw : Proposal}
    (hsend : Output.send (.proposal pw) ∈ (N.trace l hl' jp).output) (he : C.honest pw.epoch l) :
    ProposalJustified cfg leader l ((N.trace l hl').history (jp + 1)) pw := by
  have := (N.protocol l hl' (jp + 1)).proposeJustified jp pw ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self jp)), hsend⟩ he
  rwa [Trace.history_upTo _ (Nat.le_refl _)] at this

omit hcfg hcf hs hst hw hl in
/-- An honest node's re-vote request is one it may send, once it has taken the step that sends it. -/
theorem sent_revote_justified {l : PubKey} {hl' : C.Honest l} {jr : Nat} {rw : RevoteRequest}
    (hsend : Output.send (.revote rw) ∈ (N.trace l hl' jr).output) (he : C.honest rw.cert.data.epoch l) :
    RevoteJustified cfg leader l ((N.trace l hl').history (jr + 1)) rw := by
  have := (N.protocol l hl' (jr + 1)).revoteJustified jr rw ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self jr)), hsend⟩ he
  rwa [Trace.history_upTo _ (Nat.le_refl _)] at this

omit hs hst hw hl in
/-- The block a backed certificate, or the anchor's, is over is at its view or an earlier one. -/
theorem cert_block_le {c : Cert1} (hc : Cert1Backed N.trace c ∨ c = cfg.anchorCert) {b : Block}
    (hb : c.data.blockHash = blockHash b) : b.viewNumber ≤ c.view := by
  rcases hc with hcb | rfl
  · obtain ⟨b0, hv0, hh0, -, -⟩ := cert1Backed_block hcb
    rw [← hcf b0 b (by rw [← hh0, hb])]; exact hv0
  · rw [hcf b cfg.anchorBlock (by rw [← hb, hcfg.anchorCertBlock]), hcfg.anchorCertView]
    exact Nat.le_refl _

omit hcf hs hst hw hl in
/-- A backed certificate, or the anchor's, is at the anchor's view or later. -/
theorem anchor_view_le {c : Cert1} (hcg : Cert1Backed N.trace c ∨ c = cfg.anchorCert) :
    cfg.anchorView ≤ c.view := by
  rcases hcg with hb | rfl
  · exact Nat.le_of_lt (cert1_after_anchor N.toNetwork hcfg _ _ (Nat.le_refl _) hb)
  · rw [hcfg.anchorCertView]; exact Nat.le_refl _

omit hcf hs hst hw hl in
/-- The anchor's certificate is over the anchor block. -/
theorem anchor_cert_data :
    cfg.anchorCert.data = ⟨blockHash cfg.anchorBlock, cfg.anchorBlock.epoch,
      cfg.anchorBlock.blockHeader.blockNumber⟩ := by
  show (⟨cfg.anchorCert.data.blockHash, cfg.anchorCert.data.epoch, cfg.anchorCert.data.blockNumber⟩
    : Vote1Data) = _
  rw [hcfg.anchorCertBlock, hcfg.anchorCertBlockNumber, hcfg.anchorBlockEpoch]

/--
Every honest vote1 at `w` is for the honest leader's one proposal for `w`, when it
sent one: the vote is for epoch `E` (`Liveness.late_vote1`), so for the leader of `w` in
`E`, which sends at most one proposal or re-vote request per view.
-/
theorem vote1_at_proposal {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    {jp : Nat} {pw : Proposal} (hsend : Output.send (.proposal pw) ∈ (N.trace l (.of hlE) jp).output)
    (hpw : pw.viewNumber = w) (hpe : pw.epoch = E) {k : PubKey} {hk : C.Honest k} {i : Nat} {v : Vote1}
    (hi : Output.send (.vote1 v) ∈ (N.trace k hk i).output) (he : C.honest v.data.epoch k) (hv : v.view = w) :
    v = ⟨⟨blockHash pw, pw.epoch, pw.blockHeader.blockNumber⟩, w, k⟩ := by
  have hl' : C.Honest l := .of hlE
  have hlw : Late N t0 v.view := hv ▸ late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)
  have hsig := ((N.protocol k hk (i + 1)).vote1Justified i v ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self i)), hi⟩ he).1
  rcases late_vote1 hcfg hcf hs hst hi he hlw with ⟨sender, p, vid, hrec, hlead', hfor, hpE⟩
    | ⟨sender, r, hrec, hlead', hfor, hrE⟩
  · rw [hv, hlead] at hlead'
    have hsl : sender = l := (Option.some.inj hlead').symm
    subst hsl
    obtain ⟨j, -, hjin⟩ := (Trace.received_history _).mp hrec
    obtain ⟨i', hi', -⟩ := N.authentic k hk j sender (.proposal p) (by rw [hjin]; rfl) hl' (show C.honest p.epoch sender by rw [hpE]; exact hlE)
    have hsame : p = pw := (N.protocol sender hl' (max i' jp + 1)).proposal_once p pw
      ((Trace.sent_history _).mpr ⟨i', by omega, hi'⟩)
      ((Trace.sent_history _).mpr ⟨jp, by omega, hsend⟩) (by rw [hpE]; exact hlE) (hpE.trans hpe.symm)
      (by rw [← hfor.1, hv, hpw])
    subst hsame
    obtain ⟨data, view, signer⟩ := v
    have hd : data = ⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩ := hfor.2
    simp only at hv hsig
    subst hd; subst hv; subst hsig
    rfl
  · exfalso
    rw [hv, hlead] at hlead'
    have hsl : sender = l := (Option.some.inj hlead').symm
    subst hsl
    obtain ⟨j, -, hjin⟩ := (Trace.received_history _).mp hrec
    obtain ⟨i', hi', -⟩ := N.authentic k hk j sender (.revote r) (by rw [hjin]; rfl) hl' (show C.honest r.cert.data.epoch sender by rw [hrE]; exact hlE)
    exact (N.protocol sender hl' (max i' jp + 1)).revote_once r
      ((Trace.sent_history _).mpr ⟨i', by omega, hi'⟩) (by rw [hrE]; exact hlE) |>.2 pw
      ((Trace.sent_history _).mpr ⟨jp, by omega, hsend⟩) (hpe.trans hrE.symm) (by rw [hpw, ← hv, hfor.1])

/-- Every honest vote1 at `w` answers the honest leader's one re-vote request for `w`, when it sent one. -/
theorem vote1_at_revote {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    {jr : Nat} {rw : RevoteRequest} (hsend : Output.send (.revote rw) ∈ (N.trace l (.of hlE) jr).output)
    (hrw : rw.view = w) (hre : rw.cert.data.epoch = E) {k : PubKey} {hk : C.Honest k} {i : Nat} {v : Vote1}
    (hi : Output.send (.vote1 v) ∈ (N.trace k hk i).output) (he : C.honest v.data.epoch k) (hv : v.view = w) :
    v = ⟨rw.cert.data, w, k⟩ := by
  have hl' : C.Honest l := .of hlE
  have hlw : Late N t0 v.view := hv ▸ late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)
  have hsig := ((N.protocol k hk (i + 1)).vote1Justified i v ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self i)), hi⟩ he).1
  rcases late_vote1 hcfg hcf hs hst hi he hlw with ⟨sender, p, vid, hrec, hlead', hfor, hpE⟩
    | ⟨sender, r, hrec, hlead', hfor, hrE⟩
  · exfalso
    rw [hv, hlead] at hlead'
    have hsl : sender = l := (Option.some.inj hlead').symm
    subst hsl
    obtain ⟨j, -, hjin⟩ := (Trace.received_history _).mp hrec
    obtain ⟨i', hi', -⟩ := N.authentic k hk j sender (.proposal p) (by rw [hjin]; rfl) hl' (show C.honest p.epoch sender by rw [hpE]; exact hlE)
    exact (N.protocol sender hl' (max i' jr + 1)).revote_once rw
      ((Trace.sent_history _).mpr ⟨jr, by omega, hsend⟩) (by rw [hre]; exact hlE) |>.2 p
      ((Trace.sent_history _).mpr ⟨i', by omega, hi'⟩) (hpE.trans hre.symm) (by rw [hrw, ← hv, hfor.1])
  · rw [hv, hlead] at hlead'
    have hsl : sender = l := (Option.some.inj hlead').symm
    subst hsl
    obtain ⟨j, -, hjin⟩ := (Trace.received_history _).mp hrec
    obtain ⟨i', hi', -⟩ := N.authentic k hk j sender (.revote r) (by rw [hjin]; rfl) hl' (show C.honest r.cert.data.epoch sender by rw [hrE]; exact hlE)
    have hsame : r = rw := ((N.protocol sender hl' (max i' jr + 1)).revote_once rw
      ((Trace.sent_history _).mpr ⟨jr, by omega, hsend⟩) (by rw [hre]; exact hlE)).1 r
      ((Trace.sent_history _).mpr ⟨i', by omega, hi'⟩) (hrE.trans hre.symm) (by rw [← hfor.1, hv, hrw])
    subst hsame
    obtain ⟨data, view, signer⟩ := v
    have hd : data = r.cert.data := hfor.2
    simp only at hv hsig
    subst hd; subst hv; subst hsig
    rfl

omit hcfg hcf hs hst hw hl in
/--
The certificate an honest leader builds on, or votes on again: one over the anchor,
or over the block of a certificate it held, or, for the last block of an epoch,
the block of a `Cert2` it held.

Without timeout evidence the leader holds the certificate itself. With evidence it
was locked on a certificate over the same block, or, for the last block of an
epoch, held a `Cert2` over it.
-/
theorem cert_held {l : PubKey} {hl' : C.Honest l} {jp : Nat} {c : Cert1} {ev : Option TimeoutCert}
    (hcj : CertJustified cfg ((N.trace l hl').history (jp + 1)) c ev) :
    c.data = cfg.anchorCert.data
      ∨ (∃ m, m ≤ jp ∧ ∃ L, ((N.trace l hl').history (m + 1)).HasCert1 cfg L ∧ L.data = c.data)
      ∨ (IsLastBlock c.data.blockNumber cfg.epochHeight
        ∧ ∃ c2, ((N.trace l hl').history (jp + 1)).HasCert2 c2 ∧ c2.data = c.data.toVote2) := by
  cases ev with
  | none =>
    rcases (hcj : ((N.trace l hl').history (jp + 1)).Buildable cfg c) with rfl | ⟨hc, -⟩
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨jp, Nat.le_refl _, c, hc, rfl⟩)
  | some tc =>
    obtain ⟨-, m0, -, hcase⟩ := hcj
    rw [upTo_history] at hcase
    rcases hcase with ⟨L, hL, hLd⟩ | ⟨hlast, c2, hc2, hc2d⟩
    · by_cases hz : min m0 (jp + 1) = 0
      · rw [hz] at hL
        refine Or.inl ?_
        have hLa := lockable_nil hL.1
        rw [← hLd, hLa]
      · obtain ⟨m2, hm2⟩ : ∃ m2, min m0 (jp + 1) = m2 + 1 :=
          ⟨_ - 1, (Nat.succ_pred_eq_of_ne_zero hz).symm⟩
        rw [hm2] at hL
        exact Or.inr (Or.inl ⟨m2, by have := Nat.min_le_right m0 (jp + 1); omega, L,
          hasCert1_of_lockable hL.1, hLd⟩)
    · refine Or.inr (Or.inr ⟨hlast, c2, ?_, hc2d⟩)
      exact hasCert2_grows (received_grows _ (Nat.min_le_right _ _)) hc2

omit hcf hw hl hcfg in
/--
No honest node holds a `Cert2` over the last block of `E`.

An honest signer of it held the block, and it gets the `Cert2` within `Δ`; then it
holds both, which a stable epoch rules out.
-/
theorem stable_no_cert2_last {j : PubKey} {hj : C.Honest j} {n : Nat} {c2 : Cert2}
    (hc2 : ((N.trace j hj).history n).HasCert2 c2)
    (hlast : IsLastBlock c2.data.blockNumber cfg.epochHeight) (he : c2.data.epoch = E) : False := by
  have hu := hs.timeUnbounded
  obtain ⟨q, hq, hcast⟩ := cert2_held_backed N.toNetwork hc2
  obtain ⟨k', hqk, -, hk'e⟩ := C.intersect _ q q hq hq
  have hk' : C.Honest k' := .of hk'e
  obtain ⟨i, hi⟩ := hcast k' hqk hk'e
  obtain ⟨-, -, c, b, -, hb, hcert, -, hvv, hvd⟩ := (N.safe_at k' hk' i).vote2Justified i _ ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self i)), hi⟩ hk'e
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hb
  have hcq : Commits c2 b := by
    refine ⟨Nat.le_trans hcert.1 (by rw [show c2.view = c.view from hvv]; exact Nat.le_refl _), ?_⟩
    rw [show c2.data = c.data.toVote2 from hvd, hcert.2]; rfl
  cases n with
  | zero =>
    rcases hc2 with h | ⟨_, _, h⟩ <;>
      · obtain ⟨_, hj', -⟩ := (Trace.received_history _).mp h; exact absurd hj' (Nat.not_lt_zero _)
  | succ n =>
    obtain ⟨m, -, hc2'⟩ := hs.cert2Spread c2 j hj n hc2 k' hk' (.of hk'e)
    have hbe : b.epoch = E := by
      have := congrArg Vote2Data.epoch hcq.2
      simp only at this; rw [← this, he]
    have hbl : IsLastBlock b.blockHeader.blockNumber cfg.epochHeight := by
      have := congrArg Vote2Data.blockNumber hcq.2
      simp only at this; rw [← this]; exact hlast
    exact stable_no_commit hs hst
      (hasProposal_grows (received_grows _ (Nat.le_max_left (i + 1) m)) hb)
      (hasCert2_grows (received_grows _ (Nat.le_max_right (i + 1) m)) hc2') hcq hbl hbe

omit hw hl in
/--
Within `Δ` of an honest node holding a certificate, after GST, every honest node
holds the proposal and payload of the block of any certificate over the same block.
Over the anchor, it holds them from the start.
-/
theorem block_ready {l : PubKey} {hl' : C.Honest l} {m : Nat} {L c : Cert1}
    (hL : ((N.trace l hl').history (m + 1)).HasCert1 cfg L) (hLd : L.data = c.data)
    (hcg : Cert1Backed N.trace c ∨ c = cfg.anchorCert) (hce : c.data.epoch = E) {Tp : Nat}
    (htm : N.time l hl' m ≤ Tp) (hgstp : GST ≤ Tp) (k : PubKey) (hk : C.Honest k) (hkm : C.members E k)
    (hkE : C.HonestFrom E k) :
    ∃ b, ((N.trace k hk).history (cut N hs.timeUnbounded k hk (Tp + Δ))).HasProposal cfg b
      ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩ ∧ b.viewNumber ≤ c.view
      ∧ ((N.trace k hk).history (cut N hs.timeUnbounded k hk (Tp + Δ))).HasPayload cfg b.viewNumber b.payloadCommit := by
  have := Nat.max_le.mpr ⟨htm, hgstp⟩
  have hanchor : L = cfg.anchorCert → ∃ b, ((N.trace k hk).history (cut N hs.timeUnbounded k hk (Tp + Δ))).HasProposal cfg b
      ∧ c.data = ⟨blockHash b, b.epoch, b.blockHeader.blockNumber⟩ ∧ b.viewNumber ≤ c.view
      ∧ ((N.trace k hk).history (cut N hs.timeUnbounded k hk (Tp + Δ))).HasPayload cfg b.viewNumber
        b.payloadCommit := fun hLa =>
    ⟨cfg.anchorBlock, Or.inl rfl, by rw [← hLd, hLa]; exact anchor_cert_data hcfg, anchor_view_le hcfg hcg,
      Or.inl ⟨rfl, rfl⟩⟩
  by_cases hLa : L = cfg.anchorCert
  · exact hanchor hLa
  have hLb : Cert1Backed N.trace L :=
    (cert1_held_backed N.toNetwork hL).resolve_left hLa
  have hkm' : C.members L.data.epoch k := by rw [hLd, hce]; exact hkm
  have hlock := by_cut N hs.timeUnbounded (fun a b hab h => lockable_grows (received_grows _ hab) h)
    (by_later (show max (N.time l hl' m) GST + Δ ≤ Tp + Δ by omega)
      (hs.lockSpread L l hl' m hL k hk hkm' (by rw [hLd, hce]; exact hkE)))
  rcases hlock with hanc | ⟨-, b, hb, hcert, hpay⟩ | ⟨c2', p', hrec', hwf'⟩
  · exact hanchor hanc
  · refine ⟨b, hb, by rw [← hLd]; exact hcert.2, cert_block_le hcfg hcf hcg ?_, hpay⟩
    rw [← hLd]; exact congrArg Vote1Data.blockHash hcert.2
  · exfalso
    have hcq : Commits c2' p' := ⟨hwf'.1, by rw [← hwf'.sameData, hwf'.cert1Data]; rfl⟩
    refine stable_no_commit hs hst (Or.inr (Or.inr ⟨L, c2', hrec'⟩))
      (Or.inr ⟨L, p', hrec'⟩) hcq hwf'.last ?_
    have := congrArg Vote1Data.epoch hwf'.cert1Data
    simp only at this
    rw [← this, hLd, hce]


omit hcfg hcf hs hst hw hl in
/-- A late view reached by a step was reached after `t0`. -/
theorem reached_late_time {k : PubKey} {hk : C.Honest k} {j : Nat} {u : ViewNumber}
    (hr : Reached N k hk (j + 1) u) (hlu : Late N t0 u) : t0 < N.time k hk j :=
  Nat.lt_of_not_le fun hle => hlu ⟨k, hk, j + 1,
    fun _ hi => Nat.le_trans (time_mono N k hk (Nat.le_of_lt_succ hi)) hle, hr⟩

omit hcfg hcf hs hst hw hl in
/-- The certificate a proposal or re-vote request an honest node made builds on is backed or the anchor's. -/
theorem certJustified_genuine {l : PubKey} {hl' : C.Honest l} {n : Nat} {c : Cert1}
    {ev : Option TimeoutCert} (hcj : CertJustified cfg ((N.trace l hl').history n) c ev) :
    Cert1Backed N.trace c ∨ c = cfg.anchorCert := by
  have hc : ((N.trace l hl').history n).HasCert1 cfg c := by
    cases ev with
    | none => exact hasCert1_of_buildable hcj
    | some tc => exact hcj.1
  exact (cert1_held_backed N.toNetwork hc).symm

/-- The honest leader's proposal for `w` is of epoch `E`. -/
theorem sent_epoch {l : PubKey} {hl' : C.Honest l} {jp : Nat} {pw : Proposal}
    (hsend : Output.send (.proposal pw) ∈ (N.trace l hl' jp).output) (he : C.honest pw.epoch l)
    (hpw : pw.viewNumber = w) : pw.epoch = E := by
  have hj := sent_justified hsend he
  have hT := reached_late_time hj.reached (by
    rw [hpw]; exact late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega))
  have hlE : C.HonestFrom E l := Classical.byContradiction fun h => late_retired hs hst h hT he hj.current
  exact proposal_epoch_of hcfg hcf hs hst (late_inEpoch hs hst hlE hT) hj.wellFormed
    (certJustified_genuine hj.justified) hj.opens hj.current

/-- The honest leader's re-vote request for `w` is over a certificate of epoch `E`. -/
theorem sent_revote_epoch {l : PubKey} {hl' : C.Honest l} {jr : Nat} {rw : RevoteRequest}
    (hsend : Output.send (.revote rw) ∈ (N.trace l hl' jr).output) (he : C.honest rw.cert.data.epoch l)
    (hrw : rw.view = w) : rw.cert.data.epoch = E := by
  have hj := sent_revote_justified hsend he
  have hT := reached_late_time hj.reached (by
    rw [hrw]; exact late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega))
  have hlE : C.HonestFrom E l := Classical.byContradiction fun h => late_retired hs hst h hT he hj.current
  have hE := late_inEpoch hs hst hlE hT
  refine EpochNumber.ext (Nat.le_antisymm ?_ (hj.current E hE))
  rcases certJustified_genuine hj.justified with hb | ha
  · exact stable_no_later hcfg hcf hs hst hb
  · rw [ha]; exact stable_anchor hst

/--
Within `2Δ` of the honest leader's proposal for `w`, every honest member of `E`
holds it with its share and its validity report, and can read what a vote1 reads
of its parent.

A parent before the epoch's first block spreads from the leader, which holds its
certificate. The first
block of `E` comes behind the last block of the epoch before, which the leader holds
with a `Cert2`, so the epoch change brings both to every member.
-/
theorem member_ready {δ : Nat} (hex : ∃ T, ReachedBy N T w)
    {l : PubKey} {hl' : C.Honest l} {jp : Nat} {pw : Proposal}
    (hsend : Output.send (.proposal pw) ∈ (N.trace l hl' jp).output) (hpe : C.honest pw.epoch l)
    (hpw : pw.viewNumber = w) (hjt : N.time l hl' jp ≤ firstReach N hex + Δ + 3 * Δ + δ)
    (k : PubKey) (hk : C.Honest k) (hmem : C.members E k) (hkE : C.HonestFrom E k) :
    let h := (N.trace k hk).history (cut N hs.timeUnbounded k hk (firstReach N hex + Δ + 5 * Δ + δ))
    ∃ vid, ShareMatches pw vid ∧ h.Received (.proposal l pw (some vid))
      ∧ h.Received (.blockValidated pw.viewNumber (blockHash pw)) ∧ ParentReady cfg h pw
      ∧ OpensEpochJustified cfg h pw := by
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst
    (Nat.le_of_lt (late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex))
  have hu := hs.timeUnbounded
  have hjust := sent_justified hsend hpe
  have hep := sent_epoch hcfg hcf hs hst hw hl hsend hpe hpw
  have hgstp : GST ≤ firstReach N hex + Δ + 3 * Δ + δ := by omega
  have hmax0 := Nat.max_le.mpr ⟨hjt, hgstp⟩
  have hcut : cut N hu k hk (firstReach N hex + Δ + 3 * Δ + δ + Δ) ≤
      cut N hu k hk (firstReach N hex + Δ + 5 * Δ + δ) := cut_mono N hu (by omega)
  obtain ⟨n0, hn0, vid, hsm, hrec0⟩ := hs.proposal l hl' jp pw hsend hpe k hk (by rw [hep]; exact hmem)
    (by rw [hep]; exact hkE)
  obtain ⟨j0, hj0, hin0⟩ := (Trace.received_history _).mp hrec0
  have htj0 : N.time k hk j0 ≤ firstReach N hex + Δ + 4 * Δ + δ := by
    have := hn0 j0 hj0; omega
  have hmax1 := Nat.max_le.mpr ⟨htj0, show GST ≤ firstReach N hex + Δ + 4 * Δ + δ by omega⟩
  have hn0c : n0 ≤ cut N hu k hk (firstReach N hex + Δ + 5 * Δ + δ) :=
    le_cut N hu fun i hi => by have := hn0 i hi; omega
  have hpcg := certJustified_genuine hjust.justified
  refine ⟨vid, hsm, received_grows _ hn0c _ hrec0, ?_, ?_, ?_⟩
  · exact by_cut N hu (fun a b hab h => received_grows _ hab _ h)
      (by_later (show max (N.time k hk j0) GST + Δ ≤ firstReach N hex + Δ + 5 * Δ + δ by omega)
        (hs.validated k hk j0 l pw vid hin0 (hs.proposalValid l hl' jp pw hsend hpe) (by rw [hep]; exact hkE)))
  · by_cases hent : EntersEpoch cfg pw
    · exact Or.inr (Or.inl hent)
    have hnl : ¬ IsLastBlock pw.parentCert.data.blockNumber cfg.epochHeight := fun hlast => hent (by
      show IsLastBlock (pw.blockHeader.blockNumber - 1) cfg.epochHeight
      rw [← hjust.wellFormed.height]; simpa using hlast)
    have hpce : pw.parentCert.data.epoch = E := by
      rw [cert1_epoch_of_height N.toNetwork hcfg hpcg, ← epochOf_next_same hnl, hjust.wellFormed.height,
        ← hjust.wellFormed.epoch, hep]
    rcases cert_held hjust.justified with hg | ⟨m, hm, L, hL, hLd⟩ | ⟨hlast, -⟩
    · exact Or.inr (Or.inr ⟨cfg.anchorBlock, Or.inl rfl, anchor_view_le hcfg hpcg,
        by rw [hg, hcfg.anchorCertBlock], Or.inl ⟨rfl, rfl⟩⟩)
    · obtain ⟨b, hb, hbd, hbv, hpay⟩ := block_ready hcfg hcf hs hst hL hLd hpcg hpce
          (Nat.le_trans (time_mono N l hl' (by omega : m ≤ jp)) hjt) hgstp k hk hmem hkE
      exact Or.inr (Or.inr ⟨b, hasProposal_grows (received_grows _ hcut) hb, hbv,
        congrArg Vote1Data.blockHash hbd, hasPayload_grows (received_grows _ hcut) hpay⟩)
    · exact absurd hlast hnl
  · intro hent
    obtain ⟨⟨q, hq, hqv, hqh⟩, c2, hc2, hc2v, hc2d⟩ := hjust.opens hent
    have hlastpc : IsLastBlock pw.parentCert.data.blockNumber cfg.epochHeight := by
      have h := hent
      unfold EntersEpoch at h
      rwa [← hjust.wellFormed.height, BlockNumber.add_sub_cancel] at h
    rcases hc2 with hc2 | hc2a
    case inr =>
      -- The anchor's `Cert2`: the parent is the anchor, which every node holds.
      have hqa : q = cfg.anchorBlock := hcf q _ (by
        have h1 := congrArg Vote2Data.blockHash hc2d
        rw [hc2a] at h1
        have h2 : cfg.anchorCert.data.blockHash = pw.parentCert.data.blockHash := h1
        rw [← hqh, ← h2, hcfg.anchorCertBlock])
      subst hqa
      exact ⟨⟨cfg.anchorBlock, Or.inl rfl, hqv, hqh⟩, c2, Or.inr hc2a, hc2v, hc2d⟩
    rcases hpcg with hpcb | ha
    case inr =>
      -- The parent is the anchor, which every node holds; the `Cert2` spreads.
      have hqa : q = cfg.anchorBlock := hcf q _ (by rw [← hqh, ha, hcfg.anchorCertBlock])
      subst hqa
      have hc2e : c2.data.epoch.toNat ≤ E.toNat := by
        rw [show c2.data.epoch = pw.parentCert.data.epoch from congrArg Vote2Data.epoch hc2d, ha]
        exact stable_anchor hst
      obtain ⟨m, hm, hc2k⟩ := hs.cert2Spread c2 l hl' jp hc2 k hk (hkE.mono hc2e)
      have hmc : m ≤ cut N hu k hk (firstReach N hex + Δ + 5 * Δ + δ) :=
        le_cut N hu fun i hi => by have := hm i hi; omega
      exact ⟨⟨cfg.anchorBlock, Or.inl rfl, hqv, hqh⟩, c2, Or.inl (hasCert2_grows (received_grows _ hmc) hc2k),
        hc2v, hc2d⟩
    obtain ⟨b, -, hbd⟩ := cert1_data_eq hpcb
    have hbq : b = q := hcf b q (by
      have h1 := congrArg Vote1Data.blockHash hbd
      simp only at h1; rw [← h1, hqh])
    subst hbq
    have hlast : IsLastBlock b.blockHeader.blockNumber cfg.epochHeight := by
      have h2 := congrArg Vote1Data.blockNumber hbd
      simp only at h2
      rw [← h2]; exact hlastpc
    have hcq : Commits c2 b := by
      obtain ⟨c', hc'b, hc'v, hc'd⟩ := cert2_backed_data hcfg hc2
      refine ⟨?_, ?_⟩
      · rw [← hc'v]
        exact cert_block_le hcfg hcf (Or.inl hc'b) (by
          have h1 := congrArg Vote2Data.blockHash hc'd
          have h2 := congrArg Vote2Data.blockHash hc2d
          simp only [Vote1Data.toVote2] at h1 h2
          rw [← h1, h2, hqh])
      · rw [hc2d, hbd]; rfl
    have hc2e : c2.data.epoch.toNat ≤ E.toNat := by
      rw [show c2.data.epoch = pw.parentCert.data.epoch from congrArg Vote2Data.epoch hc2d]
      exact stable_no_later hcfg hcf hs hst hpcb
    obtain ⟨m, hm, c1', htook⟩ := hs.epochChange c2 b hcq hlast l hl' jp ⟨hq, hc2⟩ k hk (hkE.mono hc2e)
    have hmc : m ≤ cut N hu k hk (firstReach N hex + Δ + 5 * Δ + δ) :=
      le_cut N hu fun i hi => by have := hm i hi; omega
    exact ⟨⟨b, hasProposal_grows (received_grows _ hmc) (Or.inr (Or.inr ⟨c1', c2, htook.1⟩)),
      hqv, hqh⟩, c2, Or.inl (hasCert2_grows (received_grows _ hmc) (Or.inr ⟨c1', b, htook.1⟩)), hc2v, hc2d⟩

/-- Every honest member of `E` votes1 on the honest leader's proposal for `w` within `2Δ + δ` of it. -/
theorem members_vote1 {δ : Nat} (hp : Prompt N δ) (hex : ∃ T, ReachedBy N T w)
    {T : Nat} (hT1 : firstReach N hex + Δ + 5 * Δ + 2 * δ ≤ T) (hT : T < firstReach N hex + τ)
    (hno : ∀ j (hj : C.Honest j) c,
      ((N.trace j hj).history (cut N hs.timeUnbounded j hj T)).HasCert1 cfg c → c.view < w)
    {l : PubKey} (hl' : C.Honest l) (hlead : leader E w = some l) {jp : Nat} {pw : Proposal}
    (hsend : Output.send (.proposal pw) ∈ (N.trace l hl' jp).output) (hpe : C.honest pw.epoch l)
    (hpw : pw.viewNumber = w) (hjt : N.time l hl' jp ≤ firstReach N hex + Δ + 3 * Δ + δ)
    (k : PubKey) (hk : C.Honest k) (hkE : C.honest E k) (hmem : C.members E k) :
    N.SentByTime k hk (firstReach N hex + Δ + 5 * Δ + 2 * δ)
      (.vote1 ⟨⟨blockHash pw, pw.epoch, pw.blockHeader.blockNumber⟩, w, k⟩) := by
  have hu := hs.timeUnbounded
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  have hjust := sent_justified hsend hpe
  have hep := sent_epoch hcfg hcf hs hst hw hl hsend hpe hpw
  have hlE : C.honest E l := hep ▸ hpe
  obtain ⟨vid, hsm, hrecp, hval, hpr, hopen⟩ :=
    member_ready hcfg hcf hs hst hw hl hex hsend hpe hpw hjt k hk hmem (.of hkE)
  obtain ⟨n2', hn2', ht2⟩ := cut_pred N hu (k := k) (hk := hk) (T := firstReach N hex + Δ + 5 * Δ + δ)
    (fun h => by
      obtain ⟨_, hj, -⟩ := (Trace.received_history _).mp hval
      rw [h] at hj; exact absurd hj (Nat.not_lt_zero _))
  refine Classical.byContradiction fun hneg => not_owed_throughout N hp (k := k) (hk := hk)
    (n := n2') (B := firstReach N hex + Δ + 5 * Δ + 2 * δ) (o := .vote1 pw) (by omega) fun m hm htm => ?_
  have hle : cut N hu k hk (firstReach N hex + Δ + 5 * Δ + δ) ≤ m + 1 := by omega
  have hbefore : ∀ i, i < m + 1 → N.time k hk i ≤ firstReach N hex + Δ + 5 * Δ + 2 * δ :=
    fun i hi => Nat.le_trans (time_mono N k hk (Nat.le_of_lt_succ hi)) htm
  have hmT : m + 1 ≤ cut N hu k hk T := le_cut N hu fun i hi => by have := hbefore i hi; omega
  have hgr := received_grows (N.trace k hk) hle
  have ht0 : cut N hu k hk t0 ≤ m + 1 := Nat.le_trans (cut_mono N hu (by omega)) hle
  have hsi := sameInputs_restrict (P := (C.honest · k)) (h := (N.trace k hk).history (m + 1))
  refine ⟨show C.honest pw.epoch k by rw [hep]; exact hkE,
    ⟨l, vid, (hsi.received _).mpr (hgr _ hrecp), by rw [hep, hpw]; exact hlead, hsm⟩, hjust.wellFormed,
    (hsi.received _).mpr (hgr _ hval), (hsi.parentReady _).mpr (parentReady_grows hgr hpr), hjust.safe,
    (hsi.opens _).mpr fun hent => ?_, (hsi.notBehind _).mpr (by rw [hep]; exact stable_notBehind hst hu (.of hkE) ht0),
    fun ht => ?_, fun vote hsv _ hvw => ?_, (hsi.inView _).mpr ?_⟩
  · obtain ⟨⟨q, hq, h1, h2⟩, c2, hc2, h3, h4⟩ := hopen hent
    exact ⟨⟨q, hasProposal_grows hgr hq, h1, h2⟩, c2, hc2.imp_left (hasCert2_grows hgr), h3, h4⟩
  · refine not_past hcfg hs hw hex (n := m + 1) (fun i hi => ?_)
      (show w ≤ pw.viewNumber by rw [hpw]; exact Nat.le_refl w.toNat) (Or.inl ht)
    have := hbefore i hi; omega
  · obtain ⟨hsv, hev⟩ := restrict_sent.mp hsv
    obtain ⟨i, hi, hiout⟩ := (Trace.sent_history _).mp hsv
    have hvote := vote1_at_proposal hcfg hcf hs hst hw hl hlE hlead hsend hpw hep hiout hev (hvw.trans hpw)
    exact hneg ⟨m + 1, hbefore, (Trace.sent_history _).mpr ⟨i, hi, hvote ▸ hiout⟩⟩
  · rw [hpw]
    exact view_is_w hcfg hcf hs hst hw hex hl (by omega) hT hno k hk (.of hkE)
      (Nat.le_trans (cut_mono N hu (by omega)) hle) hmT

/--
The certificate an honest leader's re-vote request for `w` votes on again is held,
within `Δ` of it, by every honest member of `E` with the block's proposal and payload.

The leader held a certificate over the same block (`Liveness.cert_held`), which
spreads with the block's proposal and payload. That it held a `Cert2` over the block instead
is ruled out in a stable epoch (`Liveness.stable_no_cert2_last`).
-/
theorem revote_block_ready {l : PubKey} {hl' : C.Honest l} {jr : Nat} {rw : RevoteRequest}
    (hsend : Output.send (.revote rw) ∈ (N.trace l hl' jr).output) (hre' : C.honest rw.cert.data.epoch l)
    (hrw : rw.view = w) {Tp : Nat}
    (hjt : N.time l hl' jr ≤ Tp) (hgstp : GST ≤ Tp) (k : PubKey) (hk : C.Honest k) (hkm : C.members E k)
    (hkE : C.HonestFrom E k) :
    ∃ b, ((N.trace k hk).history (cut N hs.timeUnbounded k hk (Tp + Δ))).HasProposal cfg b
      ∧ Certifies rw.cert b
      ∧ ((N.trace k hk).history (cut N hs.timeUnbounded k hk (Tp + Δ))).HasPayload cfg b.viewNumber b.payloadCommit := by
  have hj := sent_revote_justified hsend hre'
  have hre := sent_revote_epoch hcfg hcf hs hst hw hl hsend hre' hrw
  have hcg := certJustified_genuine hj.justified
  have hlast := hj.wellFormed.2.2
  rcases cert_held hj.justified with hg | ⟨m, hm, L, hL, hLd⟩ | ⟨-, c2, hc2, hc2d⟩
  · -- A re-vote of the anchor: every node holds it, with its payload.
    exact ⟨cfg.anchorBlock, Or.inl rfl, ⟨anchor_view_le hcfg hcg, by rw [hg]; exact anchor_cert_data hcfg⟩,
      Or.inl ⟨rfl, rfl⟩⟩
  · obtain ⟨b, hb, hbd, hbv, hpay⟩ := block_ready hcfg hcf hs hst hL hLd hcg hre
        (Nat.le_trans (time_mono N l hl' (by omega : m ≤ jr)) hjt) hgstp k hk hkm hkE
    exact ⟨b, hb, ⟨hbv, hbd⟩, hpay⟩
  · exfalso
    exact stable_no_cert2_last hs hst hc2
      (by rw [hc2d]; exact hlast) (by rw [hc2d]; exact hre)

/-- Every honest member of `E` answers the honest leader's re-vote request for `w` within `2Δ + δ` of it. -/
theorem members_vote1_revote {δ : Nat} (hp : Prompt N δ) (hex : ∃ T, ReachedBy N T w)
    {T : Nat} (hT1 : firstReach N hex + Δ + 5 * Δ + 2 * δ ≤ T) (hT : T < firstReach N hex + τ)
    (hno : ∀ j (hj : C.Honest j) c,
      ((N.trace j hj).history (cut N hs.timeUnbounded j hj T)).HasCert1 cfg c → c.view < w)
    {l : PubKey} (hl' : C.Honest l) (hlead : leader E w = some l) {jr : Nat} {rw : RevoteRequest}
    (hsend : Output.send (.revote rw) ∈ (N.trace l hl' jr).output) (hre' : C.honest rw.cert.data.epoch l)
    (hrw : rw.view = w) (hjt : N.time l hl' jr ≤ firstReach N hex + Δ + 3 * Δ + δ)
    (k : PubKey) (hk : C.Honest k) (hkE : C.honest E k) (hmem : C.members E k) :
    N.SentByTime k hk (firstReach N hex + Δ + 5 * Δ + 2 * δ) (.vote1 ⟨rw.cert.data, w, k⟩) := by
  have hu := hs.timeUnbounded
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  have hj := sent_revote_justified hsend hre'
  have hre := sent_revote_epoch hcfg hcf hs hst hw hl hsend hre' hrw
  have hlE : C.honest E l := hre ▸ hre'
  have hgstp : GST ≤ firstReach N hex + Δ + 3 * Δ + δ := by omega
  obtain ⟨b, hb, hcert, hpay⟩ := revote_block_ready hcfg hcf hs hst hw hl hsend hre' hrw
    hjt hgstp k hk hmem (.of hkE)
  have hmax0 := Nat.max_le.mpr ⟨hjt, hgstp⟩
  obtain ⟨n0, hn0, hrec0⟩ := hs.revote l hl' jr rw hsend hre' k hk (by rw [hre]; exact hmem)
    (by rw [hre]; exact .of hkE)
  have hn0c : n0 ≤ cut N hu k hk (firstReach N hex + Δ + 3 * Δ + δ + Δ) :=
    le_cut N hu fun i hi => by have := hn0 i hi; omega
  obtain ⟨n2', hn2', ht2⟩ := cut_pred N hu (k := k) (hk := hk) (T := firstReach N hex + Δ + 3 * Δ + δ + Δ)
    (fun h => by
      obtain ⟨_, hj', -⟩ := (Trace.received_history _).mp hrec0
      rw [h] at hn0c; exact absurd (Nat.lt_of_lt_of_le hj' hn0c) (Nat.not_lt_zero _))
  refine Classical.byContradiction fun hneg => not_owed_throughout N hp (k := k) (hk := hk)
    (n := n2') (B := firstReach N hex + Δ + 5 * Δ + 2 * δ) (o := .vote1Again rw) (by omega) fun m hm htm => ?_
  have hle : cut N hu k hk (firstReach N hex + Δ + 3 * Δ + δ + Δ) ≤ m + 1 := by omega
  have hbefore : ∀ i, i < m + 1 → N.time k hk i ≤ firstReach N hex + Δ + 5 * Δ + 2 * δ :=
    fun i hi => Nat.le_trans (time_mono N k hk (Nat.le_of_lt_succ hi)) htm
  have hmT : m + 1 ≤ cut N hu k hk T := le_cut N hu fun i hi => by have := hbefore i hi; omega
  have hgr := received_grows (N.trace k hk) hle
  have ht0 : cut N hu k hk t0 ≤ m + 1 := Nat.le_trans (cut_mono N hu (by omega)) hle
  have hsi := sameInputs_restrict (P := (C.honest · k)) (h := (N.trace k hk).history (m + 1))
  refine ⟨show C.honest rw.cert.data.epoch k by rw [hre]; exact hkE,
    ⟨l, (hsi.received _).mpr (received_grows _ (Nat.le_trans hn0c hle) _ hrec0), by rw [hre, hrw]; exact hlead⟩,
    hj.wellFormed, hj.safe,
    ⟨b, (hsi.hasProposal _).mpr (hasProposal_grows hgr hb), hcert, (hsi.hasPayload _ _).mpr (hasPayload_grows hgr hpay)⟩,
    (hsi.notBehind _).mpr (by rw [hre]; exact stable_notBehind hst hu (.of hkE) ht0), fun ht => ?_,
    fun vote hsv _ hvw => ?_, (hsi.inView _).mpr ?_⟩
  · refine not_past hcfg hs hw hex (n := m + 1) (fun i hi => ?_)
      (show w ≤ rw.view by rw [hrw]; exact Nat.le_refl w.toNat) (Or.inl ht)
    have := hbefore i hi; omega
  · obtain ⟨hsv, hev⟩ := restrict_sent.mp hsv
    obtain ⟨i, hi, hiout⟩ := (Trace.sent_history _).mp hsv
    have hvote := vote1_at_revote hcfg hcf hs hst hw hl hlE hlead hsend hrw hre hiout hev (hvw.trans hrw)
    exact hneg ⟨m + 1, hbefore, (Trace.sent_history _).mpr ⟨i, hi, hvote ▸ hiout⟩⟩
  · rw [hrw]
    exact view_is_w hcfg hcf hs hst hw hex hl (by omega) hT hno k hk (.of hkE)
      (Nat.le_trans (cut_mono N hu (by omega)) hle) hmT

end Members

/-! ## A lock at `w`, its `Cert2`, and the decide -/

/--
Node `k` decided a view at `w` or later by `T`, on a `Cert2` of an epoch it is
honest in.

A node that has decided that far no longer owes a vote2 on a certificate whose
view is behind its floor (`OwedVote2.afterFloor`).
-/
def EarlyDecide (N : TimedNetwork cfg leader C) (k : PubKey) (hk : C.Honest k) (w : ViewNumber) (T : Nat) :
    Prop :=
  ∃ n, (∀ i, i < n → N.time k hk i ≤ T)
    ∧ ∃ u, w ≤ u ∧ (((N.trace k hk).history n).restrict (C.honest · k)).DecidedView u

section Commit

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ) {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
  {w : ViewNumber} (hw : cfg.anchorView.toNat + 2 ≤ w.toNat) (hl : Late N t0 (w - 1))
include hcfg hcf hs hst hw hl

omit hw in
/-- Within `7Δ + 2δ` of the first node reaching `w`, some honest node holds a certificate at `w` or later. -/
theorem early_cert {δ : Nat} (hp : Prompt N δ) (hw3 : cfg.anchorView.toNat + 3 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    (hτ : 7 * Δ + 2 * δ < τ) {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    (hmem : C.members E l) :
    ∃ j, ∃ hj : C.Honest j, ∃ c, ((N.trace j hj).history
      (cut N hs.timeUnbounded j hj (firstReach N hex + Δ + 6 * Δ + 2 * δ))).HasCert1 cfg c ∧ w ≤ c.view := by
  have hw : cfg.anchorView.toNat + 2 ≤ w.toNat := by omega
  have hl' : C.Honest l := .of hlE
  have hu := hs.timeUnbounded
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  refine Classical.byContradiction fun hneg => ?_
  have hno : ∀ j (hj : C.Honest j) c, ((N.trace j hj).history
      (cut N hu j hj (firstReach N hex + Δ + 6 * Δ + 2 * δ))).HasCert1 cfg c → c.view < w :=
    fun j hj c hc => Nat.lt_of_not_le fun hle => hneg ⟨j, hj, c, hc, hle⟩
  obtain ⟨k, ⟨hkm, -⟩, -, hk0E⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
  have hk : C.Honest k := .of hk0E
  have hmax := Nat.max_le.mpr ⟨Nat.le_refl (firstReach N hex + Δ + 5 * Δ + 2 * δ),
    show GST ≤ firstReach N hex + Δ + 5 * Δ + 2 * δ by omega⟩
  have hbound : max (firstReach N hex + Δ + 5 * Δ + 2 * δ) GST + Δ ≤ firstReach N hex + Δ + 6 * Δ + 2 * δ := by
    omega
  -- The `Cert1` the members' vote1s form at `w` reaches `k`.
  have hcert : ∀ d : Vote1Data, d.epoch = E →
      (∀ k', C.members E k' → ∀ hkE : C.honest E k',
        N.SentByTime k' (.of hkE) (firstReach N hex + Δ + 5 * Δ + 2 * δ) (.vote1 ⟨d, w, k'⟩)) →
      ((N.trace k hk).history (cut N hu k hk (firstReach N hex + Δ + 6 * Δ + 2 * δ))).HasCert1 cfg ⟨d, w⟩ :=
    fun d hde hvoted => Or.inr (Or.inl (by_cut N hu (fun a b hab h => received_grows _ hab _ h)
      (by_later hbound (hs.cert1 (fun k => C.members E k ∧ C.honest E k) d w _ (hde ▸ N.honestQuorum E)
        (fun k' ⟨hm, hkE⟩ => ⟨hde ▸ hkE, hvoted k' hm hkE⟩) k hk (by rw [hde]; exact .of hk0E)))))
  obtain ⟨j, hact, hjt⟩ := leader_acts hcfg hcf hs hst hl hp hw3 hex
    (T := firstReach N hex + Δ + 6 * Δ + 2 * δ) (by omega) (by omega) hno hlE hlead hmem
  rcases hact with ⟨pw, hsend, hpw, hpe⟩ | ⟨rw, hsend, hrw, hre'⟩
  · have hep := sent_epoch hcfg hcf hs hst hw hl hsend hpe hpw
    have hvoted := fun k' hm (hkE : C.honest E k') => members_vote1 hcfg hcf hs hst hw hl hp hex
      (T := firstReach N hex + Δ + 6 * Δ + 2 * δ) (by omega) (by omega) hno hl' hlead hsend hpe hpw hjt k'
      (.of hkE) hkE hm
    exact Nat.lt_irrefl _ (hno k hk _ (hcert _ hep hvoted))
  · have hre := sent_revote_epoch hcfg hcf hs hst hw hl hsend hre' hrw
    have hvoted := fun k' hm (hkE : C.honest E k') => members_vote1_revote hcfg hcf hs hst hw hl hp hex
      (T := firstReach N hex + Δ + 6 * Δ + 2 * δ) (by omega) (by omega) hno hl' hlead hsend hre' hrw hjt k'
      (.of hkE) hkE hm
    exact Nat.lt_irrefl _ (hno k hk _ (hcert _ hre hvoted))

/-- What an honest node can lock on at a late view it holds as a certificate, proposal and payload. -/
theorem lockable_late {k : PubKey} {hk : C.Honest k} {n : Nat} {c : Cert1}
    (hc : ((N.trace k hk).history n).Lockable cfg c) (hwc : w ≤ c.view) :
    ((N.trace k hk).history n).HasCert1 cfg c ∧ ∃ b, ((N.trace k hk).history n).HasProposal cfg b
      ∧ Certifies c b ∧ ((N.trace k hk).history n).HasPayload cfg b.viewNumber b.payloadCommit := by
  rcases hc with hanc | h' | ⟨c2, p, hrec, hwf⟩
  · exfalso
    have : w.toNat ≤ c.view.toNat := hwc
    rw [hanc, hcfg.anchorCertView] at this
    exact absurd this (by show ¬ w.toNat ≤ cfg.anchorView.toNat; omega)
  · exact h'
  · exfalso
    have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
    have hgen : cfg.anchorView < c.view := anchor_lt hw hwc
    have hcb := held_backed hcfg (Or.inr (Or.inr ⟨c2, p, hrec⟩)) hgen
    have hce := late_cert_epoch hcfg hcf hs hst hcb (late_mono hl (by
      have : w.toNat ≤ c.view.toNat := hwc
      show w.toNat - 1 ≤ c.view.toNat; omega))
    refine stable_no_commit hs hst (Or.inr (Or.inr ⟨c, c2, hrec⟩))
      (Or.inr ⟨c, p, hrec⟩) hcq hwf.last ?_
    have := congrArg Vote1Data.epoch hwf.cert1Data
    simp only at this
    rw [← this, hce]

/-- An honest vote2 at the view of a late backed `Cert1` is for it. -/
theorem vote2_is {L : Cert1} (hLb : Cert1Backed N.trace L) (hwL : w ≤ L.view) {k : PubKey}
    {hk : C.Honest k} {i : Nat} {vote : Vote2} (hiout : Output.send (.vote2 vote) ∈ (N.trace k hk i).output)
    (he : C.honest vote.data.epoch k) (hvv : vote.view = L.view) : vote = ⟨L.data.toVote2, L.view, k⟩ := by
  obtain ⟨c', hb', hv', hd'⟩ := vote2_backed hcfg (k := k) (hk := hk) (v := vote) ⟨i, hiout⟩ he
  have hLl : Late N t0 L.view := late_mono hl (by
    have : w.toNat ≤ L.view.toNat := hwL
    show w.toNat - 1 ≤ L.view.toNat; omega)
  have hdata := late_cert_data hcfg hcf hs hst hb' hLb (hv'.trans hvv) (by rw [hv', hvv]; exact hLl)
  have hsig := ((N.protocol k hk (i + 1)).vote2Justified i vote ⟨_, (Trace.history_getElem? _ (Nat.lt_succ_self i)), hiout⟩ he).1
  obtain ⟨data, view, signer⟩ := vote
  simp only at hvv hd' hsig
  rw [hd', hdata, hvv, hsig]

variable {w' : ViewNumber}

/--
Once every honest member of `E` can lock on `L`, at `w` or later, by `T`, each votes2 for it
within `δ` of `T`, unless by then some honest node holds a `Cert2` at its view.
-/
theorem vote2_or_cert2 {δ : Nat} (hp : Prompt N δ) (hex : ∃ T, ReachedBy N T w) {T : Nat}
    (hT : T + δ < firstReach N hex + τ) {L : Cert1} (hwL : w ≤ L.view) (hLb : Cert1Backed N.trace L)
    (hlock : ∀ k (hk : C.Honest k), C.members E k → C.honest E k →
      ((N.trace k hk).history (cut N hs.timeUnbounded k hk T)).Lockable cfg L)
    (k : PubKey) (hk : C.Honest k) (hkE : C.honest E k) (hkm : C.members E k) :
    N.SentByTime k hk (T + δ) (.vote2 ⟨L.data.toVote2, L.view, k⟩)
      ∨ (∃ k', ∃ hk' : C.Honest k', ∃ n c2, ((N.trace k' hk').history (n + 1)).HasCert2 c2
        ∧ c2.view = L.view ∧ N.time k' hk' n ≤ T + δ)
      ∨ EarlyDecide N k hk w (T + δ) := by
  have hu := hs.timeUnbounded
  have hgen := anchor_lt hw hwL
  obtain ⟨n1', hn1', ht1⟩ := cut_pred N hu (lockable_pos hcfg hgen (hlock k hk hkm hkE))
  obtain ⟨hc1, b, hb, hcert, hpay⟩ := lockable_late hcfg hcf hs hst hw hl (hlock k hk hkm hkE) hwL
  have hLe := late_cert_epoch hcfg hcf hs hst hLb (late_mono hl (by
    have : w.toNat ≤ L.view.toNat := hwL
    show w.toNat - 1 ≤ L.view.toNat; omega))
  refine Classical.byContradiction fun hneg => not_owed_throughout N hp (k := k) (hk := hk)
    (n := n1') (B := T + δ) (o := .vote2 L) (by omega) fun m hm htm => ?_
  have hle : cut N hu k hk T ≤ m + 1 := by omega
  have hbefore : ∀ i, i < m + 1 → N.time k hk i ≤ T + δ :=
    fun i hi => Nat.le_trans (time_mono N k hk (Nat.le_of_lt_succ hi)) htm
  have hlt : ∀ i, i < m + 1 → N.time k hk i < firstReach N hex + τ :=
    fun i hi => by have := hbefore i hi; omega
  have hgr := received_grows (N.trace k hk) hle
  have hsi := sameInputs_restrict (P := (C.honest · k)) (h := (N.trace k hk).history (m + 1))
  refine ⟨show C.honest L.data.epoch k by rw [hLe]; exact hkE,
    ⟨b, (hsi.hasCert1 _).mpr (hasCert1_grows hgr hc1), (hsi.hasProposal _).mpr (hasProposal_grows hgr hb), hcert,
      (hsi.hasPayload _ _).mpr (hasPayload_grows hgr hpay)⟩,
    fun vote hsv _ hvv => ?_,
    fun c2 hc2 _ hv => hneg (Or.inr (Or.inl ⟨k, hk, m, c2, (hsi.hasCert2 _).mp hc2, hv, htm⟩)),
    not_past hcfg hs hw hex hlt hwL, ⟨hgen, fun u hu => ?_⟩⟩
  · obtain ⟨hsv, hev⟩ := restrict_sent.mp hsv
    obtain ⟨i, hi, hiout⟩ := (Trace.sent_history _).mp hsv
    have hvote := vote2_is hcfg hcf hs hst hw hl hLb hwL hiout hev hvv
    exact hneg (Or.inl ⟨m + 1, hbefore, (Trace.sent_history _).mpr ⟨i, hi, hvote ▸ hiout⟩⟩)
  · -- A view decided this late would already be past `w`.
    refine Nat.lt_of_not_le fun hle => hneg (Or.inr (Or.inr ⟨m + 1, hbefore, u, ?_, hu⟩))
    have : (u - cfg.decideBuffer).toNat ≤ u.toNat := Nat.sub_le _ _
    have : w.toNat ≤ L.view.toNat := hwL
    show w.toNat ≤ u.toNat
    omega

/--
Once every honest member of `E` can lock on `L`, at `w` or later, by `T`, every
honest node holds its `Cert2` within `Δ + δ`.
-/
theorem cert2_everywhere {δ : Nat} (hp : Prompt N δ) (hex : ∃ T, ReachedBy N T w) {T : Nat}
    (hgst : GST ≤ T) (hT : T + δ < firstReach N hex + τ) {L : Cert1} (hwL : w ≤ L.view)
    (hLb : Cert1Backed N.trace L)
    (hlock : ∀ k (hk : C.Honest k), C.members E k → C.honest E k →
      ((N.trace k hk).history (cut N hs.timeUnbounded k hk T)).Lockable cfg L)
    (k : PubKey) (hk : C.Honest k) (hkE : C.HonestFrom E k) :
    N.By k hk (T + Δ + δ) (fun h => h.HasCert2 ⟨L.data.toVote2, L.view⟩)
      ∨ ∃ j, ∃ hj : C.Honest j, C.honest E j ∧ EarlyDecide N j hj w (T + δ) := by
  have hLl : Late N t0 L.view := late_mono hl (by
    have : w.toNat ≤ L.view.toNat := hwL
    show w.toNat - 1 ≤ L.view.toNat; omega)
  have hLe := late_cert_epoch hcfg hcf hs hst hLb hLl
  by_cases hgood : ∃ k', ∃ hk' : C.Honest k', ∃ n c2, ((N.trace k' hk').history (n + 1)).HasCert2 c2
      ∧ c2.view = L.view ∧ N.time k' hk' n ≤ T + δ
  · obtain ⟨k', hk', n, c2, hc2, hv, htn⟩ := hgood
    obtain ⟨c, hb, hcv, hcd⟩ := cert2_backed_data hcfg hc2
    have hdata := late_cert_data hcfg hcf hs hst hb hLb (hcv.trans hv) (by rw [hcv, hv]; exact hLl)
    have hc2eq : c2 = ⟨L.data.toVote2, L.view⟩ := by
      obtain ⟨d, v⟩ := c2
      simp only at hv hcd
      rw [hcd, hdata, hv]
    rw [← hc2eq]
    have := Nat.max_le.mpr ⟨htn, show GST ≤ T + δ by omega⟩
    exact Or.inl (by_later (by omega) (hs.cert2Spread c2 k' hk' n hc2 k hk
      (by rw [hc2eq]; show C.HonestFrom L.data.epoch k; rw [hLe]; exact hkE)))
  by_cases hearly : ∃ j, ∃ hj : C.Honest j, C.honest E j ∧ EarlyDecide N j hj w (T + δ)
  · exact Or.inr hearly
  have hall : ∀ k', (C.members E k' ∧ C.honest E k') → ∃ hk' : C.honest L.data.epoch k',
      N.SentByTime k' (.of hk') (T + δ) (.vote2 ⟨L.data.toVote2, L.view, k'⟩) := fun k' ⟨hm', hkE⟩ =>
    ⟨hLe ▸ hkE, ((vote2_or_cert2 hcfg hcf hs hst hw hl hp hex hT hwL hLb hlock k' (.of hkE) hkE
      hm').resolve_right fun h => h.elim hgood fun he => hearly ⟨k', .of hkE, hkE, he⟩)⟩
  obtain ⟨n, hn, hr⟩ := hs.cert2 _ L.data.toVote2 L.view (T + δ) (hLe ▸ N.honestQuorum E) hall k hk
    (by show C.HonestFrom L.data.epoch k; rw [hLe]; exact hkE)
  have := Nat.max_le.mpr ⟨Nat.le_refl (T + δ), show GST ≤ T + δ by omega⟩
  exact Or.inl ⟨n, fun i hi => by have := hn i hi; omega, Or.inl hr⟩

omit hw in
/--
If view `w` has an honest leader of `E`, then by `8Δ + 2δ` after it is first reached
every honest member of `E` can lock on one certificate at `w` or later.
-/
theorem lock_spreads {δ : Nat} (hp : Prompt N δ) (hw3 : cfg.anchorView.toNat + 3 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    (hτ : 7 * Δ + 2 * δ < τ) {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    (hmem : C.members E l) :
    ∃ L, w ≤ L.view ∧ Cert1Backed N.trace L ∧ ∀ k (hk : C.Honest k), C.members E k → C.honest E k →
      ((N.trace k hk).history (cut N hs.timeUnbounded k hk (firstReach N hex + Δ + 6 * Δ + 2 * δ + Δ))).Lockable cfg L := by
  have hw : cfg.anchorView.toNat + 2 ≤ w.toNat := by omega
  have hl' : C.Honest l := .of hlE
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  obtain ⟨j, hj, c, hc, hwc⟩ := early_cert hcfg hcf hs hst hl hp hw3 hex (by omega) hlE hlead hmem
  have hcb := held_backed hcfg hc (anchor_lt hw hwc)
  have hce := late_cert_epoch hcfg hcf hs hst hcb (late_mono hl (by
    have : w.toNat ≤ c.view.toNat := hwc
    show w.toNat - 1 ≤ c.view.toNat; omega))
  exact ⟨c, hwc, hcb, fun k hk hkm hkE => lock_of_held hs (by omega) (hce ▸ hkm) (.of (by rw [hce]; exact hkE)) hc⟩

/--
A node honest in `E` that holds a `Cert2` of `E` at `w` or later, with the block it
commits and a `Cert1` over the block, decides the `Cert2`'s view or a later one.

The block is not the last of `E` and the `Cert2` is at its own view
(`Liveness.late_commit`), so the decide owed is of that view.
-/
theorem decides_of_held {δ : Nat} (hp : Prompt N δ) {k : PubKey} {hk : C.Honest k} (hkE : C.honest E k)
    {a b : Nat} (hab : a ≤ b) {c1 : Cert1} {c2 : Cert2} {blk : Block}
    (hc1 : ((N.trace k hk).history a).HasCert1 cfg c1) (hb : ((N.trace k hk).history a).HasProposal cfg blk)
    (hcert : Certifies c1 blk) (hc2 : ((N.trace k hk).history b).HasCert2 c2) (hcq : Commits c2 blk)
    (hce : c2.data.epoch = E) (hwc : w ≤ c2.view) :
    ∃ n v, c2.view ≤ v ∧ (((N.trace k hk).history n).restrict (C.honest · k)).DecidedView v := by
  have hgr0 := received_grows (N.trace k hk) hab
  obtain ⟨hbv, -⟩ := late_commit hcfg hcf hs hst (hasProposal_grows hgr0 hb) hc2 hcq (late_mono hl (by
    have : w.toNat ≤ c2.view.toNat := hwc
    show w.toNat - 1 ≤ c2.view.toNat; omega))
  have hgen : cfg.anchorView < blk.viewNumber := by rw [hbv]; exact anchor_lt hw hwc
  refine Classical.byContradiction fun hneg => ?_
  have howed : ∀ m, b ≤ m → OwedIn cfg leader k (C.honest · k) ((N.trace k hk).history (m + 1))
      (.decide c2) := by
    intro m hm
    have hgr := received_grows (N.trace k hk) (Nat.le_succ_of_le hm)
    have hgr1 := received_grows (N.trace k hk) (Nat.le_succ_of_le (Nat.le_trans hab hm))
    have hsi := sameInputs_restrict (P := (C.honest · k)) (h := (N.trace k hk).history (m + 1))
    refine ⟨show C.honest c2.data.epoch k by rw [hce]; exact hkE,
      blk, c1, (hsi.hasCert2 _).mpr (hasCert2_grows hgr hc2), (hsi.hasProposal _).mpr (hasProposal_grows hgr1 hb),
      hcq, (hsi.hasCert1 _).mpr (hasCert1_grows hgr1 hc1), hcert,
      fun hd => hneg ⟨m + 1, blk.viewNumber, by rw [hbv]; exact Nat.le_refl _, hd⟩, hgen, fun u hu => ?_⟩
    refine Nat.lt_of_not_le fun hle => hneg ⟨m + 1, u, ?_, hu⟩
    have : (u - cfg.decideBuffer).toNat ≤ u.toNat := Nat.sub_le _ _
    rw [← hbv]
    exact Nat.le_trans hle this
  obtain ⟨m, hm, -, hnow⟩ := hp k hk b _ (howed b (Nat.le_refl _))
  exact hnow (howed m hm)

/--
A node that holds `L`, at `w` or later, with its block's proposal, and later holds its `Cert2`
decides `L`'s view or a later one (`Liveness.decides_of_held`).
-/
theorem decides_of_cert2 {δ : Nat} (hp : Prompt N δ) {L : Cert1} (hwL : w ≤ L.view) (hLe : L.data.epoch = E)
    {k : PubKey} {hk : C.Honest k} (hkE : C.honest E k) {a b : Nat} (hab : a ≤ b) {blk : Block}
    (hc1 : ((N.trace k hk).history a).HasCert1 cfg L) (hb : ((N.trace k hk).history a).HasProposal cfg blk)
    (hcert : Certifies L blk)
    (hc2 : ((N.trace k hk).history b).HasCert2 ⟨L.data.toVote2, L.view⟩) :
    ∃ n v, L.view ≤ v ∧ (((N.trace k hk).history n).restrict (C.honest · k)).DecidedView v :=
  decides_of_held hcfg hcf hs hst hw hl hp hkE hab hc1 hb hcert hc2
    ⟨hcert.1, by show L.data.toVote2 = _; rw [hcert.2]; rfl⟩ hLe hwL

/--
A view at `w` or later that a node honest in `E` decided comes with what it decided
on: a `Cert2` of `E` at `w` or later, the block it commits, and a `Cert1` over it.

The decided block is before the committed one (`ChainLinked.ancestor`), so it is at
the `Cert2`'s view or an earlier one (`ancestor_view_le`).
-/
theorem decided_commit {j : PubKey} {hj : C.Honest j} {n : Nat} {u : ViewNumber}
    (hd : (((N.trace j hj).history n).restrict (C.honest · j)).DecidedView u) (hwu : w ≤ u) :
    ∃ m c1 c2 q, m < n ∧ ((N.trace j hj).history (m + 1)).HasCert2 c2
      ∧ ((N.trace j hj).history (m + 1)).HasProposal cfg q ∧ ((N.trace j hj).history (m + 1)).HasCert1 cfg c1
      ∧ Commits c2 q ∧ Certifies c1 q ∧ c2.data.epoch = E ∧ w ≤ c2.view := by
  obtain ⟨st, hstm, blocks, c1, c2, b, hout, hep, hb, hv⟩ := restrict_decidedView.mp hd
  obtain ⟨m, hm, rfl⟩ := (Trace.mem_history _).mp hstm
  obtain ⟨head, rest, rfl, hc2, hcommit, hc1, hcert, hlinked, hall⟩ :=
    (N.safe_at j hj m).decideJustified m (N.trace j hj m) blocks c1 c2
      (Trace.history_getElem? _ (Nat.lt_succ_self m)) hout hep
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hc2 hc1 hall
  have htree : ∀ x ∈ head :: rest, heldTree N.toNetwork (blockHash x) = some x :=
    fun x hx => heldTree_resolves N.toNetwork hcf j hj (m + 1) x (hall x hx).1
  have hgen : ∀ x ∈ head :: rest, x.viewNumber ≠ cfg.anchorView :=
    fun x hx hz => absurd (hz ▸ (hall x hx).2) (Nat.lt_irrefl _)
  have hanc := ChainLinked.ancestor htree hgen hlinked b hb
  obtain ⟨cc, hccb, hccv, hcch, hcce⟩ :=
    cert2_implies_cert1 cfg N.toNetwork hcfg (cert2_held_backed N.toNetwork hc2)
  have hhead : cc.data.blockHash = blockHash head :=
    hcch.trans (congrArg Vote2Data.blockHash hcommit.2)
  have hbv := ancestor_view_le (heldTree N.toNetwork) N.toNetwork hcfg (heldTree_coherent N.toNetwork) hcf
    (heldTree_resolves N.toNetwork hcf) _ cc (Nat.le_refl _) hccb _ b (hhead ▸ hanc) (htree b hb)
  have hwc : w ≤ c2.view := by
    rw [← hccv]
    have : w.toNat ≤ u.toNat := hwu
    have : b.viewNumber.toNat ≤ cc.view.toNat := hbv
    show w.toNat ≤ cc.view.toNat
    rw [← hv] at *
    omega
  have hce := late_cert_epoch hcfg hcf hs hst hccb (late_mono hl (by
    have : w.toNat ≤ cc.view.toNat := by rw [hccv]; exact hwc
    show w.toNat - 1 ≤ cc.view.toNat; omega))
  exact ⟨m, c1, c2, head, hm, hc2, (hall head (List.mem_cons_self ..)).1, hc1, hcommit, hcert,
    hcce ▸ hce, hwc⟩

/--
A view at `w` or later that a node honest in `E` decided by `T`, after GST, every
node honest in `E` decides too, or a later view, within `Δ` and `δ` of `T`: what it
decided on spreads (`Synchrony.cert2Spread`, `Synchrony.certSpread`,
`Synchrony.blockSpread`) and is owed as a decide (`Liveness.decides_of_held`).
-/
theorem early_decide_spreads {δ : Nat} (hp : Prompt N δ) {T : Nat} (hgst : GST ≤ T) {j : PubKey}
    {hj : C.Honest j} (hjd : EarlyDecide N j hj w T) (k : PubKey) (hkE : C.honest E k) :
    ∃ n v, w ≤ v ∧ (((N.trace k (.of hkE)).history n).restrict (C.honest · k)).DecidedView v := by
  have hk : C.Honest k := .of hkE
  have hu := hs.timeUnbounded
  obtain ⟨n, hn, u, hwu, hd⟩ := hjd
  obtain ⟨m, c1, c2, q, hmn, hc2, hq, hc1, hcq, hcert, hce, hwc⟩ := decided_commit hcfg hcf hs hst hw hl hd hwu
  have htm : N.time j hj m ≤ T := hn m hmn
  have hmax := Nat.max_le.mpr ⟨htm, hgst⟩
  have hc1e : c1.data.epoch = E := by
    have h1 := congrArg Vote1Data.epoch hcert.2
    have h2 := congrArg Vote2Data.epoch hcq.2
    simp only at h1 h2
    rw [h1, ← h2, hce]
  have hlate : max (N.time j hj m) GST + Δ ≤ T + Δ := by omega
  have hkc2 := by_cut N hu (fun a b hab h => hasCert2_grows (received_grows _ hab) h)
    (by_later hlate (hs.cert2Spread c2 j hj m hc2 k hk (by rw [hce]; exact .of hkE)))
  have hkc1 := by_cut N hu (fun a b hab h => hasCert1_grows (received_grows _ hab) h)
    (by_later hlate (hs.certSpread c1 j hj m hc1 k hk (by rw [hc1e]; exact .of hkE)))
  have hkq := by_cut N hu (fun a b hab h => hasProposal_grows (received_grows _ hab) h)
    (by_later hlate (hs.blockSpread c1 q hcert j hj m ⟨hc1, hq⟩ k hk (by rw [hc1e]; exact .of hkE)))
  obtain ⟨n', v, hcv, hd'⟩ := decides_of_held hcfg hcf hs hst hw hl hp hkE (Nat.le_refl _) hkc1 hkq hcert hkc2
    hcq hce hwc
  exact ⟨n', v, Nat.le_trans hwc hcv, hd'⟩

omit hw in
/--
**A view with an honest leader decides.** In a stable epoch `E`, if the leader of
`w` in `E` is honest in `E` and no honest node reached `w - 1` by the time `E`
became stable, every node honest in `E` decides `w` or a later view, on a `Cert2`
of `E`.
-/
theorem late_decide {δ : Nat} (hp : Prompt N δ) (hw3 : cfg.anchorView.toNat + 3 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    (hτ : 8 * Δ + 3 * δ < τ) {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    (hmem : C.members E l) (k : PubKey) (hkE : C.honest E k) :
    ∃ n v, w ≤ v ∧ (((N.trace k (.of hkE)).history n).restrict (C.honest · k)).DecidedView v := by
  have hw : cfg.anchorView.toNat + 2 ≤ w.toNat := by omega
  have hk : C.Honest k := .of hkE
  have hl' : C.Honest l := .of hlE
  have hu := hs.timeUnbounded
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  obtain ⟨L, hwL, hLb, hlock⟩ := lock_spreads hcfg hcf hs hst hl hp hw3 hex (by omega) hlE hlead hmem
  have hLe := late_cert_epoch hcfg hcf hs hst hLb (late_mono hl (by
    have : w.toNat ≤ L.view.toNat := hwL
    show w.toNat - 1 ≤ L.view.toNat; omega))
  rcases cert2_everywhere hcfg hcf hs hst hw hl hp hex (by omega) (by omega) hwL hLb hlock k hk (.of hkE)
    with hc2 | ⟨j, hj, -, hjd⟩
  case inr => exact early_decide_spreads hcfg hcf hs hst hw hl hp (by omega) hjd k hkE
  have hc2 := by_cut N hu (fun a b hab h => hasCert2_grows (received_grows _ hab) h) hc2
  -- A member holds `L` with its block's proposal, and the proposal reaches `k`.
  obtain ⟨m0, ⟨hm0, -⟩, -, hk0E⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
  have hk0 : C.Honest m0 := .of hk0E
  have hl0 := hlock m0 hk0 hm0 hk0E
  obtain ⟨hc10, b, hb0, hcert, -⟩ := lockable_late hcfg hcf hs hst hw hl hl0 hwL
  obtain ⟨n', hn', htn⟩ := cut_pred N hu (lockable_pos hcfg (anchor_lt hw hwL) hl0)
  rw [hn'] at hc10 hb0
  have hmax := Nat.max_le.mpr ⟨htn, show GST ≤ firstReach N hex + Δ + 6 * Δ + 2 * δ + Δ by omega⟩
  have hlate : max (N.time m0 hk0 n') GST + Δ ≤ firstReach N hex + Δ + 6 * Δ + 2 * δ + Δ + Δ + δ := by omega
  have hc1 := by_cut N hu (fun a b hab h => hasCert1_grows (received_grows _ hab) h)
    (by_later hlate (hs.certSpread L m0 hk0 n' hc10 k hk (by rw [hLe]; exact .of hkE)))
  have hb := by_cut N hu (fun a b hab h => hasProposal_grows (received_grows _ hab) h)
    (by_later hlate (hs.blockSpread L b hcert m0 hk0 n' ⟨hc10, hb0⟩ k hk (by rw [hLe]; exact .of hkE)))
  obtain ⟨n, v, hLv, hd⟩ := decides_of_cert2 hcfg hcf hs hst hw hl hp hwL hLe hkE (Nat.le_refl _) hc1 hb hcert hc2
  exact ⟨n, v, Nat.le_trans hwL hLv, hd⟩

omit hw in
/--
**A view with an honest leader commits a block at its own view or later.** Some
node honest in `E` holds a `Cert2` of `E` at `w` or later and the block it commits.
-/
theorem late_cert2 {δ : Nat} (hp : Prompt N δ) (hw3 : cfg.anchorView.toNat + 3 ≤ w.toNat) (hex : ∃ T, ReachedBy N T w)
    (hτ : 8 * Δ + 3 * δ < τ) {l : PubKey} (hlE : C.honest E l) (hlead : leader E w = some l)
    (hmem : C.members E l) :
    ∃ j, ∃ hj : C.Honest j, ∃ n c2 q, ((N.trace j hj).history n).HasProposal cfg q
      ∧ ((N.trace j hj).history n).HasCert2 c2 ∧ Commits c2 q ∧ c2.data.epoch = E ∧ w ≤ c2.view := by
  have hw : cfg.anchorView.toNat + 2 ≤ w.toNat := by omega
  have hl' : C.Honest l := .of hlE
  have hu := hs.timeUnbounded
  have hF := late_firstReach (late_mono hl (by show w.toNat - 1 ≤ w.toNat; omega)) hex
  have hgst : GST ≤ firstReach N hex := Nat.le_trans hst.gst (Nat.le_of_lt hF)
  obtain ⟨L, hwL, hLb, hlock⟩ := lock_spreads hcfg hcf hs hst hl hp hw3 hex (by omega) hlE hlead hmem
  have hLe := late_cert_epoch hcfg hcf hs hst hLb (late_mono hl (by
    have : w.toNat ≤ L.view.toNat := hwL
    show w.toNat - 1 ≤ L.view.toNat; omega))
  obtain ⟨m0, ⟨hm0, -⟩, -, hk0E⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
  have hk0 : C.Honest m0 := .of hk0E
  obtain ⟨-, b, hb0, hcert, -⟩ := lockable_late hcfg hcf hs hst hw hl (hlock m0 hk0 hm0 hk0E) hwL
  rcases cert2_everywhere hcfg hcf hs hst hw hl hp hex (by omega) (by omega) hwL hLb hlock m0 hk0 (.of hk0E)
    with hc2 | ⟨j, hj, -, n, -, u, hwu, hd⟩
  case inr =>
    obtain ⟨m, -, c2, q, -, hc2, hq, -, hcq, -, hce, hwc⟩ := decided_commit hcfg hcf hs hst hw hl hd hwu
    exact ⟨j, hj, m + 1, c2, q, hq, hc2, hcq, hce, hwc⟩
  have hc2 := by_cut N hu (fun a b hab h => hasCert2_grows (received_grows _ hab) h) hc2
  have hcut := cut_mono N hu (k := m0) (hk := hk0)
    (show firstReach N hex + Δ + 6 * Δ + 2 * δ + Δ ≤ firstReach N hex + Δ + 6 * Δ + 2 * δ + Δ + Δ + δ by omega)
  exact ⟨m0, hk0, _, _, b, hasProposal_grows (received_grows _ hcut) hb0, hc2,
    ⟨hcert.1, by show L.data.toVote2 = _; rw [hcert.2]; rfl⟩, hLe, hwL⟩

end Commit

end Liveness
end NewProtocol
