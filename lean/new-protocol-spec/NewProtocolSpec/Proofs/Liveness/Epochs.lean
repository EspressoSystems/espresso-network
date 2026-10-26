module

public import NewProtocolSpec.Proofs.Liveness.View

/-!
# Across epochs

`ChainGrows` for any epoch height. Take a node honest in each epoch, and suppose
what each decides, on `Cert2`s of epochs it is honest in, stays below a bound.
Then:

* No honest node ever has solid grounds for an epoch beyond a bound
  (`Liveness.solid_bound`). Solid grounds for a later epoch come from an epoch
  change, or from a certified block of that epoch, whose chain entered it behind a
  last block an honest node held with a `Cert2`. Either way the epoch change
  reaches the node honest in that epoch, and it decides that last block, whose
  view grows with the epoch. A timeout certificate's epoch is one some honest node
  has solid grounds for (`Liveness.ground_solid`).
* So some epoch `E` is the latest any honest node ever has grounds for, every node
  honest in it or later gets solid grounds for it within `Δ` of the first, and
  every other honest node gets the epoch change that ended the last epoch it is
  honest in: the epoch is stable (`Liveness.stabilises`).
* In a stable epoch every view is reached (`Liveness.views_unbounded`), and the
  next view with a leader honest in `E` makes every node honest in `E` decide it or
  a later view (`Liveness.late_decide`), which the supposition rules out
  (`Liveness.decides_unbounded`).
-/

@[expose] public section

namespace NewProtocol
namespace Liveness

open History Lists

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}
variable {N : TimedNetwork cfg leader C}

/-! ## Deciding the last block of an epoch -/

section Decide

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ)
include hcfg hcf

/-- `Liveness.view_ge_height`, with the induction counter in the statement. -/
private theorem view_ge_height_aux :
    ∀ n (c : Cert1), c.view.toNat ≤ n → Cert1Backed N.trace c → ∀ b,
      heldTree N.toNetwork c.data.blockHash = some b →
        b.blockHeader.blockNumber.toNat + cfg.anchorView.toNat
          ≤ b.viewNumber.toNat + cfg.anchorBlock.blockHeader.blockNumber.toNat := by
  have hres := heldTree_resolves N.toNetwork hcf
  have hheights := heightSucceedsParent (heldTree N.toNetwork) N.toNetwork hcfg
    (heldTree_coherent N.toNetwork) hcf hres
  intro n
  induction n using Nat.strongRecOn with
  | ind n ih =>
    intro c hle hc b hb
    obtain ⟨p, hpv, hph, -, hwf, hpt, hpar, -, -, -⟩ := cert1_proposal (heldTree N.toNetwork) N.toNetwork hres hc
    have hbp : b = p := Option.some_inj.mp (hb.symm.trans (by rw [hph]; exact hpt))
    subst hbp
    have h1 : b.parentCert.view.toNat < b.viewNumber.toNat := hwf.1
    rcases hpar with hpb | ha
    · obtain ⟨pp, hppv, hpph, -, -, hppt, -, -, -, -⟩ := cert1_proposal (heldTree N.toNetwork) N.toNetwork hres hpb
      have hsucc := hheights c hc b pp hb (by rw [hpph]; exact hppt)
      have hih := ih b.parentCert.view.toNat (by have : b.viewNumber.toNat ≤ c.view.toNat := hpv; omega)
        b.parentCert (Nat.le_refl _) hpb pp (by rw [hpph]; exact hppt)
      have h2 : pp.viewNumber.toNat ≤ b.parentCert.view.toNat := hppv
      rw [hsucc, BlockNumber.toNat_add]
      omega
    · have hanct : heldTree N.toNetwork (blockHash cfg.anchorBlock) = some cfg.anchorBlock :=
        anchor_in_tree _ N.toNetwork hres hc
      have hsucc := hheights c hc b cfg.anchorBlock hb (by rw [ha, hcfg.anchorCertBlock]; exact hanct)
      have h3 : b.parentCert.view.toNat = cfg.anchorView.toNat := by rw [ha, hcfg.anchorCertView]
      rw [hsucc, BlockNumber.toNat_add]
      omega

/--
A certified block is at least as many views after the anchor as it is blocks after it.

Each block is at a later view than its parent and one block higher.
-/
theorem view_ge_height {c : Cert1} (hc : Cert1Backed N.trace c) {b : Block}
    (hb : heldTree N.toNetwork c.data.blockHash = some b) :
    b.blockHeader.blockNumber.toNat + cfg.anchorView.toNat
      ≤ b.viewNumber.toNat + cfg.anchorBlock.blockHeader.blockNumber.toNat :=
  view_ge_height_aux hcfg hcf _ c (Nat.le_refl _) hc b hb

/-- `Liveness.view_ge_height`, for a block an honest node holds a `Cert2` over. -/
theorem committed_view_ge_height {j : PubKey} {hj : C.Honest j} {n : Nat} {q : Block} {c2 : Cert2}
    (hq : ((N.trace j hj).history n).HasProposal cfg q) (hc2 : ((N.trace j hj).history n).HasCert2 c2)
    (hcq : Commits c2 q) :
    q.blockHeader.blockNumber.toNat + cfg.anchorView.toNat
      ≤ q.viewNumber.toNat + cfg.anchorBlock.blockHeader.blockNumber.toNat := by
  obtain ⟨cc, hccb, -, hcch, -⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg (cert2_held_backed N.toNetwork hc2)
  refine view_ge_height hcfg hcf hccb ?_
  rw [hcch, show c2.data.blockHash = blockHash q from congrArg Vote2Data.blockHash hcq.2]
  exact heldTree_resolves N.toNetwork hcf j hj n q hq

/--
A node that took an epoch change, honest in the epoch that ended, decides the
epoch's last block or a later view, on a `Cert2` of an epoch it is honest in.
-/
theorem decides_of_epochChange {δ : Nat} (hp : Prompt N δ) {k : PubKey} {hk : C.Honest k} {m : Nat}
    {c1 : Cert1} {c2 : Cert2} {q : Block} (htook : ((N.trace k hk).history m).TookEpochChange cfg c1 c2 q)
    (hke : C.honest c2.data.epoch k) :
    ∃ n v, q.viewNumber ≤ v ∧ (((N.trace k hk).history n).restrict (C.honest · k)).DecidedView v := by
  obtain ⟨hrec, hwf⟩ := htook
  have hcq : Commits c2 q := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
  have hcert : Certifies c1 q := ⟨Nat.le_of_eq (congrArg ViewNumber.toNat hwf.cert1View), hwf.cert1Data⟩
  have hgen : cfg.anchorView < q.viewNumber := by
    obtain ⟨i, -, hin⟩ := (Trace.received_history _).mp hrec
    have hc2b : Cert2Backed N.trace c2 :=
      N.cert2Genuine k hk i c2 (by rw [hin]; rfl)
    obtain ⟨cc, hccb, -, hcch, -⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg hc2b
    exact backed_block_after_anchor N.toNetwork hcfg hcf hccb (by rw [hcch, ← hwf.sameData, hwf.cert1Data]; rfl)
  cases m with
  | zero => obtain ⟨_, hj, -⟩ := (Trace.received_history _).mp hrec; exact absurd hj (Nat.not_lt_zero _)
  | succ m =>
    refine Classical.byContradiction fun hneg => ?_
    have howed : ∀ x, m ≤ x →
        OwedIn cfg leader k (C.honest · k) ((N.trace k hk).history (x + 1)) (.decide c2) := by
      intro x hx
      have hgr := received_grows (N.trace k hk) (Nat.succ_le_succ hx)
      have hsi := sameInputs_restrict (P := (C.honest · k)) (h := (N.trace k hk).history (x + 1))
      have hr := (hsi.received _).mpr (hgr _ hrec)
      refine ⟨hke, q, c1, Or.inr ⟨c1, q, hr⟩, Or.inr (Or.inr ⟨c1, c2, hr⟩), hcq,
        Or.inr (Or.inr ⟨c2, q, hr⟩), hcert,
        fun hd => hneg ⟨x + 1, q.viewNumber, Nat.le_refl _, hd⟩, hgen, fun u hu => ?_⟩
      refine Nat.lt_of_not_le fun hle => hneg ⟨x + 1, u, ?_, hu⟩
      have : (u - cfg.decideBuffer).toNat ≤ u.toNat := Nat.sub_le _ _
      exact Nat.le_trans hle this
    obtain ⟨x, hx, -, hnow⟩ := hp k hk m _ (howed m (Nat.le_refl _))
    exact hnow (howed x hx)

include hs in
/--
The last block of an epoch an honest node holds with a `Cert2` over it, every node
honest in that epoch decides, or decides a later view: the epoch change reaches it.
-/
theorem holder_decides {δ : Nat} (hp : Prompt N δ) {j : PubKey} {hj : C.Honest j} {n : Nat}
    {q : Block} {c2 : Cert2} (hq : ((N.trace j hj).history n).HasProposal cfg q)
    (hc2 : ((N.trace j hj).history n).HasCert2 c2) (hcq : Commits c2 q)
    (hlast : IsLastBlock q.blockHeader.blockNumber cfg.epochHeight) (k : PubKey) (hke : C.honest q.epoch k) :
    ∃ n' v, q.viewNumber ≤ v ∧ (((N.trace k (.of hke)).history n').restrict (C.honest · k)).DecidedView v := by
  have hk : C.Honest k := .of hke
  cases n with
  | zero =>
    rcases hc2 with h | ⟨_, _, h⟩ <;>
      · obtain ⟨_, hj', -⟩ := (Trace.received_history _).mp h; exact absurd hj' (Nat.not_lt_zero _)
  | succ n =>
    have hce : c2.data.epoch = q.epoch := congrArg Vote2Data.epoch hcq.2
    obtain ⟨m, -, c1, htook⟩ := hs.epochChange c2 q hcq hlast j hj n ⟨hq, hc2⟩ k hk (.of (by rw [hce]; exact hke))
    exact decides_of_epochChange hcfg hcf hp htook (by rw [hce]; exact hke)

end Decide

/-! ## Epochs are bounded unless decides keep coming

`dk e` is a node honest in epoch `e`, and `V` bounds what any of them decides on
`Cert2`s of epochs it is honest in.
-/

section Bounded

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ δ : Nat}
  (hs : Synchrony N GST Δ τ) (hp : Prompt N δ) {dk : EpochNumber → PubKey} (hdk : ∀ e, C.honest e (dk e))
  {V : Nat} (hV : ∀ e n v, (((N.trace (dk e) (.of (hdk e))).history n).restrict (C.honest · (dk e))).DecidedView v →
    v.toNat < V)
include hcfg hcf hs hp hdk hV

omit hs hp hdk hV in
/-- A block an honest node holds a `Cert2` over names the epoch its height falls in. -/
theorem commit_epoch {j : PubKey} {hj : C.Honest j} {n : Nat} {q : Block} {c2 : Cert2}
    (hc2 : ((N.trace j hj).history n).HasCert2 c2) (hcq : Commits c2 q) :
    q.epoch = epochOf q.blockHeader.blockNumber cfg.epochHeight := by
  obtain ⟨cc, hccb, -, hcch, -⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg (cert2_held_backed N.toNetwork hc2)
  obtain ⟨-, -, -, -, b0, -, -, hwf0, -, hd0, -, -⟩ := cert1_origin N.toNetwork _ cc (Nat.le_refl _) hccb
  have hb0 : b0 = q := hcf b0 q (by
    have h1 := congrArg Vote1Data.blockHash hd0
    have h2 := congrArg Vote2Data.blockHash hcq.2
    simp only at h1 h2
    rw [← h1, hcch, h2])
  subst hb0; exact hwf0.epoch

/--
A last block some honest node holds with a `Cert2` is of an epoch before `V` plus the
anchor's height.
-/
theorem held_commit_epoch {j : PubKey} {hj : C.Honest j} {n : Nat} {q : Block} {c2 : Cert2}
    (hq : ((N.trace j hj).history n).HasProposal cfg q) (hc2 : ((N.trace j hj).history n).HasCert2 c2)
    (hcq : Commits c2 q) (hlast : IsLastBlock q.blockHeader.blockNumber cfg.epochHeight) :
    q.epoch.toNat < V + cfg.anchorBlock.blockHeader.blockNumber.toNat := by
  obtain ⟨n', v, hqv, hd⟩ := holder_decides hcfg hcf hs hp hq hc2 hcq hlast (dk q.epoch) (hdk q.epoch)
  have hvV := hV q.epoch n' v hd
  have hhv := committed_view_ge_height hcfg hcf hq hc2 hcq
  have hheight : q.blockHeader.blockNumber.toNat = q.epoch.toNat * cfg.epochHeight :=
    lastBlock_height hlast (commit_epoch hcfg hcf hc2 hcq).symm
  have hH : 1 ≤ cfg.epochHeight := Nat.pos_of_ne_zero hlast.2.1
  have h1 : q.epoch.toNat ≤ q.epoch.toNat * cfg.epochHeight := Nat.le_mul_of_pos_right _ hH
  have h2 : q.viewNumber.toNat ≤ v.toNat := hqv
  have h3 : v.toNat < V := hvV
  rw [hheight] at hhv
  omega

/-- Solid grounds for an epoch are for one no later than the start epoch, or than `V` plus the anchor's height. -/
theorem solid_bound {j : PubKey} {hj : C.Honest j} {n : Nat} {e : EpochNumber}
    (hg : SolidGround cfg ((N.trace j hj).history n) e) :
    e.toNat ≤ max cfg.startEpoch.toNat (V + cfg.anchorBlock.blockHeader.blockNumber.toNat) := by
  rcases hg with rfl | ⟨c1, c2, p, ⟨hrec, hwf⟩, rfl⟩ | ⟨c, hc, hce, rfl⟩
  · exact Nat.le_max_left _ _
  · have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
    have hlt := held_commit_epoch hcfg hcf hs hp hdk hV (Or.inr (Or.inr ⟨c1, c2, hrec⟩))
      (Or.inr ⟨c1, p, hrec⟩) hcq hwf.last
    have : c2.data.epoch = p.epoch := congrArg Vote2Data.epoch hcq.2
    show c2.data.epoch.toNat + 1 ≤ _
    rw [this]
    exact Nat.le_trans (Nat.succ_le_of_lt hlt) (Nat.le_max_right _ _)
  · by_cases hle : c.data.epoch.toNat ≤ cfg.startEpoch.toNat
    · exact Nat.le_trans hle (Nat.le_max_left _ _)
    have hcb : Cert1Backed N.trace c := by
      rcases cert1_held_backed N.toNetwork hc with ha | hb
      · exact absurd (by rw [ha]; exact anchor_le_start) hle
      · exact hb
    have hh : cfg.epochHeight ≠ 0 := by
      intro hh
      apply hle
      rw [hce]
      simp [epochOf_eq, hh]
    obtain ⟨j', hj', m, q, c2, hq, hc2, hcq, hlast, hqe⟩ :=
      crossing_holder N.toNetwork hcfg hcf hh hcb ⟨c.data.epoch.toNat - 1⟩
        (by show cfg.startEpoch.toNat ≤ c.data.epoch.toNat - 1; omega) (by show c.data.epoch.toNat - 1 < c.data.epoch.toNat; omega)
    have hqwf := commit_epoch hcfg hcf hc2 hcq
    have hlt := held_commit_epoch hcfg hcf hs hp hdk hV hq hc2 hcq hlast
    rw [hqe] at hlt
    have : c.data.epoch.toNat - 1 < V + cfg.anchorBlock.blockHeader.blockNumber.toNat := hlt
    exact Nat.le_trans (by omega) (Nat.le_max_right _ _)

omit hcfg hcf hs hp hdk hV in
/-- `Liveness.tc_solid`, with the induction counter in the statement. -/
private theorem tc_solid_aux : ∀ T j (hj : C.Honest j) i tc, N.time j hj i = T →
    (N.trace j hj i).input = .timeoutCertificate tc →
    ∃ j', ∃ hj' : C.Honest j', ∃ n, SolidGround cfg ((N.trace j' hj').history n) tc.data.epoch := by
  intro T
  induction T using Nat.strongRecOn with
  | ind T ih =>
    intro j hj i tc hT hin
    obtain ⟨q, hq, hvotes⟩ := N.timeoutCertCausal j hj i tc hin
    obtain ⟨k', hqk, -, hk'e⟩ := C.intersect _ q q hq hq
    obtain ⟨m, vote, ⟨-, -, hve, -⟩, hm, htm⟩ := hvotes k' hqk hk'e
    have hk' : C.Honest k' := .of hk'e
    obtain ⟨-, hep, -⟩ := (N.protocol k' hk' (m + 1)).timeoutJustified m (N.trace k' hk' m) vote
      (Trace.history_getElem? _ (Nat.lt_succ_self m)) hm (by rw [hve]; exact hk'e)
    rw [Trace.history_upTo _ (Nat.le_succ m)] at hep
    rw [← hve]
    rcases hep.1 with h1 | h1 | ⟨tc', htc', he⟩ | h1
    · exact ⟨k', hk', m, Or.inl h1⟩
    · exact ⟨k', hk', m, Or.inr (Or.inl h1)⟩
    · obtain ⟨i', hi', hin'⟩ := (Trace.received_history _).mp htc'
      rw [he]
      exact ih (N.time k' hk' i') (by have := time_mono N k' hk' (Nat.le_of_lt hi'); omega)
        k' hk' i' tc' rfl hin'
    · exact ⟨k', hk', m, Or.inr (Or.inr h1)⟩

omit hcfg hcf hs hp hdk hV in
/--
Every timeout certificate an honest node receives is of an epoch some honest node
has solid grounds for.

Its quorum has an honest signer, which named the epoch it was in, by grounds it had
earlier. Induction on the time.
-/
theorem tc_solid {j : PubKey} {hj : C.Honest j} {i : Nat} {tc : TimeoutCert}
    (hin : (N.trace j hj i).input = .timeoutCertificate tc) :
    ∃ j', ∃ hj' : C.Honest j', ∃ n, SolidGround cfg ((N.trace j' hj').history n) tc.data.epoch :=
  tc_solid_aux _ j hj i tc rfl hin

omit hcfg hcf hs hp hdk hV in
/-- Every ground an honest node has is for an epoch some honest node has solid grounds for. -/
theorem ground_solid {j : PubKey} {hj : C.Honest j} {n : Nat} {e : EpochNumber}
    (hg : ((N.trace j hj).history n).EpochGround cfg e) :
    ∃ j', ∃ hj' : C.Honest j', ∃ m, SolidGround cfg ((N.trace j' hj').history m) e := by
  rcases hg with h1 | h1 | ⟨tc, htc, rfl⟩ | h1
  · exact ⟨j, hj, n, Or.inl h1⟩
  · exact ⟨j, hj, n, Or.inr (Or.inl h1)⟩
  · obtain ⟨i, -, hin⟩ := (Trace.received_history _).mp htc
    exact tc_solid hin
  · exact ⟨j, hj, n, Or.inr (Or.inr h1)⟩

end Bounded

/-! ## Some epoch becomes stable -/

section Stabilise

variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ δ : Nat}
  (hs : Synchrony N GST Δ τ) (hp : Prompt N δ) {dk : EpochNumber → PubKey} (hdk : ∀ e, C.honest e (dk e))
  {V : Nat} (hV : ∀ e n v, (((N.trace (dk e) (.of (hdk e))).history n).restrict (C.honest · (dk e))).DecidedView v →
    v.toNat < V)
include hcfg hcf hs hp hdk hV

omit hs hp hdk hV in
/--
Below an epoch some honest node has solid grounds for, every epoch from the
anchor's on ended at a last block some honest node held with a `Cert2` over it.
-/
theorem holder_below {j0 : PubKey} {hj0 : C.Honest j0} {n0 : Nat} {e em : EpochNumber}
    (hg : SolidGround cfg ((N.trace j0 hj0).history n0) e)
    (h1 : cfg.startEpoch.toNat ≤ em.toNat) (hlt : em.toNat < e.toNat) :
    ∃ j, ∃ hj : C.Honest j, ∃ m q c2, ((N.trace j hj).history m).HasProposal cfg q
      ∧ ((N.trace j hj).history m).HasCert2 c2 ∧ Commits c2 q
      ∧ IsLastBlock q.blockHeader.blockNumber cfg.epochHeight ∧ q.epoch = em := by
  have hcross : ∀ c : Cert1, Cert1Backed N.trace c → em.toNat < c.data.epoch.toNat →
      cfg.epochHeight ≠ 0 → ∃ j, ∃ hj : C.Honest j, ∃ m q c2, ((N.trace j hj).history m).HasProposal cfg q
        ∧ ((N.trace j hj).history m).HasCert2 c2 ∧ Commits c2 q
        ∧ IsLastBlock q.blockHeader.blockNumber cfg.epochHeight ∧ q.epoch = em := fun c hc hlt' hh => by
    exact crossing_holder N.toNetwork hcfg hcf hh hc em h1 hlt'
  rcases hg with rfl | ⟨c1, c2, p, ⟨hrec, hwf⟩, rfl⟩ | ⟨c, hc, hce, rfl⟩
  · omega
  · have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
    have hpe : c2.data.epoch = p.epoch := congrArg Vote2Data.epoch hcq.2
    have hlt1 : em.toNat < c2.data.epoch.toNat + 1 := hlt
    by_cases heq : em.toNat = c2.data.epoch.toNat
    · exact ⟨j0, hj0, n0, p, c2, Or.inr (Or.inr ⟨c1, c2, hrec⟩), Or.inr ⟨c1, p, hrec⟩, hcq,
        hwf.last, by rw [← hpe]; exact (EpochNumber.ext heq).symm⟩
    · have hc1e : c1.data.epoch = c2.data.epoch := congrArg Vote2Data.epoch hwf.sameData
      rcases cert1_held_backed N.toNetwork (Or.inr (Or.inr ⟨c2, p, hrec⟩) :
          ((N.trace j0 hj0).history n0).HasCert1 cfg c1) with ha | hb
      · have := congrArg EpochNumber.toNat hc1e; rw [ha] at this
        have := anchor_le_start (cfg := cfg); omega
      · exact hcross c1 hb (by rw [hc1e]; omega) hwf.last.2.1
  · rcases cert1_held_backed N.toNetwork hc with ha | hb
    · rw [ha] at hlt; have := anchor_le_start (cfg := cfg); omega
    · refine hcross c hb hlt fun hh => ?_
      rw [hce] at hlt
      have : (epochOf c.data.blockNumber cfg.epochHeight).toNat = 0 := by simp [epochOf_eq, hh]
      omega

omit hp hdk hV in
/--
A node honest only in epochs before one some honest node has solid grounds for
gets solid grounds for an epoch after every epoch it is honest in. The epoch change
that ended the last of them reaches it; a node honest only before the anchor's
epoch starts past them.
-/
theorem retires {j0 : PubKey} {hj0 : C.Honest j0} {n0 : Nat} {e : EpochNumber}
    (hg : SolidGround cfg ((N.trace j0 hj0).history n0) e) (j : PubKey) (hj : C.Honest j)
    (hjE : ¬ C.HonestFrom e j) :
    ∃ T, N.By j hj T fun h => ∃ e', (∀ e'', C.honest e'' j → e''.toNat < e'.toNat) ∧ SolidGround cfg h e' := by
  have hbelow : ∀ e'', C.honest e'' j → e''.toNat < e.toNat := fun e'' he'' =>
    Nat.lt_of_not_le fun hle => hjE ⟨e'', hle, he''⟩
  have hex : ∃ x, ∀ e'', C.honest e'' j → e''.toNat < x := ⟨e.toNat, hbelow⟩
  obtain ⟨x, hxdef⟩ : ∃ x, x = least _ hex := ⟨_, rfl⟩
  have hx : ∀ e'', C.honest e'' j → e''.toNat < x := by rw [hxdef]; exact least_spec hex
  by_cases hxa : x ≤ cfg.startEpoch.toNat
  · exact ⟨0, 0, fun _ hi => absurd hi (Nat.not_lt_zero _), cfg.startEpoch,
      fun e'' he'' => Nat.lt_of_lt_of_le (hx e'' he'') hxa, Or.inl rfl⟩
  -- The last epoch `j` is honest in.
  obtain ⟨em, hem, hemx⟩ : ∃ em : EpochNumber, C.honest em j ∧ em.toNat + 1 = x := by
    have hnot : ¬ ∀ e'', C.honest e'' j → e''.toNat < x - 1 := by
      rw [hxdef]; exact least_min hex (by rw [← hxdef]; omega)
    refine Classical.byContradiction fun hne => hnot fun e'' he'' => Nat.lt_of_not_le fun hle => hne ?_
    have := hx e'' he''
    exact ⟨e'', he'', by omega⟩
  obtain ⟨i, hi, m, q, c2, hq, hc2, hcq, hlast, hqe⟩ :=
    holder_below hcfg hcf hg (em := em) (by omega) (hbelow em hem)
  cases m with
  | zero =>
    rcases hc2 with h | ⟨_, _, h⟩ <;>
      · obtain ⟨_, hj', -⟩ := (Trace.received_history _).mp h; exact absurd hj' (Nat.not_lt_zero _)
  | succ m =>
    have hce : c2.data.epoch = em := (congrArg Vote2Data.epoch hcq.2).trans hqe
    obtain ⟨n, hn, c1, htook⟩ := hs.epochChange c2 q hcq hlast i hi m ⟨hq, hc2⟩ j hj
      (.of (by rw [hce]; exact hem))
    refine ⟨_, n, hn, c2.data.epoch + 1, fun e'' he'' => ?_, Or.inr (Or.inl ⟨c1, c2, q, htook, rfl⟩)⟩
    have := hx e'' he''
    show e''.toNat < c2.data.epoch.toNat + 1
    rw [hce]; omega

omit hp hdk hV in
/--
**Some epoch becomes stable**, when solid grounds are bounded. Take the latest epoch
any honest node ever has solid grounds for. Grounds for it reach every node honest
in it or later within `Δ`: an epoch change by its delivery, a certificate of the
epoch as it is. Every other honest node retires (`Liveness.retires`).
-/
theorem stabilises {B0 : Nat} (hB0 : ∀ j (hj : C.Honest j) n e, SolidGround cfg ((N.trace j hj).history n) e →
    e.toNat ≤ B0) : ∃ E t0, Stable N GST E t0 := by
  have hu := hs.timeUnbounded
  obtain ⟨B, hBdef⟩ : ∃ B, B = max cfg.startEpoch.toNat B0 := ⟨_, rfl⟩
  obtain ⟨k0, -, -, hk0⟩ := C.intersect _ _ _ (N.honestQuorum cfg.startEpoch)
    (N.honestQuorum cfg.startEpoch)
  have hk0 : C.Honest k0 := .of hk0
  have hex : ∃ x, ∃ j, ∃ hj : C.Honest j, ∃ n, SolidGround cfg ((N.trace j hj).history n) ⟨B - x⟩ :=
    ⟨B - cfg.startEpoch.toNat, k0, hk0, 0, Or.inl (EpochNumber.ext (by
      show B - (B - cfg.startEpoch.toNat) = cfg.startEpoch.toNat
      have := Nat.le_max_left cfg.startEpoch.toNat B0
      omega))⟩
  obtain ⟨x, hxdef⟩ : ∃ x, x = least _ hex := ⟨_, rfl⟩
  obtain ⟨j0, hj0, n0, hg0⟩ : ∃ j, ∃ hj : C.Honest j, ∃ n, SolidGround cfg ((N.trace j hj).history n) ⟨B - x⟩ := by
    rw [hxdef]; exact least_spec hex
  -- Every solid ground is for an epoch no later than this one.
  have hmax : ∀ j (hj : C.Honest j) n e, SolidGround cfg ((N.trace j hj).history n) e →
      e.toNat ≤ B - x := fun j hj n e hg => by
    have hb : e.toNat ≤ B := by rw [hBdef]; exact Nat.le_trans (hB0 j hj n e hg) (Nat.le_max_right _ _)
    have := hxdef ▸ least_le hex (m := B - e.toNat) ⟨j, hj, n, by
      have : (⟨B - (B - e.toNat)⟩ : EpochNumber) = e := EpochNumber.ext (by show B - (B - e.toNat) = e.toNat; omega)
      rw [this]; exact hg⟩
    omega
  have hbound : ∀ j (hj : C.Honest j) n e, ((N.trace j hj).history n).EpochGround cfg e →
      e.toNat ≤ (⟨B - x⟩ : EpochNumber).toNat := fun j hj n e hg => by
    obtain ⟨j', hj', m, hsg⟩ := ground_solid hg
    exact hmax j' hj' m e hsg
  -- Every node honest in that epoch or later gets the solid ground.
  have hreach : ∃ T0, GST ≤ T0 ∧ ∀ j (hj : C.Honest j), C.HonestFrom ⟨B - x⟩ j →
      N.By j hj T0 fun h => SolidGround cfg h ⟨B - x⟩ := by
    rcases hg0 with hanc | ⟨c1, c2, p, ⟨hrec, hwf⟩, hE⟩ | ⟨c, hc, hce, hE⟩
    · exact ⟨GST, Nat.le_refl _, fun j hj _ => ⟨0, fun _ hi => absurd hi (Nat.not_lt_zero _), Or.inl hanc⟩⟩
    · obtain ⟨i, hi, hin⟩ := (Trace.received_history _).mp hrec
      have hcq : Commits c2 p := ⟨hwf.1, by rw [← hwf.sameData, hwf.cert1Data]; rfl⟩
      have hrec' : ((N.trace j0 hj0).history (i + 1)).Received (.epochChange c1 c2 p) :=
        hin ▸ Trace.received_self _ i
      refine ⟨max (N.time j0 hj0 i) GST + Δ, by omega, fun j hj hjE => ?_⟩
      obtain ⟨m, hm, c1', htook⟩ := hs.epochChange c2 p hcq hwf.last j0 hj0 i
        ⟨Or.inr (Or.inr ⟨c1, c2, hrec'⟩), Or.inr ⟨c1, p, hrec'⟩⟩ j hj
        (hjE.mono (by rw [hE]; exact Nat.le_succ _))
      exact ⟨m, hm, Or.inr (Or.inl ⟨c1', c2, p, htook, hE⟩)⟩
    · by_cases hca : c = cfg.anchorCert
      · subst hca
        exact ⟨GST, Nat.le_refl _, fun j hj _ => ⟨0, fun _ hi => absurd hi (Nat.not_lt_zero _),
          Or.inr (Or.inr ⟨cfg.anchorCert, Or.inl rfl, hce, hE⟩)⟩⟩
      have hn0 : n0 ≠ 0 := fun h => hca (hasCert1_nil (h ▸ hc))
      obtain ⟨n', rfl⟩ : ∃ n', n0 = n' + 1 := ⟨n0 - 1, by omega⟩
      refine ⟨max (N.time j0 hj0 n') GST + Δ, by omega, fun j hj hjE => ?_⟩
      obtain ⟨m, hm, hheld⟩ := hs.certSpread c j0 hj0 n' hc j hj (by rw [← hE]; exact hjE)
      exact ⟨m, hm, Or.inr (Or.inr ⟨c, hheld, hce, hE⟩)⟩
  -- Every other honest node retires.
  obtain ⟨Tr, hTr⟩ := honest_bound (C := C)
    (fun j T => ∀ hj : C.Honest j, ¬ C.HonestFrom ⟨B - x⟩ j →
      N.By j hj T fun h => ∃ e', (∀ e, C.honest e j → e.toNat < e'.toNat) ∧ SolidGround cfg h e')
    (fun _ _ _ hle h hj hjE => by_later hle (h hj hjE))
    (fun j hj => by
      by_cases hjE : C.HonestFrom ⟨B - x⟩ j
      · exact ⟨0, fun _ h => absurd hjE h⟩
      · obtain ⟨T, hT⟩ := retires hcfg hcf hs hg0 j hj hjE
        exact ⟨T, fun _ _ => hT⟩)
  obtain ⟨T0, hT0, hr⟩ := hreach
  exact ⟨⟨B - x⟩, max T0 Tr, ⟨hbound, fun j hj hjE => by_later (Nat.le_max_left _ _) (hr j hj hjE),
    fun j hj hjE => by_later (Nat.le_max_right _ _) (hTr j hj hj hjE), Nat.le_trans hT0 (Nat.le_max_left _ _)⟩⟩

end Stabilise

/-! ## Views keep advancing -/

section Progress

variable (hcfg : ConfigCoherent cfg) {GST Δ τ : Nat} (hs : Synchrony N GST Δ τ)
include hcfg hs

omit hcfg hs in
/-- A node that has reached `y` and has grounds for nothing later is in `y`. -/
theorem inView_of_cap {k : PubKey} {hk : C.Honest k} {n : Nat} {y : ViewNumber}
    (hcap : ∀ v, ((N.trace k hk).history n).ViewGround cfg v → v.toNat ≤ y.toNat)
    (hr : Reached N k hk n y) : ((N.trace k hk).history n).InView cfg y := by
  obtain ⟨v, hv, hyv⟩ := hr
  have hveq : v = y := ViewNumber.ext (by have := hcap v hv; have : y.toNat ≤ v.toNat := hyv; omega)
  subst hveq
  exact ⟨hv, fun v' hv' => hcap v' hv'⟩

omit hcfg in
/--
A node that reaches `y` and never has grounds for a later view times `y` out again
and again: its timer re-fires every `τ` while it stays in the view.
-/
theorem times_out_late {k : PubKey} {hk : C.Honest k} {y : ViewNumber}
    (hcap : ∀ n v, ((N.trace k hk).history n).ViewGround cfg v → v.toNat ≤ y.toNat)
    {n : Nat} (hr : Reached N k hk n y) (T : Nat) :
    ∃ m, T < N.time k hk m ∧ (N.trace k hk m).input = .timeout y
      ∧ ((N.trace k hk).history m).InView cfg y := by
  have hu := hs.timeUnbounded
  have hin : ∀ n, Reached N k hk n y → ((N.trace k hk).history n).InView cfg y :=
    fun n hr => inView_of_cap (hcap n) hr
  obtain ⟨n0, hin1, hfresh⟩ : ∃ n0, ((N.trace k hk).history (n0 + 1)).InView cfg y
      ∧ (n0 = 0 ∨ ¬ ((N.trace k hk).history n0).InView cfg y) := by
    by_cases hz : ((N.trace k hk).history 0).InView cfg y
    · exact ⟨0, hin 1 (reached_mono (Nat.zero_le 1) ⟨y, hz.1, Nat.le_refl _⟩), Or.inl rfl⟩
    · obtain ⟨n0, -, hin1, hnot⟩ := entered _ (hin n hr) hz
      exact ⟨n0, hin1, Or.inr hnot⟩
  have hrn0 : Reached N k hk (n0 + 1) y := ⟨y, hin1.1, Nat.le_refl _⟩
  have hnext : ∀ m, n0 < m → ¬ ∃ w, y < w ∧ ((N.trace k hk).history (m + 1)).InView cfg w :=
    fun m _ ⟨w, hlt, hw⟩ => by
      have := hcap (m + 1) w hw.1
      have : y.toNat < w.toNat := hlt
      omega
  -- Timeouts for `y` at arbitrarily late steps.
  have hchain : ∀ N0, ∃ m, N0 ≤ m ∧ n0 < m ∧ (N.trace k hk m).input = .timeout y := by
    intro N0
    induction N0 with
    | zero =>
      obtain ⟨m, hm, -, hcase⟩ := hs.timerFires k hk n0 y hin1 (hfresh.imp id Or.inl)
      rcases hcase with htm | hw
      · exact ⟨m, Nat.zero_le _, hm, htm⟩
      · exact absurd hw (hnext m hm)
    | succ N0 ih =>
      obtain ⟨m, hm, hm0, htm⟩ := ih
      have hinm := hin (m + 1) (reached_mono (by omega) hrn0)
      obtain ⟨m', hm', -, hcase⟩ := hs.timerFires k hk m y hinm (Or.inr (Or.inr htm))
      rcases hcase with htm' | hw
      · exact ⟨m', by omega, by omega, htm'⟩
      · exact absurd hw (hnext m' (by omega))
  obtain ⟨m, hm, hm0, htm⟩ := hchain (cut N hu k hk T)
  exact ⟨m, cut_after N hu hm, htm, hin m (reached_mono (by omega) hrn0)⟩

variable {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
include hst

omit hcfg in
/--
A member of `E` reaches a view an honest node reached, when no honest node ever has
grounds for a later one.

Otherwise it comes to rest in an earlier view and times it out again and again,
after `t0` and so in `E`, while an honest node has grounds past it: it gets grounds
past it too (`Synchrony.timeoutCatchUp`).
-/
theorem member_reaches {y : ViewNumber}
    (hcap : ∀ k (hk : C.Honest k) n v, ((N.trace k hk).history n).ViewGround cfg v → v.toNat ≤ y.toNat)
    {j : PubKey} {hj : C.Honest j} {n : Nat} (hr : Reached N j hj n y)
    {k : PubKey} (hkE : C.members E k ∧ C.honest E k) : ∃ n, Reached N k (.of hkE.2) n y := by
  have hk : C.Honest k := .of hkE.2
  have hu := hs.timeUnbounded
  refine Classical.byContradiction fun hno => ?_
  -- The latest view `k` ever reaches, `y - d`.
  have hex : ∃ d, ∃ n, Reached N k hk n ⟨y.toNat - d⟩ :=
    ⟨y.toNat, 0, cfg.anchorCert.view + 1, Or.inl ⟨cfg.anchorCert, Or.inl rfl, rfl⟩,
      by show y.toNat - y.toNat ≤ cfg.anchorCert.view.toNat + 1; omega⟩
  obtain ⟨d, hd⟩ : ∃ d, d = least _ hex := ⟨_, rfl⟩
  obtain ⟨n0, hr0⟩ : ∃ n, Reached N k hk n ⟨y.toNat - d⟩ := hd ▸ least_spec hex
  have hd0 : d ≠ 0 := fun h => hno ⟨n0, by
    have : (⟨y.toNat - d⟩ : ViewNumber) = y := ViewNumber.ext (by show y.toNat - d = y.toNat; omega)
    rw [this] at hr0; exact hr0⟩
  have hcapk : ∀ n v, ((N.trace k hk).history n).ViewGround cfg v → v.toNat ≤ y.toNat - d := fun n v hv => by
    refine Nat.le_of_not_lt fun hlt => ?_
    have hvy := hcap k hk n v hv
    have hlt' : y.toNat - v.toNat < least _ hex := by rw [← hd]; omega
    have hmin := least_min hex hlt'
    exact hmin ⟨n, v, hv, by show y.toNat - (y.toNat - v.toNat) ≤ v.toNat; omega⟩
  -- It times that view out again and again, after `t0`, in `E`.
  have hrep : ∀ T, ∃ m vote, T < N.time k hk m ∧ vote.view = ⟨y.toNat - d⟩
      ∧ Output.send (.timeoutVote vote) ∈ (N.trace k hk m).output := by
    intro T
    obtain ⟨m, hmT, htm, hinm⟩ := times_out_late hs hcapk hr0 (max T t0)
    have hE := stable_inEpoch hst hu (.of hkE.2) (cut_le_of_lt hu (show t0 < N.time k hk m by omega))
    obtain ⟨e, L, -, hout⟩ := (N.protocol k hk (m + 1)).timeoutAnswered m _ _
      (Trace.history_getElem? _ (Nat.lt_succ_self m))
      (Or.inl ⟨htm, by rw [Trace.history_upTo _ (Nat.le_succ m)]; exact hinm⟩)
      (fun e' he' => by
        rw [Trace.history_upTo _ (Nat.le_succ m)] at he'
        rw [inEpoch_unique he' hE]; exact ⟨hkE.2, hkE.1⟩)
    exact ⟨m, _, by omega, rfl, hout⟩
  obtain ⟨v, hv, hyv⟩ := hr
  have hyv' : y.toNat ≤ v.toNat := hyv
  have hv1 : 1 ≤ v.toNat := by
    rcases hv with ⟨c, -, rfl⟩ | ⟨tc, -, rfl⟩ | ⟨c1, c2, p, -, rfl⟩ <;> exact Nat.le_add_left 1 _
  obtain ⟨n', u', hu', hlt⟩ := hs.timeoutCatchUp k hk _ hrep j hj n v hv
    (show y.toNat - d < v.toNat by omega)
  have := hcapk n' u' hu'
  have : y.toNat - d < u'.toNat := hlt
  omega

omit hcfg in
/--
**Every view is reached** in a stable epoch.

Otherwise honest nodes come to rest in the last view any of them reaches. Every
member of `E` reaches it (`Liveness.member_reaches`) and times it out again after
`t0`, when it is in `E`, so the votes name one epoch, and their timeout
certificate takes them past it.
-/
theorem views_unbounded (x : ViewNumber) : ∃ T, ReachedBy N T x := by
  have hu := hs.timeUnbounded
  refine Classical.byContradiction fun hx => ?_
  have hex : ∃ n, ¬ ∃ T, ReachedBy N T ⟨n⟩ := ⟨x.toNat, hx⟩
  have hspec := least_spec hex
  have hmin : ∀ m, m < least _ hex → ¬ ¬ ∃ T, ReachedBy N T ⟨m⟩ := fun m hm => least_min hex hm
  generalize least (fun n => ¬ ∃ T, ReachedBy N T ⟨n⟩) hex = x' at hspec hmin
  obtain ⟨k0, -, -, hk0E⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
  have hk0 : C.Honest k0 := .of hk0E
  have hpos : x' ≠ 0 := by
    rintro rfl
    exact hspec ⟨0, k0, hk0, 0, fun _ hi => absurd hi (Nat.not_lt_zero _),
      cfg.anchorCert.view + 1, Or.inl ⟨cfg.anchorCert, Or.inl rfl, rfl⟩, Nat.zero_le _⟩
  obtain ⟨y, hyx⟩ : ∃ y : ViewNumber, y.toNat + 1 = x' := ⟨⟨x' - 1⟩, by show x' - 1 + 1 = x'; omega⟩
  have hcap : ∀ k (hk : C.Honest k) n v, ((N.trace k hk).history n).ViewGround cfg v →
      v.toNat ≤ y.toNat := fun k hk n v hv => by
    have : v.toNat < x' := Nat.lt_of_not_le fun hle => hspec ⟨N.time k hk n, k, hk, n,
      fun i hi => time_mono N k hk (Nat.le_of_lt hi), v, hv, hle⟩
    omega
  obtain ⟨t0', j, hj, n, -, hr⟩ := Classical.byContradiction (hmin y.toNat (by omega))
  obtain ⟨t, ht⟩ := honest_bound (C := C)
    (fun k t => ∀ he : C.members E k ∧ C.honest E k, ∃ L,
      N.SentByTime k (.of he.2) t (.timeoutVote ⟨⟨E, L⟩, y, k⟩))
    (fun _ _ _ hle h he => let ⟨L, hL⟩ := h he; ⟨L, by_later hle hL⟩)
    (fun k hk => by
      by_cases hkE : C.members E k ∧ C.honest E k
      · obtain ⟨nk, hrk⟩ := member_reaches hs hst hcap hr hkE
        obtain ⟨m, hmT, htm, hinm⟩ := times_out_late hs (hcap k hk) hrk t0
        have hE := stable_inEpoch hst hu (.of hkE.2) (cut_le_of_lt hu hmT)
        obtain ⟨e, L, hep, hout⟩ := (N.protocol k hk (m + 1)).timeoutAnswered m _ y
          (Trace.history_getElem? _ (Nat.lt_succ_self m))
          (Or.inl ⟨htm, by rw [Trace.history_upTo _ (Nat.le_succ m)]; exact hinm⟩)
          (fun e' he' => by
            rw [Trace.history_upTo _ (Nat.le_succ m)] at he'
            rw [inEpoch_unique he' hE]; exact ⟨hkE.2, hkE.1⟩)
        rw [Trace.history_upTo _ (Nat.le_succ m)] at hep
        have he : e = E := inEpoch_unique hep hE
        subst he
        exact ⟨N.time k hk m, fun _ => ⟨L, m + 1, fun i hi => time_mono N k hk (Nat.le_of_lt_succ hi),
          (Trace.sent_history _).mpr ⟨m, Nat.lt_succ_self m, hout⟩⟩⟩
      · exact ⟨0, fun he => absurd he hkE⟩)
  obtain ⟨n1, -, tc, -, htcv, hrec⟩ := hs.timeoutCert E _ y t (N.honestQuorum E)
    (fun k ⟨hm, hk⟩ => ⟨hk, ht k (.of hk) ⟨hm, hk⟩⟩) k0 hk0 (.of hk0E)
  have := hcap k0 hk0 n1 _ (Or.inr (Or.inl ⟨tc, hrec, rfl⟩))
  have : tc.view.toNat + 1 ≤ y.toNat := this
  rw [htcv] at this
  omega

/--
**No epoch stays stable when epochs have blocks.** Every view of `E` with a leader
honest in `E` is reached (`Liveness.views_unbounded`), and commits a block of `E` at
its own view (`Liveness.late_cert2`, `Liveness.late_commit`). Each such block comes
after the one committed at an earlier such view (`noFork`), so its block number is
larger. No block of `E` has a larger block number than `E`'s last
(`epochOf_height_le`).
-/
theorem not_stable (hcf : CollisionFree) {δ : Nat} (hp : Prompt N δ) (hτ : 8 * Δ + 3 * δ < τ)
    (hrot : LeaderRotation C leader) (hh : cfg.epochHeight ≠ 0) : False := by
  have hu := hs.timeUnbounded
  obtain ⟨B, hB⟩ := ground_bound N hu t0
  have hlateOf : ∀ x : ViewNumber, B ≤ x.toNat → Late N t0 x := fun x hx ⟨j, hj, n, hn, v, hv, hxv⟩ => by
    have := hB j hj n hn v hv
    have : x.toNat ≤ v.toNat := hxv
    omega
  -- A block of `E` committed at a late view, past any bound.
  have hcommit : ∀ X : Nat, ∃ j, ∃ hj : C.Honest j, ∃ n c2 q, ((N.trace j hj).history n).HasProposal cfg q
      ∧ ((N.trace j hj).history n).HasCert2 c2 ∧ Commits c2 q ∧ c2.data.epoch = E
      ∧ X + B + 3 ≤ c2.view.toNat := by
    intro X
    obtain ⟨w, hwge, l, hlead, hl, hlm⟩ := hrot E ⟨X + B + cfg.anchorView.toNat + 3⟩
    have hwge' : X + B + cfg.anchorView.toNat + 3 ≤ w.toNat := hwge
    obtain ⟨j, hj, n, c2, q, hq, hc2, hcq, hce, hwc⟩ := late_cert2 hcfg hcf hs hst
      (hlateOf _ (by show B ≤ w.toNat - 1; omega)) hp (by omega) (views_unbounded hs hst w) hτ
      hl hlead hlm
    have : w.toNat ≤ c2.view.toNat := hwc
    exact ⟨j, hj, n, c2, q, hq, hc2, hcq, hce, by omega⟩
  have hres := heldTree_resolves N.toNetwork hcf
  have hcoh := heldTree_coherent N.toNetwork
  -- The committed blocks get higher, one view after another.
  have hgrow : ∀ i, ∃ j, ∃ hj : C.Honest j, ∃ n c2 q, ((N.trace j hj).history n).HasProposal cfg q
      ∧ ((N.trace j hj).history n).HasCert2 c2 ∧ Commits c2 q ∧ c2.data.epoch = E
      ∧ B + 3 ≤ c2.view.toNat ∧ i ≤ q.blockHeader.blockNumber.toNat := by
    intro i
    induction i with
    | zero =>
      obtain ⟨j, hj, n, c2, q, hq, hc2, hcq, hce, hv⟩ := hcommit 0
      exact ⟨j, hj, n, c2, q, hq, hc2, hcq, hce, by omega, Nat.zero_le _⟩
    | succ i ih =>
      obtain ⟨j, hj, n, c2, q, hq, hc2, hcq, hce, hv, hi⟩ := ih
      obtain ⟨j', hj', n', c2', q', hq', hc2', hcq', hce', hv'⟩ := hcommit c2.view.toNat
      refine ⟨j', hj', n', c2', q', hq', hc2', hcq', hce', by omega, ?_⟩
      have hb2 := cert2_held_backed N.toNetwork hc2
      have hb2' := cert2_held_backed N.toNetwork hc2'
      have hanc := noFork cfg C N.toNetwork (heldTree N.toNetwork) hcfg hcoh hcf hres c2 c2' hb2 hb2'
        (Or.inr ⟨hce.trans hce'.symm, show c2.view.toNat ≤ c2'.view.toNat by omega⟩)
      have hqv := (late_commit hcfg hcf hs hst hq hc2 hcq (hlateOf _ (by omega))).1
      have hqv' := (late_commit hcfg hcf hs hst hq' hc2' hcq' (hlateOf _ (by omega))).1
      obtain ⟨c1, hc1b, -, hc1h, -⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg hb2'
      have hqh : c2.data.blockHash = blockHash q := congrArg Vote2Data.blockHash hcq.2
      have hqh' : c2'.data.blockHash = blockHash q' := congrArg Vote2Data.blockHash hcq'.2
      -- The two blocks are at different views, so they differ.
      have hne : blockHash q ≠ c1.data.blockHash := fun he => by
        have hqq : q = q' := hcf q q' (by rw [he, hc1h, hqh'])
        subst hqq
        have := congrArg ViewNumber.toNat (hqv.symm.trans hqv')
        omega
      have hlt := ancestor_height_lt (heldTree N.toNetwork) N.toNetwork hcfg hcoh hcf hres hc1b
        (by rw [hc1h, ← hqh]; exact hanc) hne (hres j hj n q hq)
        (by rw [hc1h, hqh']; exact hres j' hj' n' q' hq')
      omega
  obtain ⟨j, hj, n, c2, q, hq, hc2, hcq, hce, -, hi⟩ := hgrow (E.toNat * cfg.epochHeight + 1)
  -- A block of `E` is no higher than `E`'s last.
  obtain ⟨c1, hc1b, -, hc1h, hc1e⟩ := cert2_implies_cert1 cfg N.toNetwork hcfg (cert2_held_backed N.toNetwork hc2)
  obtain ⟨b, -, hbh, hbe, hbn⟩ := cert1Backed_block hc1b
  have hbq : b = q := hcf b q (by rw [← hbh, hc1h]; exact congrArg Vote2Data.blockHash hcq.2)
  subst hbq
  have hep : epochOf b.blockHeader.blockNumber cfg.epochHeight = E := by
    rw [← hbn, ← cert1_epoch_of_height N.toNetwork hcfg (Or.inl hc1b), hc1e, hce]
  have hle := epochOf_height_le (n := b.blockHeader.blockNumber) hh
  rw [hep] at hle
  omega

end Progress

/-! ## The result -/

/--
**No bound holds on what a node honest in a stable epoch decides.** A view of the
stable epoch `E` with a leader honest in `E`, later than `V` and everything reached
when `E` became stable, is reached (`Liveness.views_unbounded`) and decided by the
node (`Liveness.late_decide`).
-/
theorem stable_decides {GST Δ δ τ : Nat} (hcfg : ConfigCoherent cfg) (hcf : CollisionFree)
    (hs : Synchrony N GST Δ τ) (hp : Prompt N δ) (hτ : 8 * Δ + 3 * δ < τ) (hrot : LeaderRotation C leader)
    {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0) {d : PubKey} (hd : C.honest E d) {V : Nat}
    (hV : ∀ n v, (((N.trace d (.of hd)).history n).restrict (C.honest · d)).DecidedView v → v.toNat < V) :
    False := by
  have hu := hs.timeUnbounded
  obtain ⟨B, hB⟩ := ground_bound N hu t0
  obtain ⟨w, hwge, l, hlead, hl, hlm⟩ := hrot E ⟨V + B + cfg.anchorView.toNat + 3⟩
  have hwge' : V + B + cfg.anchorView.toNat + 3 ≤ w.toNat := hwge
  have hw3 : cfg.anchorView.toNat + 3 ≤ w.toNat := by omega
  have hlate : Late N t0 (w - 1) := fun ⟨j, hj, n, hn, v, hv, hwv⟩ => by
    have := hB j hj n hn v hv
    have : w.toNat - 1 ≤ v.toNat := hwv
    omega
  have hex := views_unbounded hs hst w
  obtain ⟨n, v, hwv, hdec⟩ := late_decide hcfg hcf hs hst hlate hp hw3 hex hτ hl hlead hlm d hd
  have := hV n v hdec
  have : w.toNat ≤ v.toNat := hwv
  omega

/--
**No bound holds on what nodes honest in each epoch decide.** Given a node honest in
each epoch, all of whose decides on `Cert2`s of epochs it is honest in are below
`V`: solid grounds are bounded (`Liveness.solid_bound`), some epoch `E` becomes
stable (`stabilises`), and the node honest in `E` decides past `V`
(`Liveness.stable_decides`).
-/
theorem decides_unbounded {GST Δ δ τ : Nat} (hcfg : ConfigCoherent cfg) (hcf : CollisionFree)
    (hs : Synchrony N GST Δ τ) (hp : Prompt N δ) (hτ : 8 * Δ + 3 * δ < τ) (hrot : LeaderRotation C leader)
    {dk : EpochNumber → PubKey} (hdk : ∀ e, C.honest e (dk e)) {V : Nat}
    (hV : ∀ e n v, (((N.trace (dk e) (.of (hdk e))).history n).restrict (C.honest · (dk e))).DecidedView v →
      v.toNat < V) : False := by
  obtain ⟨E, t0, hst⟩ := stabilises hcfg hcf hs (fun _ _ _ _ hg => solid_bound hcfg hcf hs hp hdk hV hg)
  exact stable_decides hcfg hcf hs hp hτ hrot hst (hdk E) (hV E)

/-- With epoch height zero, every solid ground is the start epoch. -/
theorem solidGround_start_of_height_zero (hcfg : ConfigCoherent cfg) (hh : cfg.epochHeight = 0)
    {h : History} {e : EpochNumber} (hg : SolidGround cfg h e) : e = cfg.startEpoch := by
  have hstart : cfg.startEpoch = 0 := by
    simp only [Config.startEpoch, IsLastBlock, hh, ne_eq, not_true_eq_false, false_and, and_false,
      ite_false, hcfg.anchorCertEpoch, epochOf, ite_true]
  rcases hg with rfl | ⟨c1, c2, p, ht, -⟩ | ⟨c, -, hce, rfl⟩
  · rfl
  · exact absurd ht.2.last.2.1 (by simp [hh])
  · rw [hstart, hce]; simp [epochOf, hh]

/--
**No bound holds on what a node honest in the start epoch decides**, when epochs
have no blocks. Every solid ground is then the start epoch
(`Liveness.solidGround_start_of_height_zero`), so the start epoch becomes stable
(`stabilises`), and the node decides past any bound (`Liveness.stable_decides`).
-/
theorem single_epoch_decides {GST Δ δ τ : Nat} (hcfg : ConfigCoherent cfg) (hcf : CollisionFree)
    (hs : Synchrony N GST Δ τ) (hp : Prompt N δ) (hτ : 8 * Δ + 3 * δ < τ) (hrot : LeaderRotation C leader)
    (hh : cfg.epochHeight = 0) {k : PubKey} (hks : C.honest cfg.startEpoch k) {V : Nat}
    (hV : ∀ n v, (((N.trace k (.of hks)).history n).restrict (C.honest · k)).DecidedView v → v.toNat < V) :
    False := by
  obtain ⟨E, t0, hst⟩ := stabilises hcfg hcf hs (B0 := cfg.startEpoch.toNat)
    fun _ _ _ _ hg => Nat.le_of_eq (congrArg EpochNumber.toNat (solidGround_start_of_height_zero hcfg hh hg))
  -- `E` has an honest member, which has solid grounds for it.
  obtain ⟨j, -, -, hjE⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
  obtain ⟨n, -, hgn⟩ := hst.reach j (.of hjE) ⟨E, Nat.le_refl _, hjE⟩
  have hE : E = cfg.startEpoch := solidGround_start_of_height_zero hcfg hh hgn
  subst hE
  exact stable_decides hcfg hcf hs hp hτ hrot hst hks hV

/--
**No bound holds on what a node honest in infinitely many epochs decides**, when
epochs have blocks. Were it bounded by `V`, the node would be honest in some epoch
`e` that is no earlier than the start epoch, nor than `V`. No honest node holds `e`'s last block with a
`Cert2`: the epoch change would reach the node, which would decide past `V`
(`Liveness.holder_decides`). So no honest node has solid grounds for an epoch after
`e` (`Liveness.holder_below`), some epoch becomes stable (`Liveness.stabilises`),
and none does (`Liveness.not_stable`).
-/
theorem often_decides {GST Δ δ τ : Nat} (hcfg : ConfigCoherent cfg) (hcf : CollisionFree)
    (hs : Synchrony N GST Δ τ) (hp : Prompt N δ) (hτ : 8 * Δ + 3 * δ < τ) (hrot : LeaderRotation C leader)
    (hh : cfg.epochHeight ≠ 0) {k : PubKey} (hko : C.HonestOften k) {V : Nat}
    (hV : ∀ n v, (((N.trace k hko.honest).history n).restrict (C.honest · k)).DecidedView v → v.toNat < V) :
    False := by
  have hk : C.Honest k := hko.honest
  obtain ⟨e, hle, hke⟩ := hko ⟨max cfg.startEpoch.toNat (V + cfg.anchorBlock.blockHeader.blockNumber.toNat)⟩
  have hle' : max cfg.startEpoch.toNat (V + cfg.anchorBlock.blockHeader.blockNumber.toNat) ≤ e.toNat := hle
  have hH : 1 ≤ cfg.epochHeight := Nat.pos_of_ne_zero hh
  have hB : ∀ j (hj : C.Honest j) n e', SolidGround cfg ((N.trace j hj).history n) e' →
      e'.toNat ≤ e.toNat := by
    intro j hj n e' hg
    refine Nat.le_of_not_lt fun hlt => ?_
    obtain ⟨i, hi, m, q, c2, hq, hc2, hcq, hlast, hqe⟩ :=
      holder_below hcfg hcf hg (em := e) (Nat.le_trans (Nat.le_max_left _ _) hle') hlt
    obtain ⟨n', v, hqv, hd⟩ := holder_decides hcfg hcf hs hp hq hc2 hcq hlast k (by rw [hqe]; exact hke)
    have hvV := hV n' v hd
    have hhv := committed_view_ge_height hcfg hcf hq hc2 hcq
    have hqwf := commit_epoch hcfg hcf hc2 hcq
    -- `q` is `e`'s last block, at height `e * epochHeight`, and its view is no lower.
    have hheight : q.blockHeader.blockNumber.toNat = e.toNat * cfg.epochHeight :=
      lastBlock_height hlast (by rw [← hqwf, hqe])
    have h1 : e.toNat ≤ e.toNat * cfg.epochHeight := Nat.le_mul_of_pos_right _ hH
    have h2 : q.viewNumber.toNat ≤ v.toNat := hqv
    omega
  obtain ⟨E, t0, hst⟩ := stabilises hcfg hcf hs hB
  exact not_stable hcfg hs hst hcf hp hτ hrot hh

/--
**`ChainGrows` holds**, for every epoch height.

Chain growth: were every view a node decides, on a `Cert2` of an epoch it is honest
in, no later than what it had decided by `t`, those decides would be bounded, with
a node honest in each epoch (`Committee.intersect` of the epoch's honest quorum) to
apply `Liveness.decides_unbounded` to. A steady node is honest in every epoch, so it
can be that node for all of them. With epochs of no blocks, a node honest in the
start epoch keeps deciding by `Liveness.single_epoch_decides`. A node honest in
infinitely many epochs, with epochs of blocks, keeps deciding by
`Liveness.often_decides`.
-/
theorem chainGrows : ChainGrows cfg leader C := by
  intro N GST Δ δ τ hcfg hcf hs hp hτ hrot
  have hu := hs.timeUnbounded
  -- A node whose decides are all bounded by what it decided by `t` decides past no `V`.
  have after : ∀ k (hk : C.Honest k) t, ¬ N.DecidesAfter k hk t → ∃ V, ∀ n v,
      (((N.trace k hk).history n).restrict (C.honest · k)).DecidedView v → v.toNat < V := by
    intro k hk t hneg
    obtain ⟨V, hV⟩ := decided_bound (((N.trace k hk).history (cut N hu k hk t)).restrict (C.honest · k))
    refine ⟨V, fun n v hd => Nat.lt_of_not_le fun hle => hneg ⟨v, ⟨n, hd⟩, fun n' hn' w hw => ?_⟩⟩
    have := hV w (decidedView_restrict_mono _ _ (le_cut N hu hn') hw)
    show w.toNat < v.toNat
    omega
  refine ⟨fun t => ?_, fun t k hk hkp => ?_⟩
  · refine Classical.byContradiction fun hneg => ?_
    have hex : ∀ e, ∃ k, C.honest e k := fun e =>
      let ⟨k, _, _, hk⟩ := C.intersect _ _ _ (N.honestQuorum e) (N.honestQuorum e); ⟨k, hk⟩
    obtain ⟨V, hVb⟩ := honest_bound (C := C) (fun k B => ∀ hk : C.Honest k, ∀ v,
        (((N.trace k hk).history (cut N hu k hk t)).restrict (C.honest · k)).DecidedView v → v.toNat < B)
      (fun _ _ _ hle h hk v hd => Nat.lt_of_lt_of_le (h hk v hd) hle)
      (fun k hk => by
        obtain ⟨V, hV⟩ := decided_bound (((N.trace k hk).history (cut N hu k hk t)).restrict (C.honest · k))
        exact ⟨V, fun _ v hd => hV v hd⟩)
    refine decides_unbounded hcfg hcf hs hp hτ hrot (dk := fun e => Classical.choose (hex e))
      (fun e => Classical.choose_spec (hex e)) (V := V) fun e n v hd => ?_
    refine Nat.lt_of_not_le fun hle => hneg ⟨_, .of (Classical.choose_spec (hex e)), v, ⟨n, hd⟩,
      fun n' hn' w hw => ?_⟩
    have := hVb _ (.of (Classical.choose_spec (hex e))) (.of (Classical.choose_spec (hex e))) w
      (decidedView_restrict_mono _ _ (le_cut N hu hn') hw)
    show w.toNat < v.toNat
    omega
  refine Classical.byContradiction fun hneg => ?_
  obtain ⟨V, hV⟩ := after k hk t hneg
  rcases hkp with hks | ⟨hh, hk0⟩ | ⟨hh, hko⟩
  · exact decides_unbounded hcfg hcf hs hp hτ hrot (dk := fun _ => k) (fun e => hks e) (V := V)
      fun _ n v hd => hV n v hd
  · exact single_epoch_decides hcfg hcf hs hp hτ hrot hh hk0 (V := V) fun n v hd => hV n v hd
  · exact often_decides hcfg hcf hs hp hτ hrot hh hko (V := V) hV

end Liveness
end NewProtocol
