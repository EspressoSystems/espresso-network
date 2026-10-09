module

public import NewProtocolSpec.Proofs.Liveness.First
public import NewProtocolSpec.Proofs.Safety

/-!
# A stable epoch

The cross-epoch argument looks for a time after which no honest node ever gets
grounds for a later epoch (`Liveness.Stable`). From then on every honest node is
in that epoch, every honest vote is cast for it, and no honest node holds its last
block with a `Cert2`: otherwise the epoch change would reach every honest node,
which would then have grounds for the next epoch.

The facts about certificates this needs come from the safety argument, walked
down a tree built from the proposals honest nodes hold (`Liveness.heldTree`), since
`ChainGrows` names no tree of its own.
-/

@[expose] public section

namespace NewProtocol
namespace Liveness

open History Lists

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}

/-! ## A tree of the proposals honest nodes hold -/

section HeldTree

variable (N : Network cfg C)

/-- Some honest node holds proposal `p` at some point. -/
def HeldProposal (p : Proposal) : Prop :=
  ∃ k, ∃ hk : C.Honest k, ∃ n, ((N.trace k hk).history n).HasProposal cfg p

open Classical in
/-- For each hash, a proposal with that hash some honest node holds, if there is one. -/
noncomputable def heldTree : BlockTable := fun h =>
  if hx : ∃ b, HeldProposal N b ∧ blockHash b = h then some hx.choose else none

theorem heldTree_coherent : TreeCoherent (heldTree N) := by
  intro h b hb
  unfold heldTree at hb
  split at hb
  · next hx => cases hb; exact hx.choose_spec.2
  · cases hb

theorem heldTree_resolves (hcf : CollisionFree) : Resolves cfg (heldTree N) N := by
  intro k hk n b hb
  have hx : ∃ b', HeldProposal N b' ∧ blockHash b' = blockHash b := ⟨b, ⟨k, hk, n, hb⟩, rfl⟩
  unfold heldTree
  rw [dite_eq_left_of_eq_true (eq_true hx)]
  exact congrArg some (hcf _ _ hx.choose_spec.2)

end HeldTree

/-! ## Entering an epoch takes a held commit -/

section Crossing

variable (N : Network cfg C) (hcfg : ConfigCoherent cfg) (hcf : CollisionFree)
include hcfg hcf

/-- `Liveness.crossing_holder`, with the induction counter in the statement. -/
private theorem crossing_holder_aux (hh : cfg.epochHeight ≠ 0) :
    ∀ n (c1 : Cert1), c1.view.toNat ≤ n → Cert1Backed N.trace c1 →
      ∀ e : EpochNumber, cfg.startEpoch.toNat ≤ e.toNat → e.toNat < c1.data.epoch.toNat →
        ∃ j, ∃ hj : C.Honest j, ∃ m q c2, ((N.trace j hj).history m).HasProposal cfg q
          ∧ ((N.trace j hj).history m).HasCert2 c2 ∧ Commits c2 q
          ∧ IsLastBlock q.blockHeader.blockNumber cfg.epochHeight ∧ q.epoch = e := by
  have hres := heldTree_resolves N hcf
  have hcoh := heldTree_coherent N
  have hheights := heightSucceedsParent (heldTree N) N hcfg hcoh hcf hres
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1
    exact absurd (cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) hc1)
      (by show ¬ cfg.anchorView.toNat < c1.view.toNat; omega)
  | succ n ih =>
    intro c1 hle hc1 e he hlt
    rcases cert1_step (heldTree N) N hcfg hres hc1 with
      ⟨p, hview1, hhash, hepc, hwf, htree, hpar, hopen, -⟩ | ⟨pc, hpcb, hpcd, hpcv, -, -⟩
    case inr =>
      -- A re-vote: the same block at an earlier view.
      exact ih pc (by have : pc.view.toNat < c1.view.toNat := hpcv; omega) hpcb e he
        (by rw [hpcd]; exact hlt)
    have hviewlt : p.parentCert.view.toNat < c1.view.toNat := by rw [hview1]; exact hwf.1
    rcases parent_cases N hcfg hpar with ⟨hpar', -⟩ | ⟨hanchor, -⟩
    case inr =>
      -- Right after the anchor the block is in the epoch the run starts in, with
      -- nothing before it.
      exfalso
      have hanct : heldTree N (blockHash cfg.anchorBlock) = some cfg.anchorBlock :=
        anchor_in_tree (heldTree N) N hres hc1
      have hsucc : p.blockHeader.blockNumber = cfg.anchorBlock.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 p cfg.anchorBlock (hhash ▸ htree)
          (by rw [hanchor, hcfg.anchorCertBlock]; exact hanct)
      have hstart : c1.data.epoch = cfg.startEpoch := by
        rw [hepc, hwf.epoch, hsucc, epochOf_after_anchor hcfg]
      rw [hstart] at hlt
      omega
    obtain ⟨pp, -, hpph, hppe, hppwf, hpptree, -, -, hppn, -⟩ :=
      cert1_proposal (heldTree N) N hres hpar'
    have hsucc : p.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
      hheights c1 hc1 p pp (hhash ▸ htree) (hpph ▸ hpptree)
    by_cases hlast : IsLastBlock pp.blockHeader.blockNumber cfg.epochHeight
    · have hent : EntersEpoch cfg p := by
        show IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
        rw [hsucc]; simpa using hlast
      have hstep : pp.epoch + 1 = c1.data.epoch := by
        rw [hepc, hwf.epoch, hppwf.epoch, hsucc, epochOf_succ _ _ hlast.2.1 (isLastBlock_iff.mp hlast).1,
          ite_eq_left hlast]
      have hstepN : pp.epoch.toNat + 1 = c1.data.epoch.toNat := congrArg EpochNumber.toNat hstep
      by_cases heq : pp.epoch = e
      · -- The proposal opens the epoch after `e`: its honest voter held the commit.
        obtain ⟨j, hj, m, q, c2, hq, -, hqh, hc2, -, hc2d⟩ := hopen hent
        have hqpp : q = pp := hcf q pp (by rw [← hqh, hpph])
        subst hqpp
        rcases hc2 with hc2 | hc2a
        case inr =>
          -- The anchor's `Cert2` would make `q` the anchor, but it is a certified block.
          exfalso
          have h2 : cfg.anchorCert.data.blockHash = p.parentCert.data.blockHash := by
            have := congrArg Vote2Data.blockHash hc2d
            rw [hc2a] at this
            exact this
          have hqa : q = cfg.anchorBlock := hcf q _ (by rw [← hqh, ← h2, hcfg.anchorCertBlock])
          have := backed_block_after_anchor N hcfg hcf hpar' hqh
          rw [hqa] at this
          exact Nat.lt_irrefl _ this
        have hc2b := cert2_held_backed N hc2
        have hc2h : c2.data.blockHash = blockHash q := by rw [hc2d, ← hqh]; rfl
        have hqt : heldTree N c2.data.blockHash = some q := by rw [hc2h]; exact hres j hj m q hq
        refine ⟨j, hj, m, q, c2, hq, hc2, ⟨cert2_block_view (heldTree N) N hcfg hres hc2b hqt, ?_⟩,
          hlast, heq⟩
        rw [hc2d]
        show (⟨p.parentCert.data.blockHash, p.parentCert.data.epoch, p.parentCert.data.blockNumber⟩
          : Vote2Data) = _
        rw [hpph, hppe, hppn]
      · exact ih p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar' e he
          (by
            have hne : e.toNat ≠ pp.epoch.toNat := fun h => heq (EpochNumber.ext h).symm
            rw [hppe]; omega)
    · have hsame : c1.data.epoch = p.parentCert.data.epoch := by
        rw [hepc, hppe, hwf.epoch, hppwf.epoch, hsucc]
        by_cases hz : pp.blockHeader.blockNumber.toNat = 0
        · rw [BlockNumber.ext hz]; exact epochOf_one _ hh
        · rw [epochOf_succ _ _ hh hz, ite_eq_right hlast]
      exact ih p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar' e he
        (hsame ▸ hlt)

/--
A certified block of a later epoch than `e` has, behind it, a last block of `e`
that some honest node held with a `Cert2` over it.

Walking back from the certificate, the chain enters each epoch through a
proposal opening it. An honest node voted on that proposal (`cert1_step`), and
to do so it held the parent, the last block of the epoch before, and a `Cert2`
over it (`OpensEpochJustified`).
-/
theorem crossing_holder (hh : cfg.epochHeight ≠ 0) {c1 : Cert1} (hc1 : Cert1Backed N.trace c1)
    (e : EpochNumber) (he1 : cfg.startEpoch.toNat ≤ e.toNat) (he2 : e.toNat < c1.data.epoch.toNat) :
    ∃ j, ∃ hj : C.Honest j, ∃ m q c2, ((N.trace j hj).history m).HasProposal cfg q
      ∧ ((N.trace j hj).history m).HasCert2 c2 ∧ Commits c2 q
      ∧ IsLastBlock q.blockHeader.blockNumber cfg.epochHeight ∧ q.epoch = e :=
  crossing_holder_aux N hcfg hcf hh _ c1 (Nat.le_refl _) hc1 e he1 he2

end Crossing

/-! ## The regime -/

section Regime

variable (N : TimedNetwork cfg leader C)

/--
Grounds for epoch `e` other than a timeout certificate: the epoch the run starts in,
an epoch change into `e`, or a certificate of `e` the node holds.

A timeout certificate names the epoch its signers were in, so it only follows
these.
-/
def SolidGround (cfg : Config) (h : History) (e : EpochNumber) : Prop :=
  e = cfg.startEpoch
    ∨ (∃ c1 c2 p, h.TookEpochChange cfg c1 c2 p ∧ e = c2.data.epoch + 1)
    ∨ ∃ c, h.HasCert1 cfg c ∧ c.data.epoch = epochOf c.data.blockNumber cfg.epochHeight ∧ e = c.data.epoch

theorem SolidGround.ground {h : History} {e : EpochNumber} (hg : SolidGround cfg h e) :
    h.EpochGround cfg e := by
  rcases hg with h1 | h1 | h1
  · exact Or.inl h1
  · exact Or.inr (Or.inl h1)
  · exact Or.inr (Or.inr (Or.inr h1))

theorem solidGround_grows {h1 h2 : History} (hr : ∀ i, h1.Received i → h2.Received i)
    {e : EpochNumber} (he : SolidGround cfg h1 e) : SolidGround cfg h2 e := by
  rcases he with rfl | ⟨c1, c2, p, ⟨hrec, hwf⟩, rfl⟩ | ⟨c, hc, hce, rfl⟩
  · exact Or.inl rfl
  · exact Or.inr (Or.inl ⟨c1, c2, p, ⟨hr _ hrec, hwf⟩, rfl⟩)
  · exact Or.inr (Or.inr ⟨c, hasCert1_grows hr hc, hce, rfl⟩)

/--
From `t0` on, epoch `E` is the latest any honest node ever has grounds for, every
node honest in `E` or later has solid grounds for it, and every other honest node
has retired: it has solid grounds for an epoch after every epoch it is honest in.
-/
structure Stable (GST : Nat) (E : EpochNumber) (t0 : Nat) : Prop where
  /-- No honest node ever has grounds for an epoch after `E`. -/
  bound : ∀ j (hj : C.Honest j) n e, ((N.trace j hj).history n).EpochGround cfg e → e.toNat ≤ E.toNat

  /-- Every node honest in `E` or later has solid grounds for `E` by `t0`. -/
  reach : ∀ j (hj : C.Honest j), C.HonestFrom E j → N.By j hj t0 fun h => SolidGround cfg h E

  /-- Every other honest node is past every epoch it is honest in by `t0`. -/
  retired : ∀ j (hj : C.Honest j), ¬ C.HonestFrom E j →
    N.By j hj t0 fun h => ∃ e', (∀ e, C.honest e j → e.toNat < e'.toNat) ∧ SolidGround cfg h e'

  /-- `t0` is after GST. -/
  gst : GST ≤ t0

variable {N}
variable (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {GST Δ τ : Nat}
  (hs : Synchrony N GST Δ τ) {E : EpochNumber} {t0 : Nat} (hst : Stable N GST E t0)
include hst

/-- From `t0` on, every node honest in `E` or later has solid grounds for `E`. -/
theorem stable_solid (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n)
    {j : PubKey} {hj : C.Honest j} (hjE : C.HonestFrom E j) {n : Nat} (hn : cut N hu j hj t0 ≤ n) :
    SolidGround cfg ((N.trace j hj).history n) E :=
  solidGround_grows (received_grows _ hn)
    (by_cut N hu (fun _ _ hab h => solidGround_grows (received_grows _ hab) h) (hst.reach j hj hjE))

/-- From `t0` on, every node honest in `E` or later is in epoch `E`. -/
theorem stable_inEpoch (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n)
    {j : PubKey} {hj : C.Honest j} (hjE : C.HonestFrom E j) {n : Nat} (hn : cut N hu j hj t0 ≤ n) :
    ((N.trace j hj).history n).InEpoch cfg E :=
  ⟨SolidGround.ground (stable_solid hst hu hjE hn), fun e he => hst.bound j hj n e he⟩

omit hst in
/-- A history with grounds for an epoch after `e` is past `e`. -/
theorem not_notBehind {h : History} {e g : EpochNumber} (hg : h.EpochGround cfg g) (hlt : e.toNat < g.toNat) :
    ¬ NotBehind cfg h e := fun hnb => by
  have hi := inEpoch_epochOfHistory cfg h
  have h1 : (epochOfHistory cfg h).toNat ≤ e.toNat := hnb _ hi
  have h2 : g.toNat ≤ (epochOfHistory cfg h).toNat := hi.2 g hg
  omega

/-- From `t0` on, a node not honest in `E` or later is past every epoch it is honest in. -/
theorem stable_retired (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n)
    {j : PubKey} {hj : C.Honest j} (hjE : ¬ C.HonestFrom E j) {n : Nat} (hn : cut N hu j hj t0 ≤ n)
    {e : EpochNumber} (he : C.honest e j) : ¬ NotBehind cfg ((N.trace j hj).history n) e := by
  obtain ⟨e', hlt, hg⟩ := by_cut N hu
    (fun _ _ hab ⟨e', hlt, hg⟩ => ⟨e', hlt, solidGround_grows (received_grows _ hab) hg⟩)
    (hst.retired j hj hjE)
  exact not_notBehind (solidGround_grows (received_grows _ hn) hg).ground (hlt e he)

/-- The epoch the run starts in is no later than `E`. -/
theorem stable_start : cfg.startEpoch.toNat ≤ E.toNat := by
  obtain ⟨k, -, -, hk⟩ := C.intersect _ _ _ (N.honestQuorum cfg.startEpoch) (N.honestQuorum cfg.startEpoch)
  exact hst.bound k (.of hk) 0 _ (Or.inl rfl)

/-- The anchor's epoch is no later than `E`. -/
theorem stable_anchor : cfg.anchorCert.data.epoch.toNat ≤ E.toNat :=
  Nat.le_trans (anchor_le_start (cfg := cfg)) (stable_start hst)

include hs in
/--
No honest node ever holds the last block of `E` with a `Cert2` over it.

It would send the epoch change, and every honest node would take it and have
grounds for the epoch after `E`.
-/
theorem stable_no_commit {j : PubKey} {hj : C.Honest j} {n : Nat} {q : Block} {c2 : Cert2}
    (hq : ((N.trace j hj).history n).HasProposal cfg q) (hc2 : ((N.trace j hj).history n).HasCert2 c2)
    (hcq : Commits c2 q) (hlast : IsLastBlock q.blockHeader.blockNumber cfg.epochHeight)
    (hqe : q.epoch = E) : False := by
  have hu := hs.timeUnbounded
  cases n with
  | zero =>
    rcases hc2 with h | ⟨_, _, h⟩ <;>
      · obtain ⟨_, hj', -⟩ := (Trace.received_history _).mp h; exact absurd hj' (Nat.not_lt_zero _)
  | succ n =>
    obtain ⟨k', -, -, hk'E⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
    have hk' : C.Honest k' := .of hk'E
    have he : c2.data.epoch = q.epoch := congrArg Vote2Data.epoch hcq.2
    obtain ⟨m, -, c1, htook⟩ := hs.epochChange c2 q hcq hlast j hj n ⟨hq, hc2⟩ k' hk'
      (.of (by rw [he, hqe]; exact hk'E))
    have := hst.bound k' hk' m _ (Or.inr (Or.inl ⟨c1, c2, q, htook, rfl⟩))
    have : (c2.data.epoch + 1).toNat ≤ E.toNat := this
    rw [he, hqe] at this
    exact absurd this (by show ¬ E.toNat + 1 ≤ E.toNat; omega)

include hcfg hcf hs in
/-- No backed `Cert1` is of an epoch after `E`. -/
theorem stable_no_later {c : Cert1} (hc : Cert1Backed N.trace c) : c.data.epoch.toNat ≤ E.toNat := by
  refine Nat.le_of_not_lt fun hlt => ?_
  obtain ⟨k0, -, -, hk0⟩ := C.intersect _ _ _ (N.honestQuorum E) (N.honestQuorum E)
  have hk0 : C.Honest k0 := .of hk0
  have hE1 : cfg.startEpoch.toNat ≤ E.toNat := stable_start hst
  have hh : cfg.epochHeight ≠ 0 := by
    intro hh
    obtain ⟨-, -, -, -, b', -, -, hwf, -, hdata, -, -⟩ := cert1_origin N.toNetwork _ c (Nat.le_refl _) hc
    have : c.data.epoch.toNat = 0 := by
      rw [hdata]; show (b'.epoch).toNat = 0; rw [hwf.epoch]; simp [epochOf_eq, hh]
    omega
  obtain ⟨j, hj, m, q, c2, hq, hc2, hcq, hlast, hqe⟩ :=
    crossing_holder N.toNetwork hcfg hcf hh hc E hE1 hlt
  exact stable_no_commit hs hst hq hc2 hcq hlast hqe

end Regime

end Liveness
end NewProtocol
