module

public import NewProtocolImpl.WitnessKit
public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.FiveNodes

/-!
# A network whose first epoch change splits the nodes

Before GST, the first block's `Cert2` and the epoch change reach `a` and `nw` but
not `b` and `c`. So when the timer for the view after the block fires, `a` and
`nw` are in epoch two and `b` and `c` still in epoch one: the timeout votes name
two epochs, and neither has a quorum. `b` and `c` answer `a`'s re-vote request
too, but `a` does not, being in the next epoch, so no `Cert1` forms from it. The
header for the view reaches `a` only after its timer fired, so `a` proposes no
block there. The one-honest indication brings a second round of votes, still split.

After GST, `b` and `c` take the epoch change within `Δ`. The timer fires again,
`τ` after it last did (`Synchrony.timerFires`), and this time every node names
epoch two, so the timeout certificate forms and epoch two opens behind it. From
there on every epoch changes as in `NewProtocolImpl.EpochWitness`: `a` asks for a
re-vote on an epoch's last block and proposes the next epoch's first block in the
same view, and nobody times out (`direct_after_gst`).

The nodes and committees are `NewProtocolImpl.FiveNodes`'s. Block `u + 1` is the
only block of epoch `u + 1`: the first at view one, the second at view three behind
the timeout certificate for view two, and each later one at the view after the one
before (`bv`). A node receives eight inputs to each block after the first, as in
`EpochWitness` (`phase`), and fourteen in the first epoch:

* the header, the proposal with the node's share (for `nw`, outside the
  committee, the proposal alone), the validity report, the `Cert1`, and the payload
  (for `nw`, a second validity report);
* at `a` and `nw`, the `Cert2` and the epoch change; at `b` and `c`, `a`'s re-vote
  request and a second validity report;
* the timer for the view after the block, the header for that view, which `a` can
  no longer use, and the one-honest indication;
* at `a` and `nw`, the re-vote request and a validity report; at `b` and `c`, the
  `Cert2` and the epoch change;
* the timer again, and the timeout certificate.

Times (`tm`): one unit a step to the timer, which fires `τ = 33` after the nodes
entered the view; GST is then, and the next steps follow one unit apart; the timer
fires again `τ` later, and every step after it one unit after the one before.
`Δ = 4` and `δ = 0`.

`BlockValid` is taken as a hypothesis, being opaque; `decides` also takes
`CollisionFree`.
-/

@[expose] public section

namespace NewProtocolImpl
namespace SplitWitness

open NewProtocol History
open FourNodes (hdr)
open Kit (received received_mono upTo_H upTo_self getElem_H tr_step sent_iff view_inj number_inj lockLE_antisymm)
open FiveNodes (a b c nw d third leader C members_quorum d_faulty quorum_a anchorB certOf cfg cfg_coherent
  lag lag_cases lag_member lag_a epochOf_one_height last_block)

/-! ## Blocks and the schedule -/

/-- The view of block `u + 1`: one for the first, `u + 2` for each later one. -/
def bv : Nat → Nat
  | 0 => 1
  | u + 1 => u + 3

theorem bv_facts (u : Nat) : (u = 0 → bv u = 1) ∧ (1 ≤ u → bv u = u + 2) := by
  cases u with
  | zero => exact ⟨fun _ => rfl, fun h => absurd h (by omega)⟩
  | succ u => exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩

theorem bv_inj {x y : Nat} (h : bv x = bv y) : x = y := by
  have := bv_facts x; have := bv_facts y; omega

/--
Block `u + 1`, at view `bv u`, the only block of epoch `u + 1`, on the one before.
The second opens its epoch behind the timeout certificate for view two.
-/
def blk : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨1⟩, certOf anchorB, none, ⟨0⟩⟩
  | 1 => ⟨hdr 2, ⟨3⟩, ⟨2⟩, certOf (blk 0), some ⟨⟨⟨2⟩, certOf (blk 0)⟩, ⟨2⟩⟩, ⟨0⟩⟩
  | u + 2 => ⟨hdr (u + 3), ⟨u + 4⟩, ⟨u + 3⟩, certOf (blk (u + 1)), none, ⟨0⟩⟩

/-- The timeout certificate for view two, of epoch two, locked on the first block's `Cert1`. -/
def T : TimeoutCert := ⟨⟨⟨2⟩, certOf (blk 0)⟩, ⟨2⟩⟩

/-- The parent of `blk u`. -/
def parentOf : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

/-- The `Cert2` over `blk u`. -/
def C2 (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨bv u⟩⟩

/-- The re-vote request `a` sends in the view after `blk u`, once it can lock on it. -/
def R (u : Nat) : RevoteRequest := ⟨certOf (blk u), ⟨bv u + 1⟩, none⟩

theorem blk_view (u : Nat) : (blk u).viewNumber = ⟨bv u⟩ := by
  match u with
  | 0 => rfl
  | 1 => rfl
  | u + 2 => rfl

theorem blk_number (u : Nat) : (blk u).blockHeader.blockNumber = ⟨u + 1⟩ := by
  match u with
  | 0 => rfl
  | 1 => rfl
  | u + 2 => rfl

theorem blk_epoch (u : Nat) : (blk u).epoch = ⟨u + 1⟩ := by
  match u with
  | 0 => rfl
  | 1 => rfl
  | u + 2 => rfl

theorem blk_header (u : Nat) : (blk u).blockHeader = hdr (u + 1) := by
  match u with
  | 0 => rfl
  | 1 => rfl
  | u + 2 => rfl

theorem blk_parent (u : Nat) : (blk u).parentCert = certOf (parentOf u) := by
  match u with
  | 0 => rfl
  | 1 => rfl
  | u + 2 => rfl

/-- Only the second block carries timeout evidence. -/
theorem blk_evidence (u : Nat) : (blk u).timeoutEvidence = none ∨ (u = 1 ∧ (blk u).timeoutEvidence = some T) := by
  match u with
  | 0 => exact Or.inl rfl
  | 1 => exact Or.inr ⟨rfl, rfl⟩
  | u + 2 => exact Or.inl rfl

theorem cert_view (u : Nat) : (certOf (blk u)).view = ⟨bv u⟩ := blk_view u

theorem cert_number (u : Nat) : (certOf (blk u)).data.blockNumber = ⟨u + 1⟩ := blk_number u

theorem cert_epoch (u : Nat) : (certOf (blk u)).data.epoch = ⟨u + 1⟩ := blk_epoch u

theorem blk_wellFormed (u : Nat) : ProposalWellFormed cfg (blk u) := by
  match u with
  | 0 => exact ⟨by decide, Or.inl ⟨rfl, rfl⟩, rfl, rfl⟩
  | 1 => exact ⟨by decide, Or.inr ⟨T, rfl, rfl⟩, (epochOf_one_height (n := 2) (by omega)).symm, rfl⟩
  | u + 2 =>
    refine ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_, ?_⟩
    · show (blk (u + 1)).viewNumber.toNat < u + 4
      rw [blk_view]; show u + 3 < u + 4; omega
    · show (blk (u + 1)).viewNumber + 1 = ⟨u + 4⟩
      rw [blk_view]; rfl
    · show (⟨u + 3⟩ : EpochNumber) = epochOf ⟨u + 3⟩ 1
      rw [epochOf_one_height (by omega)]
    · show (blk (u + 1)).blockHeader.blockNumber + 1 = ⟨u + 3⟩
      rw [blk_number]; rfl

theorem blk_enters (u : Nat) : EntersEpoch cfg (blk (u + 1)) := by
  show IsLastBlock ((blk (u + 1)).blockHeader.blockNumber - 1) 1
  rw [blk_number]
  exact last_block (n := u + 1) (by omega)

theorem blk_enters_zero : ¬ EntersEpoch cfg (blk 0) := fun h => h.1 rfl

theorem blk_safe (u : Nat) : SafeParent (blk u) := by
  match u with
  | 0 => exact fun _ h => by cases h
  | 1 =>
    intro tc h
    cases h
    exact ⟨rfl, Or.inl (show (1 : Nat) < 2 by omega)⟩
  | u + 2 => exact fun _ h => by cases h

/-- The nodes the first `Cert2` reaches before GST. -/
def Early (k : PubKey) : Prop := k = a ∨ k = nw

instance (k : PubKey) : Decidable (Early k) := inferInstanceAs (Decidable (_ ∨ _))

/--
What the environment hands node `k` in the steps of block `u + 1`. A node outside
the block's committee gets the proposal without a share, and no payload.
-/
def phase (k : PubKey) (u : Nat) : Nat → Input
  | 0 => .headerBuilt ⟨bv u⟩ (blockHash (parentOf u)) (hdr (u + 1))
  | 1 => if lag k u = 0 then .proposal a (blk u) (some ⟨⟨bv u⟩, (blk u).payloadCommit⟩) else .proposal a (blk u) none
  | 2 => .blockValidated ⟨bv u⟩ (blockHash (blk u))
  | 3 => .certificate1 (certOf (blk u))
  | 4 => if lag k u = 0 then .blockReconstructed ⟨bv u⟩ (blk u).payloadCommit
      else .blockValidated ⟨bv u⟩ (blockHash (blk u))
  | 5 => .certificate2 (C2 u)
  | 6 => .epochChange (certOf (blk u)) (C2 u) (blk u)
  | _ => .revote a (R u)

/-- What the environment hands node `k` in the first epoch, step `n`. -/
def first (k : PubKey) : Nat → Input
  | 5 => if Early k then .certificate2 (C2 0) else .revote a (R 0)
  | 6 => if Early k then .epochChange (certOf (blk 0)) (C2 0) (blk 0) else .blockValidated ⟨1⟩ (blockHash (blk 0))
  | 7 => .timeout ⟨2⟩
  | 8 => .headerBuilt ⟨2⟩ (blockHash (blk 0)) (hdr 2)
  | 9 => .timeoutOneHonest ⟨2⟩
  | 10 => if Early k then .revote a (R 0) else .certificate2 (C2 0)
  | 11 => if Early k then .blockValidated ⟨1⟩ (blockHash (blk 0))
      else .epochChange (certOf (blk 0)) (C2 0) (blk 0)
  | 12 => .timeout ⟨2⟩
  | 13 => .timeoutCertificate T
  | n => phase k 0 n

/-- Fourteen steps to the first epoch, then eight to each later block. -/
def input (k : PubKey) (n : Nat) : Input :=
  if n < 14 then first k n else phase k ((n - 6) / 8) ((n - 6) % 8)

/-- Node `k` running the machine on its schedule, and its history after `n` steps. -/
local notation "tr" => Kit.tr cfg leader input

local notation "H" => Kit.H cfg leader input

theorem input_first {k : PubKey} {n : Nat} (h : n < 14) : input k n = first k n := ite_eq_left h

theorem input_at (k : PubKey) (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) : input k (8 * u + r + 6) = phase k u r := by
  unfold input
  rw [ite_eq_right (by omega), show (8 * u + r + 6 - 6) / 8 = u by omega, show (8 * u + r + 6 - 6) % 8 = r by omega]

/-- A first-epoch step `x`, or step `8u + y` of a later block `u + 1`. -/
def at0 (x y u : Nat) : Nat := if u = 0 then x else 8 * u + y

theorem at0_facts (x y u : Nat) : (u = 0 → at0 x y u = x) ∧ (1 ≤ u → at0 x y u = 8 * u + y) :=
  ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

/-- A step of the first epoch, or step `r` of a later block `u + 1`. -/
theorem steps (n : Nat) : n < 14 ∨ ∃ u r, 1 ≤ u ∧ r < 8 ∧ n = 8 * u + r + 6 :=
  if h : n < 14 then Or.inl h else Or.inr ⟨(n - 6) / 8, (n - 6) % 8, by omega, Nat.mod_lt _ (by omega), by omega⟩

theorem r14 (n : Nat) (h : n < 14) :
    n = 0 ∨ n = 1 ∨ n = 2 ∨ n = 3 ∨ n = 4 ∨ n = 5 ∨ n = 6 ∨ n = 7 ∨ n = 8 ∨ n = 9 ∨ n = 10 ∨ n = 11
      ∨ n = 12 ∨ n = 13 := by omega

theorem r8 (r : Nat) (hr : r < 8) :
    r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 ∨ r = 5 ∨ r = 6 ∨ r = 7 := by omega

/-! ## What the nodes receive -/

section Inputs

variable {k : PubKey}

theorem input_cases {n : Nat} {i : Input} (hi : input k n = i) :
    (n < 14 ∧ first k n = i) ∨ ∃ u r, 1 ≤ u ∧ r < 8 ∧ n = 8 * u + r + 6 ∧ phase k u r = i := by
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · exact Or.inl ⟨hn, by rw [← input_first hn]; exact hi⟩
  · exact Or.inr ⟨u, r, hu, hr, rfl, by rw [← input_at k u r hu hr]; exact hi⟩

theorem input_header {n : Nat} {v : ViewNumber} {h : BlockHash} {x : BlockHeader}
    (hi : input k n = .headerBuilt v h x) :
    (n = 8 ∧ v = ⟨2⟩ ∧ h = blockHash (blk 0) ∧ x = hdr 2)
      ∨ ∃ u, n = at0 0 6 u ∧ v = ⟨bv u⟩ ∧ h = blockHash (parentOf u) ∧ x = hdr (u + 1) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals first
      | exact Or.inl ⟨rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
      | exact Or.inr ⟨0, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact Or.inr ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal {n : Nat} {s : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal s p (some vid)) :
    ∃ u, n = at0 1 7 u ∧ lag k u = 0 ∧ s = a ∧ p = blk u ∧ vid = ⟨⟨bv u⟩, (blk u).payloadCommit⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, rfl, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal_none {n : Nat} {s : PubKey} {p : Proposal} (hi : input k n = .proposal s p none) : ∃ u, n = at0 1 7 u ∧ s = a ∧ p = blk u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, rfl, he.1.symm, he.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.symm⟩

/-- Every proposal a node receives is `a`'s block for its round, with a share or without. -/
theorem input_proposal_any {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ u, n = at0 1 7 u ∧ s = a ∧ p = blk u := by
  cases share with
  | some vid => obtain ⟨u, h1, -, h2, h3, -⟩ := input_proposal hi; exact ⟨u, h1, h2, h3⟩
  | none => exact input_proposal_none hi

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    ∃ u, n = at0 3 9 u ∧ x = certOf (blk u) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.symm⟩

/-- A payload arrives at the members of the block's committee only. -/
theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = at0 4 10 u ∧ lag k u = 0 ∧ v = ⟨bv u⟩ ∧ pc = (blk u).payloadCommit := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, rfl, hl, he.1.symm, he.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, hl, he.1.symm, he.2.symm⟩

/-- When the first `Cert2` reaches a node: before GST at `a` and `nw`, after it at `b` and `c`. -/
def c2At (k : PubKey) : Nat := if Early k then 5 else 10

/-- When the first epoch change reaches a node. -/
def ecAt (k : PubKey) : Nat := if Early k then 6 else 11

/-- When the first re-vote request reaches a node. -/
def rvAt (k : PubKey) : Nat := if Early k then 10 else 5

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, n = at0 (c2At k) 11 u ∧ x = C2 u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, by simp [at0, c2At, hE], he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.symm⟩

theorem input_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : input k n = .epochChange c1 c2 p) :
    ∃ u, n = at0 (ecAt k) 12 u ∧ c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, by simp [at0, ecAt, hE], he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hi : input k n = .revote s r) :
    ∃ u, n = at0 (rvAt k) 13 u ∧ s = a ∧ r = R u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r', hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨0, by simp [at0, rvAt, hE], he.1.symm, he.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.symm⟩

/-- The timer fires twice, both times for the view after the first block. -/
theorem input_timeout {n : Nat} {v : ViewNumber} (hi : input k n = .timeout v) :
    (n = 7 ∨ n = 12) ∧ v = ⟨2⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals first
      | exact ⟨Or.inl rfl, he.symm⟩
      | exact ⟨Or.inr rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he

theorem input_tc {n : Nat} {tc : TimeoutCert} (hi : input k n = .timeoutCertificate tc) : n = 13 ∧ tc = T := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he

theorem input_oneHonest {n : Nat} {v : ViewNumber} (hi : input k n = .timeoutOneHonest v) : n = 9 ∧ v = ⟨2⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · by_cases hE : Early k <;> rcases lag_cases k 0 with hl | hl <;>
      rcases r14 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hE, hl] at he
    all_goals exact ⟨rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

/-- The two kinds of node, and when the first `Cert2`, epoch change and re-vote request reach them. -/
theorem classes (k : PubKey) : (Early k ∧ c2At k = 5 ∧ ecAt k = 6 ∧ rvAt k = 10)
    ∨ (¬ Early k ∧ c2At k = 10 ∧ ecAt k = 11 ∧ rvAt k = 5) := by
  by_cases hE : Early k
  · exact Or.inl ⟨hE, by simp [c2At, hE], by simp [ecAt, hE], by simp [rvAt, hE]⟩
  · exact Or.inr ⟨hE, by simp [c2At, hE], by simp [ecAt, hE], by simp [rvAt, hE]⟩

theorem ecAt_a : ecAt a = 6 := by simp [ecAt, show Early a from Or.inl rfl]

theorem ecAt_b : ecAt b = 11 := by simp [ecAt, show ¬ Early b by decide]

/-- The node outside the first committee is `nw`, which the first `Cert2` reaches before GST. -/
theorem lag_zero (k : PubKey) : lag k 0 = 0 ∨ (lag k 0 = 2 ∧ ecAt k = 6) := by
  by_cases h : k = third ⟨0 + 2⟩
  · refine Or.inr ⟨by simp [lag, h], ?_⟩
    have : k = nw := by rw [h]; rfl
    simp [ecAt, Early, this]
  · exact Or.inl (by simp [lag, h])

theorem recv_first {n : Nat} (j : Nat) (hj : j < 14) (h : j < n) : (H k n).Received (first k j) :=
  received.mpr ⟨j, h, input_first hj⟩

theorem recv_later {n : Nat} (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) (h : 8 * u + r + 6 < n) :
    (H k n).Received (phase k u r) :=
  received.mpr ⟨_, h, input_at k u r hu hr⟩

/-- What a node received at step `r` of block `u + 1`, whichever block. -/
theorem recv_phase {n : Nat} (u r : Nat) (hr : r ≤ 4) (h : at0 r (r + 6) u < n) :
    (H k n).Received (phase k u r) := by
  cases u with
  | zero =>
    rw [(at0_facts _ _ _).1 rfl] at h
    have := recv_first (k := k) r (by omega) h
    rcases (show r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 by omega) with rfl | rfl | rfl | rfl | rfl <;> exact this
  | succ u => exact recv_later (u + 1) r (by omega) (by omega) (by rw [(at0_facts _ _ _).2 (by omega)] at h; omega)

/-- The first `Cert2`, at whichever step reaches the node, and every later one. -/
theorem recv_c2 {m : Nat} (u : Nat) (h : at0 (c2At k) 11 u < m) : (H k m).Received (.certificate2 (C2 u)) := by
  cases u with
  | zero =>
    rcases classes k with ⟨hE, h5, -⟩ | ⟨hE, h10, -⟩ <;> rw [(at0_facts _ _ _).1 rfl] at h
    · have := recv_first (k := k) 5 (by omega) (by omega : 5 < m); simp only [first, hE, ite_true] at this
      exact this
    · have := recv_first (k := k) 10 (by omega) (by omega : 10 < m); simp only [first, hE, ite_false] at this
      exact this
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)] at h
    exact recv_later (u + 1) 5 (by omega) (by omega) (by omega)

/-- The first re-vote request, at whichever step reaches the node, and every later one. -/
theorem recv_revote {m : Nat} (u : Nat) (h : at0 (rvAt k) 13 u < m) : (H k m).Received (.revote a (R u)) := by
  cases u with
  | zero =>
    rcases classes k with ⟨hE, -, -, h10⟩ | ⟨hE, -, -, h5⟩ <;> rw [(at0_facts _ _ _).1 rfl] at h
    · have := recv_first (k := k) 10 (by omega) (by omega : 10 < m); simp only [first, hE, ite_true] at this
      exact this
    · have := recv_first (k := k) 5 (by omega) (by omega : 5 < m); simp only [first, hE, ite_false] at this
      exact this
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)] at h
    exact recv_later (u + 1) 7 (by omega) (by omega) (by omega)

theorem hasProposal {n : Nat} {x : Block} (hb : (H k n).HasProposal cfg x) :
    x = anchorB ∨ ∃ u, x = blk u ∧ at0 1 7 u < n := by
  rcases hb with rfl | ⟨s, share, hr⟩ | ⟨c1, c2, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    cases share with
    | some vid =>
      obtain ⟨u, rfl, -, -, rfl, -⟩ := input_proposal hji
      exact Or.inr ⟨u, rfl, hj⟩
    | none =>
      obtain ⟨u, rfl, -, rfl⟩ := input_proposal_none hji
      exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, -, -, rfl⟩ := input_epochChange hji
    have := at0_facts (ecAt k) 12 u
    have := at0_facts 1 7 u
    have := classes k
    exact Or.inr ⟨u, rfl, by omega⟩

/-- Every node holds a proposal from the step after it arrives: with a share, or without. -/
theorem hasProposal_of {n : Nat} (u : Nat) (h : at0 1 7 u < n) : (H k n).HasProposal cfg (blk u) := by
  have hr := recv_phase (k := k) u 1 (by omega) h
  simp only [phase] at hr
  rcases lag_cases k u with hl | hl
  · rw [hl, ite_eq_left rfl] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)
  · rw [hl, ite_eq_right (by decide)] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)

theorem hasParent_of {n : Nat} (u : Nat) (h : at0 0 6 u < n) : (H k n).HasProposal cfg (parentOf u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u =>
    refine hasProposal_of u ?_
    have := at0_facts 1 7 u; have := at0_facts 0 6 (u + 1); omega

/-- Which held proposal a height names. -/
theorem block_of_number {n x : Nat} {y : Block} (hb : (H k n).HasProposal cfg y)
    (hv : y.blockHeader.blockNumber = ⟨x + 1⟩) : y = blk x ∧ at0 1 7 x < n := by
  rcases hasProposal hb with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [blk_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

theorem hasCert1 {n : Nat} {x : Cert1} (hc : (H k n).HasCert1 cfg x) :
    x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ at0 3 9 u < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert1 hji
    exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl, -, -⟩ := input_epochChange hji
    have := at0_facts (ecAt k) 12 u
    have := at0_facts 3 9 u
    have := classes k
    exact Or.inr ⟨u, rfl, by omega⟩

theorem hasCert1_of {n : Nat} (u : Nat) (h : at0 3 9 u < n) : (H k n).HasCert1 cfg (certOf (blk u)) :=
  Or.inr (Or.inl (recv_phase u 3 (by omega) h))

/-- Which held certificate a height names. -/
theorem cert_of_number {n x : Nat} {y : Cert1} (hc : (H k n).HasCert1 cfg y)
    (hv : y.data.blockNumber = ⟨x + 1⟩) : y = certOf (blk x) ∧ at0 3 9 x < n := by
  rcases hasCert1 hc with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [cert_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

theorem hasCert2 {n : Nat} {x : Cert2} (hc : (H k n).HasCert2 x) : ∃ u, x = C2 u ∧ at0 (c2At k) 11 u < n := by
  rcases hc with hr | ⟨c1, p, hr⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert2 hji
    exact ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, -, rfl, -⟩ := input_epochChange hji
    have := at0_facts (ecAt k) 12 u
    have := at0_facts (c2At k) 11 u
    have := classes k
    exact ⟨u, rfl, by omega⟩

theorem hasCert2_of {n : Nat} (u : Nat) (h : at0 (c2At k) 11 u < n) : (H k n).HasCert2 (C2 u) :=
  Or.inl (recv_c2 u h)

theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis
      ∨ ∃ u, v = ⟨bv u⟩ ∧ pc = (blk u).payloadCommit ∧ lag k u = 0 ∧ at0 4 10 u < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, hl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hl, hj⟩

theorem payload_of {n : Nat} (u : Nat) (hl : lag k u = 0) (h : at0 4 10 u < n) :
    (H k n).HasPayload cfg (blk u).viewNumber (blk u).payloadCommit := by
  have hr := recv_phase (k := k) u 4 (by omega) h
  simp only [phase, hl, ite_eq_left] at hr
  rw [blk_view]; exact Or.inr hr

theorem received_tc {n : Nat} {tc : TimeoutCert} (hr : (H k n).Received (.timeoutCertificate tc)) :
    tc = T ∧ 13 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨rfl, rfl⟩ := input_tc hji
  exact ⟨rfl, hj⟩

theorem received_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hr : (H k n).Received (.revote s r)) :
    ∃ u, s = a ∧ r = R u ∧ at0 (rvAt k) 13 u < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨u, rfl, rfl, rfl⟩ := input_revote hji
  exact ⟨u, rfl, rfl, hj⟩

theorem epochChange_wellFormed (u : Nat) : EpochChangeWellFormed cfg (certOf (blk u)) (C2 u) (blk u) := by
  refine ⟨by rw [blk_view]; exact Nat.le_refl _, rfl, rfl, rfl, blk_wellFormed u, ?_⟩
  rw [blk_number]; exact last_block (by omega)

theorem tookEpochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (ht : (H k n).TookEpochChange cfg c1 c2 p) :
    ∃ u, c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u ∧ at0 (ecAt k) 12 u < n := by
  obtain ⟨j, hj, hji⟩ := received.mp ht.1
  obtain ⟨u, rfl, rfl, rfl, rfl⟩ := input_epochChange hji
  exact ⟨u, rfl, rfl, rfl, hj⟩

theorem tookEpochChange_of {n : Nat} (u : Nat) (h : at0 (ecAt k) 12 u < n) :
    (H k n).TookEpochChange cfg (certOf (blk u)) (C2 u) (blk u) := by
  refine ⟨?_, epochChange_wellFormed u⟩
  cases u with
  | zero =>
    rcases classes k with ⟨hE, -, h6, -⟩ | ⟨hE, -, h11, -⟩ <;> rw [(at0_facts _ _ _).1 rfl] at h
    · have := recv_first (k := k) 6 (by omega) (by omega : 6 < n)
      simp only [first, hE, ite_true] at this
      exact this
    · have := recv_first (k := k) 11 (by omega) (by omega : 11 < n)
      simp only [first, hE, ite_false] at this
      exact this
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)] at h
    exact recv_later (u + 1) 6 (by omega) (by omega) (by omega)

/--
What a node can lock on after `n` steps: genesis, and each block from the step
after its payload, or, outside the block's committee, after the epoch change.
-/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x
      ↔ x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ at0 (4 + lag k 0) (10 + lag k u) u < n := by
  constructor
  · rintro (rfl | ⟨hc, y, hy, ⟨-, hcd⟩, hp⟩ | ⟨c2, p, ht⟩)
    · exact Or.inl rfl
    · rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩
      · exact Or.inl rfl
      · have hyn : y.blockHeader.blockNumber = ⟨u + 1⟩ := by
          have := congrArg Vote1Data.blockNumber hcd
          rw [← cert_number u]; exact this.symm
        obtain ⟨rfl, -⟩ := block_of_number hy hyn
        rcases hasPayload hp with hg | ⟨u', hv, -, hl, hlt⟩
        · rw [blk_view] at hg; have := bv_facts u; exact absurd (view_inj hg) (by omega)
        · rw [blk_view] at hv
          obtain rfl : u = u' := bv_inj (view_inj hv)
          refine Or.inr ⟨u, rfl, ?_⟩
          have := at0_facts 4 10 u
          have := at0_facts (4 + lag k 0) (10 + lag k u) u
          cases u with
          | zero => omega
          | succ u => omega
    · obtain ⟨u, rfl, -, -, hlt⟩ := tookEpochChange ht
      refine Or.inr ⟨u, rfl, ?_⟩
      have := at0_facts (ecAt k) 12 u
      have := at0_facts (4 + lag k 0) (10 + lag k u) u
      have := lag_cases k u
      have := lag_zero k
      have := classes k
      cases u with
      | zero => omega
      | succ u => omega
  · rintro (rfl | ⟨u, rfl, hlt⟩)
    · exact Or.inl rfl
    · have hf := at0_facts (4 + lag k 0) (10 + lag k u) u
      rcases lag_cases k u with hl | hl
      · refine Or.inr (Or.inl ⟨hasCert1_of u ?_, blk u, hasProposal_of u ?_, ⟨Nat.le_refl _, rfl⟩,
          payload_of u hl ?_⟩) <;>
        · have := at0_facts 3 9 u; have := at0_facts 1 7 u; have := at0_facts 4 10 u
          cases u with
          | zero => omega
          | succ u => omega
      · refine Or.inr (Or.inr ⟨_, _, tookEpochChange_of u ?_⟩)
        have := at0_facts (ecAt k) 12 u
        cases u with
        | zero =>
          rcases lag_zero k with h0 | ⟨h0, h6⟩
          · rw [h0] at hl; cases hl
          · omega
        | succ u => omega

/-! ### The view, the epoch and the lock after `n` steps -/

/--
The view every node is in after `n` steps: view two from the first `Cert1` until
the timeout certificate, and from then on as in `EpochWitness`, one view later.
-/
def vAt (n : Nat) : Nat :=
  if n < 14 then (if 4 ≤ n then 2 else 1) else if 4 ≤ (n - 6) % 8 then (n - 6) / 8 + 3 else (n - 6) / 8 + 2

theorem vAt_first {n : Nat} (h : n < 14) : (n < 4 → vAt n = 1) ∧ (4 ≤ n → vAt n = 2) := by
  unfold vAt
  rw [ite_eq_left h]
  exact ⟨fun h4 => ite_eq_right (by omega), fun h4 => ite_eq_left h4⟩

theorem vAt_later (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) :
    (r < 4 → vAt (8 * u + r + 6) = u + 2) ∧ (4 ≤ r → vAt (8 * u + r + 6) = u + 3) := by
  unfold vAt
  rw [ite_eq_right (by omega), show (8 * u + r + 6 - 6) % 8 = r by omega, show (8 * u + r + 6 - 6) / 8 = u by omega]
  exact ⟨fun h => ite_eq_right (by omega), fun h => ite_eq_left h⟩

/-- `vAt n` against the steps a ground arrives at, as facts `omega` reads. -/
theorem vAt_ge (n : Nat) : 1 ≤ vAt n ∧ (∀ u, at0 3 9 u < n → bv u + 1 ≤ vAt n) ∧ (13 < n → 3 ≤ vAt n) := by
  rcases steps n with hn | ⟨u', r, hu', hr, rfl⟩
  · have := vAt_first hn
    refine ⟨by omega, fun u hu => ?_, fun h => by omega⟩
    have := at0_facts 3 9 u; have := bv_facts u
    omega
  · have := vAt_later u' r hu' hr
    refine ⟨by omega, fun u hu => ?_, fun h => by omega⟩
    have := at0_facts 3 9 u; have := bv_facts u
    omega

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ vAt n := by
  obtain ⟨h1, hc, ht⟩ := vAt_ge n
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, hte, rfl⟩
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ _; omega
    · show (certOf (blk u)).view.toNat + 1 ≤ _
      rw [cert_view]; exact hc u hlt
  · obtain ⟨rfl, hlt⟩ := received_tc htc
    exact ht hlt
  · obtain ⟨u, -, rfl, -, hlt⟩ := tookEpochChange hte
    show bv u + 1 ≤ _
    refine hc u ?_
    have := at0_facts (ecAt k) 12 u; have := at0_facts 3 9 u; have := classes k
    omega

theorem inView (n : Nat) : (H k n).InView cfg ⟨vAt n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · obtain ⟨h1, h4⟩ := vAt_first hn
    by_cases r4 : 4 ≤ n
    · rw [h4 r4]
      exact Or.inl ⟨certOf (blk 0), hasCert1_of 0 (by rw [(at0_facts _ _ _).1 rfl]; omega), rfl⟩
    · rw [h1 (by omega)]; exact Or.inl ⟨certOf anchorB, Or.inl rfl, rfl⟩
  · obtain ⟨h0, h4⟩ := vAt_later u r hu hr
    by_cases r4 : 4 ≤ r
    · rw [h4 r4]
      exact Or.inl ⟨certOf (blk u), hasCert1_of u (by rw [(at0_facts _ _ _).2 hu]; omega),
        by rw [cert_view, (bv_facts u).2 hu]; rfl⟩
    · rw [h0 (by omega)]
      obtain ⟨u', rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
      cases u' with
      | zero => exact Or.inr (Or.inl ⟨T, recv_first 13 (by omega) (by omega), rfl⟩)
      | succ u' =>
        exact Or.inl ⟨certOf (blk (u' + 1)),
          hasCert1_of (u' + 1) (by rw [(at0_facts _ _ _).2 (by omega)]; omega), by rw [cert_view]; rfl⟩

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨vAt n⟩ :=
  Kit.inView_unique hv (inView n)

theorem viewOf_eq (n : Nat) : viewOf cfg (H k n) = ⟨vAt n⟩ := viewOf_of_inView (inView n)

/-- The epoch node `k` is in after `n` steps: the next one from each epoch change on. -/
def eAt (k : PubKey) (n : Nat) : Nat :=
  if n < 14 then (if ecAt k < n then 2 else 1) else if 7 ≤ (n - 6) % 8 then (n - 6) / 8 + 2 else (n - 6) / 8 + 1

theorem eAt_first {n : Nat} (h : n < 14) : (ecAt k < n → eAt k n = 2) ∧ (n ≤ ecAt k → eAt k n = 1) := by
  unfold eAt
  rw [ite_eq_left h]
  exact ⟨fun h1 => ite_eq_left h1, fun h1 => ite_eq_right (by omega)⟩

theorem eAt_later (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) :
    (7 ≤ r → eAt k (8 * u + r + 6) = u + 2) ∧ (r < 7 → eAt k (8 * u + r + 6) = u + 1) := by
  unfold eAt
  rw [ite_eq_right (by omega), show (8 * u + r + 6 - 6) % 8 = r by omega, show (8 * u + r + 6 - 6) / 8 = u by omega]
  exact ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

/-- `eAt` against the steps a ground arrives at, as facts `omega` reads. -/
theorem eAt_ge (n : Nat) : 1 ≤ eAt k n ∧ (∀ u, at0 (ecAt k) 12 u < n → u + 2 ≤ eAt k n)
    ∧ (13 < n → 2 ≤ eAt k n) ∧ (∀ u, at0 3 9 u < n → u + 1 ≤ eAt k n) := by
  have hcl := classes k
  rcases steps n with hn | ⟨u', r, hu', hr, rfl⟩
  · have := eAt_first (k := k) hn
    refine ⟨by omega, fun u hu => ?_, fun h => by omega, fun u hu => ?_⟩ <;> have := at0_facts (ecAt k) 12 u <;>
      have := at0_facts 3 9 u <;> omega
  · have := eAt_later (k := k) u' r hu' hr
    refine ⟨by omega, fun u hu => ?_, fun h => by omega, fun u hu => ?_⟩ <;> have := at0_facts (ecAt k) 12 u <;>
      have := at0_facts 3 9 u <;> omega

theorem epochGround {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e.toNat ≤ eAt k n := by
  obtain ⟨h1, hec, htc, hc⟩ := eAt_ge (k := k) n
  rcases he with rfl | ⟨c1, c2, p, ht, rfl⟩ | ⟨tc, htc', rfl⟩ | ⟨x, hx, -, rfl⟩
  · exact h1
  · obtain ⟨u, -, rfl, -, hlt⟩ := tookEpochChange ht
    show (certOf (blk u)).data.epoch.toNat + 1 ≤ _
    rw [cert_epoch]; exact hec u hlt
  · obtain ⟨rfl, hlt⟩ := received_tc htc'
    exact htc hlt
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · exact h1
    · rw [cert_epoch]; exact hc u hlt

theorem c2_epoch (u : Nat) : (⟨u + 2⟩ : EpochNumber) = (C2 u).data.epoch + 1 := by
  show _ = (certOf (blk u)).data.epoch + 1
  rw [cert_epoch]; rfl

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨eAt k n⟩ := by
  refine ⟨?_, fun e he => epochGround he⟩
  have hcl := classes k
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · obtain ⟨h2, h1⟩ := eAt_first (k := k) hn
    by_cases hec : ecAt k < n
    · rw [h2 hec]
      exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of 0 (by rw [(at0_facts _ _ _).1 rfl]; exact hec), c2_epoch 0⟩)
    · rw [h1 (by omega)]; exact Or.inl rfl
  · obtain ⟨h7, h0⟩ := eAt_later (k := k) u r hu hr
    by_cases r7 : 7 ≤ r
    · rw [h7 r7]
      exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u (by rw [(at0_facts _ _ _).2 hu]; omega), c2_epoch u⟩)
    · rw [h0 (by omega)]
      obtain ⟨u', rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
      refine Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u' ?_, c2_epoch u'⟩)
      have := at0_facts (ecAt k) 12 u'
      omega

theorem inEpoch_eq {n : Nat} {e : EpochNumber} (he : (H k n).InEpoch cfg e) : e = ⟨eAt k n⟩ :=
  Liveness.inEpoch_unique he (inEpoch n)

theorem epochOf_eq (n : Nat) : epochOfHistory cfg (H k n) = ⟨eAt k n⟩ :=
  inEpoch_eq (inEpoch_epochOfHistory cfg _)

theorem notBehind {n : Nat} {e : EpochNumber} (h : eAt k n ≤ e.toNat) : NotBehind cfg (H k n) e :=
  Kit.notBehind_of (inEpoch n) h

theorem notBehind_le {n : Nat} {e : EpochNumber} (h : NotBehind cfg (H k n) e) : eAt k n ≤ e.toNat :=
  Kit.le_of_notBehind (inEpoch n) h

/--
The lock after `n` steps: each block from the step after its payload, outside its
committee from the epoch change.
-/
def lockAt (k : PubKey) (n : Nat) : Cert1 :=
  if n < 14 then (if 4 + lag k 0 < n then certOf (blk 0) else certOf anchorB)
  else if 5 + lag k ((n - 6) / 8) ≤ (n - 6) % 8 then certOf (blk ((n - 6) / 8))
  else certOf (blk ((n - 6) / 8 - 1))

theorem lockAt_later (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) :
    (5 + lag k u ≤ r → lockAt k (8 * u + r + 6) = certOf (blk u))
      ∧ (r < 5 + lag k u → lockAt k (8 * u + r + 6) = certOf (blk (u - 1))) := by
  unfold lockAt
  rw [ite_eq_right (by omega), show (8 * u + r + 6 - 6) / 8 = u by omega, show (8 * u + r + 6 - 6) % 8 = r by omega]
  exact ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

/-- From the first timer until the timeout certificate, every node is locked on the first block. -/
theorem lockAt_first {n : Nat} (h1 : 7 ≤ n) (h2 : n < 14) : lockAt k n = certOf (blk 0) := by
  unfold lockAt
  rw [ite_eq_left h2]
  exact ite_eq_left (by have := lag_cases k 0; omega)

theorem lockLE_blk {j m : Nat} (h : j ≤ m) : LockLE (certOf (blk j)) (certOf (blk m)) := by
  rw [LockLE, cert_epoch, cert_epoch, cert_view, cert_view]
  by_cases hjm : j = m
  · subst hjm; exact Or.inr ⟨rfl, Nat.le_refl _⟩
  · exact Or.inl (show j + 1 < m + 1 by omega)

theorem lockLE_anchor (m : Nat) : LockLE (certOf anchorB) (certOf (blk m)) := by
  rw [LockLE, cert_epoch, cert_view]
  cases m with
  | zero => exact Or.inr ⟨rfl, Nat.zero_le _⟩
  | succ m => exact Or.inl (show 1 < m + 1 + 1 by omega)

/-- Two certificates a node can lock on, at the same view, are the same. -/
theorem lockable_ext {n : Nat} {x y : Cert1} (hx : (H k n).Lockable cfg x) (hy : (H k n).Lockable cfg y)
    (hv : x.view = y.view) : x = y := by
  rcases lockable_iff.mp hx with rfl | ⟨j, rfl, -⟩ <;> rcases lockable_iff.mp hy with rfl | ⟨m, rfl, -⟩
  · rfl
  · rw [cert_view] at hv; have := bv_facts m; exact absurd (view_inj hv) (by omega)
  · rw [cert_view] at hv; have := bv_facts j; exact absurd (view_inj hv) (by omega)
  · rw [cert_view, cert_view] at hv
    obtain rfl : j = m := bv_inj (view_inj hv)
    rfl

theorem lockedOn_lockAt (n : Nat) : (H k n).LockedOn cfg (lockAt k n) := by
  have hl0 := lag_cases k 0
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · unfold lockAt
    rw [ite_eq_left hn]
    by_cases hc : 4 + lag k 0 < n
    · rw [ite_eq_left hc]
      refine ⟨lockable_iff.mpr (Or.inr ⟨0, rfl, by rw [(at0_facts _ _ _).1 rfl]; exact hc⟩), fun x hx => ?_⟩
      rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hlt⟩
      · exact lockLE_anchor 0
      · have := at0_facts (4 + lag k 0) (10 + lag k j) j
        exact lockLE_blk (by omega)
    · rw [ite_eq_right hc]
      refine ⟨lockable_iff.mpr (Or.inl rfl), fun x hx => ?_⟩
      rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hlt⟩
      · exact Or.inr ⟨rfl, Nat.le_refl _⟩
      · have := at0_facts (4 + lag k 0) (10 + lag k j) j
        exact absurd hlt (by omega)
  · obtain ⟨hhi, hlo⟩ := lockAt_later (k := k) u r hu hr
    have hlu := lag_cases k u
    by_cases hc : 5 + lag k u ≤ r
    · rw [hhi hc]
      refine ⟨lockable_iff.mpr (Or.inr ⟨u, rfl, by rw [(at0_facts _ _ _).2 hu]; omega⟩), fun x hx => ?_⟩
      rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hlt⟩
      · exact lockLE_anchor u
      · have := at0_facts (4 + lag k 0) (10 + lag k j) j
        exact lockLE_blk (by have := lag_cases k j; omega)
    · rw [hlo (by omega)]
      refine ⟨lockable_iff.mpr (Or.inr ⟨u - 1, rfl, ?_⟩), fun x hx => ?_⟩
      · have := at0_facts (4 + lag k 0) (10 + lag k (u - 1)) (u - 1)
        have := lag_cases k (u - 1)
        omega
      · rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hlt⟩
        · exact lockLE_anchor _
        · have := at0_facts (4 + lag k 0) (10 + lag k j) j
          have hj : j < u := by
            by_cases hju : j = u
            · subst hju; omega
            · have := lag_cases k j; omega
          exact lockLE_blk (by omega)

theorem lockedOn_eq {n : Nat} {L : Cert1} (hL : (H k n).LockedOn cfg L) : L = lockAt k n := by
  have h0 := lockedOn_lockAt (k := k) n
  exact lockable_ext hL.1 h0.1 (lockLE_antisymm (h0.2 _ hL.1) (hL.2 _ h0.1)).2

theorem lockOf_eq (n : Nat) : lockOf cfg (H k n) = lockAt k n := lockedOn_eq (lockOf_lockedOn _)

/-- The view and epoch every node is in from the header of block `u + 1` until its `Cert1`. -/
theorem at_block (k : PubKey) (u r : Nat) (h3 : r ≤ 3) :
    vAt (at0 0 6 u + r) = bv u ∧ eAt k (at0 0 6 u + r) = u + 1 := by
  have hcl := classes k
  cases u with
  | zero =>
    rw [(at0_facts _ _ _).1 rfl, show 0 + r = r by omega]
    exact ⟨(vAt_first (by omega)).1 (by omega), (eAt_first (by omega)).2 (by omega)⟩
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega), show 8 * (u + 1) + 6 + r = 8 * (u + 1) + r + 6 by omega]
    exact ⟨(vAt_later (u + 1) r (by omega) (by omega)).1 (by omega),
      (eAt_later (u + 1) r (by omega) (by omega)).2 (by omega)⟩

/-- The view and epoch `a` is in when it can lock on block `u + 1`. -/
theorem at_lock (u : Nat) : vAt (at0 4 10 u + 1) = bv u + 1 ∧ eAt a (at0 4 10 u + 1) = u + 1 := by
  cases u with
  | zero =>
    rw [(at0_facts _ _ _).1 rfl]
    exact ⟨(vAt_first (by omega)).2 (by omega), (eAt_first (by omega)).2 (by rw [ecAt_a]; omega)⟩
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega), show 8 * (u + 1) + 10 + 1 = 8 * (u + 1) + 5 + 6 by omega]
    exact ⟨(vAt_later (u + 1) 5 (by omega) (by omega)).2 (by omega),
      (eAt_later (u + 1) 5 (by omega) (by omega)).2 (by omega)⟩

end Holds

/-! ## What the honest nodes send -/

section Sends

theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H k (n + 1)) o :=
  Kit.settled input k n o

/--
A node's timeout votes: three, all for the view after the first block, at the two
timers and the one-honest indication, naming the epoch it is in then.
-/
theorem sent_timeout {k : PubKey} {j : Nat} {vote : TimeoutVote}
    (hx : Output.send (.timeoutVote vote) ∈ (tr k j).output) :
    (j = 7 ∨ j = 9 ∨ j = 12) ∧ vote = ⟨⟨⟨eAt k j⟩, certOf (blk 0)⟩, ⟨2⟩, k⟩ := by
  rw [tr_step] at hx
  rcases step_mem hx with hx | ⟨o, out', -, -, ha⟩
  · obtain ⟨v, hv, (⟨hi, -⟩ | ⟨hi, -⟩)⟩ := mem_timeoutAnswer hx
    · simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      obtain ⟨hj, rfl⟩ := input_timeout hi
      rw [hv, epochOf_eq, lockOf_eq, lockAt_first (by omega) (by omega)]
      exact ⟨by omega, rfl⟩
    · simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      obtain ⟨rfl, rfl⟩ := input_oneHonest hi
      rw [hv, epochOf_eq, lockOf_eq, lockAt_first (by omega) (by omega)]
      exact ⟨by omega, rfl⟩
  · exact absurd ha act_no_timeoutVote

/-- At the timer, a node times out the view it is in, naming its epoch and lock. -/
theorem times_out (k : PubKey) {j : Nat} {v : ViewNumber} (hi : input k j = .timeout v) (hv : vAt j = v.toNat) :
    Output.send (.timeoutVote ⟨⟨⟨eAt k j⟩, lockAt k j⟩, v, k⟩) ∈ (tr k j).output := by
  rw [tr_step, step_output]
  apply discharge_sup
  rw [hi]
  simp only [timeoutAnswer, viewOf_eq, epochOf_eq, lockOf_eq]
  rw [ite_eq_left (by rw [hv])]
  exact List.mem_singleton_self _

/-- The first timer's vote. -/
theorem times_out_first (k : PubKey) :
    Output.send (.timeoutVote ⟨⟨⟨eAt k 7⟩, certOf (blk 0)⟩, ⟨2⟩, k⟩) ∈ (tr k 7).output := by
  have h := times_out k (j := 7) (v := ⟨2⟩) (by rw [input_first (by omega)]; rfl)
    ((vAt_first (by omega)).2 (by omega))
  rwa [lockAt_first (by omega) (by omega)] at h

theorem timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h : (H k n).TimedOut v) : v.toNat ≤ 2 ∧ 7 < n := by
  obtain ⟨vote, hs, hle⟩ := h
  obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
  obtain ⟨hj', rfl⟩ := sent_timeout hjv
  exact ⟨hle, by omega⟩

/-- Nothing times out a view before the first timer, nor any view after the second. -/
theorem not_timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h : (H k n).TimedOut v)
    (h0 : v.toNat ≤ 2 → n ≤ 7) : False := by
  obtain ⟨h1, h2⟩ := timedOut h
  exact absurd (h0 h1) (by omega)

variable (hv : ∀ b, BlockValid b)

include hv in
theorem protocol (k : PubKey) (n : Nat) : ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H k n) :=
  Kit.protocol input hv k n

/-- What a justified proposal or request names, the node holds. -/
theorem justified_held {k : PubKey} {n : Nat} {x : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg (H k n) x ev) :
    (H k n).HasCert1 cfg x ∧ ∀ tc, ev = some tc → tc = T ∧ 13 < n := by
  refine ⟨hasCert1_of_certJustified hj, fun tc hte => ?_⟩
  subst hte
  obtain ⟨-, m, hr, -⟩ := hj
  rw [upTo_H] at hr
  obtain ⟨rfl, hy⟩ := received_tc hr
  exact ⟨rfl, by omega⟩

include hv in
/-- A node sends only `a`'s re-vote requests, one per block, once it can lock on the block. -/
theorem sent_revote {k : PubKey} {j : Nat} {r : RevoteRequest} (hr : Output.send (.revote r) ∈ (tr k j).output) :
    k = a ∧ ∃ x, r = R x ∧ at0 4 10 x ≤ j := by
  have hj := (protocol hv k (j + 1)).revoteJustified j r ⟨_, (getElem_H j), hr⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨hc, hev⟩ := justified_held hj.justified
  obtain ⟨hlt, hnext, hlast⟩ := hj.wellFormed
  obtain ⟨x, hx⟩ : ∃ x, r.cert = certOf (blk x) := by
    rcases hasCert1 hc with he | ⟨x, he, -⟩
    · rw [he] at hlast; exact absurd hlast.1 (by decide)
    · exact ⟨x, he⟩
  have hlx : bv x < r.view.toNat := by
    have : (certOf (blk x)).view.toNat < r.view.toNat := by rw [← hx]; exact hlt
    rwa [cert_view] at this
  -- Timeout evidence would be of the second epoch, for a view no later than the second block's.
  obtain ⟨hnone, hview⟩ : r.timeoutEvidence = none ∧ r.view = ⟨bv x + 1⟩ := by
    rcases hnext with ⟨hn0, hn⟩ | ⟨tc, hte, htv⟩
    · exact ⟨hn0, by rw [← hn, hx, cert_view]; rfl⟩
    · obtain ⟨rfl, -⟩ := hev tc hte
      obtain ⟨hep, -⟩ := hj.safe _ hte
      rw [hx, cert_epoch] at hep
      have hy : 2 = x + 1 := congrArg EpochNumber.toNat hep
      have h3 : 3 = r.view.toNat := congrArg ViewNumber.toNat htv
      have := bv_facts x
      exact absurd hlx (by omega)
  have hl : (H a (j + 1)).Lockable cfg (certOf (blk x)) := by
    have := hj.lockable hnone; rw [hx] at this; exact this
  refine ⟨x, ?_, ?_⟩
  · obtain ⟨c0, v0, e0⟩ := r
    simp only at hx hview hnone
    rw [hx, hview, hnone]; rfl
  · rcases lockable_iff.mp hl with h | ⟨y, hy, hlt'⟩
    · have := congrArg (·.view.toNat) h; simp only [cert_view] at this
      exact absurd this (by show ¬ bv x = 0; have := bv_facts x; omega)
    · obtain rfl : y = x := by
        have := congrArg (·.view.toNat) hy; simp only [cert_view] at this; exact (bv_inj this).symm
      simp only [lag_a] at hlt'
      have := at0_facts 4 10 y
      have := at0_facts (4 + 0) (10 + 0) y
      omega

include hv in
/-- A proposal is for the view of a header the node was handed, after it was handed it. -/
theorem proposal_header {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    (8 ≤ j ∧ p.viewNumber = ⟨2⟩)
      ∨ ∃ u, at0 0 6 u ≤ j ∧ p.viewNumber = ⟨bv u⟩ ∧ p.blockHeader = hdr (u + 1) := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨i, hi, hii⟩ := received.mp hj.built
  rcases input_header hii with ⟨rfl, hvw, -, -⟩ | ⟨u, rfl, hvw, -, hx⟩
  · exact Or.inl ⟨by omega, hvw⟩
  · exact Or.inr ⟨u, by omega, hvw, hx⟩

/-- No node proposes in view two after the first timer: it owes no proposal for a view it timed out. -/
theorem no_late_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) (hvw : p.viewNumber = ⟨2⟩) (h8 : 8 ≤ j) : False := by
  rw [tr_step] at hp
  rcases step_mem hp with h0 | ⟨o, out', -, ho, ha⟩
  · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer h0; cases hx
  · obtain ⟨e, v, rfl, hmem, -, -⟩ := act_proposal ha
    obtain ⟨-, -, hto, -⟩ := ho
    obtain ⟨st, hst, hout⟩ := (sent_iff (n := j)).mpr ⟨7, by omega, times_out_first k⟩
    refine hto ⟨_, ⟨st, List.mem_append_left _ hst, hout⟩, ?_⟩
    rw [← (mem_proposalCandidates hmem).2.1, hvw]
    exact Nat.le_refl _

include hv in
/-- Every proposal a node sends is `a`'s block for its view, sent once the header arrived. -/
theorem sent_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) : k = a ∧ ∃ u, p = blk u ∧ at0 0 6 u ≤ j := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨u, hju, hvw, hhdr⟩ : ∃ u, at0 0 6 u ≤ j ∧ p.viewNumber = ⟨bv u⟩ ∧ p.blockHeader = hdr (u + 1) := by
    rcases proposal_header hv hp with ⟨h8, hvw⟩ | h
    · exact (no_late_proposal hp hvw h8).elim
    · exact h
  refine ⟨u, ?_, hju⟩
  obtain ⟨-, hnext, hep, hnum⟩ := hj.wellFormed
  obtain ⟨hpc, hev⟩ := justified_held hj.justified
  have hid : p.identity = ⟨0⟩ := by
    rw [tr_step] at hp
    rcases step_mem hp with h0 | ⟨o, out', -, -, ha⟩
    · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨_, v, -, hmem, -, -⟩ := act_proposal ha
      exact (mem_proposalCandidates hmem).1
  have hnum' : p.parentCert.data.blockNumber + 1 = ⟨u + 1⟩ := by rw [hnum, hhdr]; rfl
  have hepu : p.epoch = ⟨u + 1⟩ := by rw [hep, hhdr]; exact epochOf_one_height (by omega)
  cases u with
  | zero =>
    have hpcA : p.parentCert = certOf anchorB := by
      rcases hasCert1 hpc with h | ⟨x, h, -⟩
      · exact h
      · rw [h, cert_number] at hnum'
        have : x + 1 + 1 = 0 + 1 := congrArg BlockNumber.toNat hnum'
        omega
    have hte : p.timeoutEvidence = none := by
      cases hte : p.timeoutEvidence with
      | none => rfl
      | some tc =>
        obtain ⟨rfl, -⟩ := hev tc hte
        obtain ⟨he, -⟩ := hj.safe _ hte
        rw [hepu] at he
        exact absurd (congrArg EpochNumber.toNat he) (by show ¬ (2 : Nat) = 0 + 1; omega)
    obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
    simp only at hvw hhdr hid hepu hpcA hte ⊢
    rw [hvw, hhdr, hid, hepu, hpcA, hte]; rfl
  | succ u =>
    cases u with
    | zero =>
      -- View three, after the timeout certificate for view two.
      obtain ⟨hpcB, -⟩ := cert_of_number (x := 0) hpc (by
        have : p.parentCert.data.blockNumber.toNat + 1 = 0 + 1 + 1 := congrArg BlockNumber.toNat hnum'
        exact BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = 0 + 1 by omega))
      have hte : p.timeoutEvidence = some T := by
        rcases hnext with ⟨-, hn⟩ | ⟨tc, hte, -⟩
        · rw [hpcB, cert_view, hvw] at hn
          have : bv 0 + 1 = bv (0 + 1) := congrArg ViewNumber.toNat hn
          exact absurd this (by decide)
        · obtain ⟨rfl, -⟩ := hev tc hte
          exact hte
      obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
      simp only at hvw hhdr hid hepu hpcB hte ⊢
      rw [hvw, hhdr, hid, hepu, hpcB, hte]; rfl
    | succ u =>
      obtain ⟨hpcB, -⟩ := cert_of_number (x := u + 1) hpc (by
        have : p.parentCert.data.blockNumber.toNat + 1 = u + 1 + 1 + 1 := congrArg BlockNumber.toNat hnum'
        exact BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = u + 1 + 1 by omega))
      have hte : p.timeoutEvidence = none := by
        rcases hnext with ⟨hn0, -⟩ | ⟨tc, hte, htv⟩
        · exact hn0
        · obtain ⟨rfl, -⟩ := hev tc hte
          rw [hvw] at htv
          have : 3 = bv (u + 1 + 1) := congrArg ViewNumber.toNat htv
          exact absurd this (by show ¬ 3 = u + 4; omega)
      obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
      simp only at hvw hhdr hid hepu hpcB hte ⊢
      rw [hvw, hhdr, hid, hepu, hpcB, hte]; rfl

/-- The parent `a` builds block `u + 1` on is justified from the step its header arrives until the block's `Cert1` does. -/
theorem parent_justified (u r : Nat) (h3 : r ≤ 3) : ParentJustified cfg (H a (at0 0 6 u + r)) (blk u) := by
  unfold ParentJustified
  cases u with
  | zero =>
    show (H a _).Buildable cfg (certOf anchorB)
    exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)]
    cases u with
    | zero =>
      show CertJustified cfg (H a (8 * (0 + 1) + 6 + r)) (certOf (blk 0)) (some T)
      refine ⟨hasCert1_of 0 (by rw [(at0_facts _ _ _).1 rfl]; omega), 8 * (0 + 1) + 6 + r, ?_,
        Or.inl ⟨certOf (blk 0), ?_, rfl⟩⟩ <;> rw [upTo_self]
      · exact recv_first 13 (by omega) (by omega)
      · have := lockedOn_lockAt (k := a) (8 * 1 + r + 6)
        rwa [(lockAt_later 1 r (by omega) (by omega)).2 (by rw [lag_a]; omega),
          show 8 * 1 + r + 6 = 8 * (0 + 1) + 6 + r by omega] at this
    | succ u =>
      show (H a _).Buildable cfg (certOf (blk (u + 1)))
      refine Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inr ⟨u + 1, rfl, ?_⟩))
      rw [lag_a, lag_a, (at0_facts _ _ _).2 (by omega)]
      omega

/-- `a` may propose block `u + 1` from the step its header arrives until the block's `Cert1` does. -/
theorem blk_justified (u r : Nat) (h1 : 1 ≤ r) (h3 : r ≤ 3) :
    ProposalJustified cfg leader a (H a (at0 0 6 u + r)) (blk u) := by
  obtain ⟨hvw, hew⟩ := at_block a u r h3
  refine ⟨⟨rfl, blk_wellFormed u, parent_justified u r h3, ⟨parentOf u, hasParent_of u (by omega),
      by rw [blk_parent]; exact Nat.le_refl _, by rw [blk_parent]; rfl⟩, ?_, blk_safe u, notBehind ?_,
      ⟨_, (inView _).1, ?_⟩⟩, ?_⟩
  · intro hen
    cases u with
    | zero => exact absurd hen blk_enters_zero
    | succ u =>
      have := at0_facts 0 6 (u + 1); have := at0_facts 1 7 u; have := at0_facts (c2At a) 11 u
      have : c2At a = 5 := by simp [c2At, show Early a from Or.inl rfl]
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), ?_, by rw [blk_parent]; rfl⟩
      rw [blk_view]; show bv u < bv (u + 1)
      have := bv_facts u; have := bv_facts (u + 1); omega
  · rw [blk_epoch, hew]; exact Nat.le_refl _
  · rw [blk_view, hvw]; exact Nat.le_refl _
  · rw [blk_view, blk_header, blk_parent]
    exact recv_phase u 0 (by omega) (by show at0 0 6 u < _; omega)

include hv in
/-- `a` proposes each block by the step its header arrives in. -/
theorem proposes (u : Nat) : ∃ j, j ≤ at0 0 6 u ∧ Output.send (.proposal (blk u)) ∈ (tr a j).output := by
  obtain ⟨hvw, -⟩ := at_block a u 1 (by omega)
  refine Classical.byContradiction fun hneg => settled a (at0 0 6 u) (.propose (blk u).epoch ⟨bv u⟩)
    ⟨Or.inl ⟨blk u, blk_justified u 1 (Nat.le_refl _) (by omega), blk_view u, rfl⟩, ?_, fun ht => ?_, ?_⟩
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv
      obtain rfl : x = u := bv_inj (view_inj hpv)
      exact hneg ⟨j, by omega, hjp⟩
    · -- `a`'s re-vote request in that view is of the epoch before.
      obtain ⟨j, -, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_revote hv hjr
      rw [blk_epoch] at hre
      have h1 : bv x + 1 = bv u := view_inj hrv
      have h2 : x + 1 = u + 1 := congrArg EpochNumber.toNat ((cert_epoch x).symm.trans hre)
      obtain rfl : x = u := by omega
      omega
  · refine not_timedOut ht (fun h => ?_)
    have h' : bv u ≤ 2 := h
    have := bv_facts u; have := at0_facts 0 6 u
    omega
  · have := inView (k := a) (at0 0 6 u + 1)
    rwa [hvw] at this

include hv in
/-- `a` asks for a re-vote on each block as soon as it can lock on it. -/
theorem revotes (u : Nat) : ∃ j, j ≤ at0 4 10 u ∧ Output.send (.revote (R u)) ∈ (tr a j).output := by
  obtain ⟨hvw, hew⟩ := at_lock u
  have hf := at0_facts 4 10 u
  refine Classical.byContradiction fun hneg => settled a (at0 4 10 u) (.propose ⟨u + 1⟩ ⟨bv u + 1⟩)
    ⟨Or.inr ⟨R u, ⟨rfl, ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_⟩, Liveness.buildable_of_lockable ?lk, fun _ => ?lk,
      (fun _ h => by cases h), notBehind ?_, ?_⟩, rfl, cert_epoch u⟩, ?_, fun ht => ?_, ?_⟩
  · show (certOf (blk u)).view < ⟨bv u + 1⟩; rw [cert_view]; show bv u < bv u + 1; omega
  · show (certOf (blk u)).view + 1 = ⟨bv u + 1⟩; rw [cert_view]; rfl
  · show IsLastBlock (certOf (blk u)).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
  · refine lockable_iff.mpr (Or.inr ⟨u, rfl, ?_⟩)
    rw [lag_a, lag_a]
    have := at0_facts (4 + 0) (10 + 0) u
    omega
  · show eAt a (at0 4 10 u + 1) ≤ (certOf (blk u)).data.epoch.toNat
    rw [cert_epoch, hew]; exact Nat.le_refl _
  · exact ⟨_, (inView _).1, by rw [hvw]; exact Nat.le_refl _⟩
  · rintro (⟨p, hs, hpv, hpe⟩ | ⟨r, hs, hrv, -⟩)
    · obtain ⟨j, -, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv; rw [blk_epoch] at hpe
      have h1 : bv x = bv u + 1 := view_inj hpv
      have h2 : x + 1 = u + 1 := congrArg EpochNumber.toNat hpe
      obtain rfl : x = u := by omega
      omega
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_revote hv hjr
      obtain rfl : x = u := by
        have : bv x + 1 = bv u + 1 := view_inj hrv
        exact bv_inj (by omega)
      exact hneg ⟨j, by omega, hjr⟩
  · refine not_timedOut ht (fun h => ?_)
    have h' : bv u + 1 ≤ 2 := h
    have := bv_facts u
    omega
  · have := inView (k := a) (at0 4 10 u + 1)
    rwa [hvw] at this

include hv in
/--
Every vote1 a node sends is for a block of its committee, after the proposal
arrived, or answers the first re-vote request, at `b` or `c`, still in epoch one.
-/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1} (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    ∃ u, (lag k u = 0 ∧ vote = ⟨(certOf (blk u)).data, ⟨bv u⟩, k⟩ ∧ at0 1 7 u ≤ j)
      ∨ (¬ Early k ∧ vote = ⟨(certOf (blk 0)).data, ⟨2⟩, k⟩ ∧ 5 ≤ j) := by
  have hpr := protocol hv k (j + 1)
  obtain ⟨hsig, -⟩ := hpr.vote1Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  obtain ⟨-, ⟨s', p, vid, hrec, -, hfor, -⟩ | ⟨s', r, hrec, -, hagain, hnb⟩⟩ :=
    hpr.vote1Leader j vote ⟨_, (getElem_H j), hx⟩ trivial
  · rw [upTo_self] at hrec
    obtain ⟨i, hi, hii⟩ := received.mp hrec
    obtain ⟨u, rfl, hl, -, rfl, -⟩ := input_proposal hii
    refine ⟨u, Or.inl ⟨hl, ?_, by omega⟩⟩
    obtain ⟨d, v, sg⟩ := vote
    obtain ⟨hv', hd⟩ := hfor
    simp only at hsig hv' hd
    rw [hsig, hv', hd, blk_view]; rfl
  · -- After the epoch change a node does not go back; only `b` and `c` get the first request before it.
    rw [upTo_self] at hrec hnb
    obtain ⟨u, -, rfl, hlt⟩ := received_revote hrec
    have h1 := notBehind_le hnb
    rw [show (R u).cert.data.epoch = ⟨u + 1⟩ from cert_epoch u] at h1
    have h1' : eAt k (j + 1) ≤ u + 1 := h1
    obtain ⟨-, hec, -, -⟩ := eAt_ge (k := k) (j + 1)
    have hf := at0_facts (rvAt k) 13 u
    have hf' := at0_facts (ecAt k) 12 u
    have hcl := classes k
    cases u with
    | zero =>
      have hE : ¬ Early k := fun hE => by
        have := hec 0 (by rcases hcl with ⟨-, -, h6, h10⟩ | ⟨h, -⟩
                          · omega
                          · exact absurd hE h)
        omega
      have h5 : rvAt k = 5 := by
        rcases hcl with ⟨h, -⟩ | ⟨-, -, -, h5⟩
        · exact absurd h hE
        · exact h5
      refine ⟨0, Or.inr ⟨hE, ?_, by omega⟩⟩
      obtain ⟨d, v, sg⟩ := vote
      obtain ⟨hv', hd⟩ := hagain
      simp only at hsig hv' hd
      rw [hsig, hv', hd]; rfl
    | succ u =>
      have := hec (u + 1) (by omega)
      omega

include hv in
/-- Every member of a block's committee votes1 for it by the step its validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ at0 2 8 u
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨bv u⟩, k⟩) ∈ (tr k j).output := by
  have hf := at0_facts 2 8 u
  have hf0 := at0_facts 0 6 u
  have hcl := classes k
  obtain ⟨hvw, hew⟩ := at_block k u 3 (by omega)
  have h3 : at0 2 8 u + 1 = at0 0 6 u + 3 := by omega
  have hopen : OpensEpochJustified cfg (H k (at0 2 8 u + 1)) (blk u) := fun he => by
    cases u with
    | zero => exact absurd he blk_enters_zero
    | succ u =>
      have := at0_facts 1 7 u
      have := at0_facts (c2At k) 11 u
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), ?_, by rw [blk_parent]; rfl⟩
      rw [blk_view]; show bv u < bv (u + 1)
      have := bv_facts u; have := bv_facts (u + 1); omega
  have hprop : (H k (at0 2 8 u + 1)).Received (.proposal a (blk u) (some ⟨⟨bv u⟩, (blk u).payloadCommit⟩)) := by
    have := recv_phase (k := k) (n := at0 2 8 u + 1) u 1 (by omega)
      (by have := at0_facts 1 7 u; show at0 1 7 u < _; omega)
    simp only [phase, hl, ite_eq_left] at this
    exact this
  refine Classical.byContradiction fun hneg => settled k (at0 2 8 u) (.vote1 (blk u))
    ⟨⟨a, _, hprop, rfl, by rw [blk_view], rfl⟩, blk_wellFormed u, ?_, ?_, blk_safe u, hopen,
      notBehind ?_, fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · rw [blk_view]; exact recv_phase u 2 (by omega) (by show at0 2 8 u < _; omega)
  · cases u with
    | zero => exact Or.inl rfl
    | succ u => exact Or.inr (Or.inl (blk_enters u))
  · rw [blk_epoch, h3, hew]; exact Nat.le_refl _
  · refine not_timedOut ht (fun h => ?_)
    rw [blk_view] at h
    have h' : bv u ≤ 2 := h
    have := bv_facts u
    omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', ⟨-, rfl, -⟩ | ⟨-, rfl, -⟩⟩ := sent_vote1 hv hjv
    · rw [blk_view] at hvv
      obtain rfl : u' = u := bv_inj (view_inj hvv)
      exact hneg ⟨j, by omega, hjv⟩
    · rw [blk_view] at hvv
      have : 2 = bv u := view_inj hvv
      have := bv_facts u
      omega
  · rw [blk_view, h3]
    have := inView (k := k) (at0 0 6 u + 3)
    rwa [hvw] at this

include hv in
/-- `b` and `c` answer the first re-vote request, still in epoch one. -/
theorem votes1R (k : PubKey) (hl : lag k 0 = 0) (hE : ¬ Early k) : ∃ j, j ≤ 5
    ∧ Output.send (.vote1 ⟨(certOf (blk 0)).data, ⟨2⟩, k⟩) ∈ (tr k j).output := by
  have hvw : vAt 6 = 2 := (vAt_first (by omega)).2 (by omega)
  have hew : eAt k 6 = 1 := (eAt_first (by omega)).2 (by simp [ecAt, hE])
  have hrec : (H k 6).Received (.revote a (R 0)) := by
    have := recv_first (k := k) 5 (by omega) (by omega : 5 < 6)
    simp only [first, hE, ite_false] at this
    exact this
  refine Classical.byContradiction fun hneg => settled k 5 (.vote1Again (R 0))
    ⟨⟨a, hrec, rfl⟩, ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_⟩, (fun _ h => by cases h),
      ⟨blk 0, hasProposal_of 0 (by simp [at0]), ⟨Nat.le_refl _, rfl⟩, payload_of 0 hl (by simp [at0])⟩,
      notBehind (by rw [hew]; exact Nat.le_refl _), fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · show (certOf (blk 0)).view < ⟨2⟩; rw [cert_view]; show (1 : Nat) < 2; omega
  · show (certOf (blk 0)).view + 1 = ⟨2⟩; rw [cert_view]; rfl
  · show IsLastBlock (certOf (blk 0)).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
  · exact not_timedOut ht (fun _ => by omega)
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', ⟨-, rfl, -⟩ | ⟨-, rfl, -⟩⟩ := sent_vote1 hv hjv
    · have : bv u' = 2 := view_inj hvv
      have := bv_facts u'
      omega
    · exact hneg ⟨j, by omega, hjv⟩
  · have := inView (k := k) 6
    rwa [hvw] at this

include hv in
/-- Every vote2 a node sends is on a block of its committee, after its payload arrived. -/
theorem sent_vote2 {k : PubKey} {j : Nat} {vote : Vote2} (hx : Output.send (.vote2 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(C2 u).data, ⟨bv u⟩, k⟩ ∧ lag k u = 0 ∧ at0 4 10 u ≤ j := by
  obtain ⟨hsig, hgen, x, y, hc, hb, hcert, hpay, hvc, hdc⟩ :=
    (protocol hv k (j + 1)).vote2Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  rw [upTo_self] at hc hb hpay
  obtain ⟨d, v, sg⟩ := vote
  simp only at hsig hvc hdc hgen
  rcases hasPayload hpay with hg | ⟨u, hyv, -, hl, hlt⟩
  · -- At genesis: the anchor, whose certificate is at genesis too.
    exfalso
    rcases hasProposal hb with rfl | ⟨u, rfl, -⟩
    · have hx0 : x.data.blockNumber = ⟨0⟩ := by rw [hcert.2]; rfl
      rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩
      · rw [hvc] at hgen; exact Nat.lt_irrefl _ hgen
      · rw [cert_number] at hx0; exact absurd (number_inj hx0) (by omega)
    · rw [blk_view] at hg; have := bv_facts u; exact absurd (view_inj hg) (by omega)
  · obtain rfl : y = blk u := by
      rcases hasProposal hb with rfl | ⟨u', rfl, -⟩
      · have := bv_facts u; exact absurd (view_inj hyv) (by omega)
      · rw [blk_view] at hyv
        obtain rfl : u' = u := bv_inj (view_inj hyv)
        rfl
    obtain ⟨rfl, -⟩ := cert_of_number (x := u) hc (by rw [hcert.2]; exact blk_number u)
    refine ⟨u, ?_, hl, by omega⟩
    rw [hsig, hvc, hdc, cert_view]; rfl

include hv in
/-- Every member of a block's committee votes2 for it by the step its payload arrives in. -/
theorem votes2 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ at0 4 10 u
    ∧ Output.send (.vote2 ⟨(C2 u).data, ⟨bv u⟩, k⟩) ∈ (tr k j).output := by
  have hf := at0_facts 4 10 u
  have hcl := classes k
  refine Classical.byContradiction fun hneg => settled k (at0 4 10 u) (.vote2 (certOf (blk u)))
    ⟨⟨blk u, hasCert1_of u (by have := at0_facts 3 9 u; omega), hasProposal_of u (by have := at0_facts 1 7 u; omega),
      ⟨Nat.le_refl _, rfl⟩, payload_of u hl (by omega)⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -, -⟩ := sent_vote2 hv hjv
    rw [cert_view] at hvv
    obtain rfl : u' = u := bv_inj (view_inj hvv)
    exact hneg ⟨j, by omega, hjv⟩
  · obtain ⟨x, rfl, hlt⟩ := hasCert2 hc2
    rw [cert_view] at hv2
    obtain rfl : x = u := bv_inj (view_inj hv2)
    have := at0_facts (c2At k) 11 x
    omega
  · rw [cert_view]
    rintro (ht | ⟨tc, htc, hle⟩)
    · refine not_timedOut ht (fun h => ?_)
      have h' : bv u ≤ 2 := h
      have := bv_facts u
      omega
    · obtain ⟨rfl, hlt⟩ := received_tc htc
      have h' : bv u ≤ 2 := hle
      have := bv_facts u
      omega
  · refine Kit.afterFloor_of input hv (by rw [cert_view]; show 0 < bv u; have := bv_facts u; omega) fun x hx => ?_
    rw [cert_view]
    have := bv_facts u
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
    · show 0 < bv u + 20; omega
    · rw [blk_view]; show bv y < bv u + 20
      have := bv_facts y; have := at0_facts 1 7 y; have := at0_facts 4 10 u
      omega

end Sends

/-! ## Time -/

section Time

/-- When step `n` happens: the first timer at `36`, which is GST, the second at `69`, and one unit a step otherwise. -/
def tm (n : Nat) : Nat := if n ≤ 6 then n else if n ≤ 11 then n + 29 else n + 57

theorem tm_facts (n : Nat) : (n ≤ 6 → tm n = n) ∧ (7 ≤ n → n ≤ 11 → tm n = n + 29) ∧ (12 ≤ n → tm n = n + 57) := by
  unfold tm
  refine ⟨fun h => ite_eq_left h, fun h1 h2 => ?_, fun h => ?_⟩
  · rw [ite_eq_right (by omega), ite_eq_left h2]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega)]

theorem tm_lt {n m : Nat} (h : n < m) : tm n < tm m := by
  have := tm_facts n; have := tm_facts m; omega

theorem tm_succ (n : Nat) : tm n ≤ tm (n + 1) := Nat.le_of_lt (tm_lt (Nat.lt_succ_self n))

theorem tm_mono {n m : Nat} (h : n ≤ m) : tm n ≤ tm m := Kit.mono_of_succ tm_succ h

theorem tm_ge (n : Nat) : n ≤ tm n := by have := tm_facts n; omega

/-- A node enters view two at step three. -/
theorem vAt_entry {n : Nat} (h1 : vAt (n + 1) = 2) (h2 : vAt n ≠ 2) : n = 3 := by
  have := (vAt_ge (n + 1)).2.2
  by_cases h13 : 13 < n + 1
  · omega
  · have := vAt_first (n := n) (by omega); have := vAt_first (n := n + 1) (by omega); omega

end Time

/-! ## The network -/

section Net

variable (hv : ∀ b, BlockValid b)

/-- An honest quorum of any epoch holds `b`. -/
theorem quorum_b {e : EpochNumber} {q : PubKey → Prop} (hq : C.Quorum e q) (hh : ∀ k, q k → C.Honest k) : q b := by
  rcases hq with ⟨-, h, -⟩ | ⟨-, h, -⟩ | ⟨-, -, h⟩ | ⟨h, -, -⟩
  · exact h
  · exact h
  · exact absurd (hh d h) d_faulty
  · exact h

include hv in
theorem backed1 (u : Nat) : Cert1Backed (C := C) (fun k _ => tr k) (certOf (blk u)) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k, by rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes1 hv k u (lag_member hk.1)
  rw [cert_view]
  exact ⟨j, hj⟩

include hv in
theorem backed2 (u : Nat) : Cert2Backed (C := C) (fun k _ => tr k) (C2 u) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k,
    by show C.Quorum (certOf (blk u)).data.epoch _; rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes2 hv k u (lag_member hk.1)
  exact ⟨j, hj⟩

/-- The votes behind the timeout certificate: the second timer's, all in epoch two. -/
theorem tc_votes (k : PubKey) :
    Output.send (.timeoutVote ⟨⟨⟨2⟩, certOf (blk 0)⟩, ⟨2⟩, k⟩) ∈ (tr k 12).output := by
  have h := times_out k (j := 12) (v := ⟨2⟩) (by rw [input_first (by omega)]; rfl)
    ((vAt_first (by omega)).2 (by omega))
  rwa [(eAt_first (k := k) (n := 12) (by omega)).1 (by have := classes k; omega),
    lockAt_first (by omega) (by omega)] at h

theorem tcBacked : TimeoutCertBacked (C := C) (fun k _ => tr k) T :=
  ⟨fun k => C.members ⟨2⟩ k ∧ C.honest ⟨2⟩ k, members_quorum _, fun k _ _ =>
    ⟨_, ⟨rfl, rfl, rfl, Or.inr ⟨rfl, Nat.le_refl _⟩⟩, ⟨_, tc_votes k⟩⟩⟩

include hv in
theorem tcChecked : TimeoutLockChecked (C := C) (fun k _ => tr k) cfg T :=
  ⟨Or.inr (backed1 hv 0), by
    show (certOf (blk 0)).view.toNat ≤ 2
    rw [cert_view]; show (1 : Nat) ≤ 2; omega⟩

/-- The honest nodes running the machine on their schedules. -/
def net : TimedNetwork cfg leader C where
  honestQuorum := members_quorum
  trace k _ := tr k
  safe k _ n := .of_every (protocol hv k n).toSafeHistory
  cert1Genuine k _ n x hc := by
    rcases Input.mem_cert1.mp hc with hin | ⟨c2, p, hin⟩ | ⟨s, p, vid, hin, rfl⟩
    · obtain ⟨u, -, rfl⟩ := input_cert1 hin
      exact Or.inr (backed1 hv u)
    · obtain ⟨u, -, rfl, -, -⟩ := input_epochChange hin
      exact Or.inr (backed1 hv u)
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      rw [blk_parent]
      cases u with
      | zero => exact Or.inl rfl
      | succ u => exact Or.inr (backed1 hv u)
  cert2Genuine k _ n x hc := by
    rcases Input.mem_cert2.mp hc with hin | ⟨c1, p, hin⟩
    · obtain ⟨u, -, rfl⟩ := input_cert2 hin
      exact backed2 hv u
    · obtain ⟨u, -, -, rfl, -⟩ := input_epochChange hin
      exact backed2 hv u
  timeoutCertGenuine k _ n tc hc := by
    rcases Input.mem_timeoutCert.mp hc with hin | ⟨s, p, vid, hin, hte⟩ | ⟨s, r, hin, hte⟩
    · obtain ⟨-, rfl⟩ := input_tc hin
      exact ⟨tcBacked, tcChecked hv⟩
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      rcases blk_evidence u with h | ⟨-, h⟩ <;> rw [h] at hte
      · cases hte
      · cases hte; exact ⟨tcBacked, tcChecked hv⟩
    · obtain ⟨u, -, -, rfl⟩ := input_revote hin
      cases hte
  revoteGenuine k _ n _ r hin := by
    obtain ⟨u, -, -, rfl⟩ := input_revote hin
    exact backed1 hv u
  time _ _ n := tm n
  timeMono _ _ n := tm_succ n
  protocol k _ n := .of_every (protocol hv k n)
  timeoutCertCausal k _ n tc hin := by
    obtain ⟨rfl, rfl⟩ := input_tc hin
    exact ⟨fun k => C.members ⟨2⟩ k ∧ C.honest ⟨2⟩ k, members_quorum _, fun k' _ _ =>
      ⟨_, _, ⟨rfl, rfl, rfl, Or.inr ⟨rfl, Nat.le_refl _⟩⟩, tc_votes k', tm_lt (by omega)⟩⟩
  oneHonestCausal k _ n v hin := by
    obtain ⟨rfl, rfl⟩ := input_oneHonest hin
    have h := times_out a (j := 7) (v := ⟨2⟩) (by rw [input_first (by omega)]; rfl)
      ((vAt_first (by omega)).2 (by omega))
    exact ⟨a, _, Or.inl rfl, 7, _, rfl, h, rfl, tm_lt (by omega)⟩
  authentic k _ n l msg hin _ _ := by
    cases hi : (tr k n).input <;> rw [hi] at hin <;> simp only [Input.sentBy, reduceCtorEq] at hin
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨u, rfl, rfl, rfl⟩ := input_proposal_any hi
      obtain ⟨j, hj, hjp⟩ := proposes hv u
      exact ⟨j, hjp, tm_lt (by have := at0_facts 0 6 u; have := at0_facts 1 7 u; omega)⟩
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨u, rfl, rfl, rfl⟩ := input_revote hi
      obtain ⟨j, hj, hjr⟩ := revotes hv u
      have := classes k
      exact ⟨j, hjr, tm_lt (by have := at0_facts 4 10 u; have := at0_facts (rvAt k) 13 u; omega)⟩

theorem net_time {k : PubKey} {hk : C.Honest k} {n : Nat} : (net hv).time k hk n = tm n := rfl

theorem by_at {k : PubKey} {hk : C.Honest k} {T m : Nat} {P : History → Prop}
    (hm : 0 < m → tm (m - 1) ≤ T) (hp : P (H k m)) : (net hv).By k hk T P :=
  Kit.by_at (fun _ _ => rfl) (fun _ _ _ => rfl) tm_succ hm hp

/-- What holds after step `j`, holds by its time. -/
theorem by_step {k : PubKey} {hk : C.Honest k} {T : Nat} {P : History → Prop} (j : Nat)
    (hj : tm j ≤ T) (hp : P (H k (j + 1))) : (net hv).By k hk T P :=
  by_at hv (fun _ => by simp only [Nat.add_sub_cancel]; exact hj) hp

/-- What a node holds after the first epoch's twelfth step, it holds by GST plus `Δ`. -/
theorem by_gst {k : PubKey} {hk : C.Honest k} {t : Nat} {P : History → Prop} (hp : P (H k 12)) :
    (net hv).By k hk (max t 36 + 4) P :=
  by_step hv 11 (by have := (tm_facts 11).2.1 (by omega) (by omega); omega) hp

theorem sentBy {k : PubKey} {hk : C.Honest k} {t : Nat} {m : Message} (hs : (net hv).SentByTime k hk t m) :
    ∃ j, tm j ≤ t ∧ Output.send m ∈ (tr k j).output :=
  Kit.sentBy (N := net hv) (fun _ _ => rfl) (fun _ _ _ => rfl) hs

/-- What every node holds by a step it held before, or by GST plus `Δ` if it got it in the first epoch. -/
theorem by_same {k' : PubKey} {hk' : C.Honest k'} {n : Nat} {P : History → Prop} (x : Nat)
    (hp : ∀ m, x < m → P (H k' m)) (hx : x < n + 1 ∨ x < 12) : (net hv).By k' hk' (max (tm n) 36 + 4) P := by
  have := Nat.le_max_left (tm n) 36
  by_cases hn : x < n + 1
  · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hp _ hn)
  · exact by_gst hv (hp 12 (by omega))

include hv in
/-- A header for every view `a` is ready to propose in arrives within `Δ`, after GST. -/
theorem header_arrives {k : PubKey} {hk : C.Honest k} {n : Nat} {p : Proposal}
    (hready : ProposalReady cfg leader k (H k (n + 1)) p) :
    (net hv).By k hk (max (tm n) 36 + 4) fun hist =>
      ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
  obtain ⟨hlead, ⟨-, hnext, hep, hnum⟩, hj, -, hop, hsafe, -, -⟩ := hready
  obtain rfl := FiveNodes.leader_eq hlead
  have hmax := Nat.le_max_left (tm n) 36
  have hmax' := Nat.le_max_right (tm n) 36
  have hc := hasCert1_of_certJustified hj
  -- Every header arrives within `Δ` of the node's being able to use it; `m` is when.
  have hdone : ∀ m, m ≤ n + 1 ∨ tm (m - 1) ≤ max (tm n) 36 + 4 → ∀ hdr',
      hdr'.blockNumber = p.blockHeader.blockNumber →
      (H a m).Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') →
      (net hv).By a hk (max (tm n) 36 + 4) fun hist =>
        ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
          ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
    intro m hm hdr' h1 h2
    rcases hm with hm | hm
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨hdr', h1, received_mono hm h2⟩
    · exact by_at hv (m := m) (fun _ => hm) ⟨hdr', h1, h2⟩
  rcases hasCert1 hc with hpc | ⟨x, hpc, -⟩
  · -- On genesis: view one, whose header comes first.
    have hpv : p.viewNumber = ⟨1⟩ := by
      rcases hnext with ⟨-, h⟩ | ⟨tc, hte, -⟩
      · rw [← h, hpc]; rfl
      · exfalso
        obtain ⟨rfl, -⟩ := (justified_held hj).2 tc hte
        obtain ⟨he, -⟩ := hsafe _ hte
        rw [hep, ← hnum, hpc] at he
        have : 2 = 1 := congrArg EpochNumber.toNat he
        omega
    refine hdone 1 (Or.inr (by show tm 0 ≤ _; have := (tm_facts 0).1 (by omega); omega)) (hdr 1)
      (by rw [← hnum, hpc]; rfl) ?_
    rw [hpv, hpc]; exact recv_first 0 (by omega) (by omega)
  · have hnum' : p.blockHeader.blockNumber = ⟨x + 2⟩ := by rw [← hnum, hpc, cert_number]; rfl
    rcases hnext with ⟨-, h⟩ | ⟨tc, hte, htv⟩
    · have hpv : p.viewNumber = ⟨bv x + 1⟩ := by rw [← h, hpc, cert_view]; rfl
      cases x with
      | zero =>
        -- View two: its header arrives after the first timer.
        refine hdone 9 (Or.inr (by have := (tm_facts 8).2.1 (by omega) (by omega); show tm 8 ≤ _; omega))
          (hdr 2) hnum'.symm ?_
        rw [hpv, hpc]
        exact recv_first 8 (by omega) (by omega)
      | succ x =>
        -- Opening the next epoch, behind the `Cert2` over the parent, which `a` holds.
        have hen : EntersEpoch cfg p := by
          show IsLastBlock (p.blockHeader.blockNumber - 1) 1
          rw [hnum']; exact last_block (n := x + 2) (by omega)
        obtain ⟨-, c2, hc2, -, hd⟩ := hop hen
        rcases hc2 with hc2 | rfl
        case inr =>
          exfalso
          have h0 := congrArg Vote2Data.blockNumber hd
          rw [hpc] at h0
          have h2 : (certOf (blk (x + 1))).data.toVote2.blockNumber = ⟨x + 1 + 1⟩ := cert_number _
          rw [h2] at h0
          exact absurd (congrArg BlockNumber.toNat h0) (by show ¬ (0 : Nat) = x + 1 + 1; omega)
        obtain ⟨y, rfl, hlt⟩ := hasCert2 hc2
        obtain rfl : y = x + 1 := by
          have h0 := congrArg Vote2Data.blockNumber hd
          rw [hpc] at h0
          have h1 : (C2 y).data.blockNumber = ⟨y + 1⟩ := cert_number y
          have h2 : (certOf (blk (x + 1))).data.toVote2.blockNumber = ⟨x + 1 + 1⟩ := cert_number (x + 1)
          rw [h1, h2] at h0
          have := number_inj h0; omega
        have := at0_facts (c2At a) 11 (x + 1)
        have := tm_mono (show 8 * (x + 1) + 11 ≤ n by omega)
        have := (tm_facts (8 * (x + 1) + 11)).2.2 (by omega)
        have := (tm_facts (8 * (x + 2) + 6)).2.2 (by omega)
        refine hdone (8 * (x + 2) + 6 + 1) (by
            by_cases hn : 8 * (x + 2) + 6 + 1 ≤ n + 1
            · exact Or.inl hn
            · exact Or.inr (by show tm (8 * (x + 2) + 6) ≤ _; omega)) (hdr (x + 3)) hnum'.symm ?_
        rw [hpv, hpc]
        exact recv_later (x + 2) 0 (by omega) (by omega) (by omega)
    · -- After the timeout certificate: view three, opening epoch two.
      obtain ⟨rfl, hy⟩ := (justified_held hj).2 tc hte
      obtain ⟨he, -⟩ := hsafe _ hte
      rw [hep, hnum', show cfg.epochHeight = 1 from rfl, epochOf_one_height (by omega)] at he
      obtain rfl : x = 0 := by have : 2 = x + 2 := congrArg EpochNumber.toNat he; omega
      have hpv : p.viewNumber = ⟨3⟩ := by rw [← htv]; rfl
      have := tm_mono (show 13 ≤ n by omega)
      have := (tm_facts 13).2.2 (by omega)
      have := (tm_facts 14).2.2 (by omega)
      refine hdone 15 (by
          by_cases hn : 15 ≤ n + 1
          · exact Or.inl hn
          · exact Or.inr (by show tm 14 ≤ _; omega)) (hdr 2) hnum'.symm ?_
      rw [hpv, hpc]
      exact recv_later 1 0 (by omega) (by omega) (by omega)

/-- The timer for a view does not fire before `τ` has passed since the node entered it. -/
theorem timer_not_early {k : PubKey} {n m : Nat} {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v)
    (hnot : n = 0 ∨ ¬ (H k n).InView cfg v) (hinm : input k m = .timeout v) : tm n + 33 ≤ tm m := by
  have h1 : vAt (n + 1) = v.toNat := (congrArg ViewNumber.toNat (inView_eq hin)).symm
  have h2 : vAt n ≠ v.toNat := fun h => by
    rcases hnot with rfl | hnot
    · obtain ⟨-, rfl⟩ := input_timeout hinm; exact absurd h (by decide)
    · exact hnot (by have := inView (k := k) n; rwa [h] at this)
  obtain ⟨hm, rfl⟩ := input_timeout hinm
  rw [vAt_entry h1 h2]
  have := tm_facts 3; have := tm_facts m
  omega

/-- The timer for a view fires within `τ` of the node's entering it, or of its last firing, unless the node has moved on. -/
theorem timer_fires {k : PubKey} (n : Nat) {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v) :
    ∃ m, n < m ∧ tm m ≤ tm n + 33 ∧ (input k m = .timeout v ∨ ∃ w, v < w ∧ (H k (m + 1)).InView cfg w) := by
  have hv' := inView_eq hin
  subst hv'
  have hT := tm_facts n
  rcases steps (n + 1) with hn | ⟨u, r, hu, hr, hn1⟩
  · obtain ⟨h1, h2⟩ := vAt_first hn
    by_cases r4 : 4 ≤ n + 1
    · rw [h2 r4]
      by_cases h12 : n = 12
      · -- Timed out again: the timeout certificate moves the node on.
        subst h12
        refine ⟨13, by omega, by have := tm_facts 13; omega, Or.inr ⟨_, ?_, inView _⟩⟩
        rw [show vAt (13 + 1) = 3 from (vAt_later 1 0 (by omega) (by omega)).1 (by omega)]
        show (2 : Nat) < 3; omega
      by_cases h7 : 7 ≤ n
      · exact ⟨12, by omega, by have := tm_facts 12; omega, Or.inl (by rw [input_first (by omega)]; rfl)⟩
      · exact ⟨7, by omega, by have := tm_facts 7; omega, Or.inl (by rw [input_first (by omega)]; rfl)⟩
    · rw [h1 (by omega)]
      refine ⟨3, by omega, by have := tm_facts 3; omega, Or.inr ⟨_, ?_, inView _⟩⟩
      rw [(vAt_first (n := 3 + 1) (by omega)).2 (by omega)]; show (1 : Nat) < 2; omega
  · rw [hn1]
    obtain ⟨h0, h4⟩ := vAt_later u r hu hr
    by_cases r4 : 4 ≤ r
    · -- The view after a block: the next block's `Cert1` moves the node on.
      rw [h4 r4]
      refine ⟨8 * (u + 1) + 3 + 6, by omega, by have := tm_facts (8 * (u + 1) + 3 + 6); omega,
        Or.inr ⟨_, ?_, inView _⟩⟩
      rw [show 8 * (u + 1) + 3 + 6 + 1 = 8 * (u + 1) + 4 + 6 by omega,
        (vAt_later (u + 1) 4 (by omega) (by omega)).2 (by omega)]
      show u + 3 < u + 1 + 3; omega
    · -- The view of a block: its `Cert1` moves the node on.
      rw [h0 (by omega)]
      refine ⟨8 * u + 3 + 6, by omega, by have := tm_facts (8 * u + 3 + 6); omega, Or.inr ⟨_, ?_, inView _⟩⟩
      rw [show 8 * u + 3 + 6 + 1 = 8 * u + 4 + 6 by omega, (vAt_later u 4 hu (by omega)).2 (by omega)]
      show u + 2 < u + 3; omega

include hv in
/-- Every delivery within `Δ = 4` after GST `36`, with view timer `τ = 33`. -/
theorem sync : Synchrony (net hv) 36 4 33 where
  proposal l hl n p hsend _ k hk hmem _ := by
    simp only [net_time hv]
    obtain ⟨rfl, u, rfl, hu⟩ := sent_proposal hv hsend
    rw [blk_epoch] at hmem
    have hl0 := lag_member hmem
    have hP : ∀ m, at0 1 7 u < m → (∃ vid, ShareMatches (blk u) vid ∧ (H k m).Received (.proposal a (blk u) (some vid))) :=
      fun m hm => by
        have := recv_phase (k := k) (n := m) u 1 (by omega) hm
        simp only [phase, hl0, ite_eq_left] at this
        exact ⟨⟨⟨bv u⟩, (blk u).payloadCommit⟩, ⟨by rw [blk_view], rfl⟩, this⟩
    cases u with
    | zero => exact by_gst hv (hP 12 (by simp [at0]))
    | succ u =>
      rw [(at0_facts _ _ _).2 (by omega)] at hu
      have := (tm_facts n).2.2 (by omega)
      have := (tm_facts (8 * (u + 1) + 7)).2.2 (by omega)
      have := Nat.le_max_left (tm n) 36
      exact by_step hv (8 * (u + 1) + 7) (by omega) (hP _ (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
  revote l hl n r hsend _ k hk _ _ := by
    simp only [net_time hv]
    obtain ⟨rfl, x, rfl, hx⟩ := sent_revote hv hsend
    have hcl := classes k
    cases x with
    | zero => exact by_gst hv (recv_revote 0 (by rw [(at0_facts _ _ _).1 rfl]; omega))
    | succ x =>
      rw [(at0_facts _ _ _).2 (by omega)] at hx
      have := (tm_facts n).2.2 (by omega)
      have := (tm_facts (8 * (x + 1) + 13)).2.2 (by omega)
      have := Nat.le_max_left (tm n) 36
      exact by_step hv (8 * (x + 1) + 13) (by omega)
        (recv_revote (x + 1) (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
  cert1 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, ⟨-, heq, hx⟩ | ⟨hE, -, -⟩⟩ := sent_vote1 hv hj
    · simp only [Vote.mk.injEq] at heq
      obtain ⟨rfl, rfl, -⟩ := heq
      rw [show (⟨(certOf (blk x)).data, ⟨bv x⟩⟩ : Cert1) = certOf (blk x) by rw [← cert_view x]]
      cases x with
      | zero => exact by_gst hv (recv_first 3 (by omega) (by omega))
      | succ x =>
        rw [(at0_facts _ _ _).2 (by omega)] at hx
        have := tm_mono hx
        have := (tm_facts (8 * (x + 1) + 7)).2.2 (by omega)
        have := (tm_facts (8 * (x + 1) + 9)).2.2 (by omega)
        have := Nat.le_max_left t 36
        exact by_step hv (8 * (x + 1) + 9) (by omega) (recv_later (x + 1) 3 (by omega) (by omega) (by omega))
    · exact absurd (Or.inl rfl) hE
  cert2 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, heq, -, hx⟩ := sent_vote2 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    have hcl := classes k
    cases x with
    | zero => exact by_gst hv (recv_c2 0 (by rw [(at0_facts _ _ _).1 rfl]; omega))
    | succ x =>
      rw [(at0_facts _ _ _).2 (by omega)] at hx
      have := tm_mono hx
      have := (tm_facts (8 * (x + 1) + 10)).2.2 (by omega)
      have := (tm_facts (8 * (x + 1) + 11)).2.2 (by omega)
      have := Nat.le_max_left t 36
      exact by_step hv (8 * (x + 1) + 11) (by omega)
        (recv_c2 (x + 1) (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
  cert2Spread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc
    have := classes k'
    cases x with
    | zero =>
      exact by_same hv _ (fun m hm => hasCert2_of 0 hm) (Or.inr (by rw [(at0_facts _ _ _).1 rfl]; omega))
    | succ x =>
      rw [(at0_facts _ _ _).2 (by omega)] at hx
      exact by_same hv _ (fun m hm => hasCert2_of (x + 1) hm)
        (Or.inl (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
  certSpread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_same hv (at0 3 9 x) (fun m hm => hasCert1_of x hm) (Or.inl hx)
  lockSpread c k hk n hc k' hk' hm _ := by
    simp only [net_time hv]
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · have hl : lag k' x = 0 := lag_member (by rw [cert_epoch] at hm; exact hm)
      have hf := at0_facts (4 + lag k' 0) (10 + lag k' x) x
      have hf3 := at0_facts 3 9 x
      have hlock : ∀ m, at0 (4 + lag k' 0) (10 + lag k' x) x < m → (H k' m).Lockable cfg (certOf (blk x)) :=
        fun m hm => lockable_iff.mpr (Or.inr ⟨x, rfl, hm⟩)
      cases x with
      | zero => exact by_same hv _ hlock (Or.inr (by omega))
      | succ x =>
        by_cases hn : at0 (4 + lag k' 0) (10 + lag k' (x + 1)) (x + 1) < n + 1
        · exact by_same hv _ hlock (Or.inl hn)
        · have := tm_mono (show 8 * (x + 1) + 9 ≤ n by omega)
          have := (tm_facts (8 * (x + 1) + 9)).2.2 (by omega)
          have := (tm_facts (8 * (x + 1) + 10)).2.2 (by omega)
          have := Nat.le_max_left (tm n) 36
          exact by_step hv (8 * (x + 1) + 10) (by omega) (hlock _ (by omega))
  blockSpread c b _ k hk n hcb k' hk' _ := by
    simp only [net_time hv]
    rcases hasProposal hcb.2 with rfl | ⟨y, rfl, hy⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_same hv (at0 1 7 y) (fun m hm => hasProposal_of y hm) (Or.inl hy)
  timeoutCert e q v t hq hvotes k hk _ := by
    have hh := fun k hk => Committee.Honest.of (hvotes k hk).1
    obtain ⟨_hka, L, hsent⟩ := hvotes a (quorum_a hq hh)
    obtain ⟨j, -, hj⟩ := sentBy hv hsent
    have := Nat.le_max_left t 36
    -- Only the second timer's votes are all of one epoch; `b`'s comes last.
    obtain ⟨hj', heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
    obtain ⟨⟨he, -⟩, rfl, -⟩ := heq
    have hea : eAt a j = 2 := (eAt_first (by omega)).1 (by rw [ecAt_a]; omega)
    obtain ⟨_hkb, Lb, hsb⟩ := hvotes b (quorum_b hq hh)
    obtain ⟨jb, hjbt, hjb⟩ := sentBy hv hsb
    obtain ⟨hjb', heqb⟩ := sent_timeout hjb
    simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heqb
    obtain ⟨⟨heb, -⟩, -, -⟩ := heqb
    have hb12 : jb = 12 := by
      by_cases hne : jb = 12
      · exact hne
      · have h1 : eAt b jb = 1 := (eAt_first (by omega)).2 (by rw [ecAt_b]; omega)
        have h2 : eAt a j = eAt b jb := by
          have := congrArg EpochNumber.toNat (he.symm.trans heb); simpa using this
        omega
    subst hb12
    have := (tm_facts 12).2.2 (by omega)
    have := (tm_facts 13).2.2 (by omega)
    refine by_step hv 13 (by omega) ⟨T, ?_, rfl, recv_first 13 (by omega) (by omega)⟩
    rw [he, hea]; rfl
  timeoutOneHonest e q v t hq hvotes k hk _ := by
    obtain ⟨k0, hq0, -, hk0⟩ := C.intersect _ q q hq hq
    obtain ⟨L, hs⟩ := hvotes k0 hq0 hk0
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨hj', heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨-, rfl, -⟩ := heq
    -- Every honest node timed view two out at step 7, no later than any vote.
    have h7 := tm_facts 7
    have htj : tm 7 ≤ t := Nat.le_trans (tm_mono (by omega)) hjt
    have hin7 : (H k 7).InView cfg ⟨2⟩ := by
      have := inView (k := k) 7
      rwa [(vAt_first (by omega)).2 (by omega)] at this
    have hinput : (tr k 7).input = .timeout ⟨2⟩ := by
      rw [Kit.tr_step]; show input k 7 = _; rw [input_first (by omega)]; rfl
    obtain ⟨e', L', -, hout⟩ := (Kit.protocol input hv k 8).timeoutAnswered 7 _ ⟨2⟩ (Kit.getElem_H 7)
      (Or.inl ⟨hinput, by rw [Kit.upTo_H]; exact hin7⟩) (fun _ _ => ⟨trivial, trivial⟩)
    obtain ⟨_, hke⟩ := hk
    exact by_at hv (m := 8) (fun _ => by show tm 7 ≤ _; omega)
      (Or.inr (Or.inr ⟨_, Kit.sent_iff.mpr ⟨7, by omega, hout⟩, rfl, hke⟩))
  timeoutCertForward tc k hk n hin _ _ k' hk' _ := by
    refine Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate tc))
      (fun _ h => ⟨tc.view + 1, Or.inr (Or.inl ⟨tc, h, rfl⟩),
        show tc.view.toNat < tc.view.toNat + 1 by omega⟩) ?_
    simp only [net_time hv]
    obtain ⟨rfl, hu⟩ := received_tc (hin ▸ Trace.received_self _ n : (H k (n + 1)).Received _)
    exact by_same hv 13 (fun m hm => recv_first 13 (by omega) hm) (Or.inl hu)
  timeoutCatchUp k hk v hrep :=
    (Kit.catchUp_vacuous (N := net hv) (tm := tm) (fun _ _ => rfl) (fun _ _ _ => rfl) tm_succ 12
      (fun m vote hout _ => by rcases (sent_timeout hout).1 with rfl | rfl | rfl <;> omega) hrep).elim
  timeoutLockSpread tc k hk n hin k' hk' hm _ := by
    simp only [net_time hv]
    obtain ⟨rfl, hu⟩ := received_tc hin
    have hl : lag k' 0 = 0 := lag_member (by
      have : T.data.lock.data.epoch = ⟨0 + 1⟩ := cert_epoch 0
      rw [this] at hm; exact hm)
    have := at0_facts (4 + lag k' 0) (10 + lag k' 0) 0
    exact by_same hv 13 (fun m hm' => lockable_iff.mpr (Or.inr ⟨0, rfl, by omega⟩)) (Or.inl hu)
  epochChange c2 b hcm _ k hk n hbc k' hk' _ := by
    obtain ⟨hb, hc2⟩ := hbc
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc2
    have hbn : b.blockHeader.blockNumber = ⟨x + 1⟩ := by
      rw [← cert_number x]; exact (congrArg Vote2Data.blockNumber hcm.2).symm
    obtain ⟨rfl, -⟩ := block_of_number hb hbn
    have := classes k; have := classes k'
    have := at0_facts (c2At k) 11 x; have := at0_facts (ecAt k') 12 x
    have hEC : ∀ m, at0 (ecAt k') 12 x < m → ∃ c1, (H k' m).TookEpochChange cfg c1 (C2 x) (blk x) :=
      fun m hm => ⟨_, tookEpochChange_of x hm⟩
    cases x with
    | zero => exact by_same hv _ hEC (Or.inr (by rw [(at0_facts _ _ _).1 rfl]; omega))
    | succ x =>
      by_cases hn : at0 (ecAt k') 12 (x + 1) < n + 1
      · exact by_same hv _ hEC (Or.inl hn)
      · have := tm_mono (show 8 * (x + 1) + 11 ≤ n by omega)
        have := (tm_facts (8 * (x + 1) + 11)).2.2 (by omega)
        have := (tm_facts (8 * (x + 1) + 12)).2.2 (by omega)
        have := Nat.le_max_left (tm n) 36
        exact by_step hv (8 * (x + 1) + 12) (by omega) (hEC _ (by omega))
  proposalValid _ _ _ p _ _ := hv p
  validatedSound _ _ _ _ _ _ b _ := hv b
  validated k hk n s p vid hin _ _ := by
    simp only [net_time hv]
    obtain ⟨u, rfl, -, -, rfl, -⟩ := input_proposal hin
    rw [blk_view]
    have hval : ∀ m, at0 2 8 u < m → (H k m).Received (.blockValidated ⟨bv u⟩ (blockHash (blk u))) :=
      fun m hm => recv_phase (k := k) u 2 (by omega) hm
    cases u with
    | zero => exact by_gst hv (hval 12 (by simp [at0]))
    | succ u =>
      rw [(at0_facts _ _ _).2 (by omega)]
      have := (tm_facts (8 * (u + 1) + 7)).2.2 (by omega)
      have := (tm_facts (8 * (u + 1) + 8)).2.2 (by omega)
      have := Nat.le_max_left (tm (8 * (u + 1) + 7)) 36
      exact by_step hv (8 * (u + 1) + 8) (by omega) (hval _ (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
  header k hk n p _ _ hready := by simp only [net_time hv]; exact header_arrives hv hready
  timeUnbounded _ _ T := ⟨T + 1, by have := tm_ge (T + 1); show T < tm (T + 1); omega⟩
  timerNotEarly k hk n m v hin hnot _ hinm := timer_not_early hin hnot hinm
  timerFires k hk n v hin _ := timer_fires n hin

theorem rotation : LeaderRotation C leader := fun _ v => ⟨v, Nat.le_refl _, a, rfl, Or.inl rfl, Or.inl rfl⟩

include hv in
/-- **The liveness premises can be met together, with a first epoch change that splits the nodes before GST.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv) 36 4 33 ∧ Prompt (net hv) 0
      ∧ 8 * 4 + 3 * 0 < 33 ∧ LeaderRotation C leader :=
  ⟨cfg_coherent, sync hv, prompt_of_machine _ 0 (fun k _ => ⟨input k, rfl⟩)
    (fun _ hk => Kit.steady_of_uniform (fun _ _ _ h => h) hk), by decide, rotation⟩

include hv in
/--
And the first epoch change splits the nodes: at the first timer `a` and `nw` are
in epoch two and `b` and `c` in epoch one, and so are their timeout votes; `b`
answers the re-vote request, yet no `Cert1` forms from it; nobody proposes in the
view; no timeout certificate forms before the second timer, when every node votes
in epoch two.
-/
theorem split_boundary :
    (eAt a 7 = 2 ∧ eAt nw 7 = 2 ∧ eAt b 7 = 1 ∧ eAt c 7 = 1)
      ∧ (∀ k, Output.send (.timeoutVote ⟨⟨⟨eAt k 7⟩, certOf (blk 0)⟩, ⟨2⟩, k⟩) ∈ (tr k 7).output)
      ∧ (∃ j, Output.send (.vote1 ⟨(certOf (blk 0)).data, ⟨2⟩, b⟩) ∈ (tr b j).output)
      ∧ (∀ k n, ¬ (H k n).HasCert1 cfg ⟨(certOf (blk 0)).data, ⟨2⟩⟩)
      ∧ (∀ k j (p : Proposal), Output.send (.proposal p) ∈ (tr k j).output → p.viewNumber ≠ ⟨2⟩)
      ∧ (∀ k n tc, n ≤ 13 → ¬ (H k n).Received (.timeoutCertificate tc))
      ∧ (∀ k, Output.send (.timeoutVote ⟨⟨⟨2⟩, certOf (blk 0)⟩, ⟨2⟩, k⟩) ∈ (tr k 12).output) := by
  refine ⟨⟨rfl, rfl, rfl, rfl⟩, times_out_first, ?_, fun k n hc => ?_, fun k j p hp hvw => ?_,
    fun k n tc hn hr => ?_, tc_votes⟩
  · obtain ⟨j, -, hj⟩ := votes1R hv b (by simp [lag]; decide) (by simp [Early]; decide)
    exact ⟨j, hj⟩
  · rcases hasCert1 hc with h | ⟨u, h, -⟩
    · have := congrArg (·.view.toNat) h
      exact absurd this (by show ¬ (2 : Nat) = 0; omega)
    · have := congrArg (·.view.toNat) h; simp only [cert_view] at this
      have := bv_facts u
      omega
  · obtain ⟨-, u, rfl, -⟩ := sent_proposal hv hp
    rw [blk_view] at hvw
    have := view_inj hvw
    have := bv_facts u
    omega
  · obtain ⟨-, hu⟩ := received_tc hr
    omega

include hv in
/--
After GST every epoch changes without a view timer: `a` asks for a re-vote on each
later epoch's last block and proposes the next epoch's first block in the same
view, on the last block's own `Cert1` and with no timeout evidence, and no node
times out after the second timer.
-/
theorem direct_after_gst :
    (∀ u, 1 ≤ u → (∃ j, Output.send (.revote (R u)) ∈ (tr a j).output)
      ∧ (∃ j, Output.send (.proposal (blk (u + 1))) ∈ (tr a j).output)
      ∧ (R u).view = (blk (u + 1)).viewNumber ∧ EntersEpoch cfg (blk (u + 1))
      ∧ (blk (u + 1)).parentCert = certOf (blk u) ∧ (blk (u + 1)).timeoutEvidence = none)
    ∧ (∀ k j (vote : TimeoutVote), Output.send (.timeoutVote vote) ∈ (tr k j).output → j ≤ 12) := by
  refine ⟨fun u hu => ?_, fun k j vote hx => by have := (sent_timeout hx).1; omega⟩
  obtain ⟨u, rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
  exact ⟨let ⟨j, _, hj⟩ := revotes hv (u + 1); ⟨j, hj⟩, let ⟨j, _, hj⟩ := proposes hv (u + 2); ⟨j, hj⟩,
    by rw [blk_view]; rfl, blk_enters (u + 1), blk_parent (u + 2), rfl⟩

include hv in
/-- So every honest node keeps deciding, after GST. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : C.Honest k) :
    (net hv).DecidesAfter k hk t := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met hv
  exact (Liveness.chainGrows (cfg := cfg) (leader := leader) (C := C) (net hv) 36 4 0 33
    hc hcf hs hp hb hr).2 t k hk (Or.inl (Kit.steady_of_uniform (fun _ _ _ h => h) hk))

end Net

end SplitWitness
end NewProtocolImpl
