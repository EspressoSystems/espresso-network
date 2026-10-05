module

public import NewProtocolImpl.WitnessKit
public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.FiveNodes

/-!
# A network whose epoch ends on the re-vote's `Cert2`

The case re-votes exist for: an epoch's last block whose `Cert1` arrives only after
every node timed its view out. Nobody votes2 at the block's own view, since a node
votes2 only at a view it has not timed out (`SafeHistory.vote2BeforeTimeout`), so
the block never gets a `Cert2` there. `a`, the leader of the next view, asks for a
re-vote once it can lock on the block. The committee votes1 on the block again,
locks on the re-vote's `Cert1`, and votes2 on it. The re-vote's `Cert2` is the only
`Cert2` over the block, and the epoch change carries it (`late_cert`).

The next epoch's first block names the last block by its own `Cert1`, at the
block's view (`OpensEpochJustified`). The re-vote's `Cert1` moved every node to the
view after the re-vote's, so that view times out before the block can be proposed,
behind the timeout certificate. `a` asks for a second re-vote, on the re-vote's
`Cert1`, in that view; it arrives after the epoch change, so nobody answers it.

GST is at the first timer, so the first `Cert1` may arrive that late. After the
first epoch every epoch changes as in `NewProtocolImpl.EpochWitness`, with the
re-vote request and the next epoch's first block in one view.

The nodes and committees are `NewProtocolImpl.FiveNodes`'s. Block `u + 1` is the
only block of epoch `u + 1`: the first at view one, the second at view four, and
each later one at the view after the one before (`bv`). A node receives eight inputs
to each block after the first, as in `EpochWitness` (`phase`), and fifteen in the
first epoch:

* the header, the proposal with the node's share (for `nw`, outside the
  committee, the proposal alone), the validity report, and the payload (for `nw`, a
  second validity report);
* the timer for view one, then the block's `Cert1`, and the timeout certificate;
* the header for view two, which `a` no longer needs: it has sent its request for
  the view;
* `a`'s re-vote request, the re-vote's `Cert1`, its `Cert2`, and the epoch change;
* `a`'s second request, the timer for view three, and its timeout certificate.

Times (`tm`): one unit a step, but the first timer fires at `τ = 33`, which is GST,
and the timer for view three `τ` after the nodes entered it. `Δ = 4` and `δ = 0`.

`BlockValid` is taken as a hypothesis, being opaque; `decides` also takes
`CollisionFree`.
-/

@[expose] public section

namespace NewProtocolImpl
namespace LateCertWitness

open NewProtocol History
open FourNodes (hdr)
open Kit (received received_mono upTo_H upTo_self getElem_H tr_step sent_iff view_inj number_inj lockLE_antisymm)
open FiveNodes (a b c nw d third leader C members_quorum d_faulty quorum_a anchorB certOf cfg cfg_coherent
  lag lag_cases lag_member lag_a epochOf_one_height last_block)

/-! ## Blocks and the schedule -/

/-- The view of block `u + 1`: one for the first, `u + 3` for each later one. -/
def bv : Nat → Nat
  | 0 => 1
  | u + 1 => u + 4

theorem bv_facts (u : Nat) : (u = 0 → bv u = 1) ∧ (1 ≤ u → bv u = u + 3) := by
  cases u with
  | zero => exact ⟨fun _ => rfl, fun h => absurd h (by omega)⟩
  | succ u => exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩

theorem bv_inj {x y : Nat} (h : bv x = bv y) : x = y := by
  have := bv_facts x; have := bv_facts y; omega

/-- The view of the `Cert2` over block `u + 1`: the re-vote's for the first, the block's own for each later one. -/
def cv : Nat → Nat
  | 0 => 2
  | u + 1 => u + 4

theorem cv_facts (u : Nat) : (u = 0 → cv u = 2) ∧ (1 ≤ u → cv u = u + 3) := by
  cases u with
  | zero => exact ⟨fun _ => rfl, fun h => absurd h (by omega)⟩
  | succ u => exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩

/--
Block `u + 1`, at view `bv u`, the only block of epoch `u + 1`, on the one before.
The second opens its epoch behind the timeout certificate for view three, whose lock
is the re-vote's `Cert1`.
-/
def blk : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨1⟩, certOf anchorB, none, ⟨0⟩⟩
  | 1 => ⟨hdr 2, ⟨4⟩, ⟨2⟩, certOf (blk 0), some ⟨⟨⟨2⟩, ⟨(certOf (blk 0)).data, ⟨2⟩⟩⟩, ⟨3⟩⟩, ⟨0⟩⟩
  | u + 2 => ⟨hdr (u + 3), ⟨u + 5⟩, ⟨u + 3⟩, certOf (blk (u + 1)), none, ⟨0⟩⟩

/-- The re-vote's `Cert1`: the first block's data, at view two. -/
def CR : Cert1 := ⟨(certOf (blk 0)).data, ⟨2⟩⟩

/-- The timeout certificate for view one, of epoch one, locked on genesis. -/
def T1 : TimeoutCert := ⟨⟨⟨1⟩, certOf anchorB⟩, ⟨1⟩⟩

/-- The timeout certificate for view three, of epoch two, locked on the re-vote's `Cert1`. -/
def T3 : TimeoutCert := ⟨⟨⟨2⟩, CR⟩, ⟨3⟩⟩

/-- The parent of `blk u`. -/
def parentOf : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

/-- The `Cert2` over `blk u`, at view `cv u`. -/
def C2 (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨cv u⟩⟩

/-- The re-vote request `a` sends in the view after `blk u`, once it can lock on it. -/
def R (u : Nat) : RevoteRequest := ⟨certOf (blk u), ⟨bv u + 1⟩, none⟩

/-- `a`'s second request, on the re-vote's `Cert1`, in view three. -/
def R2 : RevoteRequest := ⟨CR, ⟨3⟩, none⟩

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
theorem blk_evidence (u : Nat) : (blk u).timeoutEvidence = none ∨ (u = 1 ∧ (blk u).timeoutEvidence = some T3) := by
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
  | 1 => exact ⟨by decide, Or.inr ⟨T3, rfl, rfl⟩, (epochOf_one_height (n := 2) (by omega)).symm, rfl⟩
  | u + 2 =>
    refine ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_, ?_⟩
    · show (blk (u + 1)).viewNumber.toNat < u + 5
      rw [blk_view]; show u + 4 < u + 5; omega
    · show (blk (u + 1)).viewNumber + 1 = ⟨u + 5⟩
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
  | 0 => phase k 0 0
  | 1 => phase k 0 1
  | 2 => phase k 0 2
  | 3 => phase k 0 4
  | 4 => .timeout ⟨1⟩
  | 5 => .certificate1 (certOf (blk 0))
  | 6 => .timeoutCertificate T1
  | 7 => .headerBuilt ⟨2⟩ (blockHash anchorB) (hdr 1)
  | 8 => .revote a (R 0)
  | 9 => .certificate1 CR
  | 10 => .certificate2 (C2 0)
  | 11 => .epochChange (certOf (blk 0)) (C2 0) (blk 0)
  | 12 => .revote a R2
  | 13 => .timeout ⟨3⟩
  | _ => .timeoutCertificate T3

/-- Fifteen steps to the first epoch, then eight to each later block. -/
def input (k : PubKey) (n : Nat) : Input :=
  if n < 15 then first k n else phase k ((n - 7) / 8) ((n - 7) % 8)

/-- Node `k` running the machine on its schedule, and its history after `n` steps. -/
local notation "tr" => Kit.tr cfg leader input

local notation "H" => Kit.H cfg leader input

theorem input_first {k : PubKey} {n : Nat} (h : n < 15) : input k n = first k n := ite_eq_left h

theorem input_at (k : PubKey) (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) : input k (8 * u + r + 7) = phase k u r := by
  unfold input
  rw [ite_eq_right (by omega), show (8 * u + r + 7 - 7) / 8 = u by omega, show (8 * u + r + 7 - 7) % 8 = r by omega]

/-- A first-epoch step `x`, or step `8u + y` of a later block `u + 1`. -/
def at0 (x y u : Nat) : Nat := if u = 0 then x else 8 * u + y

theorem at0_facts (x y u : Nat) : (u = 0 → at0 x y u = x) ∧ (1 ≤ u → at0 x y u = 8 * u + y) :=
  ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

/-- A step of the first epoch, or step `r` of a later block `u + 1`. -/
theorem steps (n : Nat) : n < 15 ∨ ∃ u r, 1 ≤ u ∧ r < 8 ∧ n = 8 * u + r + 7 :=
  if h : n < 15 then Or.inl h else Or.inr ⟨(n - 7) / 8, (n - 7) % 8, by omega, Nat.mod_lt _ (by omega), by omega⟩

theorem r15 (n : Nat) (h : n < 15) :
    n = 0 ∨ n = 1 ∨ n = 2 ∨ n = 3 ∨ n = 4 ∨ n = 5 ∨ n = 6 ∨ n = 7 ∨ n = 8 ∨ n = 9 ∨ n = 10 ∨ n = 11
      ∨ n = 12 ∨ n = 13 ∨ n = 14 := by omega

theorem r8 (r : Nat) (hr : r < 8) :
    r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 ∨ r = 5 ∨ r = 6 ∨ r = 7 := by omega

/-! ## What the nodes receive -/

section Inputs

variable {k : PubKey}

theorem input_cases {n : Nat} {i : Input} (hi : input k n = i) :
    (n < 15 ∧ first k n = i) ∨ ∃ u r, 1 ≤ u ∧ r < 8 ∧ n = 8 * u + r + 7 ∧ phase k u r = i := by
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · exact Or.inl ⟨hn, by rw [← input_first hn]; exact hi⟩
  · exact Or.inr ⟨u, r, hu, hr, rfl, by rw [← input_at k u r hu hr]; exact hi⟩

theorem input_header {n : Nat} {v : ViewNumber} {h : BlockHash} {x : BlockHeader}
    (hi : input k n = .headerBuilt v h x) :
    (n = 7 ∧ v = ⟨2⟩ ∧ h = blockHash anchorB ∧ x = hdr 1)
      ∨ ∃ u, n = at0 0 7 u ∧ v = ⟨bv u⟩ ∧ h = blockHash (parentOf u) ∧ x = hdr (u + 1) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals first
      | exact Or.inl ⟨rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
      | exact Or.inr ⟨0, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact Or.inr ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal {n : Nat} {s : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal s p (some vid)) :
    ∃ u, n = at0 1 8 u ∧ lag k u = 0 ∧ s = a ∧ p = blk u ∧ vid = ⟨⟨bv u⟩, (blk u).payloadCommit⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals exact ⟨0, rfl, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal_none {n : Nat} {s : PubKey} {p : Proposal} (hi : input k n = .proposal s p none) : ∃ u, n = at0 1 8 u ∧ s = a ∧ p = blk u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals exact ⟨0, rfl, he.1.symm, he.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.symm⟩

/-- Every proposal a node receives is `a`'s block for its round, with a share or without. -/
theorem input_proposal_any {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ u, n = at0 1 8 u ∧ s = a ∧ p = blk u := by
  cases share with
  | some vid => obtain ⟨u, h1, -, h2, h3, -⟩ := input_proposal hi; exact ⟨u, h1, h2, h3⟩
  | none => exact input_proposal_none hi

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    (n = 9 ∧ x = CR) ∨ ∃ u, n = at0 5 10 u ∧ x = certOf (blk u) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals first
      | exact Or.inl ⟨rfl, he.symm⟩
      | exact Or.inr ⟨0, rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact Or.inr ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.symm⟩

/-- A payload arrives at the members of the block's committee only. -/
theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = at0 3 11 u ∧ lag k u = 0 ∧ v = ⟨bv u⟩ ∧ pc = (blk u).payloadCommit := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals exact ⟨0, rfl, hl, he.1.symm, he.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, hl, he.1.symm, he.2.symm⟩

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, n = at0 10 12 u ∧ x = C2 u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals exact ⟨0, rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.symm⟩

theorem input_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : input k n = .epochChange c1 c2 p) :
    ∃ u, n = at0 11 13 u ∧ c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals exact ⟨0, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hi : input k n = .revote s r) :
    (n = 12 ∧ s = a ∧ r = R2) ∨ ∃ u, n = at0 8 14 u ∧ s = a ∧ r = R u := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r', hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals first
      | exact Or.inl ⟨rfl, he.1.symm, he.2.symm⟩
      | exact Or.inr ⟨0, rfl, he.1.symm, he.2.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
    all_goals exact Or.inr ⟨u, by rw [(at0_facts _ _ _).2 hu] <;> omega, he.1.symm, he.2.symm⟩

/-- The timer fires twice, for views one and three. -/
theorem input_timeout {n : Nat} {v : ViewNumber} (hi : input k n = .timeout v) :
    (n = 4 ∧ v = ⟨1⟩) ∨ (n = 13 ∧ v = ⟨3⟩) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals first
      | exact Or.inl ⟨rfl, he.symm⟩
      | exact Or.inr ⟨rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he

theorem input_tc {n : Nat} {tc : TimeoutCert} (hi : input k n = .timeoutCertificate tc) :
    (n = 6 ∧ tc = T1) ∨ (n = 14 ∧ tc = T3) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
    all_goals first
      | exact Or.inl ⟨rfl, he.symm⟩
      | exact Or.inr ⟨rfl, he.symm⟩
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he

theorem input_oneHonest {n : Nat} {v : ViewNumber} (hi : input k n = .timeoutOneHonest v) : False := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hu, hr, rfl, he⟩
  · rcases lag_cases k 0 with hl | hl <;>
      rcases r15 _ hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [first, phase, hl] at he
  · rcases lag_cases k u with hl | hl <;>
      rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

theorem recv_first {n : Nat} (j : Nat) (hj : j < 15) (h : j < n) : (H k n).Received (first k j) :=
  received.mpr ⟨j, h, input_first hj⟩

theorem recv_later {n : Nat} (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) (h : 8 * u + r + 7 < n) :
    (H k n).Received (phase k u r) :=
  received.mpr ⟨_, h, input_at k u r hu hr⟩

/-- What a node received at step `r` of block `u + 1`, at step `x` of the first epoch. -/
theorem recv_u {n : Nat} (u x r : Nat) (hx : x < 15) (hr : r < 8) (hfx : first k x = phase k 0 r)
    (h : at0 x (r + 7) u < n) : (H k n).Received (phase k u r) := by
  cases u with
  | zero =>
    rw [(at0_facts _ _ _).1 rfl] at h
    have := recv_first (k := k) x hx h
    rwa [hfx] at this
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)] at h
    exact recv_later (u + 1) r (by omega) hr (by omega)

theorem recv_cert1 {n : Nat} (u : Nat) (h : at0 5 10 u < n) : (H k n).Received (.certificate1 (certOf (blk u))) :=
  recv_u u 5 3 (by omega) (by omega) rfl h

theorem recv_c2 {n : Nat} (u : Nat) (h : at0 10 12 u < n) : (H k n).Received (.certificate2 (C2 u)) :=
  recv_u u 10 5 (by omega) (by omega) rfl h

theorem recv_ec {n : Nat} (u : Nat) (h : at0 11 13 u < n) :
    (H k n).Received (.epochChange (certOf (blk u)) (C2 u) (blk u)) :=
  recv_u u 11 6 (by omega) (by omega) rfl h

theorem recv_revote {n : Nat} (u : Nat) (h : at0 8 14 u < n) : (H k n).Received (.revote a (R u)) :=
  recv_u u 8 7 (by omega) (by omega) rfl h

theorem recv_CR {n : Nat} (h : 9 < n) : (H k n).Received (.certificate1 CR) := recv_first 9 (by omega) h

theorem recv_R2 {n : Nat} (h : 12 < n) : (H k n).Received (.revote a R2) := recv_first 12 (by omega) h

theorem recv_T1 {n : Nat} (h : 6 < n) : (H k n).Received (.timeoutCertificate T1) := recv_first 6 (by omega) h

theorem recv_T3 {n : Nat} (h : 14 < n) : (H k n).Received (.timeoutCertificate T3) := recv_first 14 (by omega) h

theorem hasProposal {n : Nat} {x : Block} (hb : (H k n).HasProposal cfg x) :
    x = anchorB ∨ ∃ u, x = blk u ∧ at0 1 8 u < n := by
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
    have := at0_facts 11 13 u
    have := at0_facts 1 8 u
    exact Or.inr ⟨u, rfl, by omega⟩

/-- Every node holds a proposal from the step after it arrives: with a share, or without. -/
theorem hasProposal_of {n : Nat} (u : Nat) (h : at0 1 8 u < n) : (H k n).HasProposal cfg (blk u) := by
  have hr := recv_u (k := k) u 1 1 (by omega) (by omega) rfl h
  simp only [phase] at hr
  rcases lag_cases k u with hl | hl
  · rw [hl, ite_eq_left rfl] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)
  · rw [hl, ite_eq_right (by decide)] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)

theorem hasParent_of {n : Nat} (u : Nat) (h : at0 0 7 u < n) : (H k n).HasProposal cfg (parentOf u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u =>
    refine hasProposal_of u ?_
    have := at0_facts 1 8 u; have := at0_facts 0 7 (u + 1); omega

/-- Which held proposal a height names. -/
theorem block_of_number {n x : Nat} {y : Block} (hb : (H k n).HasProposal cfg y)
    (hv : y.blockHeader.blockNumber = ⟨x + 1⟩) : y = blk x ∧ at0 1 8 x < n := by
  rcases hasProposal hb with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [blk_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

theorem hasCert1 {n : Nat} {x : Cert1} (hc : (H k n).HasCert1 cfg x) :
    x = certOf anchorB ∨ (x = CR ∧ 9 < n) ∨ ∃ u, x = certOf (blk u) ∧ at0 5 10 u < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    rcases input_cert1 hji with ⟨rfl, rfl⟩ | ⟨u, rfl, rfl⟩
    · exact Or.inr (Or.inl ⟨rfl, hj⟩)
    · exact Or.inr (Or.inr ⟨u, rfl, hj⟩)
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl, -, -⟩ := input_epochChange hji
    have := at0_facts 11 13 u
    have := at0_facts 5 10 u
    exact Or.inr (Or.inr ⟨u, rfl, by omega⟩)

theorem hasCert1_of {n : Nat} (u : Nat) (h : at0 5 10 u < n) : (H k n).HasCert1 cfg (certOf (blk u)) :=
  Or.inr (Or.inl (recv_cert1 u h))

theorem hasCR {n : Nat} (h : 9 < n) : (H k n).HasCert1 cfg CR := Or.inr (Or.inl (recv_CR h))

theorem hasCert2 {n : Nat} {x : Cert2} (hc : (H k n).HasCert2 x) : ∃ u, x = C2 u ∧ at0 10 12 u < n := by
  rcases hc with hr | ⟨c1, p, hr⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert2 hji
    exact ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, -, rfl, -⟩ := input_epochChange hji
    have := at0_facts 11 13 u
    have := at0_facts 10 12 u
    exact ⟨u, rfl, by omega⟩

theorem hasCert2_of {n : Nat} (u : Nat) (h : at0 10 12 u < n) : (H k n).HasCert2 (C2 u) :=
  Or.inl (recv_c2 u h)

theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis
      ∨ ∃ u, v = ⟨bv u⟩ ∧ pc = (blk u).payloadCommit ∧ lag k u = 0 ∧ at0 3 11 u < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, hl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hl, hj⟩

theorem payload_of {n : Nat} (u : Nat) (hl : lag k u = 0) (h : at0 3 11 u < n) :
    (H k n).HasPayload cfg (blk u).viewNumber (blk u).payloadCommit := by
  have hr := recv_u (k := k) u 3 4 (by omega) (by omega) rfl h
  simp only [phase, hl, ite_eq_left] at hr
  rw [blk_view]; exact Or.inr hr

theorem received_tc {n : Nat} {tc : TimeoutCert} (hr : (H k n).Received (.timeoutCertificate tc)) :
    (tc = T1 ∧ 6 < n) ∨ (tc = T3 ∧ 14 < n) := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  rcases input_tc hji with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
  · exact Or.inl ⟨rfl, hj⟩
  · exact Or.inr ⟨rfl, hj⟩

theorem received_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hr : (H k n).Received (.revote s r)) :
    (s = a ∧ r = R2 ∧ 12 < n) ∨ ∃ u, s = a ∧ r = R u ∧ at0 8 14 u < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  rcases input_revote hji with ⟨rfl, rfl, rfl⟩ | ⟨u, rfl, rfl, rfl⟩
  · exact Or.inl ⟨rfl, rfl, hj⟩
  · exact Or.inr ⟨u, rfl, rfl, hj⟩

theorem epochChange_wellFormed (u : Nat) : EpochChangeWellFormed cfg (certOf (blk u)) (C2 u) (blk u) := by
  refine ⟨?_, rfl, rfl, rfl, blk_wellFormed u, ?_⟩
  · rw [blk_view]; show bv u ≤ cv u; have := bv_facts u; have := cv_facts u; omega
  · rw [blk_number]; exact last_block (by omega)

theorem tookEpochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (ht : (H k n).TookEpochChange cfg c1 c2 p) :
    ∃ u, c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u ∧ at0 11 13 u < n := by
  obtain ⟨j, hj, hji⟩ := received.mp ht.1
  obtain ⟨u, rfl, rfl, rfl, rfl⟩ := input_epochChange hji
  exact ⟨u, rfl, rfl, rfl, hj⟩

theorem tookEpochChange_of {n : Nat} (u : Nat) (h : at0 11 13 u < n) :
    (H k n).TookEpochChange cfg (certOf (blk u)) (C2 u) (blk u) :=
  ⟨recv_ec u h, epochChange_wellFormed u⟩

/-- The step after which a node can lock on block `u + 1`: its payload, or, outside the committee, the epoch change. -/
def lk (k : PubKey) (u : Nat) : Nat := at0 (if lag k 0 = 0 then 5 else 11) (11 + lag k u) u

theorem lk_facts (k : PubKey) (u : Nat) :
    (u = 0 → lag k 0 = 0 → lk k u = 5) ∧ (u = 0 → lag k 0 = 2 → lk k u = 11)
      ∧ (1 ≤ u → lk k u = 8 * u + 11 + lag k u) := by
  unfold lk
  refine ⟨fun hu hl => ?_, fun hu hl => ?_, fun hu => ?_⟩
  · rw [(at0_facts _ _ _).1 hu, ite_eq_left hl]
  · rw [(at0_facts _ _ _).1 hu, ite_eq_right (by rw [hl]; decide)]
  · rw [(at0_facts _ _ _).2 hu]; omega

/--
What a node can lock on after `n` steps: genesis; the first block from its `Cert1`
on, outside its committee from the epoch change; the re-vote's `Cert1` from the
step after it arrives, in the committee only; and each later block from the step
after its payload, or, outside its committee, after the epoch change.
-/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x ↔ x = certOf anchorB ∨ (x = CR ∧ lag k 0 = 0 ∧ 9 < n)
      ∨ ∃ u, x = certOf (blk u) ∧ lk k u < n := by
  constructor
  · rintro (rfl | ⟨hc, y, hy, ⟨hyv, hcd⟩, hp⟩ | ⟨c2, p, ht⟩)
    · exact Or.inl rfl
    · rcases hasCert1 hc with rfl | ⟨rfl, h9⟩ | ⟨u, rfl, hcu⟩
      · exact Or.inl rfl
      · -- The re-vote's `Cert1` is over the first block.
        have hyn : y.blockHeader.blockNumber = ⟨0 + 1⟩ := by
          have := congrArg Vote1Data.blockNumber hcd
          rw [← cert_number 0]; exact this.symm
        obtain ⟨rfl, -⟩ := block_of_number hy hyn
        rcases hasPayload hp with hg | ⟨u', hv, -, hl, hlt⟩
        · rw [blk_view] at hg; exact absurd (view_inj hg) (by decide)
        · rw [blk_view] at hv
          obtain rfl : 0 = u' := bv_inj (view_inj hv)
          exact Or.inr (Or.inl ⟨rfl, hl, h9⟩)
      · have hyn : y.blockHeader.blockNumber = ⟨u + 1⟩ := by
          have := congrArg Vote1Data.blockNumber hcd
          rw [← cert_number u]; exact this.symm
        obtain ⟨rfl, -⟩ := block_of_number hy hyn
        rcases hasPayload hp with hg | ⟨u', hv, -, hl, hlt⟩
        · rw [blk_view] at hg; have := bv_facts u; exact absurd (view_inj hg) (by omega)
        · rw [blk_view] at hv
          obtain rfl : u = u' := bv_inj (view_inj hv)
          refine Or.inr (Or.inr ⟨u, rfl, ?_⟩)
          have := lk_facts k u
          have := at0_facts 3 11 u
          have := at0_facts 5 10 u
          cases u with
          | zero => omega
          | succ u => omega
    · obtain ⟨u, rfl, -, -, hlt⟩ := tookEpochChange ht
      refine Or.inr (Or.inr ⟨u, rfl, ?_⟩)
      have := lk_facts k u
      have := at0_facts 11 13 u
      have := lag_cases k u
      have := lag_cases k 0
      cases u with
      | zero => omega
      | succ u => omega
  · rintro (rfl | ⟨rfl, hl, h9⟩ | ⟨u, rfl, hlt⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hasCR h9, blk 0, hasProposal_of 0 (by rw [(at0_facts _ _ _).1 rfl]; omega),
        ⟨show (1 : Nat) ≤ 2 by omega, rfl⟩, payload_of 0 hl (by rw [(at0_facts _ _ _).1 rfl]; omega)⟩)
    · have hf := lk_facts k u
      rcases lag_cases k u with hl | hl
      · refine Or.inr (Or.inl ⟨hasCert1_of u ?_, blk u, hasProposal_of u ?_, ⟨Nat.le_refl _, rfl⟩,
          payload_of u hl ?_⟩) <;>
        · have := at0_facts 5 10 u; have := at0_facts 1 8 u; have := at0_facts 3 11 u
          cases u with
          | zero => omega
          | succ u => omega
      · refine Or.inr (Or.inr ⟨_, _, tookEpochChange_of u ?_⟩)
        have := at0_facts 11 13 u
        cases u with
        | zero => omega
        | succ u => omega

/-! ### The view, the epoch and the lock after `n` steps -/

/--
The view every node is in after `n` steps: view two from the timeout certificate
for view one, view three from the re-vote's `Cert1`, and from the timeout
certificate for view three on as in `EpochWitness`, three views later.
-/
def vAt (n : Nat) : Nat :=
  if n < 15 then (if n ≤ 5 then 1 else if n ≤ 9 then 2 else 3)
  else if 4 ≤ (n - 7) % 8 then (n - 7) / 8 + 4 else (n - 7) / 8 + 3

theorem vAt_first {n : Nat} (h : n < 15) :
    (n ≤ 5 → vAt n = 1) ∧ (6 ≤ n → n ≤ 9 → vAt n = 2) ∧ (10 ≤ n → vAt n = 3) := by
  unfold vAt
  rw [ite_eq_left h]
  refine ⟨fun h1 => ite_eq_left h1, fun h1 h2 => ?_, fun h1 => ?_⟩
  · rw [ite_eq_right (by omega), ite_eq_left h2]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega)]

theorem vAt_later (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) :
    (r < 4 → vAt (8 * u + r + 7) = u + 3) ∧ (4 ≤ r → vAt (8 * u + r + 7) = u + 4) := by
  unfold vAt
  rw [ite_eq_right (by omega), show (8 * u + r + 7 - 7) % 8 = r by omega, show (8 * u + r + 7 - 7) / 8 = u by omega]
  exact ⟨fun h => ite_eq_right (by omega), fun h => ite_eq_left h⟩

/-- `vAt n` against the steps a ground arrives at, as facts `omega` reads. -/
theorem vAt_ge (n : Nat) : 1 ≤ vAt n ∧ (∀ u, at0 5 10 u < n → bv u + 1 ≤ vAt n) ∧ (9 < n → 3 ≤ vAt n)
    ∧ (6 < n → 2 ≤ vAt n) ∧ (14 < n → 4 ≤ vAt n) ∧ (∀ u, at0 11 13 u < n → cv u + 1 ≤ vAt n) := by
  rcases steps n with hn | ⟨u', r, hu', hr, rfl⟩
  · have := vAt_first hn
    refine ⟨by omega, fun u hu => ?_, fun h => by omega, fun h => by omega, fun h => by omega, fun u hu => ?_⟩
    · have := at0_facts 5 10 u; have := bv_facts u; omega
    · have := at0_facts 11 13 u; have := cv_facts u; omega
  · have := vAt_later u' r hu' hr
    refine ⟨by omega, fun u hu => ?_, fun h => by omega, fun h => by omega, fun h => by omega, fun u hu => ?_⟩
    · have := at0_facts 5 10 u; have := bv_facts u; omega
    · have := at0_facts 11 13 u; have := cv_facts u; omega

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ vAt n := by
  obtain ⟨h1, hc, hCR, hT1, hT3, hec⟩ := vAt_ge n
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, hte, rfl⟩
  · rcases hasCert1 hx with rfl | ⟨rfl, h9⟩ | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ _; omega
    · show 2 + 1 ≤ _; exact hCR h9
    · show (certOf (blk u)).view.toNat + 1 ≤ _
      rw [cert_view]; exact hc u hlt
  · rcases received_tc htc with ⟨rfl, h6⟩ | ⟨rfl, h14⟩
    · show 1 + 1 ≤ _; exact hT1 h6
    · show 3 + 1 ≤ _; exact hT3 h14
  · obtain ⟨u, -, rfl, -, hlt⟩ := tookEpochChange hte
    show cv u + 1 ≤ _
    exact hec u hlt

theorem inView (n : Nat) : (H k n).InView cfg ⟨vAt n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · obtain ⟨h1, h2, h3⟩ := vAt_first hn
    by_cases r10 : 10 ≤ n
    · rw [h3 r10]; exact Or.inl ⟨CR, hasCR (by omega), rfl⟩
    by_cases r6 : 6 ≤ n
    · rw [h2 r6 (by omega)]
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
      | zero => exact Or.inr (Or.inl ⟨T3, recv_T3 (by omega), rfl⟩)
      | succ u' =>
        exact Or.inl ⟨certOf (blk (u' + 1)),
          hasCert1_of (u' + 1) (by rw [(at0_facts _ _ _).2 (by omega)]; omega), by rw [cert_view]; rfl⟩

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨vAt n⟩ :=
  Kit.inView_unique hv (inView n)

theorem viewOf_eq (n : Nat) : viewOf cfg (H k n) = ⟨vAt n⟩ := viewOf_of_inView (inView n)

/-- The epoch every node is in after `n` steps: the next one from each epoch change on. -/
def eAt (n : Nat) : Nat :=
  if n < 15 then (if n ≤ 11 then 1 else 2) else if 7 ≤ (n - 7) % 8 then (n - 7) / 8 + 2 else (n - 7) / 8 + 1

theorem eAt_first {n : Nat} (h : n < 15) : (n ≤ 11 → eAt n = 1) ∧ (12 ≤ n → eAt n = 2) := by
  unfold eAt
  rw [ite_eq_left h]
  exact ⟨fun h1 => ite_eq_left h1, fun h1 => ite_eq_right (by omega)⟩

theorem eAt_later (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) :
    (7 ≤ r → eAt (8 * u + r + 7) = u + 2) ∧ (r < 7 → eAt (8 * u + r + 7) = u + 1) := by
  unfold eAt
  rw [ite_eq_right (by omega), show (8 * u + r + 7 - 7) % 8 = r by omega, show (8 * u + r + 7 - 7) / 8 = u by omega]
  exact ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

/-- `eAt` against the steps a ground arrives at, as facts `omega` reads. -/
theorem eAt_ge (n : Nat) : 1 ≤ eAt n ∧ (∀ u, at0 11 13 u < n → u + 2 ≤ eAt n) ∧ (14 < n → 2 ≤ eAt n)
    ∧ (∀ u, at0 5 10 u < n → u + 1 ≤ eAt n) := by
  rcases steps n with hn | ⟨u', r, hu', hr, rfl⟩
  · have := eAt_first hn
    refine ⟨by omega, fun u hu => ?_, fun h => by omega, fun u hu => ?_⟩ <;> have := at0_facts 11 13 u <;>
      have := at0_facts 5 10 u <;> omega
  · have := eAt_later u' r hu' hr
    refine ⟨by omega, fun u hu => ?_, fun h => by omega, fun u hu => ?_⟩ <;> have := at0_facts 11 13 u <;>
      have := at0_facts 5 10 u <;> omega

theorem epochGround {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e.toNat ≤ eAt n := by
  obtain ⟨h1, hec, hT3, hc⟩ := eAt_ge n
  rcases he with rfl | ⟨c1, c2, p, ht, rfl⟩ | ⟨tc, htc', rfl⟩ | ⟨x, hx, -, rfl⟩
  · exact h1
  · obtain ⟨u, -, rfl, -, hlt⟩ := tookEpochChange ht
    show (certOf (blk u)).data.epoch.toNat + 1 ≤ _
    rw [cert_epoch]; exact hec u hlt
  · rcases received_tc htc' with ⟨rfl, -⟩ | ⟨rfl, h14⟩
    · exact h1
    · exact hT3 h14
  · rcases hasCert1 hx with rfl | ⟨rfl, -⟩ | ⟨u, rfl, hlt⟩
    · exact h1
    · exact h1
    · rw [cert_epoch]; exact hc u hlt

theorem c2_epoch (u : Nat) : (⟨u + 2⟩ : EpochNumber) = (C2 u).data.epoch + 1 := by
  show _ = (certOf (blk u)).data.epoch + 1
  rw [cert_epoch]; rfl

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨eAt n⟩ := by
  refine ⟨?_, fun e he => epochGround he⟩
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · obtain ⟨h1, h2⟩ := eAt_first hn
    by_cases h12 : 12 ≤ n
    · rw [h2 h12]
      exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of 0 (by rw [(at0_facts _ _ _).1 rfl]; omega), c2_epoch 0⟩)
    · rw [h1 (by omega)]; exact Or.inl rfl
  · obtain ⟨h7, h0⟩ := eAt_later u r hu hr
    by_cases r7 : 7 ≤ r
    · rw [h7 r7]
      exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u (by rw [(at0_facts _ _ _).2 hu]; omega), c2_epoch u⟩)
    · rw [h0 (by omega)]
      obtain ⟨u', rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
      refine Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u' ?_, c2_epoch u'⟩)
      have := at0_facts 11 13 u'
      omega

theorem inEpoch_eq {n : Nat} {e : EpochNumber} (he : (H k n).InEpoch cfg e) : e = ⟨eAt n⟩ :=
  Liveness.inEpoch_unique he (inEpoch n)

theorem epochOf_eq (n : Nat) : epochOfHistory cfg (H k n) = ⟨eAt n⟩ :=
  inEpoch_eq (inEpoch_epochOfHistory cfg _)

theorem notBehind {n : Nat} {e : EpochNumber} (h : eAt n ≤ e.toNat) : NotBehind cfg (H k n) e :=
  Kit.notBehind_of (inEpoch n) h

theorem notBehind_le {n : Nat} {e : EpochNumber} (h : NotBehind cfg (H k n) e) : eAt n ≤ e.toNat :=
  Kit.le_of_notBehind (inEpoch n) h

/-- What a node is locked on after the re-vote: the re-vote's `Cert1` in the first committee, the block's own outside it. -/
def base (k : PubKey) : Cert1 := if lag k 0 = 0 then CR else certOf (blk 0)

/--
The lock after `n` steps. In the first epoch, a member of the committee locks on
the block from its `Cert1` and on the re-vote's `Cert1` from that one; `nw` locks
on the block from the epoch change. Then each block from the step after its
payload, outside its committee from the epoch change.
-/
def lockAt (k : PubKey) (n : Nat) : Cert1 :=
  if n < 15 then
    (if lag k 0 = 0 then (if n ≤ 5 then certOf anchorB else if n ≤ 9 then certOf (blk 0) else CR)
      else if n ≤ 11 then certOf anchorB else certOf (blk 0))
  else if 5 + lag k ((n - 7) / 8) ≤ (n - 7) % 8 then certOf (blk ((n - 7) / 8))
  else if (n - 7) / 8 = 1 then base k else certOf (blk ((n - 7) / 8 - 1))

theorem lockAt_later (u r : Nat) (hu : 1 ≤ u) (hr : r < 8) :
    (5 + lag k u ≤ r → lockAt k (8 * u + r + 7) = certOf (blk u))
      ∧ (r < 5 + lag k u → u = 1 → lockAt k (8 * u + r + 7) = base k)
      ∧ (r < 5 + lag k u → 2 ≤ u → lockAt k (8 * u + r + 7) = certOf (blk (u - 1))) := by
  unfold lockAt
  rw [ite_eq_right (by omega), show (8 * u + r + 7 - 7) / 8 = u by omega, show (8 * u + r + 7 - 7) % 8 = r by omega]
  refine ⟨fun h => ite_eq_left h, fun h h1 => ?_, fun h h2 => ?_⟩
  · rw [ite_eq_right (by omega), ite_eq_left h1]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega)]

/-- From the timer for view three until its timeout certificate, every node is locked as after the re-vote. -/
theorem lockAt_three {n : Nat} (h1 : 12 ≤ n) (h2 : n < 15) : lockAt k n = base k := by
  unfold lockAt base
  rw [ite_eq_left h2]
  by_cases hl : lag k 0 = 0
  · rw [ite_eq_left hl, ite_eq_left hl, ite_eq_right (by omega), ite_eq_right (by omega)]
  · rw [ite_eq_right hl, ite_eq_right hl, ite_eq_right (by omega)]

theorem lockAt_one {n : Nat} (h : n ≤ 5) : lockAt k n = certOf anchorB := by
  unfold lockAt
  rw [ite_eq_left (by omega)]
  by_cases hl : lag k 0 = 0
  · rw [ite_eq_left hl, ite_eq_left h]
  · rw [ite_eq_right hl, ite_eq_left (by omega)]

theorem lockLE_blk {j m : Nat} (h : j ≤ m) : LockLE (certOf (blk j)) (certOf (blk m)) := by
  rw [LockLE, cert_epoch, cert_epoch, cert_view, cert_view]
  by_cases hjm : j = m
  · subst hjm; exact Or.inr ⟨rfl, Nat.le_refl _⟩
  · exact Or.inl (show j + 1 < m + 1 by omega)

theorem lockLE_refl (x : Cert1) : LockLE x x := Or.inr ⟨rfl, Nat.le_refl _⟩

theorem lockLE_anchor (m : Nat) : LockLE (certOf anchorB) (certOf (blk m)) := by
  rw [LockLE, cert_epoch, cert_view]
  cases m with
  | zero => exact Or.inr ⟨rfl, Nat.zero_le _⟩
  | succ m => exact Or.inl (show 1 < m + 1 + 1 by omega)

theorem lockLE_anchor_CR : LockLE (certOf anchorB) CR := Or.inr ⟨rfl, show (0 : Nat) ≤ 2 by omega⟩

theorem lockLE_blk0_CR : LockLE (certOf (blk 0)) CR := Or.inr ⟨rfl, show (1 : Nat) ≤ 2 by omega⟩

theorem lockLE_CR_blk {m : Nat} (hm : 1 ≤ m) : LockLE CR (certOf (blk m)) := by
  rw [LockLE, cert_epoch]
  exact Or.inl (show 1 < m + 1 by omega)

theorem lockLE_base_blk {m : Nat} (hm : 1 ≤ m) : LockLE (base k) (certOf (blk m)) := by
  unfold base; split
  · exact lockLE_CR_blk hm
  · exact lockLE_blk (Nat.zero_le _)

/-- Two certificates a node can lock on, at the same view, are the same. -/
theorem lockable_ext {n : Nat} {x y : Cert1} (hx : (H k n).Lockable cfg x) (hy : (H k n).Lockable cfg y)
    (hv : x.view = y.view) : x = y := by
  have hview : ∀ {z : Cert1}, (H k n).Lockable cfg z →
      (z = certOf anchorB ∧ z.view.toNat = 0) ∨ (z = CR ∧ z.view.toNat = 2)
        ∨ ∃ u, z = certOf (blk u) ∧ z.view.toNat = bv u := fun hz => by
    rcases lockable_iff.mp hz with rfl | ⟨rfl, -⟩ | ⟨u, rfl, -⟩
    · exact Or.inl ⟨rfl, rfl⟩
    · exact Or.inr (Or.inl ⟨rfl, rfl⟩)
    · exact Or.inr (Or.inr ⟨u, rfl, by rw [cert_view]⟩)
  have hvn : x.view.toNat = y.view.toNat := congrArg ViewNumber.toNat hv
  rcases hview hx with ⟨rfl, h1⟩ | ⟨rfl, h1⟩ | ⟨j, rfl, h1⟩ <;>
    rcases hview hy with ⟨rfl, h2⟩ | ⟨rfl, h2⟩ | ⟨m, rfl, h2⟩
  all_goals first
    | rfl
    | (have := bv_facts j; have := bv_facts m; omega)
    | (have := bv_facts m; omega)
    | (have := bv_facts j; omega)
    | omega
    | (obtain rfl : j = m := bv_inj (by omega); rfl)

theorem lockedOn_lockAt (n : Nat) : (H k n).LockedOn cfg (lockAt k n) := by
  have hl0 := lag_cases k 0
  have hlk0 := lk_facts k 0
  rcases steps n with hn | ⟨u, r, hu, hr, rfl⟩
  · unfold lockAt
    rw [ite_eq_left hn]
    by_cases hl : lag k 0 = 0
    · rw [ite_eq_left hl]
      by_cases h5 : n ≤ 5
      · rw [ite_eq_left h5]
        refine ⟨lockable_iff.mpr (Or.inl rfl), fun x hx => ?_⟩
        rcases lockable_iff.mp hx with rfl | ⟨rfl, -, h9⟩ | ⟨j, rfl, hj⟩
        · exact lockLE_refl _
        · omega
        · have := lk_facts k j; cases j with
          | zero => omega
          | succ j => omega
      by_cases h9 : n ≤ 9
      · rw [ite_eq_right h5, ite_eq_left h9]
        refine ⟨lockable_iff.mpr (Or.inr (Or.inr ⟨0, rfl, by omega⟩)), fun x hx => ?_⟩
        rcases lockable_iff.mp hx with rfl | ⟨rfl, -, h9'⟩ | ⟨j, rfl, hj⟩
        · exact lockLE_anchor 0
        · omega
        · have := lk_facts k j; cases j with
          | zero => exact lockLE_refl _
          | succ j => omega
      · rw [ite_eq_right h5, ite_eq_right h9]
        refine ⟨lockable_iff.mpr (Or.inr (Or.inl ⟨rfl, hl, by omega⟩)), fun x hx => ?_⟩
        rcases lockable_iff.mp hx with rfl | ⟨rfl, -, -⟩ | ⟨j, rfl, hj⟩
        · exact lockLE_anchor_CR
        · exact lockLE_refl _
        · have := lk_facts k j; cases j with
          | zero => exact lockLE_blk0_CR
          | succ j => omega
    · rw [ite_eq_right hl]
      have hl2 : lag k 0 = 2 := by omega
      by_cases h11 : n ≤ 11
      · rw [ite_eq_left h11]
        refine ⟨lockable_iff.mpr (Or.inl rfl), fun x hx => ?_⟩
        rcases lockable_iff.mp hx with rfl | ⟨rfl, hl', -⟩ | ⟨j, rfl, hj⟩
        · exact lockLE_refl _
        · omega
        · have := lk_facts k j; cases j with
          | zero => omega
          | succ j => omega
      · rw [ite_eq_right h11]
        refine ⟨lockable_iff.mpr (Or.inr (Or.inr ⟨0, rfl, by omega⟩)), fun x hx => ?_⟩
        rcases lockable_iff.mp hx with rfl | ⟨rfl, hl', -⟩ | ⟨j, rfl, hj⟩
        · exact lockLE_anchor 0
        · omega
        · have := lk_facts k j; cases j with
          | zero => exact lockLE_refl _
          | succ j => omega
  · obtain ⟨hhi, hlo1, hlo2⟩ := lockAt_later (k := k) u r hu hr
    have hlu := lag_cases k u
    have hlku := lk_facts k u
    by_cases hc : 5 + lag k u ≤ r
    · rw [hhi hc]
      refine ⟨lockable_iff.mpr (Or.inr (Or.inr ⟨u, rfl, by omega⟩)), fun x hx => ?_⟩
      rcases lockable_iff.mp hx with rfl | ⟨rfl, -, -⟩ | ⟨j, rfl, hj⟩
      · exact lockLE_anchor u
      · exact lockLE_CR_blk hu
      · have := lk_facts k j
        exact lockLE_blk (by have := lag_cases k j; cases j with
          | zero => omega
          | succ j => omega)
    · by_cases hu1 : u = 1
      · subst hu1
        rw [hlo1 (by omega) rfl]
        unfold base
        by_cases hl : lag k 0 = 0
        · rw [ite_eq_left hl]
          refine ⟨lockable_iff.mpr (Or.inr (Or.inl ⟨rfl, hl, by omega⟩)), fun x hx => ?_⟩
          rcases lockable_iff.mp hx with rfl | ⟨rfl, -, -⟩ | ⟨j, rfl, hj⟩
          · exact lockLE_anchor_CR
          · exact lockLE_refl _
          · have := lk_facts k j
            by_cases hj0 : j = 0
            · subst hj0; exact lockLE_blk0_CR
            · exfalso
              have hj1 : j = 1 := by have := lag_cases k j; omega
              subst hj1; omega
        · rw [ite_eq_right hl]
          refine ⟨lockable_iff.mpr (Or.inr (Or.inr ⟨0, rfl, by omega⟩)), fun x hx => ?_⟩
          rcases lockable_iff.mp hx with rfl | ⟨rfl, hl', -⟩ | ⟨j, rfl, hj⟩
          · exact lockLE_anchor 0
          · omega
          · have := lk_facts k j
            by_cases hj0 : j = 0
            · subst hj0; exact lockLE_refl _
            · exfalso
              have hj1 : j = 1 := by have := lag_cases k j; omega
              subst hj1; omega
      · rw [hlo2 (by omega) (by omega)]
        refine ⟨lockable_iff.mpr (Or.inr (Or.inr ⟨u - 1, rfl, ?_⟩)), fun x hx => ?_⟩
        · have := lk_facts k (u - 1)
          have := lag_cases k (u - 1)
          omega
        · rcases lockable_iff.mp hx with rfl | ⟨rfl, -, -⟩ | ⟨j, rfl, hj⟩
          · exact lockLE_anchor _
          · exact lockLE_CR_blk (by omega)
          · have := lk_facts k j
            have hj : j < u := by
              by_cases hju : j = u
              · subst hju; omega
              · have := lag_cases k j
                cases j with
                | zero => omega
                | succ j => omega
            exact lockLE_blk (by omega)

theorem lockedOn_eq {n : Nat} {L : Cert1} (hL : (H k n).LockedOn cfg L) : L = lockAt k n := by
  have h0 := lockedOn_lockAt (k := k) n
  exact lockable_ext hL.1 h0.1 (lockLE_antisymm (h0.2 _ hL.1) (hL.2 _ h0.1)).2

theorem lockOf_eq (n : Nat) : lockOf cfg (H k n) = lockAt k n := lockedOn_eq (lockOf_lockedOn _)

end Holds

/-! ## What the honest nodes send -/

section Sends

theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H k (n + 1)) o :=
  Kit.settled input k n o

/-- A node's timeout votes: one at each timer, for views one and three, naming its epoch and lock then. -/
theorem sent_timeout {k : PubKey} {j : Nat} {vote : TimeoutVote}
    (hx : Output.send (.timeoutVote vote) ∈ (tr k j).output) :
    (j = 4 ∧ vote = ⟨⟨⟨1⟩, certOf anchorB⟩, ⟨1⟩, k⟩) ∨ (j = 13 ∧ vote = ⟨⟨⟨2⟩, base k⟩, ⟨3⟩, k⟩) := by
  rw [tr_step] at hx
  rcases step_mem hx with hx | ⟨o, out', -, -, ha⟩
  · obtain ⟨v, hv, (⟨hi, -⟩ | ⟨hi, -⟩)⟩ := mem_timeoutAnswer hx
    · simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      rcases input_timeout hi with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · rw [hv, epochOf_eq, lockOf_eq, lockAt_one (by omega)]; exact Or.inl ⟨rfl, rfl⟩
      · rw [hv, epochOf_eq, lockOf_eq, lockAt_three (by omega) (by omega)]; exact Or.inr ⟨rfl, rfl⟩
    · exact (input_oneHonest hi).elim
  · exact absurd ha act_no_timeoutVote

/-- At the timer, a node times out the view it is in, naming its epoch and lock. -/
theorem times_out (k : PubKey) {j : Nat} {v : ViewNumber} (hi : input k j = .timeout v) (hv : vAt j = v.toNat) :
    Output.send (.timeoutVote ⟨⟨⟨eAt j⟩, lockAt k j⟩, v, k⟩) ∈ (tr k j).output := by
  rw [tr_step, step_output]
  apply discharge_sup
  rw [hi]
  simp only [timeoutAnswer, viewOf_eq, epochOf_eq, lockOf_eq]
  rw [ite_eq_left (by rw [hv])]
  exact List.mem_singleton_self _

/-- Every node times view one out at the first timer, before the block's `Cert1` arrives. -/
theorem times_out_one (k : PubKey) :
    Output.send (.timeoutVote ⟨⟨⟨1⟩, certOf anchorB⟩, ⟨1⟩, k⟩) ∈ (tr k 4).output := by
  have h := times_out k (j := 4) (v := ⟨1⟩) (by rw [input_first (by omega)]; rfl)
    ((vAt_first (by omega)).1 (by omega))
  rwa [lockAt_one (by omega)] at h

theorem times_out_three (k : PubKey) :
    Output.send (.timeoutVote ⟨⟨⟨2⟩, base k⟩, ⟨3⟩, k⟩) ∈ (tr k 13).output := by
  have h := times_out k (j := 13) (v := ⟨3⟩) (by rw [input_first (by omega)]; rfl)
    ((vAt_first (by omega)).2.2 (by omega))
  rwa [lockAt_three (by omega) (by omega)] at h

theorem timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h : (H k n).TimedOut v) :
    (v.toNat ≤ 1 ∧ 4 < n) ∨ (v.toNat ≤ 3 ∧ 13 < n) := by
  obtain ⟨vote, hs, hle⟩ := h
  obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
  rcases sent_timeout hjv with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
  · exact Or.inl ⟨hle, hj⟩
  · exact Or.inr ⟨hle, hj⟩

/-- Nothing times out a view before its timer. -/
theorem not_timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h : (H k n).TimedOut v)
    (h1 : v.toNat ≤ 1 → n ≤ 4) (h3 : v.toNat ≤ 3 → n ≤ 13) : False := by
  rcases timedOut h with ⟨ha, hb⟩ | ⟨ha, hb⟩
  · exact absurd (h1 ha) (by omega)
  · exact absurd (h3 ha) (by omega)

variable (hv : ∀ b, BlockValid b)

include hv in
theorem protocol (k : PubKey) (n : Nat) : ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H k n) :=
  Kit.protocol input hv k n

/-- What a justified proposal or request names, the node holds. -/
theorem justified_held {k : PubKey} {n : Nat} {x : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg (H k n) x ev) :
    (H k n).HasCert1 cfg x ∧ ∀ tc, ev = some tc → (tc = T1 ∧ 6 < n) ∨ (tc = T3 ∧ 14 < n) := by
  refine ⟨hasCert1_of_certJustified hj, fun tc hte => ?_⟩
  subst hte
  obtain ⟨-, m, hr, -⟩ := hj
  rw [upTo_H] at hr
  rcases received_tc hr with ⟨rfl, h⟩ | ⟨rfl, h⟩
  · exact Or.inl ⟨rfl, by omega⟩
  · exact Or.inr ⟨rfl, by omega⟩

include hv in
/--
A re-vote request a node sends: `a`'s, on an epoch's last block, from the step it
can lock on the block; the request on the first block behind the first timeout
certificate; or the second request, on the re-vote's `Cert1`.
-/
theorem sent_revote_any {k : PubKey} {j : Nat} {r : RevoteRequest} (hr : Output.send (.revote r) ∈ (tr k j).output) :
    k = a ∧ ((∃ x, r = R x ∧ at0 5 11 x ≤ j) ∨ (r = ⟨certOf (blk 0), ⟨2⟩, some T1⟩ ∧ 6 ≤ j)
      ∨ (r = R2 ∧ 9 ≤ j)) := by
  have hj := (protocol hv k (j + 1)).revoteJustified j r ⟨_, (getElem_H j), hr⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨hc, hev⟩ := justified_held hj.justified
  obtain ⟨hlt, hnext, hlast⟩ := hj.wellFormed
  rcases hasCert1 hc with he | ⟨he, h9⟩ | ⟨x, he, -⟩
  · rw [he] at hlast; exact absurd hlast.1 (by decide)
  · -- On the re-vote's `Cert1`, at view two: the second request, in view three.
    have hlt' : 2 < r.view.toNat := by rw [he] at hlt; exact hlt
    rcases hnext with ⟨hn0, hn⟩ | ⟨tc, hte, htv⟩
    · refine Or.inr (Or.inr ⟨?_, ?_⟩)
      · obtain ⟨c0, v0, e0⟩ := r
        simp only at he hn hn0
        rw [he, hn0, ← hn, he]; rfl
      · have hl : (H a (j + 1)).Lockable cfg CR := by
          have := hj.lockable hn0; rw [he] at this; exact this
        rcases lockable_iff.mp hl with h | ⟨-, -, h9'⟩ | ⟨u, h, -⟩
        · exact absurd (congrArg (·.view.toNat) h) (by show ¬ (2 : Nat) = 0; omega)
        · omega
        · have := congrArg (·.view.toNat) h; simp only [cert_view] at this
          exact absurd this (by show ¬ (2 : Nat) = bv u; have := bv_facts u; omega)
    · -- Timeout evidence would be of the second epoch, which the re-vote's `Cert1` is not.
      exfalso
      obtain ⟨hep, -⟩ := hj.safe _ hte
      rw [he] at hep
      rcases hev tc hte with ⟨rfl, -⟩ | ⟨rfl, -⟩
      · have : 1 + 1 = r.view.toNat := congrArg ViewNumber.toNat htv
        omega
      · exact absurd (congrArg EpochNumber.toNat hep) (by show ¬ (2 : Nat) = 1; omega)
  · have hlx : bv x < r.view.toNat := by
      have : (certOf (blk x)).view.toNat < r.view.toNat := by rw [← he]; exact hlt
      rwa [cert_view] at this
    rcases hnext with ⟨hn0, hn⟩ | ⟨tc, hte, htv⟩
    · refine Or.inl ⟨x, ?_, ?_⟩
      · obtain ⟨c0, v0, e0⟩ := r
        simp only at he hn hn0
        rw [he, hn0, ← hn, he, cert_view]; rfl
      · have hl : (H a (j + 1)).Lockable cfg (certOf (blk x)) := by
          have := hj.lockable hn0; rw [he] at this; exact this
        rcases lockable_iff.mp hl with h | ⟨h, -⟩ | ⟨y, hy, hlt'⟩
        · have := congrArg (·.view.toNat) h; simp only [cert_view] at this
          exact absurd this (by show ¬ bv x = 0; have := bv_facts x; omega)
        · have := congrArg (·.view.toNat) h; simp only [cert_view] at this
          exact absurd this (by show ¬ bv x = 2; have := bv_facts x; omega)
        · obtain rfl : y = x := by
            have := congrArg (·.view.toNat) hy; simp only [cert_view] at this; exact (bv_inj this).symm
          have := lk_facts a y
          have := at0_facts 5 11 y
          cases y with
          | zero => have := lag_a 0; omega
          | succ y => have := lag_a (y + 1); omega
    · obtain ⟨hep, -⟩ := hj.safe _ hte
      rw [he, cert_epoch] at hep
      rcases hev tc hte with ⟨rfl, h6⟩ | ⟨rfl, -⟩
      · -- Behind the first timeout certificate: on the first block, in view two.
        have hx0 : x = 0 := by have : 1 = x + 1 := congrArg EpochNumber.toNat hep; omega
        subst hx0
        refine Or.inr (Or.inl ⟨?_, by omega⟩)
        obtain ⟨c0, v0, e0⟩ := r
        simp only at he hte htv
        rw [he, hte, ← htv]; rfl
      · exfalso
        have hy : 2 = x + 1 := congrArg EpochNumber.toNat hep
        have h4 : 3 + 1 = r.view.toNat := congrArg ViewNumber.toNat htv
        have := bv_facts x
        omega

include hv in
/-- A proposal is for the view of a header the node was handed, after it was handed it. -/
theorem proposal_header {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    (7 ≤ j ∧ p.viewNumber = ⟨2⟩ ∧ p.blockHeader = hdr 1)
      ∨ ∃ u, at0 0 7 u ≤ j ∧ p.viewNumber = ⟨bv u⟩ ∧ p.blockHeader = hdr (u + 1) := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨i, hi, hii⟩ := received.mp hj.built
  rcases input_header hii with ⟨rfl, hvw, -, hx⟩ | ⟨u, rfl, hvw, -, hx⟩
  · exact Or.inl ⟨by omega, hvw, hx⟩
  · exact Or.inr ⟨u, by omega, hvw, hx⟩

/-- `a` may propose block `u + 1` from the step its header arrives until the block's `Cert1` does. -/
theorem at_block (u r : Nat) (h3 : r ≤ 3) : vAt (at0 0 7 u + r) = bv u ∧ eAt (at0 0 7 u + r) = u + 1 := by
  cases u with
  | zero =>
    rw [(at0_facts _ _ _).1 rfl, show 0 + r = r by omega]
    exact ⟨(vAt_first (by omega)).1 (by omega), (eAt_first (by omega)).1 (by omega)⟩
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega), show 8 * (u + 1) + 7 + r = 8 * (u + 1) + r + 7 by omega]
    exact ⟨(vAt_later (u + 1) r (by omega) (by omega)).1 (by omega),
      (eAt_later (u + 1) r (by omega) (by omega)).2 (by omega)⟩

/-- The parent `a` builds block `u + 1` on is justified from the step its header arrives until the block's `Cert1` does. -/
theorem parent_justified (u r : Nat) (h3 : r ≤ 3) : ParentJustified cfg (H a (at0 0 7 u + r)) (blk u) := by
  unfold ParentJustified
  cases u with
  | zero =>
    show (H a _).Buildable cfg (certOf anchorB)
    exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)]
    cases u with
    | zero =>
      show CertJustified cfg (H a (8 * (0 + 1) + 7 + r)) (certOf (blk 0)) (some T3)
      refine ⟨hasCert1_of 0 (by rw [(at0_facts _ _ _).1 rfl]; omega), 8 * (0 + 1) + 7 + r, ?_,
        Or.inl ⟨CR, ?_, rfl⟩⟩ <;> rw [upTo_self]
      · exact recv_T3 (by omega)
      · have := lockedOn_lockAt (k := a) (8 * 1 + r + 7)
        rwa [(lockAt_later 1 r (by omega) (by omega)).2.1 (by rw [lag_a]; omega) rfl,
          show 8 * 1 + r + 7 = 8 * (0 + 1) + 7 + r by omega, show base a = CR from rfl] at this
    | succ u =>
      show (H a _).Buildable cfg (certOf (blk (u + 1)))
      refine Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inr (Or.inr ⟨u + 1, rfl, ?_⟩)))
      have := (lk_facts a (u + 1)).2.2 (by omega)
      rw [lag_a] at this
      omega

/-- `a` may propose block `u + 1` from the step its header arrives until the block's `Cert1` does. -/
theorem blk_justified (u r : Nat) (h1 : 1 ≤ r) (h3 : r ≤ 3) :
    ProposalJustified cfg leader a (H a (at0 0 7 u + r)) (blk u) := by
  obtain ⟨hvw, hew⟩ := at_block u r h3
  refine ⟨⟨rfl, blk_wellFormed u, parent_justified u r h3, ⟨parentOf u, hasParent_of u (by omega),
      by rw [blk_parent]; exact Nat.le_refl _, by rw [blk_parent]; rfl⟩, ?_, blk_safe u, notBehind ?_,
      ⟨_, (inView _).1, ?_⟩⟩, ?_⟩
  · intro hen
    cases u with
    | zero => exact absurd hen blk_enters_zero
    | succ u =>
      have := at0_facts 0 7 (u + 1); have := at0_facts 1 8 u; have := at0_facts 10 12 u
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), ?_, by rw [blk_parent]; rfl⟩
      rw [blk_view]; show cv u < bv (u + 1)
      have := cv_facts u; have := bv_facts (u + 1); omega
  · rw [blk_epoch, hew]; exact Nat.le_refl _
  · rw [blk_view, hvw]; exact Nat.le_refl _
  · rw [blk_view, blk_header, blk_parent]
    exact recv_u u 0 0 (by omega) (by omega) rfl (by show at0 0 7 u < _; omega)

include hv in
/-- Before the header for view two arrives, every proposal a node sends is a block's, for the block's view. -/
theorem sent_proposal_early {k : PubKey} {j : Nat} {p : Proposal} (hj7 : j < 7)
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    ∃ u, at0 0 7 u ≤ j ∧ p.viewNumber = ⟨bv u⟩ ∧ p.epoch = ⟨u + 1⟩ := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  rcases proposal_header hv hp with ⟨h7, -⟩ | ⟨u, hju, hvw, hhdr⟩
  · omega
  · refine ⟨u, hju, hvw, ?_⟩
    rw [hj.wellFormed.epoch, hhdr]; exact epochOf_one_height (by omega)

include hv in
/-- Before the first timeout certificate arrives, every re-vote request a node sends is `a`'s on a block. -/
theorem sent_revote_early {k : PubKey} {j : Nat} {r : RevoteRequest} (hj6 : j < 6)
    (hr : Output.send (.revote r) ∈ (tr k j).output) : k = a ∧ ∃ x, r = R x ∧ at0 5 11 x ≤ j := by
  obtain ⟨rfl, h | ⟨-, h⟩ | ⟨-, h⟩⟩ := sent_revote_any hv hr
  · exact ⟨rfl, h⟩
  · omega
  · omega

include hv in
/-- `a` asks for a re-vote on the first block as soon as it can lock on it, on its `Cert1`, in view two. -/
theorem revotes_zero : ∃ j, j ≤ 5 ∧ Output.send (.revote (R 0)) ∈ (tr a j).output := by
  have hvw : vAt 6 = 2 := (vAt_first (by omega)).2.1 (by omega) (by omega)
  have hew : eAt 6 = 1 := (eAt_first (by omega)).1 (by omega)
  refine Classical.byContradiction fun hneg => settled a 5 (.propose ⟨1⟩ ⟨2⟩)
    ⟨Or.inr ⟨R 0, ⟨rfl, ⟨?_, Or.inl ⟨rfl, rfl⟩, ?_⟩, Liveness.buildable_of_lockable ?lk,
        fun _ => ?lk, (fun _ h => by cases h), notBehind ?_, ?_⟩, rfl,
      rfl⟩, ?_, fun ht => ?_, ?_⟩
  · show (1 : Nat) < 2; omega
  · show IsLastBlock (certOf (blk 0)).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
  · exact lockable_iff.mpr (Or.inr (Or.inr ⟨0, rfl, by rw [(lk_facts a 0).1 rfl (lag_a 0)]; omega⟩))
  · show eAt 6 ≤ 1; omega
  · exact ⟨_, (inView _).1, by rw [hvw]; exact Nat.le_refl _⟩
  · rintro (⟨p, hs, hpv, hpe⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨u, -, hvu, heu⟩ := sent_proposal_early hv (by omega) hjp
      rw [hvu] at hpv; rw [heu] at hpe
      have h1 : bv u = 2 := view_inj hpv
      have h2 : u + 1 = 1 := congrArg EpochNumber.toNat hpe
      obtain rfl : u = 0 := by omega
      exact absurd h1 (by decide)
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_revote_early hv (by omega) hjr
      have h2 : x + 1 = 1 := congrArg EpochNumber.toNat ((cert_epoch x).symm.trans hre)
      obtain rfl : x = 0 := by omega
      exact hneg ⟨j, by omega, hjr⟩
  · exact not_timedOut ht (fun h => absurd h (by show ¬ (2 : Nat) ≤ 1; omega)) (fun _ => by omega)
  · have := inView (k := a) 6
    rwa [hvw] at this

include hv in
/-- A node sends only `a`'s re-vote requests: one on each block, from the step it can lock on it, and the second. -/
theorem sent_revote {k : PubKey} {j : Nat} {r : RevoteRequest} (hr : Output.send (.revote r) ∈ (tr k j).output) :
    k = a ∧ ((∃ x, r = R x ∧ at0 5 11 x ≤ j) ∨ (r = R2 ∧ 9 ≤ j)) := by
  obtain ⟨rfl, h | ⟨rfl, h6⟩ | h⟩ := sent_revote_any hv hr
  · exact ⟨rfl, Or.inl h⟩
  · -- `a` already asked in view two, on the block's `Cert1` alone.
    exfalso
    obtain ⟨j0, hj0, hr0⟩ := revotes_zero hv
    have := ((protocol hv a (j + 1)).revote_once (R 0) (sent_iff.mpr ⟨j0, by omega, hr0⟩) trivial).1 _
      (sent_iff.mpr ⟨j, by omega, hr⟩) rfl rfl
    cases this
  · exact ⟨rfl, Or.inr h⟩

include hv in
/-- Every proposal a node sends is `a`'s block for its view, sent once the header arrived. -/
theorem sent_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) : k = a ∧ ∃ u, p = blk u ∧ at0 0 7 u ≤ j := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨-, hnext, hep, hnum⟩ := hj.wellFormed
  obtain ⟨hpc, hev⟩ := justified_held hj.justified
  obtain ⟨u, hju, hvw, hhdr⟩ : ∃ u, at0 0 7 u ≤ j ∧ p.viewNumber = ⟨bv u⟩ ∧ p.blockHeader = hdr (u + 1) := by
    rcases proposal_header hv hp with ⟨h7, hvw, hhdr⟩ | h
    · -- In view two `a` already asked for a re-vote of the same epoch.
      exfalso
      have hpe : p.epoch = ⟨1⟩ := by rw [hep, hhdr]; exact epochOf_one_height (by omega)
      obtain ⟨j0, hj0, hr0⟩ := revotes_zero hv
      exact ((protocol hv a (j + 1)).revote_once (R 0) (sent_iff.mpr ⟨j0, by omega, hr0⟩) trivial).2 p
        (sent_iff.mpr ⟨j, by omega, hp⟩) hpe hvw
    · exact h
  refine ⟨u, ?_, hju⟩
  have hid : p.identity = ⟨0⟩ := by
    rw [tr_step] at hp
    rcases step_mem hp with h0 | ⟨o, out', -, -, ha⟩
    · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨_, v, -, hmem, -, -⟩ := act_proposal ha
      exact (mem_proposalCandidates hmem).1
  have hnum' : p.parentCert.data.blockNumber + 1 = ⟨u + 1⟩ := by rw [hnum, hhdr]; rfl
  have hepu : p.epoch = ⟨u + 1⟩ := by rw [hep, hhdr]; exact epochOf_one_height (by omega)
  have hnumN : p.parentCert.data.blockNumber.toNat + 1 = u + 1 := congrArg BlockNumber.toNat hnum'
  cases u with
  | zero =>
    have hpcA : p.parentCert = certOf anchorB := by
      rcases hasCert1 hpc with h | ⟨h, -⟩ | ⟨x, h, -⟩
      · exact h
      · rw [h] at hnumN; exact absurd hnumN (by show ¬ (1 : Nat) + 1 = 0 + 1; omega)
      · rw [h, cert_number] at hnumN; exact absurd hnumN (by show ¬ x + 1 + 1 = 0 + 1; omega)
    have hte : p.timeoutEvidence = none := by
      rcases hnext with ⟨hn0, -⟩ | ⟨tc, hte, htv⟩
      · exact hn0
      · exfalso
        rw [hvw] at htv
        rcases hev tc hte with ⟨rfl, -⟩ | ⟨rfl, -⟩
        · exact absurd (congrArg ViewNumber.toNat htv) (by show ¬ 1 + 1 = bv 0; decide)
        · exact absurd (congrArg ViewNumber.toNat htv) (by show ¬ 3 + 1 = bv 0; decide)
    obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
    simp only at hvw hhdr hid hepu hpcA hte ⊢
    rw [hvw, hhdr, hid, hepu, hpcA, hte]; rfl
  | succ u =>
    cases u with
    | zero =>
      -- View four, after the timeout certificate for view three, on the first block's own `Cert1`.
      have hpcB : p.parentCert = certOf (blk 0) := by
        rcases hasCert1 hpc with h | ⟨h, -⟩ | ⟨x, h, -⟩
        · rw [h] at hnumN; exact absurd hnumN (by show ¬ (0 : Nat) + 1 = 0 + 1 + 1; omega)
        · -- The re-vote's `Cert1` is not at its block's view, which an opening proposal names.
          exfalso
          have hen : EntersEpoch cfg p := by
            show IsLastBlock (p.blockHeader.blockNumber - 1) 1
            rw [hhdr]; exact last_block (n := 1) (by omega)
          obtain ⟨⟨q, hq, hqv, -⟩, -⟩ := hj.opens hen
          rw [h] at hqv
          rcases hasProposal hq with rfl | ⟨y, rfl, -⟩
          · exact absurd (congrArg ViewNumber.toNat hqv) (by show ¬ (0 : Nat) = 2; omega)
          · rw [blk_view] at hqv
            exact absurd (congrArg ViewNumber.toNat hqv) (by show ¬ bv y = 2; have := bv_facts y; omega)
        · rw [h, cert_number] at hnumN
          have : x + 1 + 1 = 0 + 1 + 1 := hnumN
          obtain rfl : x = 0 := by omega
          exact h
      have hte : p.timeoutEvidence = some T3 := by
        rcases hnext with ⟨-, hn⟩ | ⟨tc, hte, htv⟩
        · rw [hpcB, cert_view, hvw] at hn
          have : bv 0 + 1 = bv (0 + 1) := congrArg ViewNumber.toNat hn
          exact absurd this (by decide)
        · rw [hvw] at htv
          rcases hev tc hte with ⟨rfl, -⟩ | ⟨rfl, -⟩
          · exact absurd (congrArg ViewNumber.toNat htv) (by show ¬ 1 + 1 = bv (0 + 1); decide)
          · exact hte
      obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
      simp only at hvw hhdr hid hepu hpcB hte ⊢
      rw [hvw, hhdr, hid, hepu, hpcB, hte]; rfl
    | succ u =>
      have hpcB : p.parentCert = certOf (blk (u + 1)) := by
        rcases hasCert1 hpc with h | ⟨h, -⟩ | ⟨x, h, -⟩
        · rw [h] at hnumN; exact absurd hnumN (by show ¬ (0 : Nat) + 1 = u + 1 + 1 + 1; omega)
        · rw [h] at hnumN; exact absurd hnumN (by show ¬ (1 : Nat) + 1 = u + 1 + 1 + 1; omega)
        · rw [h, cert_number] at hnumN
          have : x + 1 + 1 = u + 1 + 1 + 1 := hnumN
          obtain rfl : x = u + 1 := by omega
          exact h
      have hte : p.timeoutEvidence = none := by
        rcases hnext with ⟨hn0, -⟩ | ⟨tc, hte, htv⟩
        · exact hn0
        · exfalso
          rw [hvw] at htv
          rcases hev tc hte with ⟨rfl, -⟩ | ⟨rfl, -⟩
          · exact absurd (congrArg ViewNumber.toNat htv) (by show ¬ 1 + 1 = u + 5; omega)
          · exact absurd (congrArg ViewNumber.toNat htv) (by show ¬ 3 + 1 = u + 5; omega)
      obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
      simp only at hvw hhdr hid hepu hpcB hte ⊢
      rw [hvw, hhdr, hid, hepu, hpcB, hte]; rfl

include hv in
/-- `a` proposes each block by the step its header arrives in. -/
theorem proposes (u : Nat) : ∃ j, j ≤ at0 0 7 u ∧ Output.send (.proposal (blk u)) ∈ (tr a j).output := by
  obtain ⟨hvw, -⟩ := at_block u 1 (by omega)
  refine Classical.byContradiction fun hneg => settled a (at0 0 7 u) (.propose (blk u).epoch ⟨bv u⟩)
    ⟨Or.inl ⟨blk u, blk_justified u 1 (Nat.le_refl _) (by omega), blk_view u, rfl⟩, ?_, fun ht => ?_, ?_⟩
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv
      obtain rfl : x = u := bv_inj (view_inj hpv)
      exact hneg ⟨j, by omega, hjp⟩
    · -- `a`'s re-vote requests in that view are of the epoch before.
      obtain ⟨j, -, hjr⟩ := sent_iff.mp hs
      rw [blk_epoch] at hre
      rcases (sent_revote hv hjr).2 with ⟨x, rfl, -⟩ | ⟨rfl, -⟩
      · have h1 : bv x + 1 = bv u := view_inj hrv
        have h2 : x + 1 = u + 1 := congrArg EpochNumber.toNat ((cert_epoch x).symm.trans hre)
        obtain rfl : x = u := by omega
        omega
      · have h1 : 3 = bv u := view_inj hrv
        have h2 : 1 = u + 1 := congrArg EpochNumber.toNat hre
        obtain rfl : u = 0 := by omega
        exact absurd h1 (by decide)
  · have := bv_facts u; have := at0_facts 0 7 u
    refine not_timedOut ht (fun h => ?_) (fun h => ?_)
    · have h' : bv u ≤ 1 := h; omega
    · have h' : bv u ≤ 3 := h; omega
  · have := inView (k := a) (at0 0 7 u + 1)
    rwa [hvw] at this

include hv in
/-- `a` asks for a re-vote on each block as soon as it can lock on it. -/
theorem revotes (u : Nat) : ∃ j, j ≤ at0 5 11 u ∧ Output.send (.revote (R u)) ∈ (tr a j).output := by
  cases u with
  | zero => exact revotes_zero hv
  | succ u =>
    rw [(at0_facts _ _ _).2 (by omega)]
    have hvw : vAt (8 * (u + 1) + 11 + 1) = bv (u + 1) + 1 := by
      rw [show 8 * (u + 1) + 11 + 1 = 8 * (u + 1) + 5 + 7 by omega]
      exact (vAt_later (u + 1) 5 (by omega) (by omega)).2 (by omega)
    have hew : eAt (8 * (u + 1) + 11 + 1) = u + 1 + 1 := by
      rw [show 8 * (u + 1) + 11 + 1 = 8 * (u + 1) + 5 + 7 by omega]
      exact (eAt_later (u + 1) 5 (by omega) (by omega)).2 (by omega)
    refine Classical.byContradiction fun hneg => settled a (8 * (u + 1) + 11) (.propose ⟨u + 1 + 1⟩ ⟨bv (u + 1) + 1⟩)
      ⟨Or.inr ⟨R (u + 1), ⟨rfl, ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_⟩, Liveness.buildable_of_lockable ?lk,
        fun _ => ?lk, (fun _ h => by cases h), notBehind ?_, ?_⟩, rfl,
        cert_epoch (u + 1)⟩, ?_, fun ht => ?_, ?_⟩
    · show (certOf (blk (u + 1))).view < ⟨bv (u + 1) + 1⟩; rw [cert_view]; show bv (u + 1) < bv (u + 1) + 1; omega
    · show (certOf (blk (u + 1))).view + 1 = ⟨bv (u + 1) + 1⟩; rw [cert_view]; rfl
    · show IsLastBlock (certOf (blk (u + 1))).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
    · refine lockable_iff.mpr (Or.inr (Or.inr ⟨u + 1, rfl, ?_⟩))
      have := (lk_facts a (u + 1)).2.2 (by omega)
      rw [lag_a] at this
      omega
    · show eAt (8 * (u + 1) + 11 + 1) ≤ (certOf (blk (u + 1))).data.epoch.toNat
      rw [cert_epoch, hew]; exact Nat.le_refl _
    · exact ⟨_, (inView _).1, by rw [hvw]; exact Nat.le_refl _⟩
    · rintro (⟨p, hs, hpv, hpe⟩ | ⟨r, hs, hrv, hre⟩)
      · obtain ⟨j, -, hjp⟩ := sent_iff.mp hs
        obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
        rw [blk_view] at hpv; rw [blk_epoch] at hpe
        have h1 : bv x = bv (u + 1) + 1 := view_inj hpv
        have h2 : x + 1 = u + 1 + 1 := congrArg EpochNumber.toNat hpe
        obtain rfl : x = u + 1 := by omega
        omega
      · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
        rcases (sent_revote hv hjr).2 with ⟨x, rfl, -⟩ | ⟨rfl, -⟩
        · obtain rfl : x = u + 1 := by
            have h2 : x + 1 = u + 1 + 1 := congrArg EpochNumber.toNat ((cert_epoch x).symm.trans hre)
            omega
          exact hneg ⟨j, by omega, hjr⟩
        · have : 1 = u + 1 + 1 := congrArg EpochNumber.toNat hre
          omega
    · have := bv_facts (u + 1)
      refine not_timedOut ht (fun h => ?_) (fun h => ?_)
      · have h' : bv (u + 1) + 1 ≤ 1 := h; omega
      · have h' : bv (u + 1) + 1 ≤ 3 := h; omega
    · have := inView (k := a) (8 * (u + 1) + 11 + 1)
      rwa [hvw] at this

include hv in
/-- `a` asks for a second re-vote, on the re-vote's `Cert1`, as soon as it can lock on that. -/
theorem revotes_two : ∃ j, j ≤ 9 ∧ Output.send (.revote R2) ∈ (tr a j).output := by
  have hvw : vAt 10 = 3 := (vAt_first (by omega)).2.2 (by omega)
  have hew : eAt 10 = 1 := (eAt_first (by omega)).1 (by omega)
  refine Classical.byContradiction fun hneg => settled a 9 (.propose ⟨1⟩ ⟨3⟩)
    ⟨Or.inr ⟨R2, ⟨rfl, ⟨?_, Or.inl ⟨rfl, rfl⟩, ?_⟩, Liveness.buildable_of_lockable ?lk,
        fun _ => ?lk, (fun _ h => by cases h), notBehind ?_, ?_⟩, rfl, rfl⟩,
      ?_, fun ht => ?_, ?_⟩
  · show (2 : Nat) < 3; omega
  · show IsLastBlock (certOf (blk 0)).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
  · exact lockable_iff.mpr (Or.inr (Or.inl ⟨rfl, lag_a 0, by omega⟩))
  · show eAt 10 ≤ 1; omega
  · exact ⟨_, (inView _).1, by rw [hvw]; exact Nat.le_refl _⟩
  · rintro (⟨p, hs, hpv, hpe⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, -, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv; rw [blk_epoch] at hpe
      have h1 : bv x = 3 := view_inj hpv
      have h2 : x + 1 = 1 := congrArg EpochNumber.toNat hpe
      obtain rfl : x = 0 := by omega
      exact absurd h1 (by decide)
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      rcases (sent_revote hv hjr).2 with ⟨x, rfl, -⟩ | ⟨rfl, -⟩
      · have h1 : bv x + 1 = 3 := view_inj hrv
        have h2 : x + 1 = 1 := congrArg EpochNumber.toNat ((cert_epoch x).symm.trans hre)
        obtain rfl : x = 0 := by omega
        exact absurd h1 (by decide)
      · exact hneg ⟨j, by omega, hjr⟩
  · exact not_timedOut ht (fun h => absurd h (by show ¬ (3 : Nat) ≤ 1; omega)) (fun _ => by omega)
  · have := inView (k := a) 10
    rwa [hvw] at this

include hv in
/--
Every vote1 a node sends is for a block of its committee, after the proposal
arrived, or answers the re-vote request on the first block, in view two.
-/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1} (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    (∃ u, lag k u = 0 ∧ vote = ⟨(certOf (blk u)).data, ⟨bv u⟩, k⟩ ∧ at0 1 8 u ≤ j)
      ∨ (vote = ⟨(certOf (blk 0)).data, ⟨2⟩, k⟩ ∧ 8 ≤ j) := by
  have hpr := protocol hv k (j + 1)
  obtain ⟨hsig, -⟩ := hpr.vote1Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  obtain ⟨-, ⟨s', p, vid, hrec, -, hfor, -⟩ | ⟨s', r, hrec, -, hagain, hnb⟩⟩ :=
    hpr.vote1Leader j vote ⟨_, (getElem_H j), hx⟩ trivial
  · rw [upTo_self] at hrec
    obtain ⟨i, hi, hii⟩ := received.mp hrec
    obtain ⟨u, rfl, hl, -, rfl, -⟩ := input_proposal hii
    refine Or.inl ⟨u, hl, ?_, by omega⟩
    obtain ⟨d, v, sg⟩ := vote
    obtain ⟨hv', hd⟩ := hfor
    simp only at hsig hv' hd
    rw [hsig, hv', hd, blk_view]; rfl
  · -- After an epoch change a node does not go back; only the first request comes before it.
    rw [upTo_self] at hrec hnb
    have h1 := notBehind_le hnb
    obtain ⟨-, hec, -, -⟩ := eAt_ge (j + 1)
    rcases received_revote hrec with ⟨-, rfl, h12⟩ | ⟨u, -, rfl, hlt⟩
    · have h1' : eAt (j + 1) ≤ 1 := h1
      have := hec 0 (by rw [(at0_facts _ _ _).1 rfl]; omega)
      omega
    · rw [show (R u).cert.data.epoch = ⟨u + 1⟩ from cert_epoch u] at h1
      have h1' : eAt (j + 1) ≤ u + 1 := h1
      have hf := at0_facts 8 14 u
      have hf' := at0_facts 11 13 u
      cases u with
      | zero =>
        refine Or.inr ⟨?_, by omega⟩
        obtain ⟨d, v, sg⟩ := vote
        obtain ⟨hv', hd⟩ := hagain
        simp only at hsig hv' hd
        rw [hsig, hv', hd]; rfl
      | succ u =>
        have := hec (u + 1) (by omega)
        omega

include hv in
/-- Every member of a block's committee votes1 for it by the step its validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ at0 2 9 u
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨bv u⟩, k⟩) ∈ (tr k j).output := by
  have hf := at0_facts 2 9 u
  have hf0 := at0_facts 0 7 u
  obtain ⟨hvw, hew⟩ := at_block u 3 (by omega)
  have h3 : at0 2 9 u + 1 = at0 0 7 u + 3 := by omega
  have hopen : OpensEpochJustified cfg (H k (at0 2 9 u + 1)) (blk u) := fun he => by
    cases u with
    | zero => exact absurd he blk_enters_zero
    | succ u =>
      have := at0_facts 1 8 u
      have := at0_facts 10 12 u
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), ?_, by rw [blk_parent]; rfl⟩
      rw [blk_view]; show cv u < bv (u + 1)
      have := cv_facts u; have := bv_facts (u + 1); omega
  have hprop : (H k (at0 2 9 u + 1)).Received (.proposal a (blk u) (some ⟨⟨bv u⟩, (blk u).payloadCommit⟩)) := by
    have := recv_u (k := k) (n := at0 2 9 u + 1) u 1 1 (by omega) (by omega) rfl
      (by have := at0_facts 1 8 u; show at0 1 8 u < _; omega)
    simp only [phase, hl, ite_eq_left] at this
    exact this
  refine Classical.byContradiction fun hneg => settled k (at0 2 9 u) (.vote1 (blk u))
    ⟨⟨a, _, hprop, rfl, by rw [blk_view], rfl⟩, blk_wellFormed u, ?_, ?_, blk_safe u, hopen,
      notBehind ?_, fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · rw [blk_view]; exact recv_u u 2 2 (by omega) (by omega) rfl (by show at0 2 9 u < _; omega)
  · cases u with
    | zero => exact Or.inl rfl
    | succ u => exact Or.inr (Or.inl (blk_enters u))
  · rw [blk_epoch, h3, hew]; exact Nat.le_refl _
  · rw [blk_view] at ht
    have := bv_facts u
    refine not_timedOut ht (fun h => ?_) (fun h => ?_)
    · have h' : bv u ≤ 1 := h; omega
    · have h' : bv u ≤ 3 := h; omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    rcases sent_vote1 hv hjv with ⟨u', -, rfl, -⟩ | ⟨rfl, -⟩
    · rw [blk_view] at hvv
      obtain rfl : u' = u := bv_inj (view_inj hvv)
      exact hneg ⟨j, by omega, hjv⟩
    · rw [blk_view] at hvv
      have : 2 = bv u := view_inj hvv
      have := bv_facts u
      omega
  · rw [blk_view, h3]
    have := inView (k := k) (at0 0 7 u + 3)
    rwa [hvw] at this

include hv in
/-- The members of the first committee answer the re-vote request on the first block, in view two. -/
theorem votes1R (k : PubKey) (hl : lag k 0 = 0) : ∃ j, j ≤ 8
    ∧ Output.send (.vote1 ⟨(certOf (blk 0)).data, ⟨2⟩, k⟩) ∈ (tr k j).output := by
  have hvw : vAt 9 = 2 := (vAt_first (by omega)).2.1 (by omega) (by omega)
  have hew : eAt 9 = 1 := (eAt_first (by omega)).1 (by omega)
  refine Classical.byContradiction fun hneg => settled k 8 (.vote1Again (R 0))
    ⟨⟨a, recv_revote 0 (by rw [(at0_facts _ _ _).1 rfl]; omega), rfl⟩, ⟨?_, Or.inl ⟨rfl, rfl⟩, ?_⟩,
      (fun _ h => by cases h),
      ⟨blk 0, hasProposal_of 0 (by simp [at0]), ⟨Nat.le_refl _, rfl⟩, payload_of 0 hl (by simp [at0])⟩,
      notBehind (by rw [hew]; exact Nat.le_refl _), fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · show (1 : Nat) < 2; omega
  · show IsLastBlock (certOf (blk 0)).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
  · exact not_timedOut ht (fun h => absurd h (by show ¬ (2 : Nat) ≤ 1; omega)) (fun _ => by omega)
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    rcases sent_vote1 hv hjv with ⟨u', -, rfl, -⟩ | ⟨rfl, -⟩
    · have : bv u' = 2 := view_inj hvv
      have := bv_facts u'
      omega
    · exact hneg ⟨j, by omega, hjv⟩
  · have := inView (k := k) 9
    rwa [hvw] at this

include hv in
/-- Every vote2 a node sends is on the re-vote's `Cert1` or a later block's own, after its payload arrived. -/
theorem sent_vote2 {k : PubKey} {j : Nat} {vote : Vote2} (hx : Output.send (.vote2 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(C2 u).data, ⟨cv u⟩, k⟩ ∧ lag k u = 0 ∧ at0 9 11 u ≤ j := by
  have hpr := protocol hv k (j + 1)
  obtain ⟨hsig, hgen, x, y, hc, hb, hcert, hpay, hvc, hdc⟩ := hpr.vote2Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  have hbefore := hpr.vote2BeforeTimeout j vote ⟨_, (getElem_H j), hx⟩ trivial
  rw [upTo_self] at hc hb hpay hbefore
  obtain ⟨d, v, sg⟩ := vote
  simp only at hsig hvc hdc hgen hbefore
  rcases hasPayload hpay with hg | ⟨u, hyv, -, hl, hlt⟩
  · -- At genesis: the anchor, whose certificate is at genesis too.
    exfalso
    rcases hasProposal hb with rfl | ⟨u, rfl, -⟩
    · have hx0 : x.data.blockNumber = ⟨0⟩ := by rw [hcert.2]; rfl
      rcases hasCert1 hc with rfl | ⟨rfl, -⟩ | ⟨u, rfl, -⟩
      · rw [hvc] at hgen; exact Nat.lt_irrefl _ hgen
      · exact absurd (number_inj hx0) (by decide)
      · rw [cert_number] at hx0; exact absurd (number_inj hx0) (by omega)
    · rw [blk_view] at hg; have := bv_facts u; exact absurd (view_inj hg) (by omega)
  · obtain rfl : y = blk u := by
      rcases hasProposal hb with rfl | ⟨u', rfl, -⟩
      · have := bv_facts u; exact absurd (view_inj hyv) (by omega)
      · rw [blk_view] at hyv
        obtain rfl : u' = u := bv_inj (view_inj hyv)
        rfl
    have hxn : x.data.blockNumber = ⟨u + 1⟩ := by rw [hcert.2]; exact blk_number u
    rcases hasCert1 hc with rfl | ⟨rfl, h9⟩ | ⟨u', rfl, hcu⟩
    · rw [hvc] at hgen; exact absurd hgen (Nat.lt_irrefl _)
    · -- On the re-vote's `Cert1`, over the first block.
      have hu0 : u = 0 := by have := number_inj hxn; omega
      subst hu0
      exact ⟨0, by rw [hsig, hvc, hdc]; rfl, hl, by rw [(at0_facts _ _ _).1 rfl]; omega⟩
    · obtain rfl : u' = u := by rw [cert_number] at hxn; have := number_inj hxn; omega
      cases u' with
      | zero =>
        -- At the first block's own view, which every node timed out before its `Cert1` arrived.
        exfalso
        have := hbefore _ (sent_iff.mpr ⟨4, by rw [(at0_facts _ _ _).1 rfl] at hcu; omega, times_out_one k⟩) trivial
        rw [hvc, cert_view] at this
        exact absurd this (by show ¬ (1 : Nat) < bv 0; decide)
      | succ u =>
        refine ⟨u + 1, ?_, hl, by rw [(at0_facts _ _ _).2 (by omega)]; rw [(at0_facts _ _ _).2 (by omega)] at hlt; omega⟩
        rw [hsig, hvc, hdc, cert_view]; rfl

include hv in
/-- Every member of the first committee votes2 on the re-vote's `Cert1` by the step it arrives in. -/
theorem votes2R (k : PubKey) (hl : lag k 0 = 0) : ∃ j, j ≤ 9
    ∧ Output.send (.vote2 ⟨(C2 0).data, ⟨2⟩, k⟩) ∈ (tr k j).output := by
  refine Classical.byContradiction fun hneg => settled k 9 (.vote2 CR)
    ⟨⟨blk 0, hasCR (by omega), hasProposal_of 0 (by simp [at0]), ⟨show (1 : Nat) ≤ 2 by omega, rfl⟩,
      payload_of 0 hl (by simp [at0])⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -, -⟩ := sent_vote2 hv hjv
    have h1 : cv u' = 2 := view_inj hvv
    obtain rfl : u' = 0 := by have := cv_facts u'; omega
    exact hneg ⟨j, by omega, hjv⟩
  · obtain ⟨x, rfl, hlt⟩ := hasCert2 hc2
    have := at0_facts 10 12 x
    omega
  · rintro (ht | ⟨tc, htc, hle⟩)
    · exact not_timedOut ht (fun h => absurd h (by show ¬ (2 : Nat) ≤ 1; omega)) (fun _ => by omega)
    · rcases received_tc htc with ⟨rfl, -⟩ | ⟨rfl, h14⟩
      · exact absurd hle (by show ¬ (2 : Nat) ≤ 1; omega)
      · omega
  · refine Kit.afterFloor_of input hv (by show (0 : Nat) < 2; omega) fun x hx => ?_
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
    · show 0 < 2 + 20; omega
    · rw [blk_view]; show bv y < 2 + 20
      have := bv_facts y; have := at0_facts 1 8 y
      omega

include hv in
/-- Every member of a later block's committee votes2 for it by the step its payload arrives in. -/
theorem votes2 (k : PubKey) (u : Nat) (hu : 1 ≤ u) (hl : lag k u = 0) : ∃ j, j ≤ 8 * u + 11
    ∧ Output.send (.vote2 ⟨(C2 u).data, ⟨cv u⟩, k⟩) ∈ (tr k j).output := by
  have hf := at0_facts 3 11 u
  have hcv := (cv_facts u).2 hu
  have hbv := (bv_facts u).2 hu
  refine Classical.byContradiction fun hneg => settled k (8 * u + 11) (.vote2 (certOf (blk u)))
    ⟨⟨blk u, hasCert1_of u (by have := at0_facts 5 10 u; omega), hasProposal_of u (by have := at0_facts 1 8 u; omega),
      ⟨Nat.le_refl _, rfl⟩, payload_of u hl (by omega)⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -, -⟩ := sent_vote2 hv hjv
    rw [cert_view] at hvv
    have h1 : cv u' = bv u := view_inj hvv
    obtain rfl : u' = u := by have := cv_facts u'; omega
    exact hneg ⟨j, by omega, hjv⟩
  · obtain ⟨x, rfl, hlt⟩ := hasCert2 hc2
    rw [cert_view] at hv2
    have h1 : cv x = bv u := view_inj hv2
    obtain rfl : x = u := by have := cv_facts x; omega
    have := at0_facts 10 12 x
    omega
  · rw [cert_view]
    rintro (ht | ⟨tc, htc, hle⟩)
    · refine not_timedOut ht (fun h => ?_) (fun h => ?_)
      · have h' : bv u ≤ 1 := h; omega
      · have h' : bv u ≤ 3 := h; omega
    · have h' : bv u ≤ tc.view.toNat := hle
      rcases received_tc htc with ⟨rfl, -⟩ | ⟨rfl, -⟩
      · have : bv u ≤ 1 := h'; omega
      · have : bv u ≤ 3 := h'; omega
  · refine Kit.afterFloor_of input hv (by rw [cert_view]; show 0 < bv u; omega) fun x hx => ?_
    rw [cert_view]
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
    · show 0 < bv u + 20; omega
    · rw [blk_view]; show bv y < bv u + 20
      have := bv_facts y; have := at0_facts 1 8 y
      omega

end Sends

/-! ## Time -/

section Time

/-- When step `n` happens: the first timer at `33`, which is GST, the timer for view three `33` after the re-vote's `Cert1`, and one unit a step otherwise. -/
def tm (n : Nat) : Nat := if n ≤ 3 then n else if n ≤ 12 then n + 29 else n + 58

theorem tm_facts (n : Nat) : (n ≤ 3 → tm n = n) ∧ (4 ≤ n → n ≤ 12 → tm n = n + 29) ∧ (13 ≤ n → tm n = n + 58) := by
  unfold tm
  refine ⟨fun h => ite_eq_left h, fun h1 h2 => ?_, fun h => ?_⟩
  · rw [ite_eq_right (by omega), ite_eq_left h2]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega)]

theorem tm_lt {n m : Nat} (h : n < m) : tm n < tm m := by
  have := tm_facts n; have := tm_facts m; omega

theorem tm_succ (n : Nat) : tm n ≤ tm (n + 1) := Nat.le_of_lt (tm_lt (Nat.lt_succ_self n))

theorem tm_mono {n m : Nat} (h : n ≤ m) : tm n ≤ tm m := Kit.mono_of_succ tm_succ h

theorem tm_ge (n : Nat) : n ≤ tm n := by have := tm_facts n; omega

/-- A node enters view three at step nine, and never enters view one. -/
theorem vAt_entry {n w : Nat} (h1 : vAt (n + 1) = w) (h2 : vAt n ≠ w) : (w = 1 → False) ∧ (w = 3 → n = 9) := by
  have := (vAt_ge (n + 1)).2.2.2.2.1
  by_cases h15 : 14 < n + 1
  · refine ⟨fun h => by omega, fun h => by omega⟩
  · have := vAt_first (n := n + 1) (by omega)
    have := vAt_first (n := n) (by omega)
    refine ⟨fun h => by omega, fun h => by omega⟩

end Time

/-! ## The network -/

section Net

variable (hv : ∀ b, BlockValid b)

include hv in
theorem backed1 (u : Nat) : Cert1Backed (C := C) (fun k _ => tr k) (certOf (blk u)) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k, by rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes1 hv k u (lag_member hk.1)
  rw [cert_view]
  exact ⟨j, hj⟩

include hv in
theorem backedCR : Cert1Backed (C := C) (fun k _ => tr k) CR :=
  ⟨fun k => C.members ⟨1⟩ k ∧ C.honest ⟨1⟩ k, members_quorum _, fun k hk _ =>
    let ⟨j, _, hj⟩ := votes1R hv k (lag_member hk.1); ⟨j, hj⟩⟩

include hv in
theorem backed2 (u : Nat) : Cert2Backed (C := C) (fun k _ => tr k) (C2 u) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k,
    by show C.Quorum (certOf (blk u)).data.epoch _; rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  cases u with
  | zero => obtain ⟨j, -, hj⟩ := votes2R hv k (lag_member hk.1); exact ⟨j, hj⟩
  | succ u => obtain ⟨j, -, hj⟩ := votes2 hv k (u + 1) (by omega) (lag_member hk.1); exact ⟨j, hj⟩

theorem lockLE_base_CR (k : PubKey) : LockLE (base k) CR := by
  unfold base; split
  · exact lockLE_refl _
  · exact lockLE_blk0_CR

theorem tcBacked1 : TimeoutCertBacked (C := C) (fun k _ => tr k) T1 :=
  ⟨fun k => C.members ⟨1⟩ k ∧ C.honest ⟨1⟩ k, members_quorum _, fun k _ _ =>
    ⟨_, ⟨rfl, rfl, rfl, lockLE_refl _⟩, ⟨_, times_out_one k⟩⟩⟩

theorem tcBacked3 : TimeoutCertBacked (C := C) (fun k _ => tr k) T3 :=
  ⟨fun k => C.members ⟨2⟩ k ∧ C.honest ⟨2⟩ k, members_quorum _, fun k _ _ =>
    ⟨_, ⟨rfl, rfl, rfl, lockLE_base_CR k⟩, ⟨_, times_out_three k⟩⟩⟩

theorem tcChecked1 : TimeoutLockChecked (C := C) (fun k _ => tr k) cfg T1 :=
  ⟨Or.inl rfl, show (0 : Nat) ≤ 1 by omega⟩

include hv in
theorem tcChecked3 : TimeoutLockChecked (C := C) (fun k _ => tr k) cfg T3 :=
  ⟨Or.inr (backedCR hv), show (2 : Nat) ≤ 3 by omega⟩

/-- The honest nodes running the machine on their schedules. -/
def net : TimedNetwork cfg leader C where
  honestQuorum := members_quorum
  trace k _ := tr k
  safe k _ n := .of_every (protocol hv k n).toSafeHistory
  cert1Genuine k _ n x hc := by
    rcases Input.mem_cert1.mp hc with hin | ⟨c2, p, hin⟩ | ⟨s, p, vid, hin, rfl⟩
    · rcases input_cert1 hin with ⟨-, rfl⟩ | ⟨u, -, rfl⟩
      · exact Or.inr (backedCR hv)
      · exact Or.inr (backed1 hv u)
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
    · rcases input_tc hin with ⟨-, rfl⟩ | ⟨-, rfl⟩
      · exact ⟨tcBacked1, tcChecked1⟩
      · exact ⟨tcBacked3, tcChecked3 hv⟩
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      rcases blk_evidence u with h | ⟨-, h⟩ <;> rw [h] at hte
      · cases hte
      · cases hte; exact ⟨tcBacked3, tcChecked3 hv⟩
    · rcases input_revote hin with ⟨-, -, rfl⟩ | ⟨u, -, -, rfl⟩ <;> cases hte
  revoteGenuine k _ n _ r hin := by
    rcases input_revote hin with ⟨-, -, rfl⟩ | ⟨u, -, -, rfl⟩
    · exact backedCR hv
    · exact backed1 hv u
  time _ _ n := tm n
  timeMono _ _ n := tm_succ n
  protocol k _ n := .of_every (protocol hv k n)
  timeoutCertCausal k _ n tc hin := by
    rcases input_tc hin with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
    · exact ⟨fun k => C.members ⟨1⟩ k ∧ C.honest ⟨1⟩ k, members_quorum _, fun k' _ _ =>
        ⟨_, _, ⟨rfl, rfl, rfl, lockLE_refl _⟩, times_out_one k', tm_lt (by omega)⟩⟩
    · exact ⟨fun k => C.members ⟨2⟩ k ∧ C.honest ⟨2⟩ k, members_quorum _, fun k' _ _ =>
        ⟨_, _, ⟨rfl, rfl, rfl, lockLE_base_CR k'⟩, times_out_three k', tm_lt (by omega)⟩⟩
  oneHonestCausal k _ n v hin := (input_oneHonest hin).elim
  authentic k _ n l msg hin _ _ := by
    cases hi : (tr k n).input <;> rw [hi] at hin <;> simp only [Input.sentBy, reduceCtorEq] at hin
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨u, rfl, rfl, rfl⟩ := input_proposal_any hi
      obtain ⟨j, hj, hjp⟩ := proposes hv u
      exact ⟨j, hjp, tm_lt (by have := at0_facts 0 7 u; have := at0_facts 1 8 u; omega)⟩
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      rcases input_revote hi with ⟨rfl, rfl, rfl⟩ | ⟨u, rfl, rfl, rfl⟩
      · obtain ⟨j, hj, hjr⟩ := revotes_two hv
        exact ⟨j, hjr, tm_lt (by omega)⟩
      · obtain ⟨j, hj, hjr⟩ := revotes hv u
        exact ⟨j, hjr, tm_lt (by have := at0_facts 5 11 u; have := at0_facts 8 14 u; omega)⟩

theorem net_time {k : PubKey} {hk : C.Honest k} {n : Nat} : (net hv).time k hk n = tm n := rfl

theorem by_at {k : PubKey} {hk : C.Honest k} {T m : Nat} {P : History → Prop}
    (hm : 0 < m → tm (m - 1) ≤ T) (hp : P (H k m)) : (net hv).By k hk T P :=
  Kit.by_at (fun _ _ => rfl) (fun _ _ _ => rfl) tm_succ hm hp

/-- What holds after step `j`, holds by its time. -/
theorem by_step {k : PubKey} {hk : C.Honest k} {T : Nat} {P : History → Prop} (j : Nat)
    (hj : tm j ≤ T) (hp : P (H k (j + 1))) : (net hv).By k hk T P :=
  by_at hv (fun _ => by simp only [Nat.add_sub_cancel]; exact hj) hp

/-- What a node holds after the eighth step, it holds by GST plus `Δ`. -/
theorem by_gst {k : PubKey} {hk : C.Honest k} {t : Nat} {P : History → Prop} (hp : P (H k 9)) :
    (net hv).By k hk (max t 33 + 4) P :=
  by_step hv 8 (by have := (tm_facts 8).2.1 (by omega) (by omega); omega) hp

theorem sentBy {k : PubKey} {hk : C.Honest k} {t : Nat} {m : Message} (hs : (net hv).SentByTime k hk t m) :
    ∃ j, tm j ≤ t ∧ Output.send m ∈ (tr k j).output :=
  Kit.sentBy (N := net hv) (fun _ _ => rfl) (fun _ _ _ => rfl) hs

/-- What every node holds by a step it held before, or by GST plus `Δ` if it got it by the eighth step. -/
theorem by_same {k' : PubKey} {hk' : C.Honest k'} {n : Nat} {P : History → Prop} (x : Nat)
    (hp : ∀ m, x < m → P (H k' m)) (hx : x < n + 1 ∨ x < 9) : (net hv).By k' hk' (max (tm n) 33 + 4) P := by
  have := Nat.le_max_left (tm n) 33
  by_cases hn : x < n + 1
  · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hp _ hn)
  · exact by_gst hv (hp 9 (by omega))

/-- Something a node gets at step `m`, from what held at step `n`, at most four steps before and with no timer between. -/
theorem by_soon {k' : PubKey} {hk' : C.Honest k'} {n T : Nat} {P : History → Prop} (m : Nat)
    (hp : ∀ i, m < i → P (H k' i)) (hnm : n ≤ m) (hm4 : m ≤ n + 4) (h4 : 4 ≤ n) (h12 : ¬ (n ≤ 12 ∧ 13 ≤ m))
    (hT : tm n ≤ T) : (net hv).By k' hk' (T + 4) P := by
  have := tm_facts n; have := tm_facts m
  exact by_step hv m (by omega) (hp _ (by omega))

/-- Something a node gets at step `x`, at most four steps after step `n` and with no timer between, or already held. -/
theorem by_recv {k' : PubKey} {hk' : C.Honest k'} {n T : Nat} {P : History → Prop} (x : Nat)
    (hp : ∀ i, x < i → P (H k' i)) (hx : x ≤ n + 4) (h4 : 4 ≤ n) (h12 : ¬ (n ≤ 12 ∧ 13 ≤ x))
    (hT : tm n ≤ T) : (net hv).By k' hk' (T + 4) P := by
  by_cases hn : x < n + 1
  · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hp _ hn)
  · exact by_soon hv x hp (by omega) hx h4 h12 hT

include hv in
/-- A header for every view `a` is ready to propose in arrives within `Δ`, after GST. -/
theorem header_arrives {k : PubKey} {hk : C.Honest k} {n : Nat} {p : Proposal}
    (hready : ProposalReady cfg leader k (H k (n + 1)) p) :
    (net hv).By k hk (max (tm n) 33 + 4) fun hist =>
      ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
  obtain ⟨hlead, ⟨-, hnext, hep, hnum⟩, hj, -, hop, hsafe, -, -⟩ := hready
  obtain rfl := FiveNodes.leader_eq hlead
  have hmax := Nat.le_max_left (tm n) 33
  have hmax' := Nat.le_max_right (tm n) 33
  have hc := hasCert1_of_certJustified hj
  have hdone : ∀ m, m ≤ n + 1 ∨ tm (m - 1) ≤ max (tm n) 33 + 4 → ∀ hdr',
      hdr'.blockNumber = p.blockHeader.blockNumber →
      (H a m).Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') →
      (net hv).By a hk (max (tm n) 33 + 4) fun hist =>
        ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
          ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
    intro m hm hdr' h1 h2
    rcases hm with hm | hm
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨hdr', h1, received_mono hm h2⟩
    · exact by_at hv (m := m) (fun _ => hm) ⟨hdr', h1, h2⟩
  -- The epoch the proposal's height puts it in.
  have hpe : ∀ x, p.blockHeader.blockNumber = ⟨x + 1⟩ → p.epoch = ⟨x + 1⟩ := fun x hx => by
    rw [hep, hx]; exact epochOf_one_height (by omega)
  rcases hasCert1 hc with hpc | ⟨hpc, -⟩ | ⟨x, hpc, -⟩
  · have hnum' : p.blockHeader.blockNumber = ⟨0 + 1⟩ := by rw [← hnum, hpc]; rfl
    rcases hnext with ⟨-, h⟩ | ⟨tc, hte, htv⟩
    · -- On genesis: view one, whose header comes first.
      have hpv : p.viewNumber = ⟨1⟩ := by rw [← h, hpc]; rfl
      refine hdone 1 (Or.inr (by show tm 0 ≤ _; have := (tm_facts 0).1 (by omega); omega)) (hdr 1)
        hnum'.symm ?_
      rw [hpv, hpc]; exact recv_first 0 (by omega) (by omega)
    · obtain ⟨he, -⟩ := hsafe _ hte
      rcases (justified_held hj).2 tc hte with ⟨rfl, h6⟩ | ⟨rfl, -⟩
      · -- Behind the first timeout certificate: view two, whose header arrives after it.
        have hpv : p.viewNumber = ⟨2⟩ := by rw [← htv]; rfl
        refine hdone 8 (Or.inr (by have := (tm_facts 7).2.1 (by omega) (by omega); show tm 7 ≤ _; omega))
          (hdr 1) hnum'.symm ?_
        rw [hpv, hpc]; exact recv_first 7 (by omega) (by omega)
      · rw [hpe 0 hnum'] at he
        exact absurd (congrArg EpochNumber.toNat he) (by show ¬ (2 : Nat) = 0 + 1; omega)
  · -- The re-vote's `Cert1` is not at its block's view, which an opening proposal names.
    exfalso
    have hnum' : p.blockHeader.blockNumber = ⟨1 + 1⟩ := by rw [← hnum, hpc]; rfl
    have hen : EntersEpoch cfg p := by
      show IsLastBlock (p.blockHeader.blockNumber - 1) 1
      rw [hnum']; exact last_block (n := 1) (by omega)
    obtain ⟨⟨q, hq, hqv, -⟩, -⟩ := hop hen
    rw [hpc] at hqv
    rcases hasProposal hq with rfl | ⟨y, rfl, -⟩
    · exact absurd (congrArg ViewNumber.toNat hqv) (by show ¬ (0 : Nat) = 2; omega)
    · rw [blk_view] at hqv
      exact absurd (congrArg ViewNumber.toNat hqv) (by show ¬ bv y = 2; have := bv_facts y; omega)
  · have hnum' : p.blockHeader.blockNumber = ⟨x + 1 + 1⟩ := by rw [← hnum, hpc, cert_number]; rfl
    rcases hnext with ⟨-, h⟩ | ⟨tc, hte, htv⟩
    · -- On a block: the view after it, which opens the next epoch behind its `Cert2`.
      have hpv : p.viewNumber = ⟨bv x + 1⟩ := by rw [← h, hpc, cert_view]; rfl
      have hen : EntersEpoch cfg p := by
        show IsLastBlock (p.blockHeader.blockNumber - 1) 1
        rw [hnum']; exact last_block (n := x + 1) (by omega)
      obtain ⟨-, c2, hc2, hc2v, hd⟩ := hop hen
      rcases hc2 with hc2 | rfl
      case inr =>
        exfalso
        have h0 := congrArg Vote2Data.blockNumber hd
        rw [hpc] at h0
        have h2 : (certOf (blk x)).data.toVote2.blockNumber = ⟨x + 1⟩ := cert_number _
        rw [h2] at h0
        exact absurd (congrArg BlockNumber.toNat h0) (by show ¬ (0 : Nat) = x + 1; omega)
      obtain ⟨y, rfl, hlt⟩ := hasCert2 hc2
      obtain rfl : y = x := by
        have h0 := congrArg Vote2Data.blockNumber hd
        rw [hpc] at h0
        have h1 : (C2 y).data.blockNumber = ⟨y + 1⟩ := cert_number y
        have h2 : (certOf (blk x)).data.toVote2.blockNumber = ⟨x + 1⟩ := cert_number x
        rw [h1, h2] at h0
        have := number_inj h0; omega
      rw [hpv] at hc2v
      have hcv : cv y < bv y + 1 := hc2v
      cases y with
      | zero => exact absurd hcv (by decide)
      | succ y =>
        have := at0_facts 10 12 (y + 1)
        have := tm_mono (show 8 * (y + 1) + 12 ≤ n by omega)
        have := (tm_facts (8 * (y + 1) + 12)).2.2 (by omega)
        have := (tm_facts (8 * (y + 2) + 7)).2.2 (by omega)
        refine hdone (8 * (y + 2) + 7 + 1) (by
            by_cases hn : 8 * (y + 2) + 7 + 1 ≤ n + 1
            · exact Or.inl hn
            · exact Or.inr (by show tm (8 * (y + 2) + 7) ≤ _; omega)) (hdr (y + 3)) hnum'.symm ?_
        rw [hpv, hpc]
        exact recv_later (y + 2) 0 (by omega) (by omega) (by omega)
    · obtain ⟨he, -⟩ := hsafe _ hte
      rw [hpe (x + 1) hnum'] at he
      rcases (justified_held hj).2 tc hte with ⟨rfl, -⟩ | ⟨rfl, h14⟩
      · exact absurd (congrArg EpochNumber.toNat he) (by show ¬ (1 : Nat) = x + 1 + 1; omega)
      · -- After the timeout certificate for view three: view four, opening epoch two.
        obtain rfl : x = 0 := by have : 2 = x + 1 + 1 := congrArg EpochNumber.toNat he; omega
        have hpv : p.viewNumber = ⟨4⟩ := by rw [← htv]; rfl
        have := tm_mono (show 14 ≤ n by omega)
        have := (tm_facts 14).2.2 (by omega)
        have := (tm_facts 15).2.2 (by omega)
        refine hdone 16 (by
            by_cases hn : 16 ≤ n + 1
            · exact Or.inl hn
            · exact Or.inr (by show tm 15 ≤ _; omega)) (hdr 2) hnum'.symm ?_
        rw [hpv, hpc]
        exact recv_later 1 0 (by omega) (by omega) (by omega)

/-- The timer for a view does not fire before `τ` has passed since the node entered it. -/
theorem timer_not_early {k : PubKey} {n m : Nat} {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v)
    (hnot : n = 0 ∨ ¬ (H k n).InView cfg v) (hinm : input k m = .timeout v) : tm n + 33 ≤ tm m := by
  have h1 : vAt (n + 1) = v.toNat := (congrArg ViewNumber.toNat (inView_eq hin)).symm
  rcases hnot with rfl | hnot
  · have : vAt 1 = 1 := by decide
    rcases input_timeout hinm with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
    · have := tm_facts 0; have := tm_facts 4; omega
    · exact absurd (h1.symm.trans this) (by decide)
  have h2 : vAt n ≠ v.toNat := fun h => hnot (by have := inView (k := k) n; rwa [h] at this)
  obtain ⟨e1, e3⟩ := vAt_entry h1 h2
  rcases input_timeout hinm with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
  · exact (e1 rfl).elim
  · rw [e3 rfl]
    have := tm_facts 9; have := tm_facts 13
    omega

/-- The timer for a view fires within `τ` of the node's entering it, or of its last firing, unless the node has moved on. -/
theorem timer_fires {k : PubKey} (n : Nat) {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v) :
    ∃ m, n < m ∧ tm m ≤ tm n + 33 ∧ (input k m = .timeout v ∨ ∃ w, v < w ∧ (H k (m + 1)).InView cfg w) := by
  have hv' := inView_eq hin
  subst hv'
  have hT := tm_facts n
  rcases steps (n + 1) with hn | ⟨u, r, hu, hr, hn1⟩
  · obtain ⟨h1, h2, h3⟩ := vAt_first hn
    by_cases r10 : 10 ≤ n + 1
    · rw [h3 r10]
      by_cases h13 : n = 13
      · subst h13
        refine ⟨14, by omega, by have := tm_facts 14; omega, Or.inr ⟨_, ?_, inView _⟩⟩
        rw [show vAt (14 + 1) = 4 from (vAt_later 1 0 (by omega) (by omega)).1 (by omega)]
        show (3 : Nat) < 4; omega
      · exact ⟨13, by omega, by have := tm_facts 13; omega, Or.inl (by rw [input_first (by omega)]; rfl)⟩
    by_cases r6 : 6 ≤ n + 1
    · rw [h2 r6 (by omega)]
      refine ⟨9, by omega, by have := tm_facts 9; omega, Or.inr ⟨_, ?_, inView _⟩⟩
      rw [(vAt_first (n := 9 + 1) (by omega)).2.2 (by omega)]; show (2 : Nat) < 3; omega
    · rw [h1 (by omega)]
      by_cases h4 : n = 4
      · subst h4
        refine ⟨5, by omega, by have := tm_facts 5; omega, Or.inr ⟨_, ?_, inView _⟩⟩
        rw [(vAt_first (n := 5 + 1) (by omega)).2.1 (by omega) (by omega)]; show (1 : Nat) < 2; omega
      · exact ⟨4, by omega, by have := tm_facts 4; omega, Or.inl (by rw [input_first (by omega)]; rfl)⟩
  · rw [hn1]
    obtain ⟨h0, h4⟩ := vAt_later u r hu hr
    by_cases r4 : 4 ≤ r
    · -- The view after a block: the next block's `Cert1` moves the node on.
      rw [h4 r4]
      refine ⟨8 * (u + 1) + 3 + 7, by omega, by have := tm_facts (8 * (u + 1) + 3 + 7); omega,
        Or.inr ⟨_, ?_, inView _⟩⟩
      rw [show 8 * (u + 1) + 3 + 7 + 1 = 8 * (u + 1) + 4 + 7 by omega,
        (vAt_later (u + 1) 4 (by omega) (by omega)).2 (by omega)]
      show u + 4 < u + 1 + 4; omega
    · -- The view of a block: its `Cert1` moves the node on.
      rw [h0 (by omega)]
      refine ⟨8 * u + 3 + 7, by omega, by have := tm_facts (8 * u + 3 + 7); omega, Or.inr ⟨_, ?_, inView _⟩⟩
      rw [show 8 * u + 3 + 7 + 1 = 8 * u + 4 + 7 by omega, (vAt_later u 4 hu (by omega)).2 (by omega)]
      show u + 3 < u + 4; omega

include hv in
/-- Every delivery within `Δ = 4` after GST `33`, with view timer `τ = 33`. -/
theorem sync : Synchrony (net hv) 33 4 33 where
  proposal l hl n p hsend _ k hk hmem _ := by
    simp only [net_time hv]
    obtain ⟨rfl, u, rfl, hu⟩ := sent_proposal hv hsend
    rw [blk_epoch] at hmem
    have hl0 := lag_member hmem
    have hP : ∀ m, at0 1 8 u < m → (∃ vid, ShareMatches (blk u) vid ∧ (H k m).Received (.proposal a (blk u) (some vid))) :=
      fun m hm => by
        have := recv_u (k := k) (n := m) u 1 1 (by omega) (by omega) rfl hm
        simp only [phase, hl0, ite_eq_left] at this
        exact ⟨⟨⟨bv u⟩, (blk u).payloadCommit⟩, ⟨by rw [blk_view], rfl⟩, this⟩
    cases u with
    | zero => exact by_gst hv (hP 9 (by simp [at0]))
    | succ u =>
      rw [(at0_facts _ _ _).2 (by omega)] at hu
      exact by_recv hv (8 * (u + 1) + 8) (fun i hi => hP i (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
        (by omega) (by omega) (by omega) (Nat.le_max_left _ _)
  revote l hl n r hsend _ k hk _ _ := by
    simp only [net_time hv]
    obtain ⟨rfl, ⟨x, rfl, hx⟩ | ⟨rfl, h9⟩⟩ := sent_revote hv hsend
    · cases x with
      | zero => exact by_gst hv (recv_revote 0 (by rw [(at0_facts _ _ _).1 rfl]; omega))
      | succ x =>
        rw [(at0_facts _ _ _).2 (by omega)] at hx
        exact by_recv hv (8 * (x + 1) + 14) (fun i hi => recv_revote (x + 1) (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
          (by omega) (by omega) (by omega) (Nat.le_max_left _ _)
    · exact by_recv hv 12 (fun i hi => recv_R2 hi) (by omega) (by omega) (by omega) (Nat.le_max_left _ _)
  cert1 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    have := Nat.le_max_left t 33
    rcases sent_vote1 hv hj with ⟨x, -, heq, hx⟩ | ⟨heq, h8⟩
    · simp only [Vote.mk.injEq] at heq
      obtain ⟨rfl, rfl, -⟩ := heq
      rw [show (⟨(certOf (blk x)).data, ⟨bv x⟩⟩ : Cert1) = certOf (blk x) by rw [← cert_view x]]
      cases x with
      | zero => exact by_gst hv (recv_cert1 0 (by rw [(at0_facts _ _ _).1 rfl]; omega))
      | succ x =>
        rw [(at0_facts _ _ _).2 (by omega)] at hx
        exact by_recv hv (n := j) (8 * (x + 1) + 10) (fun i hi => recv_cert1 (x + 1) (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
          (by omega) (by omega) (by omega) (by omega)
    · simp only [Vote.mk.injEq] at heq
      obtain ⟨rfl, rfl, -⟩ := heq
      exact by_recv hv (n := j) 9 (fun i hi => recv_CR hi) (by omega) (by omega) (by omega) (by omega)
  cert2 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, heq, -, hx⟩ := sent_vote2 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    have := Nat.le_max_left t 33
    have hc : ∀ i, at0 10 12 x < i → (H k i).Received (.certificate2 ⟨(C2 x).data, ⟨cv x⟩⟩) :=
      fun i hi => recv_c2 x hi
    cases x with
    | zero =>
      rw [(at0_facts _ _ _).1 rfl] at hx
      exact by_recv hv (n := j) 10 (fun i hi => hc i (by rw [(at0_facts _ _ _).1 rfl]; omega))
        (by omega) (by omega) (by omega) (by omega)
    | succ x =>
      rw [(at0_facts _ _ _).2 (by omega)] at hx
      exact by_recv hv (n := j) (8 * (x + 1) + 12) (fun i hi => hc i (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
        (by omega) (by omega) (by omega) (by omega)
  cert2Spread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc
    exact by_same hv _ (fun m hm => hasCert2_of x hm) (Or.inl hx)
  certSpread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    rcases hasCert1 hc with rfl | ⟨rfl, h9⟩ | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_same hv 9 (fun m hm => hasCR hm) (Or.inl h9)
    · exact by_same hv (at0 5 10 x) (fun m hm => hasCert1_of x hm) (Or.inl hx)
  lockSpread c k hk n hc k' hk' hm _ := by
    simp only [net_time hv]
    rcases hasCert1 hc with rfl | ⟨rfl, h9⟩ | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · have hl : lag k' 0 = 0 := lag_member hm
      exact by_same hv 9 (fun m hm' => lockable_iff.mpr (Or.inr (Or.inl ⟨rfl, hl, hm'⟩))) (Or.inl h9)
    · have hl : lag k' x = 0 := lag_member (by rw [cert_epoch] at hm; exact hm)
      have hlock : ∀ m, lk k' x < m → (H k' m).Lockable cfg (certOf (blk x)) :=
        fun m hm' => lockable_iff.mpr (Or.inr (Or.inr ⟨x, rfl, hm'⟩))
      have hlk := lk_facts k' x
      cases x with
      | zero =>
        rw [(at0_facts _ _ _).1 rfl] at hx
        rw [hlk.1 rfl hl] at hlock
        exact by_same hv 5 hlock (Or.inl hx)
      | succ x =>
        rw [(at0_facts _ _ _).2 (by omega)] at hx
        rw [hlk.2.2 (by omega), hl] at hlock
        exact by_recv hv (8 * (x + 1) + 11) (fun i hi => hlock i (by omega)) (by omega) (by omega)
          (by omega) (Nat.le_max_left _ _)
  blockSpread c b _ k hk n hcb k' hk' _ := by
    simp only [net_time hv]
    rcases hasProposal hcb.2 with rfl | ⟨y, rfl, hy⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_same hv (at0 1 8 y) (fun m hm => hasProposal_of y hm) (Or.inl hy)
  timeoutCert e q v t hq hvotes k hk _ := by
    have hh := fun k hk => Committee.Honest.of (hvotes k hk).1
    obtain ⟨_hka, L, hsent⟩ := hvotes a (quorum_a hq hh)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hsent
    have := Nat.le_max_left t 33
    rcases sent_timeout hj with ⟨rfl, heq⟩ | ⟨rfl, heq⟩
    · simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
      obtain ⟨⟨rfl, -⟩, rfl, -⟩ := heq
      exact by_gst hv ⟨T1, rfl, rfl, recv_T1 (by omega)⟩
    · simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
      obtain ⟨⟨rfl, -⟩, rfl, -⟩ := heq
      exact by_soon hv 14 (fun i hi => ⟨T3, rfl, rfl, recv_T3 hi⟩) (show 13 ≤ 14 by omega) (by omega)
        (by omega) (by omega) (by omega)
  timeoutCertSpread tc k hk n hin k' hk' _ := by
    refine Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate tc))
      (fun _ h => ⟨tc, rfl, rfl, h⟩) (Kit.by_slower ?_)
    simp only [net_time hv]
    rcases received_tc hin with ⟨rfl, h6⟩ | ⟨rfl, h14⟩
    · exact by_same hv 6 (fun m hm => recv_T1 hm) (Or.inl h6)
    · exact by_same hv 14 (fun m hm => recv_T3 hm) (Or.inl h14)
  timeoutLockSpread tc k hk n hin k' hk' hm _ := by
    simp only [net_time hv]
    rcases received_tc hin with ⟨rfl, -⟩ | ⟨rfl, h14⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · have hl : lag k' 0 = 0 := lag_member hm
      exact by_same hv 9 (fun m hm' => lockable_iff.mpr (Or.inr (Or.inl ⟨rfl, hl, hm'⟩))) (Or.inl (by omega))
  epochChange c2 b hcm _ k hk n hbc k' hk' _ := by
    obtain ⟨hb, hc2⟩ := hbc
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc2
    have hbn : b.blockHeader.blockNumber = ⟨x + 1⟩ := by
      rw [← cert_number x]; exact (congrArg Vote2Data.blockNumber hcm.2).symm
    obtain ⟨rfl, -⟩ := block_of_number hb hbn
    have hEC : ∀ m, at0 11 13 x < m → ∃ c1, (H k' m).TookEpochChange cfg c1 (C2 x) (blk x) :=
      fun m hm => ⟨_, tookEpochChange_of x hm⟩
    have := at0_facts 10 12 x; have := at0_facts 11 13 x
    exact by_recv hv (at0 11 13 x) hEC (by omega) (by omega) (by omega) (Nat.le_max_left _ _)
  proposalValid _ _ _ p _ _ := hv p
  validatedSound _ _ _ _ _ _ b _ := hv b
  validated k hk n s p vid hin _ _ := by
    simp only [net_time hv]
    obtain ⟨u, rfl, -, -, rfl, -⟩ := input_proposal hin
    rw [blk_view]
    have hval : ∀ m, at0 2 9 u < m → (H k m).Received (.blockValidated ⟨bv u⟩ (blockHash (blk u))) :=
      fun m hm => recv_u (k := k) u 2 2 (by omega) (by omega) rfl hm
    cases u with
    | zero => exact by_gst hv (hval 9 (by simp [at0]))
    | succ u =>
      rw [(at0_facts _ _ _).2 (by omega)]
      exact by_soon hv (8 * (u + 1) + 9) (fun i hi => hval i (by rw [(at0_facts _ _ _).2 (by omega)]; omega))
        (show 8 * (u + 1) + 8 ≤ _ by omega) (by omega) (by omega) (by omega) (Nat.le_max_left _ _)
  header k hk n p _ _ hready := by simp only [net_time hv]; exact header_arrives hv hready
  timeUnbounded _ _ T := ⟨T + 1, by have := tm_ge (T + 1); show T < tm (T + 1); omega⟩
  timerNotEarly k hk n m v hin hnot _ hinm := timer_not_early hin hnot hinm
  timerFires k hk n v hin _ := timer_fires n hin

theorem rotation : LeaderRotation C leader := fun _ v => ⟨v, Nat.le_refl _, a, rfl, Or.inl rfl, Or.inl rfl⟩

include hv in
/-- **The liveness premises can be met together, with an epoch that ends on its re-vote's `Cert2`.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv) 33 4 33 ∧ Prompt (net hv) 0
      ∧ 8 * 4 + 3 * 0 < 33 ∧ LeaderRotation C leader :=
  ⟨cfg_coherent, sync hv, prompt_of_machine _ 0 (fun k _ => ⟨input k, rfl⟩)
    (fun _ hk => Kit.steady_of_uniform (fun _ _ _ h => h) hk), by decide, rotation⟩

include hv in
/--
And the first epoch ends on the re-vote's `Cert2`. Every node times view one out
before the block's `Cert1` arrives, nobody votes2 at the block's view, and the only
`Cert2` over the block is the re-vote's, at view two. `a` asks for the re-vote,
every node takes the epoch change with that `Cert2`, and the next epoch's first
block names the block's own `Cert1`, behind the timeout certificate for view three.
-/
theorem late_cert :
    (∀ k, Output.send (.timeoutVote ⟨⟨⟨1⟩, certOf anchorB⟩, ⟨1⟩, k⟩) ∈ (tr k 4).output)
      ∧ (∀ k n, (H k n).HasCert1 cfg (certOf (blk 0)) → 5 < n)
      ∧ (∀ k j (vote : Vote2), Output.send (.vote2 vote) ∈ (tr k j).output → vote.view ≠ (blk 0).viewNumber)
      ∧ (∀ k n c2, (H k n).HasCert2 c2 → c2.data = (certOf (blk 0)).data.toVote2 → c2 = C2 0)
      ∧ (C2 0).view = ⟨2⟩
      ∧ (∃ j, Output.send (.revote (R 0)) ∈ (tr a j).output)
      ∧ (∀ k, (H k 12).TookEpochChange cfg (certOf (blk 0)) (C2 0) (blk 0))
      ∧ (blk 1).parentCert = certOf (blk 0) ∧ (blk 1).timeoutEvidence = some T3
      ∧ (∃ j, Output.send (.proposal (blk 1)) ∈ (tr a j).output) := by
  refine ⟨times_out_one, fun k n hc => ?_, fun k j vote hx hvw => ?_, fun k n c2 hc2 hd => ?_, rfl,
    let ⟨j, _, hj⟩ := revotes_zero hv; ⟨j, hj⟩, fun k => tookEpochChange_of 0 (by simp [at0]), rfl, rfl,
    let ⟨j, _, hj⟩ := proposes hv 1; ⟨j, hj⟩⟩
  · rcases hasCert1 hc with h | ⟨h, -⟩ | ⟨u, h, hu⟩
    · exact absurd (congrArg (·.view.toNat) h) (by show ¬ (1 : Nat) = 0; omega)
    · exact absurd (congrArg (·.view.toNat) h) (by show ¬ (1 : Nat) = 2; omega)
    · have := congrArg (·.view.toNat) h; simp only [cert_view] at this
      obtain rfl : 0 = u := bv_inj this
      rw [(at0_facts _ _ _).1 rfl] at hu; exact hu
  · obtain ⟨u, rfl, -, -⟩ := sent_vote2 hv hx
    rw [blk_view] at hvw
    have h1 : cv u = bv 0 := view_inj hvw
    have h0 : bv 0 = 1 := rfl
    have hf := cv_facts u
    by_cases hu : u = 0
    · have := hf.1 hu; omega
    · have := hf.2 (by omega); omega
  · obtain ⟨x, rfl, -⟩ := hasCert2 hc2
    have h0 := congrArg Vote2Data.blockNumber hd
    have h1 : (C2 x).data.blockNumber = ⟨x + 1⟩ := cert_number x
    have h2 : (certOf (blk 0)).data.toVote2.blockNumber = ⟨0 + 1⟩ := cert_number 0
    rw [h1, h2] at h0
    obtain rfl : x = 0 := by have := number_inj h0; omega
    rfl

include hv in
/-- So every honest node keeps deciding, after GST. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : C.Honest k) :
    (net hv).DecidesAfter k hk t := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met hv
  exact (Liveness.chainGrows (cfg := cfg) (leader := leader) (C := C) (net hv) 33 4 0 33
    hc hcf hs hp hb hr).2 t k hk (Or.inl (Kit.steady_of_uniform (fun _ _ _ h => h) hk))

end Net

end LateCertWitness
end NewProtocolImpl
