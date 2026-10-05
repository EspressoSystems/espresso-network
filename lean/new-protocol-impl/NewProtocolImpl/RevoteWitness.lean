module

public import NewProtocolImpl.WitnessKit
public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.FiveNodes

/-!
# A network whose re-votes are answered

In `NewProtocolImpl.EpochWitness` the leader's re-vote request arrives after the
epoch change, so nobody answers it. Here it arrives first. The members of the
outgoing committee vote1 on the last block again, at the request's view; the
`Cert1` that forms is one they lock on, and they vote2 on it, so the block gets a
second `Cert2`, at the re-vote's view.

The re-vote's `Cert1` reaches every node before the block's own `Cert2`, so `a`
never holds that `Cert2` in the re-vote's view and cannot open the next epoch there
(`OpensEpochJustified` asks for it). The re-vote's `Cert1` moves every node on to the
view after it, where nothing can be proposed either: the next epoch's first block
names its parent by the parent's own `Cert1`, at the parent's view, so it needs a
timeout certificate for the view before. `a` asks for a re-vote on the re-vote's
`Cert1` in that view, but the request arrives after the epoch change, so nobody
answers it. That view times out, and the timeout certificate's lock is the
re-vote's `Cert1`, the latest lock among its signers: the outgoing committee's
members lock on it, the node new to the committee only on the block's own `Cert1`,
through the epoch change. The leader builds on the block behind it, which the lock
allows (`LockAllows`).

The nodes and committees are `NewProtocolImpl.FiveNodes`'s: `a`, `b`, `c` and `nw` honest, `d`
faulty and silent, `c` outside the even epochs' committees and `nw` outside the odd
ones'. `a` leads every view. Epoch height is one, so block `u + 1` is the only
block of epoch `u + 1`, at view `3u + 1`. A node receives fifteen inputs to a
block:

* the header, the proposal with the node's share (outside the committee, the
  block alone), the validity report, the `Cert1`, and the payload (outside the
  committee, a second validity report);
* `a`'s re-vote request for view `3u + 2`, sent once it could lock on the block;
* the re-vote's `Cert1`, the block's `Cert2`, and the epoch change;
* `a`'s request for view `3u + 3`, on the re-vote's `Cert1`, which nobody answers;
* the re-vote's `Cert2`, and the epoch change with it;
* the timer for view `3u + 3`, the timeout certificate, and the one-honest
  indication.

Times (`tm`): one unit a step, except that the timer for view `3u + 3` fires
`τ = 33` after the nodes entered it, on the re-vote's `Cert1`. GST is zero,
`Δ = 4` and `δ = 0`.

`BlockValid` is taken as a hypothesis, being opaque; `decides` also takes
`CollisionFree`.
-/

@[expose] public section

namespace NewProtocolImpl
namespace RevoteWitness

open NewProtocol History
open FourNodes (hdr)
open Kit (received received_mono upTo_H upTo_self getElem_H tr_step sent_iff epoch_ext view_inj number_inj)
open FiveNodes (a b c nw d third leader C members_quorum third_honest d_faulty quorum_a honest_quorum anchorB certOf cfg cfg_coherent lag lag_cases lag_member lag_a lag_outside third_cases epochOf_one_height last_block)

/-! ## Blocks and the schedule -/

/--
Block `u + 1`, at view `3u + 1`, the only block of epoch `u + 1`. Every block after
the first opens its epoch behind the timeout certificate for the view after the
previous block's re-vote, whose lock is the re-vote's `Cert1`.
-/
def blk : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨1⟩, certOf anchorB, none, ⟨0⟩⟩
  | u + 1 => ⟨hdr (u + 2), ⟨3 * u + 4⟩, ⟨u + 2⟩, certOf (blk u),
      some ⟨⟨⟨u + 2⟩, ⟨(certOf (blk u)).data, ⟨3 * u + 2⟩⟩⟩, ⟨3 * u + 3⟩⟩, ⟨0⟩⟩

/-- The parent of `blk u`. -/
def parentOf : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

/-- The `Cert2` over `blk u`, at its view. -/
def C2 (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨3 * u + 1⟩⟩

/-- `a`'s re-vote request for `blk u`, for the view after it. -/
def R (u : Nat) : RevoteRequest := ⟨certOf (blk u), ⟨3 * u + 2⟩, none⟩

/-- The re-vote's `Cert1`: over `blk u` again, at the request's view. -/
def CR (u : Nat) : Cert1 := ⟨(certOf (blk u)).data, ⟨3 * u + 2⟩⟩

/--
`a`'s re-vote request on the re-vote's `Cert1`, for the view after it. It arrives
after the epoch change, so nobody answers it.
-/
def R2 (u : Nat) : RevoteRequest := ⟨CR u, ⟨3 * u + 3⟩, none⟩

/-- The re-vote's `Cert2`. -/
def C2R (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨3 * u + 2⟩⟩

/-- The timeout certificate for the view after the re-vote, of epoch `u + 2`, locked on the re-vote's `Cert1`. -/
def T (u : Nat) : TimeoutCert := ⟨⟨⟨u + 2⟩, CR u⟩, ⟨3 * u + 3⟩⟩

theorem blk_view (u : Nat) : (blk u).viewNumber = ⟨3 * u + 1⟩ := by cases u <;> rfl

theorem blk_number (u : Nat) : (blk u).blockHeader.blockNumber = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_epoch (u : Nat) : (blk u).epoch = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_header (u : Nat) : (blk u).blockHeader = hdr (u + 1) := by cases u <;> rfl

theorem blk_parent (u : Nat) : (blk u).parentCert = certOf (parentOf u) := by cases u <;> rfl

theorem blk_evidence (u : Nat) : (blk (u + 1)).timeoutEvidence = some (T u) := rfl

theorem cert_view (u : Nat) : (certOf (blk u)).view = ⟨3 * u + 1⟩ := blk_view u

theorem cert_number (u : Nat) : (certOf (blk u)).data.blockNumber = ⟨u + 1⟩ := blk_number u

theorem cert_epoch (u : Nat) : (certOf (blk u)).data.epoch = ⟨u + 1⟩ := blk_epoch u

theorem cr_view (u : Nat) : (CR u).view = ⟨3 * u + 2⟩ := rfl

theorem cr_number (u : Nat) : (CR u).data.blockNumber = ⟨u + 1⟩ := blk_number u

theorem cr_epoch (u : Nat) : (CR u).data.epoch = ⟨u + 1⟩ := blk_epoch u

theorem parentOf_number (u : Nat) : (parentOf u).blockHeader.blockNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_number u

theorem blk_wellFormed (u : Nat) : ProposalWellFormed cfg (blk u) := by
  cases u with
  | zero => exact ⟨by decide, Or.inl ⟨rfl, rfl⟩, rfl, rfl⟩
  | succ u =>
    refine ⟨?_, Or.inr ⟨T u, rfl, rfl⟩, ?_, ?_⟩
    · show (certOf (blk u)).view.toNat < 3 * u + 4
      rw [cert_view]; show 3 * u + 1 < 3 * u + 4; omega
    · show (⟨u + 2⟩ : EpochNumber) = epochOf ⟨u + 2⟩ 1
      rw [epochOf_one_height (by omega)]
    · show (certOf (blk u)).data.blockNumber + 1 = ⟨u + 2⟩
      rw [cert_number]; rfl

theorem blk_enters (u : Nat) : EntersEpoch cfg (blk (u + 1)) := by
  show IsLastBlock ((blk (u + 1)).blockHeader.blockNumber - 1) 1
  rw [blk_number]
  exact last_block (n := u + 1) (by omega)

theorem blk_enters_zero : ¬ EntersEpoch cfg (blk 0) := fun h => h.1 rfl

theorem blk_last (u : Nat) : IsLastBlock (blk u).blockHeader.blockNumber cfg.epochHeight := by
  rw [blk_number]; exact last_block (by omega)

/-- What the environment hands node `k` in the steps of block `u + 1`. -/
def phase (k : PubKey) (u : Nat) : Nat → Input
  | 0 => .headerBuilt ⟨3 * u + 1⟩ (blockHash (parentOf u)) (hdr (u + 1))
  | 1 => if lag k u = 0 then .proposal a (blk u) (some ⟨⟨3 * u + 1⟩, (blk u).payloadCommit⟩) else .proposal a (blk u) none
  | 2 => .blockValidated ⟨3 * u + 1⟩ (blockHash (blk u))
  | 3 => .certificate1 (certOf (blk u))
  | 4 => if lag k u = 0 then .blockReconstructed ⟨3 * u + 1⟩ (blk u).payloadCommit
      else .blockValidated ⟨3 * u + 1⟩ (blockHash (blk u))
  | 5 => .revote a (R u)
  | 6 => .certificate1 (CR u)
  | 7 => .certificate2 (C2 u)
  | 8 => .epochChange (certOf (blk u)) (C2 u) (blk u)
  | 9 => .revote a (R2 u)
  | 10 => .certificate2 (C2R u)
  | 11 => .epochChange (certOf (blk u)) (C2R u) (blk u)
  | 12 => .timeout ⟨3 * u + 3⟩
  | 13 => .timeoutCertificate (T u)
  | _ => .timeoutOneHonest ⟨3 * u + 3⟩

/-- Fifteen steps to a block. -/
def input (k : PubKey) (n : Nat) : Input := phase k (n / 15) (n % 15)

/-- Node `k` running the machine on its schedule, and its history after `n` steps. -/
local notation "tr" => Kit.tr cfg leader input

local notation "H" => Kit.H cfg leader input

theorem input_at (k : PubKey) (u r : Nat) (hr : r < 15) : input k (15 * u + r) = phase k u r := by
  simp only [input]
  rw [show (15 * u + r) / 15 = u by omega, show (15 * u + r) % 15 = r by omega]

theorem steps (n : Nat) : ∃ u r, r < 15 ∧ n = 15 * u + r :=
  ⟨n / 15, n % 15, Nat.mod_lt _ (by omega), by omega⟩

theorem r15 (r : Nat) (hr : r < 15) :
    r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 ∨ r = 5 ∨ r = 6 ∨ r = 7 ∨ r = 8 ∨ r = 9 ∨ r = 10
      ∨ r = 11 ∨ r = 12 ∨ r = 13 ∨ r = 14 := by omega

/-! ## What the nodes receive -/

section Inputs

variable {k : PubKey}

theorem input_phase {n : Nat} {i : Input} (hi : input k n = i) :
    ∃ u r, r < 15 ∧ n = 15 * u + r ∧ phase k u r = i := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  exact ⟨u, r, hr, rfl, by rw [← input_at k u r hr]; exact hi⟩

theorem input_header {n : Nat} {v : ViewNumber} {h : BlockHash} {x : BlockHeader}
    (hi : input k n = .headerBuilt v h x) :
    ∃ u, n = 15 * u ∧ v = ⟨3 * u + 1⟩ ∧ h = blockHash (parentOf u) ∧ x = hdr (u + 1) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal {n : Nat} {s : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal s p (some vid)) :
    ∃ u, n = 15 * u + 1 ∧ lag k u = 0 ∧ s = a ∧ p = blk u ∧ vid = ⟨⟨3 * u + 1⟩, (blk u).payloadCommit⟩ := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  exact ⟨u, rfl, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal_none {n : Nat} {s : PubKey} {p : Proposal} (hi : input k n = .proposal s p none) : ∃ u, n = 15 * u + 1 ∧ s = a ∧ p = blk u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  exact ⟨u, rfl, he.1.symm, he.2.symm⟩

/-- Every proposal a node receives is `a`'s block for its round, with a share or without. -/
theorem input_proposal_any {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ u, n = 15 * u + 1 ∧ s = a ∧ p = blk u := by
  cases share with
  | some vid => obtain ⟨u, h1, -, h2, h3, -⟩ := input_proposal hi; exact ⟨u, h1, h2, h3⟩
  | none => exact input_proposal_none hi

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    ∃ u, (n = 15 * u + 3 ∧ x = certOf (blk u)) ∨ (n = 15 * u + 6 ∧ x = CR u) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals first
    | exact ⟨u, Or.inl ⟨rfl, he.symm⟩⟩
    | exact ⟨u, Or.inr ⟨rfl, he.symm⟩⟩

theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = 15 * u + 4 ∧ lag k u = 0 ∧ v = ⟨3 * u + 1⟩ ∧ pc = (blk u).payloadCommit := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  exact ⟨u, rfl, hl, he.1.symm, he.2.symm⟩

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, (n = 15 * u + 7 ∧ x = C2 u) ∨ (n = 15 * u + 10 ∧ x = C2R u) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals first
    | exact ⟨u, Or.inl ⟨rfl, he.symm⟩⟩
    | exact ⟨u, Or.inr ⟨rfl, he.symm⟩⟩

theorem input_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : input k n = .epochChange c1 c2 p) :
    ∃ u, c1 = certOf (blk u) ∧ p = blk u ∧ ((n = 15 * u + 8 ∧ c2 = C2 u) ∨ (n = 15 * u + 11 ∧ c2 = C2R u)) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals first
    | exact ⟨u, he.1.symm, he.2.2.symm, Or.inl ⟨rfl, he.2.1.symm⟩⟩
    | exact ⟨u, he.1.symm, he.2.2.symm, Or.inr ⟨rfl, he.2.1.symm⟩⟩

theorem input_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hi : input k n = .revote s r) :
    ∃ u, (n = 15 * u + 5 ∧ s = a ∧ r = R u) ∨ (n = 15 * u + 9 ∧ s = a ∧ r = R2 u) := by
  obtain ⟨u, r', hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals first
    | exact ⟨u, Or.inl ⟨rfl, he.1.symm, he.2.symm⟩⟩
    | exact ⟨u, Or.inr ⟨rfl, he.1.symm, he.2.symm⟩⟩

theorem input_timeout {n : Nat} {v : ViewNumber} (hi : input k n = .timeout v) :
    ∃ u, n = 15 * u + 12 ∧ v = ⟨3 * u + 3⟩ := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.symm⟩

theorem input_tc {n : Nat} {tc : TimeoutCert} (hi : input k n = .timeoutCertificate tc) :
    ∃ u, n = 15 * u + 13 ∧ tc = T u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.symm⟩

theorem input_oneHonest {n : Nat} {v : ViewNumber} (hi : input k n = .timeoutOneHonest v) :
    ∃ u, n = 15 * u + 14 ∧ v = ⟨3 * u + 3⟩ := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r15 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      | rfl <;>
    simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.symm⟩

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

theorem recv_at {n : Nat} (u r : Nat) (hr : r < 15) (h : 15 * u + r < n) :
    (H k n).Received (phase k u r) :=
  received.mpr ⟨15 * u + r, h, input_at k u r hr⟩

theorem hasProposal {n : Nat} {x : Block} (hb : (H k n).HasProposal cfg x) :
    x = anchorB ∨ ∃ u, x = blk u ∧ 15 * u + 1 < n := by
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
    obtain ⟨u, -, rfl, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := input_epochChange hji
    · exact Or.inr ⟨u, rfl, by omega⟩
    · exact Or.inr ⟨u, rfl, by omega⟩

theorem hasProposal_of {n : Nat} (u : Nat) (h : 15 * u + 1 < n) : (H k n).HasProposal cfg (blk u) := by
  have hr := recv_at (k := k) u 1 (by omega) h
  simp only [phase] at hr
  rcases lag_cases k u with hl | hl
  · rw [hl, ite_eq_left rfl] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)
  · rw [hl, ite_eq_right (by decide)] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)

theorem hasParent_of {n : Nat} (u : Nat) (h : 15 * u < n + 13) : (H k n).HasProposal cfg (parentOf u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u => exact hasProposal_of u (by omega)

/-- Which held proposal a height names. -/
theorem block_of_number {n x : Nat} {y : Block} (hb : (H k n).HasProposal cfg y)
    (hv : y.blockHeader.blockNumber = ⟨x + 1⟩) : y = blk x ∧ 15 * x + 1 < n := by
  rcases hasProposal hb with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [blk_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

/-- The block a held proposal's view names: there is none at a re-vote's view. -/
theorem block_of_view {n x : Nat} {y : Block} (hb : (H k n).HasProposal cfg y) :
    (y.viewNumber = ⟨3 * x + 1⟩ → y = blk x) ∧ y.viewNumber ≠ ⟨3 * x + 2⟩ := by
  rcases hasProposal hb with rfl | ⟨u, rfl, -⟩
  · exact ⟨fun h => absurd (view_inj h) (by omega), fun h => absurd (view_inj h) (by omega)⟩
  · rw [blk_view]
    refine ⟨fun h => ?_, fun h => absurd (view_inj h) (by omega)⟩
    obtain rfl : u = x := by have := view_inj h; omega
    rfl

theorem hasCert1 {n : Nat} {x : Cert1} (hc : (H k n).HasCert1 cfg x) :
    x = certOf anchorB ∨ (∃ u, x = certOf (blk u) ∧ 15 * u + 3 < n) ∨ ∃ u, x = CR u ∧ 15 * u + 6 < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩⟩ := input_cert1 hji
    · exact Or.inr (Or.inl ⟨u, rfl, hj⟩)
    · exact Or.inr (Or.inr ⟨u, rfl, hj⟩)
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, -, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := input_epochChange hji
    · exact Or.inr (Or.inl ⟨u, rfl, by omega⟩)
    · exact Or.inr (Or.inl ⟨u, rfl, by omega⟩)

theorem hasCert1_of {n : Nat} (u : Nat) (h : 15 * u + 3 < n) : (H k n).HasCert1 cfg (certOf (blk u)) :=
  Or.inr (Or.inl (recv_at u 3 (by omega) h))

theorem hasCR_of {n : Nat} (u : Nat) (h : 15 * u + 6 < n) : (H k n).HasCert1 cfg (CR u) :=
  Or.inr (Or.inl (recv_at u 6 (by omega) h))

theorem hasCert2 {n : Nat} {x : Cert2} (hc : (H k n).HasCert2 x) :
    (∃ u, x = C2 u ∧ 15 * u + 7 < n) ∨ ∃ u, x = C2R u ∧ 15 * u + 10 < n := by
  rcases hc with hr | ⟨c1, p, hr⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩⟩ := input_cert2 hji
    · exact Or.inl ⟨u, rfl, hj⟩
    · exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, -, -, ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩⟩ := input_epochChange hji
    · exact Or.inl ⟨u, rfl, by omega⟩
    · exact Or.inr ⟨u, rfl, by omega⟩

theorem hasC2_of {n : Nat} (u : Nat) (h : 15 * u + 7 < n) : (H k n).HasCert2 (C2 u) :=
  Or.inl (recv_at u 7 (by omega) h)

theorem hasC2R_of {n : Nat} (u : Nat) (h : 15 * u + 10 < n) : (H k n).HasCert2 (C2R u) :=
  Or.inl (recv_at u 10 (by omega) h)

theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis
      ∨ ∃ u, v = ⟨3 * u + 1⟩ ∧ pc = (blk u).payloadCommit ∧ lag k u = 0 ∧ 15 * u + 4 < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, hl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hl, hj⟩

theorem payload_of {n : Nat} (u : Nat) (hl : lag k u = 0) (h : 15 * u + 4 < n) :
    (H k n).HasPayload cfg (blk u).viewNumber (blk u).payloadCommit := by
  have hr := recv_at (k := k) u 4 (by omega) h
  simp only [phase, hl, ite_eq_left] at hr
  rw [blk_view]; exact Or.inr hr

theorem received_tc {n : Nat} {tc : TimeoutCert} (hr : (H k n).Received (.timeoutCertificate tc)) :
    ∃ u, tc = T u ∧ 15 * u + 13 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨u, rfl, rfl⟩ := input_tc hji
  exact ⟨u, rfl, hj⟩

theorem received_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hr : (H k n).Received (.revote s r)) :
    ∃ u, s = a ∧ ((r = R u ∧ 15 * u + 5 < n) ∨ (r = R2 u ∧ 15 * u + 9 < n)) := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨u, ⟨rfl, rfl, rfl⟩ | ⟨rfl, rfl, rfl⟩⟩ := input_revote hji
  · exact ⟨u, rfl, Or.inl ⟨rfl, hj⟩⟩
  · exact ⟨u, rfl, Or.inr ⟨rfl, hj⟩⟩

theorem epochChange_wellFormed (u : Nat) (c2 : Cert2) (hd : c2.data = (certOf (blk u)).data.toVote2)
    (hv : (blk u).viewNumber ≤ c2.view) : EpochChangeWellFormed cfg (certOf (blk u)) c2 (blk u) :=
  ⟨hv, hd.symm, rfl, rfl, blk_wellFormed u, blk_last u⟩

theorem tookEpochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (ht : (H k n).TookEpochChange cfg c1 c2 p) :
    ∃ u, c1 = certOf (blk u) ∧ p = blk u ∧ ((c2 = C2 u ∧ 15 * u + 8 < n) ∨ (c2 = C2R u ∧ 15 * u + 11 < n)) := by
  obtain ⟨j, hj, hji⟩ := received.mp ht.1
  obtain ⟨u, rfl, rfl, ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩⟩ := input_epochChange hji
  · exact ⟨u, rfl, rfl, Or.inl ⟨rfl, hj⟩⟩
  · exact ⟨u, rfl, rfl, Or.inr ⟨rfl, hj⟩⟩

theorem tookEpochChange_of {n : Nat} (u : Nat) (h : 15 * u + 8 < n) :
    (H k n).TookEpochChange cfg (certOf (blk u)) (C2 u) (blk u) :=
  ⟨recv_at u 8 (by omega) h, epochChange_wellFormed u _ rfl (by rw [blk_view]; exact Nat.le_refl _)⟩

theorem tookEpochChangeR_of {n : Nat} (u : Nat) (h : 15 * u + 11 < n) :
    (H k n).TookEpochChange cfg (certOf (blk u)) (C2R u) (blk u) :=
  ⟨recv_at u 11 (by omega) h, epochChange_wellFormed u _ rfl
    (by rw [blk_view]; show 3 * u + 1 ≤ 3 * u + 2; omega)⟩

/--
What a node can lock on after `n` steps: genesis; each block once it has the
payload, as a member of its committee, or once it took the epoch change; and the
re-vote's `Cert1`, as a member, once it arrives.
-/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x ↔ x = certOf anchorB
      ∨ (∃ u, x = certOf (blk u) ∧ ((lag k u = 0 ∧ 15 * u + 4 < n) ∨ 15 * u + 8 < n))
      ∨ ∃ u, x = CR u ∧ lag k u = 0 ∧ 15 * u + 6 < n := by
  constructor
  · rintro (rfl | ⟨hc, y, hy, ⟨-, hcd⟩, hp⟩ | ⟨c2, p, ht⟩)
    · exact Or.inl rfl
    · have hyn := congrArg Vote1Data.blockNumber hcd
      simp only at hyn
      -- The block is `blk u`, whose payload the node holds as a member.
      have hblk : ∀ u, x.data.blockNumber = ⟨u + 1⟩ → lag k u = 0 ∧ 15 * u + 4 < n := fun u hu => by
        obtain ⟨rfl, -⟩ := block_of_number hy (hyn ▸ hu)
        rcases hasPayload hp with hg | ⟨u', hv, -, hl, hlt⟩
        · rw [blk_view] at hg; exact absurd (view_inj hg) (by omega)
        · rw [blk_view] at hv
          obtain rfl : u = u' := by have := view_inj hv; omega
          exact ⟨hl, hlt⟩
      rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩ | ⟨u, rfl, hlt⟩
      · exact Or.inl rfl
      · exact Or.inr (Or.inl ⟨u, rfl, Or.inl (hblk u (cert_number u))⟩)
      · exact Or.inr (Or.inr ⟨u, rfl, (hblk u (cr_number u)).1, hlt⟩)
    · obtain ⟨u, rfl, -, ⟨-, hlt⟩ | ⟨-, hlt⟩⟩ := tookEpochChange ht
      · exact Or.inr (Or.inl ⟨u, rfl, Or.inr hlt⟩)
      · exact Or.inr (Or.inl ⟨u, rfl, Or.inr (by omega)⟩)
  · rintro (rfl | ⟨u, rfl, ⟨hl, hlt⟩ | hlt⟩ | ⟨u, rfl, hl, hlt⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hasCert1_of u (by omega), blk u, hasProposal_of u (by omega),
        ⟨Nat.le_refl _, rfl⟩, payload_of u hl hlt⟩)
    · exact Or.inr (Or.inr ⟨_, _, tookEpochChange_of u hlt⟩)
    · exact Or.inr (Or.inl ⟨hasCR_of u hlt, blk u, hasProposal_of u (by omega),
        ⟨by rw [blk_view]; show 3 * u + 1 ≤ 3 * u + 2; omega, rfl⟩, payload_of u hl (by omega)⟩)

/-! ### The view, the epoch and the lock after `n` steps -/

/--
The view every node is in after `n` steps: it moves on with the block's `Cert1`,
with the re-vote's, and with the timeout certificate.
-/
def vAt (n : Nat) : Nat :=
  if 14 ≤ n % 15 then 3 * (n / 15) + 4
  else if 7 ≤ n % 15 then 3 * (n / 15) + 3
  else if 4 ≤ n % 15 then 3 * (n / 15) + 2 else 3 * (n / 15) + 1

/-- `vAt`, as facts `omega` reads. -/
theorem vAt_facts (u r : Nat) (hr : r < 15) : (14 ≤ r → vAt (15 * u + r) = 3 * u + 4)
    ∧ (7 ≤ r → r < 14 → vAt (15 * u + r) = 3 * u + 3)
    ∧ (4 ≤ r → r < 7 → vAt (15 * u + r) = 3 * u + 2) ∧ (r < 4 → vAt (15 * u + r) = 3 * u + 1) := by
  simp only [vAt]
  rw [show (15 * u + r) / 15 = u by omega, show (15 * u + r) % 15 = r by omega]
  refine ⟨fun h => ite_eq_left h, fun h1 h2 => ?_, fun h1 h2 => ?_, fun h => ?_⟩
  · rw [ite_eq_right (by omega), ite_eq_left h1]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega), ite_eq_left h1]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega), ite_eq_right (by omega)]

/-- The epoch every node is in after `n` steps: the next one from the epoch change on. -/
def eAt (n : Nat) : Nat := if 9 ≤ n % 15 then n / 15 + 2 else n / 15 + 1

theorem eAt_facts (u r : Nat) (hr : r < 15) :
    (9 ≤ r → eAt (15 * u + r) = u + 2) ∧ (r < 9 → eAt (15 * u + r) = u + 1) := by
  simp only [eAt]
  rw [show (15 * u + r) / 15 = u by omega, show (15 * u + r) % 15 = r by omega]
  exact ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

theorem eAt_ge {n u : Nat} (h : 15 * u + 9 ≤ n) : u + 2 ≤ eAt n := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := eAt_facts u' r hr
  omega

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ vAt n := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := vAt_facts u' r hr
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, ht, rfl⟩
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩ | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ _; omega
    · show (certOf (blk u)).view.toNat + 1 ≤ _
      rw [cert_view]; show 3 * u + 1 + 1 ≤ _; omega
    · show 3 * u + 2 + 1 ≤ _; omega
  · obtain ⟨u, rfl, hlt⟩ := received_tc htc
    show 3 * u + 3 + 1 ≤ _; omega
  · obtain ⟨u, -, -, ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩⟩ := tookEpochChange ht
    · show 3 * u + 1 + 1 ≤ _; omega
    · show 3 * u + 2 + 1 ≤ _; omega

theorem inView (n : Nat) : (H k n).InView cfg ⟨vAt n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  obtain ⟨u, r, hr, rfl⟩ := steps n
  obtain ⟨h14, h7, h4, h0⟩ := vAt_facts u r hr
  by_cases r14 : 14 ≤ r
  · rw [h14 r14]; exact Or.inr (Or.inl ⟨T u, recv_at u 13 (by omega) (by omega), rfl⟩)
  by_cases r7 : 7 ≤ r
  · rw [h7 r7 (by omega)]; exact Or.inl ⟨CR u, hasCR_of u (by omega), rfl⟩
  by_cases r4 : 4 ≤ r
  · rw [h4 r4 (by omega)]
    exact Or.inl ⟨certOf (blk u), hasCert1_of u (by omega), by rw [cert_view]; rfl⟩
  rw [h0 (by omega)]
  cases u with
  | zero => exact Or.inl ⟨certOf anchorB, Or.inl rfl, rfl⟩
  | succ u => exact Or.inr (Or.inl ⟨T u, recv_at u 13 (by omega) (by omega), rfl⟩)

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨vAt n⟩ :=
  Kit.inView_unique hv (inView n)

theorem viewOf_eq (n : Nat) : viewOf cfg (H k n) = ⟨vAt n⟩ := viewOf_of_inView (inView n)

theorem epochGround {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e.toNat ≤ eAt n := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := eAt_facts u' r hr
  rcases he with rfl | ⟨c1, c2, p, ht, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨x, hx, -, rfl⟩
  · show 1 ≤ _; omega
  · obtain ⟨u, -, -, ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩⟩ := tookEpochChange ht
    · show (certOf (blk u)).data.epoch.toNat + 1 ≤ _
      rw [cert_epoch]; show u + 1 + 1 ≤ _; omega
    · show (certOf (blk u)).data.epoch.toNat + 1 ≤ _
      rw [cert_epoch]; show u + 1 + 1 ≤ _; omega
  · obtain ⟨u, rfl, hlt⟩ := received_tc htc
    show u + 2 ≤ _; omega
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩ | ⟨u, rfl, hlt⟩
    · show 1 ≤ _; omega
    · rw [cert_epoch]; show u + 1 ≤ _; omega
    · rw [cr_epoch]; show u + 1 ≤ _; omega

theorem c2_epoch (u : Nat) : (⟨u + 2⟩ : EpochNumber) = (C2 u).data.epoch + 1 := by
  show _ = (certOf (blk u)).data.epoch + 1
  rw [cert_epoch]; rfl

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨eAt n⟩ := by
  refine ⟨?_, fun e he => epochGround he⟩
  obtain ⟨u, r, hr, rfl⟩ := steps n
  obtain ⟨h9, h0⟩ := eAt_facts u r hr
  by_cases r9 : 9 ≤ r
  · rw [h9 r9]; exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u (by omega), c2_epoch u⟩)
  rw [h0 (by omega)]
  cases u with
  | zero => exact Or.inl rfl
  | succ u => exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u (by omega), c2_epoch u⟩)

theorem inEpoch_eq {n : Nat} {e : EpochNumber} (he : (H k n).InEpoch cfg e) : e = ⟨eAt n⟩ :=
  Liveness.inEpoch_unique he (inEpoch n)

theorem epochOf_eq (n : Nat) : epochOfHistory cfg (H k n) = ⟨eAt n⟩ :=
  inEpoch_eq (inEpoch_epochOfHistory cfg _)

theorem notBehind {n : Nat} {e : EpochNumber} (h : eAt n ≤ e.toNat) : NotBehind cfg (H k n) e :=
  Kit.notBehind_of (inEpoch n) h

theorem notBehind_le {n : Nat} {e : EpochNumber} (h : NotBehind cfg (H k n) e) : eAt n ≤ e.toNat :=
  Kit.le_of_notBehind (inEpoch n) h

/-- The certificates a node can hold: genesis, each block's, and each re-vote's. -/
def Known (x : Cert1) : Prop := x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∨ x = CR u

/-- View and epoch of a certificate a node can hold, as facts `omega` reads. -/
theorem known_facts {x : Cert1} (hx : Known x) :
    (x.view.toNat = 0 ∧ x.data.epoch.toNat = 1)
      ∨ (∃ u, x.view.toNat = 3 * u + 1 ∧ x.data.epoch.toNat = u + 1)
      ∨ ∃ u, x.view.toNat = 3 * u + 2 ∧ x.data.epoch.toNat = u + 1 := by
  rcases hx with rfl | ⟨u, rfl | rfl⟩
  · exact Or.inl ⟨rfl, rfl⟩
  · exact Or.inr (Or.inl ⟨u, by rw [cert_view], by rw [cert_epoch]⟩)
  · exact Or.inr (Or.inr ⟨u, rfl, by rw [cr_epoch]⟩)

/-- On the certificates a node can hold, lock order is the order of views. -/
theorem lockLE_of {x y : Cert1} (hx : Known x) (hy : Known y) (h : x.view.toNat ≤ y.view.toNat) :
    LockLE x y := by
  rcases known_facts hx with fx | ⟨_, fx⟩ | ⟨_, fx⟩ <;> rcases known_facts hy with fy | ⟨_, fy⟩ | ⟨_, fy⟩ <;>
  by_cases he : x.data.epoch.toNat < y.data.epoch.toNat
  all_goals first
    | exact Or.inl he
    | exact Or.inr ⟨epoch_ext (by omega), h⟩

theorem le_of_lockLE {x y : Cert1} (hx : Known x) (hy : Known y) (h : LockLE x y) :
    x.view.toNat ≤ y.view.toNat := by
  have hv : x.data.epoch.toNat < y.data.epoch.toNat
      ∨ (x.data.epoch.toNat = y.data.epoch.toNat ∧ x.view.toNat ≤ y.view.toNat) := by
    rcases h with h | ⟨he, hv⟩
    · exact Or.inl h
    · exact Or.inr ⟨congrArg EpochNumber.toNat he, hv⟩
  rcases known_facts hx with fx | ⟨_, fx⟩ | ⟨_, fx⟩ <;> rcases known_facts hy with fy | ⟨_, fy⟩ | ⟨_, fy⟩ <;>
    omega

/-- Two certificates a node can hold at the same view are the same. -/
theorem known_ext {x y : Cert1} (hx : Known x) (hy : Known y) (h : x.view.toNat = y.view.toNat) : x = y := by
  rcases hx with rfl | ⟨j, rfl | rfl⟩ <;> rcases hy with rfl | ⟨m, rfl | rfl⟩ <;>
    simp only [cert_view, cr_view] at h <;>
    first
      | rfl
      | (have : (certOf anchorB).view.toNat = 0 := rfl; omega)
      | (obtain rfl : j = m := by omega
         rfl)
      | omega

theorem lockable_known {n : Nat} {x : Cert1} (hx : (H k n).Lockable cfg x) : Known x := by
  rcases lockable_iff.mp hx with rfl | ⟨u, rfl, -⟩ | ⟨u, rfl, -⟩
  · exact Or.inl rfl
  · exact Or.inr ⟨u, Or.inl rfl⟩
  · exact Or.inr ⟨u, Or.inr rfl⟩

/-- The view of a certificate a node can lock on, and when it can. -/
theorem lockable_view {n : Nat} {x : Cert1} (hx : (H k n).Lockable cfg x) :
    x.view.toNat = 0 ∨ (∃ j, x.view.toNat = 3 * j + 1 ∧ ((lag k j = 0 ∧ 15 * j + 4 < n) ∨ 15 * j + 8 < n))
      ∨ ∃ j, x.view.toNat = 3 * j + 2 ∧ lag k j = 0 ∧ 15 * j + 6 < n := by
  rcases lockable_iff.mp hx with rfl | ⟨j, rfl, h⟩ | ⟨j, rfl, h⟩
  · exact Or.inl rfl
  · exact Or.inr (Or.inl ⟨j, by rw [cert_view], h⟩)
  · exact Or.inr (Or.inr ⟨j, rfl, h⟩)

/-- When a node can lock on a block's own `Cert1`. -/
theorem lockable_blk {n x : Nat} (hl : (H k n).Lockable cfg (certOf (blk x))) :
    (lag k x = 0 ∧ 15 * x + 4 < n) ∨ 15 * x + 8 < n := by
  have hv : (certOf (blk x)).view.toNat = 3 * x + 1 := by rw [cert_view]
  rcases lockable_view hl with h | ⟨j, hj, hjc⟩ | ⟨j, hj, -⟩
  · omega
  · obtain rfl : j = x := by omega
    exact hjc
  · omega

/-- The lock a node carries into epoch `u + 1`: the last one of the epoch before. -/
def prevLock (k : PubKey) : Nat → Cert1
  | 0 => certOf anchorB
  | u + 1 => if lag k u = 0 then CR u else certOf (blk u)

/--
The lock after `n` steps: a member locks on the block the step after its payload,
and on the re-vote's `Cert1` the step after it arrives; a node outside the
committee on the block with the epoch change.
-/
def lockAt (k : PubKey) (n : Nat) : Cert1 :=
  if lag k (n / 15) = 0 ∧ 7 ≤ n % 15 then CR (n / 15)
  else if (lag k (n / 15) = 0 ∧ 5 ≤ n % 15) ∨ 9 ≤ n % 15 then certOf (blk (n / 15))
  else prevLock k (n / 15)

theorem prevLock_succ (u : Nat) : prevLock k (u + 1) = if lag k u = 0 then CR u else certOf (blk u) := rfl

theorem prevLock_known (u : Nat) : Known (prevLock k u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u =>
    rw [prevLock_succ]
    by_cases hl : lag k u = 0
    · rw [ite_eq_left hl]; exact Or.inr ⟨u, Or.inr rfl⟩
    · rw [ite_eq_right hl]; exact Or.inr ⟨u, Or.inl rfl⟩

theorem prevLock_view (u : Nat) : (lag k u = 0 → (prevLock k (u + 1)).view.toNat = 3 * u + 2)
    ∧ (lag k u ≠ 0 → (prevLock k (u + 1)).view.toNat = 3 * u + 1) := by
  rw [prevLock_succ]
  exact ⟨fun hl => by rw [ite_eq_left hl]; rfl, fun hl => by rw [ite_eq_right hl, cert_view]⟩

theorem lockAt_cases (u r : Nat) (hr : r < 15) :
    (lag k u = 0 ∧ 7 ≤ r ∧ lockAt k (15 * u + r) = CR u)
      ∨ (¬ (lag k u = 0 ∧ 7 ≤ r) ∧ ((lag k u = 0 ∧ 5 ≤ r) ∨ 9 ≤ r)
        ∧ lockAt k (15 * u + r) = certOf (blk u))
      ∨ (¬ (lag k u = 0 ∧ 7 ≤ r) ∧ ¬ ((lag k u = 0 ∧ 5 ≤ r) ∨ 9 ≤ r)
        ∧ lockAt k (15 * u + r) = prevLock k u) := by
  unfold lockAt
  rw [show (15 * u + r) / 15 = u by omega, show (15 * u + r) % 15 = r by omega]
  by_cases h1 : lag k u = 0 ∧ 7 ≤ r
  · exact Or.inl ⟨h1.1, h1.2, ite_eq_left h1⟩
  · rw [ite_eq_right h1]
    by_cases h2 : (lag k u = 0 ∧ 5 ≤ r) ∨ 9 ≤ r
    · exact Or.inr (Or.inl ⟨h1, h2, ite_eq_left h2⟩)
    · exact Or.inr (Or.inr ⟨h1, h2, ite_eq_right h2⟩)

theorem lockAt_known (n : Nat) : Known (lockAt k n) := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  rcases lockAt_cases (k := k) u r hr with ⟨-, -, h⟩ | ⟨-, -, h⟩ | ⟨-, -, h⟩ <;> rw [h]
  · exact Or.inr ⟨u, Or.inr rfl⟩
  · exact Or.inr ⟨u, Or.inl rfl⟩
  · exact prevLock_known u

theorem lockedOn_lockAt (n : Nat) : (H k n).LockedOn cfg (lockAt k n) := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  have hlu := lag_cases k u
  refine ⟨?_, fun x hx => lockLE_of (lockable_known hx) (lockAt_known _) ?_⟩
  · rcases lockAt_cases (k := k) u r hr with ⟨hl, h7, h⟩ | ⟨-, h2, h⟩ | ⟨h1, h2, h⟩ <;> rw [h]
    · exact lockable_iff.mpr (Or.inr (Or.inr ⟨u, rfl, hl, by omega⟩))
    · rcases h2 with ⟨hl, h5⟩ | h9
      · exact lockable_iff.mpr (Or.inr (Or.inl ⟨u, rfl, Or.inl ⟨hl, by omega⟩⟩))
      · exact lockable_iff.mpr (Or.inr (Or.inl ⟨u, rfl, Or.inr (by omega)⟩))
    · cases u with
      | zero => exact lockable_iff.mpr (Or.inl rfl)
      | succ u =>
        rw [prevLock_succ]
        by_cases hl : lag k u = 0
        · rw [ite_eq_left hl]; exact lockable_iff.mpr (Or.inr (Or.inr ⟨u, rfl, hl, by omega⟩))
        · rw [ite_eq_right hl]; exact lockable_iff.mpr (Or.inr (Or.inl ⟨u, rfl, Or.inr (by omega)⟩))
  · -- Any lockable certificate is at a view no later than the lock's.
    have hA : (certOf (blk u)).view.toNat = 3 * u + 1 := by rw [cert_view]
    have hv := lockable_view hx
    rcases lockAt_cases (k := k) u r hr with ⟨-, -, h⟩ | ⟨h1, -, h⟩ | ⟨h1, h2, h⟩ <;> rw [h]
    · show _ ≤ 3 * u + 2
      rcases hv with h0 | ⟨j, hj, hjc⟩ | ⟨j, hj, -, hjt⟩
      · omega
      · rcases hjc with ⟨-, h'⟩ | h' <;> omega
      · omega
    · rw [hA]
      rcases hv with h0 | ⟨j, hj, hjc⟩ | ⟨j, hj, hjl, hjt⟩
      · omega
      · rcases hjc with ⟨-, h'⟩ | h' <;> omega
      · by_cases hju : j = u
        · subst hju; exact absurd ⟨hjl, by omega⟩ h1
        · omega
    · cases u with
      | zero =>
        show _ ≤ 0
        rcases hv with h0 | ⟨j, hj, hjc⟩ | ⟨j, hj, hjl, hjt⟩
        · omega
        · obtain rfl : j = 0 := by rcases hjc with ⟨-, h'⟩ | h' <;> omega
          refine absurd ?_ h2
          rcases hjc with ⟨hl, h'⟩ | h'
          · exact Or.inl ⟨hl, by omega⟩
          · exact Or.inr (by omega)
        · obtain rfl : j = 0 := by omega
          exact absurd ⟨hjl, by omega⟩ h1
      | succ u =>
        obtain ⟨hp0, hp2⟩ := prevLock_view (k := k) u
        rcases hv with h0 | ⟨j, hj, hjc⟩ | ⟨j, hj, hjl, hjt⟩
        · omega
        · by_cases hju : j = u + 1
          · subst hju
            refine absurd ?_ h2
            rcases hjc with ⟨hl, h'⟩ | h'
            · exact Or.inl ⟨hl, by omega⟩
            · exact Or.inr (by omega)
          · have : j ≤ u := by rcases hjc with ⟨-, h'⟩ | h' <;> omega
            by_cases hl : lag k u = 0
            · rw [hp0 hl]; omega
            · rw [hp2 hl]; omega
        · by_cases hju : j = u + 1
          · subst hju; exact absurd ⟨hjl, by omega⟩ h1
          · have : j ≤ u := by omega
            by_cases hl : lag k u = 0
            · rw [hp0 hl]; omega
            · by_cases hjj : j = u
              · subst hjj; exact absurd hjl hl
              · rw [hp2 hl]; omega

theorem lockedOn_eq {n : Nat} {L : Cert1} (hL : (H k n).LockedOn cfg L) : L = lockAt k n := by
  have h0 := lockedOn_lockAt (k := k) n
  exact known_ext (lockable_known hL.1) (lockAt_known n) (Nat.le_antisymm
    (le_of_lockLE (lockable_known hL.1) (lockAt_known n) (h0.2 _ hL.1))
    (le_of_lockLE (lockAt_known n) (lockable_known hL.1) (hL.2 _ h0.1)))

theorem lockOf_eq (n : Nat) : lockOf cfg (H k n) = lockAt k n := lockedOn_eq (lockOf_lockedOn _)

/-- When it times out, a member is locked on the re-vote's `Cert1`, a node outside the committee on the block's. -/
theorem lockAt_timeout (u : Nat) :
    lockAt k (15 * u + 12) = if lag k u = 0 then CR u else certOf (blk u) := by
  by_cases hl : lag k u = 0
  · rw [ite_eq_left hl]
    rcases lockAt_cases (k := k) u 12 (by omega) with ⟨-, -, h⟩ | ⟨h1, -, -⟩ | ⟨h1, -, -⟩
    · exact h
    · exact absurd ⟨hl, by omega⟩ h1
    · exact absurd ⟨hl, by omega⟩ h1
  · rw [ite_eq_right hl]
    rcases lockAt_cases (k := k) u 12 (by omega) with ⟨h0, -, -⟩ | ⟨-, -, h⟩ | ⟨-, h2, -⟩
    · exact absurd h0 hl
    · exact h
    · exact absurd (Or.inr (by omega)) h2

/-- Either lock is no later than the re-vote's `Cert1`. -/
theorem lockAt_timeout_le (u : Nat) : LockLE (lockAt k (15 * u + 12)) (CR u) := by
  rw [lockAt_timeout]
  by_cases hl : lag k u = 0
  · rw [ite_eq_left hl]; exact Or.inr ⟨rfl, Nat.le_refl _⟩
  · rw [ite_eq_right hl]
    exact Or.inr ⟨rfl, by show (certOf (blk u)).view.toNat ≤ 3 * u + 2; rw [cert_view]; show 3 * u + 1 ≤ _; omega⟩

/-- `a` carries the re-vote's `Cert1` into the next epoch. -/
theorem lockAt_a (u r : Nat) (hr : r < 5) : lockAt a (15 * (u + 1) + r) = CR u := by
  rcases lockAt_cases (k := a) (u + 1) r (by omega) with ⟨-, h7, -⟩ | ⟨-, h2, -⟩ | ⟨-, -, h⟩
  · omega
  · rcases h2 with ⟨-, h⟩ | h <;> omega
  · rw [h, prevLock_succ, lag_a, ite_eq_left rfl]

end Holds

/-! ## What the honest nodes send -/

section Sends

theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H k (n + 1)) o :=
  Kit.settled input k n o

/-- A node's only timeout votes answer the timer of the view after each re-vote, naming its epoch and lock. -/
theorem sent_timeout {k : PubKey} {j : Nat} {vote : TimeoutVote}
    (hx : Output.send (.timeoutVote vote) ∈ (tr k j).output) :
    ∃ u, j = 15 * u + 12 ∧ vote = ⟨⟨⟨u + 2⟩, lockAt k (15 * u + 12)⟩, ⟨3 * u + 3⟩, k⟩ := by
  rw [tr_step] at hx
  rcases step_mem hx with hx | ⟨o, out', -, -, ha⟩
  · obtain ⟨v, hv, (⟨hi, hview⟩ | ⟨hi, hview⟩)⟩ := mem_timeoutAnswer hx
    · obtain ⟨u, rfl, rfl⟩ := input_timeout hi
      refine ⟨u, rfl, ?_⟩
      simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      rw [hv, epochOf_eq, lockOf_eq, (eAt_facts u 12 (by omega)).1 (by omega)]
    · obtain ⟨u, rfl, rfl⟩ := input_oneHonest hi
      rw [viewOf_eq, (vAt_facts u 14 (by omega)).1 (by omega)] at hview
      exact absurd (show 3 * u + 4 ≤ 3 * u + 3 from hview) (by omega)
  · exact absurd ha act_no_timeoutVote

/-- Every node times out the view after each re-vote, at the step its timer fires. -/
theorem times_out (k : PubKey) (u : Nat) :
    Output.send (.timeoutVote ⟨⟨⟨u + 2⟩, lockAt k (15 * u + 12)⟩, ⟨3 * u + 3⟩, k⟩)
      ∈ (tr k (15 * u + 12)).output := by
  rw [tr_step, step_output]
  apply discharge_sup
  have hi : input k (15 * u + 12) = .timeout ⟨3 * u + 3⟩ := by rw [input_at k u 12 (by omega)]; rfl
  rw [hi]
  simp only [timeoutAnswer, viewOf_eq, (vAt_facts u 12 (by omega)).2.1 (by omega) (by omega), epochOf_eq,
    (eAt_facts u 12 (by omega)).1 (by omega), lockOf_eq, ite_true, List.mem_singleton]

theorem timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h : (H k n).TimedOut v) :
    ∃ u, v.toNat ≤ 3 * u + 3 ∧ 15 * u + 12 < n := by
  obtain ⟨vote, hs, hle⟩ := h
  obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
  obtain ⟨u, rfl, rfl⟩ := sent_timeout hjv
  exact ⟨u, hle, hj⟩

variable (hv : ∀ b, BlockValid b)

include hv in
theorem protocol (k : PubKey) (n : Nat) : ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H k n) :=
  Kit.protocol input hv k n

/-- What a justified proposal or request names, the node holds. -/
theorem justified_held {k : PubKey} {n : Nat} {x : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg (H k n) x ev) :
    (H k n).HasCert1 cfg x ∧ ∀ tc, ev = some tc → ∃ y, tc = T y ∧ 15 * y + 13 < n := by
  refine ⟨hasCert1_of_certJustified hj, fun tc hte => ?_⟩
  subst hte
  obtain ⟨-, m, hr, -⟩ := hj
  rw [upTo_H] at hr
  obtain ⟨y, rfl, hy⟩ := received_tc hr
  exact ⟨y, rfl, by omega⟩

include hv in
/--
A node sends only `a`'s re-vote requests, two per block: one on the block, once it
can lock on it, and one on the re-vote's `Cert1`, once that arrives. It asks for
none on a certificate of an epoch it has left.
-/
theorem sent_revote {k : PubKey} {j : Nat} {r : RevoteRequest} (hr : Output.send (.revote r) ∈ (tr k j).output) :
    k = a ∧ ∃ x, (r = R x ∧ 15 * x + 4 ≤ j) ∨ (r = R2 x ∧ 15 * x + 6 ≤ j) := by
  have hj := (protocol hv k (j + 1)).revoteJustified j r ⟨_, (getElem_H j), hr⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := (Option.some.inj hj.leads).symm
  refine ⟨rfl, ?_⟩
  obtain ⟨hc, hev⟩ := justified_held hj.justified
  obtain ⟨hlt, hnext, hlast⟩ := hj.wellFormed
  obtain ⟨x, hx⟩ : ∃ x, r.cert = certOf (blk x) ∨ r.cert = CR x := by
    rcases hasCert1 hc with he | ⟨x, he, -⟩ | ⟨x, he, -⟩
    · rw [he] at hlast; exact absurd hlast.1 (by decide)
    · exact ⟨x, Or.inl he⟩
    · exact ⟨x, Or.inr he⟩
  have hfacts : 3 * x + 1 ≤ r.cert.view.toNat ∧ r.cert.data.epoch = ⟨x + 1⟩ := by
    rcases hx with hx | hx <;> rw [hx]
    · exact ⟨by rw [cert_view]; exact Nat.le_refl _, cert_epoch x⟩
    · exact ⟨by show 3 * x + 1 ≤ 3 * x + 2; omega, cr_epoch x⟩
  -- Timeout evidence would be of the epoch after the certificate's, at a view before the certificate's.
  have hnone : r.timeoutEvidence = none ∧ r.cert.view + 1 = r.view := by
    rcases hnext with ⟨hn0, hn⟩ | ⟨tc, hte, htv⟩
    · exact ⟨hn0, hn⟩
    · exfalso
      obtain ⟨y, rfl, -⟩ := hev tc hte
      obtain ⟨hep, -⟩ := hj.safe _ hte
      rw [hfacts.2] at hep
      have hy : y + 2 = x + 1 := congrArg EpochNumber.toNat hep
      have : 3 * y + 3 + 1 = r.view.toNat := congrArg ViewNumber.toNat htv
      have : r.cert.view.toNat < r.view.toNat := hlt
      omega
  have hl : (H a (j + 1)).Lockable cfg r.cert := hj.lockable hnone.1
  obtain ⟨c0, v0, e0⟩ := r
  simp only at hx hnone hl
  obtain ⟨rfl, hv0⟩ := hnone
  subst hv0
  refine ⟨x, ?_⟩
  rcases hx with rfl | rfl
  · refine Or.inl ⟨by rw [cert_view]; rfl, ?_⟩
    rcases lockable_blk hl with ⟨-, h⟩ | h <;> omega
  · refine Or.inr ⟨rfl, ?_⟩
    have hcv : (CR x).view.toNat = 3 * x + 2 := rfl
    rcases lockable_view hl with h | ⟨i, hi, -⟩ | ⟨i, hi, -, hit⟩
    · omega
    · omega
    · obtain rfl : i = x := by omega
      omega

include hv in
/-- A proposal is for the view of a header the node was handed, after it was handed it. -/
theorem proposal_header {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    ∃ u, 15 * u ≤ j ∧ p.viewNumber = ⟨3 * u + 1⟩ ∧ p.blockHeader = hdr (u + 1) := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨i, hi, hii⟩ := received.mp hj.built
  obtain ⟨u, rfl, hvw, -, hx⟩ := input_header hii
  exact ⟨u, by omega, hvw, hx⟩

theorem blk_safe (u : Nat) : SafeParent (blk u) := by
  cases u with
  | zero => exact fun _ h => by cases h
  | succ u =>
    intro tc h
    cases h
    refine ⟨rfl, Or.inl ?_⟩
    show (CR u).data.epoch < (blk (u + 1)).epoch
    rw [cr_epoch, blk_epoch]; show u + 1 < u + 1 + 1; omega

/-- `a` may propose block `u + 1` from the step its header arrives until its `Cert1` does. -/
theorem blk_justified (u r : Nat) (h1 : 1 ≤ r) (h3 : r ≤ 3) :
    ProposalJustified cfg leader a (H a (15 * u + r)) (blk u) := by
  have hvf := vAt_facts u r (by omega)
  have hef := eAt_facts u r (by omega)
  refine ⟨⟨rfl, blk_wellFormed u, ?_, ⟨parentOf u, hasParent_of u (by omega),
      by rw [blk_parent]; exact Nat.le_refl _, by rw [blk_parent]; rfl⟩, ?_, blk_safe u, notBehind ?_,
      ⟨_, (inView _).1, ?_⟩⟩, ?_⟩
  · unfold ParentJustified
    cases u with
    | zero => exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
    | succ u =>
      -- The lock behind the timeout certificate is the re-vote's `Cert1`, over the same block.
      show CertJustified cfg (H a (15 * (u + 1) + r)) (certOf (blk u)) (some (T u))
      refine ⟨hasCert1_of u (by omega), 15 * (u + 1) + r, ?_, Or.inl ⟨CR u, ?_, rfl⟩⟩ <;> rw [upTo_self]
      · exact recv_at u 13 (by omega) (by omega)
      · have := lockedOn_lockAt (k := a) (15 * (u + 1) + r)
        rwa [lockAt_a u r (by omega)] at this
  · intro hen
    cases u with
    | zero => exact absurd hen blk_enters_zero
    | succ u =>
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasC2_of u (by omega), ?_, rfl⟩
      rw [blk_view]; show 3 * u + 1 < 3 * (u + 1) + 1; omega
  · rw [blk_epoch]; show eAt (15 * u + r) ≤ u + 1; omega
  · rw [blk_view]; show 3 * u + 1 ≤ vAt (15 * u + r); omega
  · rw [blk_view, blk_header, blk_parent]
    exact recv_at u 0 (by omega) (by omega)

include hv in
/-- `a` asks for a re-vote on each block as soon as it can lock on it. -/
theorem revotes (u : Nat) : ∃ j, j < 15 * u + 5 ∧ Output.send (.revote (R u)) ∈ (tr a j).output := by
  have hvf := vAt_facts u 5 (by omega)
  have hef := eAt_facts u 5 (by omega)
  refine Classical.byContradiction fun hneg =>
    settled a (15 * u + 4) (.propose (certOf (blk u)).data.epoch ⟨3 * u + 2⟩) ⟨?_, ?_, ?_, ?_⟩
  · refine Or.inr ⟨R u, ⟨rfl, ⟨?_, Or.inl ⟨rfl, ?_⟩, blk_last u⟩, Liveness.buildable_of_lockable ?lk, fun _ => ?lk,
      (fun _ h => by cases h), notBehind ?_, ?_⟩,
      rfl, rfl⟩
    · show (certOf (blk u)).view < ⟨3 * u + 2⟩; rw [cert_view]; show 3 * u + 1 < 3 * u + 2; omega
    · show (certOf (blk u)).view + 1 = ⟨3 * u + 2⟩; rw [cert_view]; rfl
    · exact lockable_iff.mpr (Or.inr (Or.inl ⟨u, rfl, Or.inl ⟨lag_a u, by omega⟩⟩))
    · show eAt (15 * u + 5) ≤ (certOf (blk u)).data.epoch.toNat
      rw [cert_epoch]; show _ ≤ u + 1; omega
    · refine ⟨_, (inView (15 * u + 5)).1, ?_⟩
      show 3 * u + 2 ≤ vAt (15 * u + 5); omega
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, hrv, -⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨x, -, hx, -⟩ := proposal_header hv hjp
      rw [hpv] at hx; have := view_inj hx; omega
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_revote hv hjr
      · obtain rfl : x = u := by have := view_inj hrv; omega
        exact hneg ⟨j, hj, hjr⟩
      · have := view_inj hrv; omega
  · intro ht
    obtain ⟨y, hle, hy⟩ := timedOut ht
    have : 3 * u + 2 ≤ 3 * y + 3 := hle
    omega
  · have := inView (k := a) (15 * u + 5)
    rwa [show vAt (15 * u + 5) = 3 * u + 2 by omega] at this

include hv in
/-- `a` asks for a re-vote on each re-vote's `Cert1` in the step it arrives in. -/
theorem revotes2 (u : Nat) : ∃ j, j ≤ 15 * u + 6 ∧ Output.send (.revote (R2 u)) ∈ (tr a j).output := by
  have hvf := vAt_facts u 7 (by omega)
  have hef := eAt_facts u 7 (by omega)
  refine Classical.byContradiction fun hneg =>
    settled a (15 * u + 6) (.propose (CR u).data.epoch ⟨3 * u + 3⟩) ⟨?_, ?_, ?_, ?_⟩
  · refine Or.inr ⟨R2 u, ⟨rfl, ⟨?_, Or.inl ⟨rfl, rfl⟩, blk_last u⟩, Liveness.buildable_of_lockable ?lk, fun _ => ?lk,
      (fun _ h => by cases h), notBehind ?_, ?_⟩,
      rfl, rfl⟩
    · show 3 * u + 2 < 3 * u + 3; omega
    · exact lockable_iff.mpr (Or.inr (Or.inr ⟨u, rfl, lag_a u, by omega⟩))
    · show eAt (15 * u + 7) ≤ (CR u).data.epoch.toNat
      rw [cr_epoch]; show _ ≤ u + 1; omega
    · refine ⟨_, (inView (15 * u + 7)).1, ?_⟩
      show 3 * u + 3 ≤ vAt (15 * u + 7); omega
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, hrv, -⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨x, -, hx, -⟩ := proposal_header hv hjp
      rw [hpv] at hx; have := view_inj hx; omega
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_revote hv hjr
      · have := view_inj hrv; omega
      · obtain rfl : x = u := by have := view_inj hrv; omega
        exact hneg ⟨j, by omega, hjr⟩
  · intro ht
    obtain ⟨y, hle, hy⟩ := timedOut ht
    have : 3 * u + 3 ≤ 3 * y + 3 := hle
    omega
  · have := inView (k := a) (15 * u + 7)
    rwa [show vAt (15 * u + 7) = 3 * u + 3 by omega] at this

include hv in
/-- Every proposal a node sends is `a`'s block for its view, sent once the header arrived. -/
theorem sent_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) : k = a ∧ ∃ u, p = blk u ∧ 15 * u ≤ j := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨u, hju, hvw, hhdr⟩ := proposal_header hv hp
  refine ⟨u, ?_, hju⟩
  obtain ⟨-, hnext, hep, hnum⟩ := hj.wellFormed
  obtain ⟨hpc, hev⟩ := justified_held hj.justified
  have hid : p.identity = ⟨0⟩ := by
    rw [tr_step] at hp
    rcases step_mem hp with h0 | ⟨o, out', -, -, ha⟩
    · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨_, v, -, hmem, -, -⟩ := act_proposal ha
      exact (mem_proposalCandidates hmem).1
  have hnum' : p.parentCert.data.blockNumber.toNat + 1 = u + 1 := by
    have := congrArg BlockNumber.toNat hnum; rw [hhdr] at this; exact this
  have hepu : p.epoch = ⟨u + 1⟩ := by rw [hep, hhdr]; exact epochOf_one_height (by omega)
  cases u with
  | zero =>
    have hpcA : p.parentCert = certOf anchorB := by
      rcases hasCert1 hpc with h | ⟨x, h, -⟩ | ⟨x, h, -⟩
      · exact h
      · rw [h, cert_number] at hnum'; have : x + 1 + 1 = 0 + 1 := hnum'; omega
      · rw [h, cr_number] at hnum'; have : x + 1 + 1 = 0 + 1 := hnum'; omega
    have hte : p.timeoutEvidence = none := by
      cases hte : p.timeoutEvidence with
      | none => rfl
      | some tc =>
        obtain ⟨y, rfl, -⟩ := hev tc hte
        obtain ⟨he, -⟩ := hj.safe _ hte
        rw [hepu] at he
        exact absurd (congrArg EpochNumber.toNat he) (by show ¬ y + 2 = 0 + 1; omega)
    obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
    simp only at hvw hhdr hid hepu hpcA hte ⊢
    rw [hvw, hhdr, hid, hepu, hpcA, hte]; rfl
  | succ u =>
    -- The parent is the block's own `Cert1`: an epoch opens at its parent's view, and no block is at a re-vote's.
    have hpcB : p.parentCert = certOf (blk u) := by
      have hen : EntersEpoch cfg p := by
        show IsLastBlock (p.blockHeader.blockNumber - 1) 1
        rw [hhdr]; exact last_block (n := u + 1) (by omega)
      obtain ⟨⟨q, hq, hqv, -⟩, -⟩ := hj.opens hen
      rcases hasCert1 hpc with h | ⟨x, h, -⟩ | ⟨x, h, -⟩
      · rw [h] at hnum'; have : 0 + 1 = u + 1 + 1 := hnum'; omega
      · rw [h, cert_number] at hnum'
        obtain rfl : x = u := by have : x + 1 + 1 = u + 1 + 1 := hnum'; omega
        exact h
      · rw [h] at hqv; exact absurd hqv (block_of_view hq).2
    have hte : p.timeoutEvidence = some (T u) := by
      rcases hnext with ⟨-, hn⟩ | ⟨tc, hte, htv⟩
      · rw [hpcB, cert_view, hvw] at hn
        have : 3 * u + 1 + 1 = 3 * (u + 1) + 1 := congrArg ViewNumber.toNat hn
        omega
      · obtain ⟨y, rfl, -⟩ := hev tc hte
        rw [hvw] at htv
        obtain rfl : y = u := by
          have : 3 * y + 3 + 1 = 3 * (u + 1) + 1 := congrArg ViewNumber.toNat htv
          omega
        exact hte
    obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
    simp only at hvw hhdr hid hepu hpcB hte ⊢
    rw [hvw, hhdr, hid, hepu, hpcB, hte]; rfl

include hv in
/-- Every proposal `a` sends, it sends by the step its header arrives in. -/
theorem proposes (u : Nat) : ∃ j, j ≤ 15 * u ∧ Output.send (.proposal (blk u)) ∈ (tr a j).output := by
  have hvf := vAt_facts u 1 (by omega)
  refine Classical.byContradiction fun hneg => settled a (15 * u) (.propose (blk u).epoch ⟨3 * u + 1⟩)
    ⟨Or.inl ⟨blk u, blk_justified u 1 (Nat.le_refl _) (by omega), blk_view u, rfl⟩, ?_, ?_, ?_⟩
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, hrv, -⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv
      obtain rfl : x = u := by have := view_inj hpv; omega
      exact hneg ⟨j, by omega, hjp⟩
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_revote hv hjr <;> have := view_inj hrv <;> omega
  · intro ht
    obtain ⟨y, hle, hy⟩ := timedOut ht
    have : 3 * u + 1 ≤ 3 * y + 3 := hle
    omega
  · have := inView (k := a) (15 * u + 1)
    rwa [show vAt (15 * u + 1) = 3 * u + 1 by omega] at this

include hv in
/--
Every vote1 a node sends is for a block of its committee, or answers the block's
re-vote request. None answers the request on the re-vote's `Cert1`: it arrives
after the epoch change.
-/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1} (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    ∃ u, (lag k u = 0 ∧ vote = ⟨(certOf (blk u)).data, ⟨3 * u + 1⟩, k⟩ ∧ 15 * u + 1 ≤ j)
      ∨ (vote = ⟨(certOf (blk u)).data, ⟨3 * u + 2⟩, k⟩ ∧ 15 * u + 5 ≤ j) := by
  have hpr := protocol hv k (j + 1)
  obtain ⟨hsig, -⟩ := hpr.vote1Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  obtain ⟨-, ⟨s', p, vid, hrec, -, hfor, -⟩ | ⟨s', r, hrec, -, hagain, hcur⟩⟩ :=
    hpr.vote1Leader j vote ⟨_, (getElem_H j), hx⟩ trivial
  · rw [upTo_self] at hrec
    obtain ⟨i, hi, hii⟩ := received.mp hrec
    obtain ⟨u, rfl, hl, -, rfl, -⟩ := input_proposal hii
    refine ⟨u, Or.inl ⟨hl, ?_, by omega⟩⟩
    obtain ⟨d, v, sg⟩ := vote
    obtain ⟨hv', hd⟩ := hfor
    simp only at hsig hv' hd
    rw [hsig, hv', hd, blk_view]; rfl
  · rw [upTo_self] at hrec hcur
    obtain ⟨u, -, ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩⟩ := received_revote hrec
    · refine ⟨u, Or.inr ⟨?_, by omega⟩⟩
      obtain ⟨d, v, sg⟩ := vote
      obtain ⟨hv', hd⟩ := hagain
      simp only at hsig hv' hd
      rw [hsig, hv', hd]; rfl
    · have he := notBehind_le hcur
      rw [show (R2 u).cert.data.epoch = ⟨u + 1⟩ from cr_epoch u] at he
      have := eAt_ge (n := j + 1) (u := u) (by omega)
      exact absurd he (by show ¬ eAt (j + 1) ≤ u + 1; omega)

include hv in
/-- Every member of a block's committee votes1 for it by the step its validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ 15 * u + 2
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨3 * u + 1⟩, k⟩) ∈ (tr k j).output := by
  have hvf := vAt_facts u 3 (by omega)
  have hef := eAt_facts u 3 (by omega)
  have hopen : OpensEpochJustified cfg (H k (15 * u + 3)) (blk u) := fun he => by
    cases u with
    | zero => exact absurd he blk_enters_zero
    | succ u =>
      exact ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasC2_of u (by omega), by rw [blk_view]; show 3 * u + 1 < 3 * (u + 1) + 1; omega, rfl⟩
  have hprop : (H k (15 * u + 3)).Received (.proposal a (blk u) (some ⟨⟨3 * u + 1⟩, (blk u).payloadCommit⟩)) := by
    have := recv_at (k := k) (n := 15 * u + 3) u 1 (by omega) (by omega)
    simp only [phase, hl, ite_eq_left] at this
    exact this
  refine Classical.byContradiction fun hneg => settled k (15 * u + 2) (.vote1 (blk u))
    ⟨⟨a, _, hprop, rfl, by rw [blk_view], rfl⟩, blk_wellFormed u, ?_, ?_, blk_safe u, hopen,
      notBehind ?_, fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · rw [blk_view]; exact recv_at u 2 (by omega) (by omega)
  · cases u with
    | zero => exact Or.inl rfl
    | succ u => exact Or.inr (Or.inl (blk_enters u))
  · rw [blk_epoch]; show eAt (15 * u + 3) ≤ u + 1; omega
  · obtain ⟨y, hle, hy⟩ := timedOut ht
    rw [blk_view] at hle
    have : 3 * u + 1 ≤ 3 * y + 3 := hle
    omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', ⟨-, rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_vote1 hv hjv
    · rw [blk_view] at hvv
      obtain rfl : u' = u := by have := view_inj hvv; omega
      exact hneg ⟨j, by omega, hjv⟩
    · rw [blk_view] at hvv; have := view_inj hvv; omega
  · rw [blk_view]
    have := inView (k := k) (15 * u + 3)
    rwa [show vAt (15 * u + 3) = 3 * u + 1 by omega] at this

include hv in
/-- Every member of a block's committee answers its re-vote request by the step the request arrives in. -/
theorem votes1R (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ 15 * u + 5
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨3 * u + 2⟩, k⟩) ∈ (tr k j).output := by
  have hvf := vAt_facts u 6 (by omega)
  have hef := eAt_facts u 6 (by omega)
  refine Classical.byContradiction fun hneg => settled k (15 * u + 5) (.vote1Again (R u))
    ⟨⟨a, recv_at u 5 (by omega) (by omega), rfl⟩, ⟨?_, Or.inl ⟨rfl, ?_⟩, blk_last u⟩, (fun _ h => by cases h),
      ⟨blk u, hasProposal_of u (by omega), ⟨Nat.le_refl _, rfl⟩, payload_of u hl (by omega)⟩, notBehind ?_,
      fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · show (certOf (blk u)).view < ⟨3 * u + 2⟩; rw [cert_view]; show 3 * u + 1 < 3 * u + 2; omega
  · show (certOf (blk u)).view + 1 = ⟨3 * u + 2⟩; rw [cert_view]; rfl
  · show eAt (15 * u + 6) ≤ (certOf (blk u)).data.epoch.toNat
    rw [cert_epoch]; show _ ≤ u + 1; omega
  · obtain ⟨y, hle, hy⟩ := timedOut ht
    have : 3 * u + 2 ≤ 3 * y + 3 := hle
    omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', ⟨-, rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_vote1 hv hjv
    · have := view_inj hvv; omega
    · obtain rfl : u' = u := by have := view_inj hvv; omega
      exact hneg ⟨j, by omega, hjv⟩
  · have := inView (k := k) (15 * u + 6)
    rwa [show vAt (15 * u + 6) = 3 * u + 2 by omega] at this

include hv in
/-- Every vote2 a node sends is on a block of its committee, at its view or its re-vote's, after the payload. -/
theorem sent_vote2 {k : PubKey} {j : Nat} {vote : Vote2} (hx : Output.send (.vote2 vote) ∈ (tr k j).output) :
    ∃ u, lag k u = 0 ∧ ((vote = ⟨(C2 u).data, ⟨3 * u + 1⟩, k⟩ ∧ 15 * u + 4 ≤ j)
      ∨ (vote = ⟨(C2R u).data, ⟨3 * u + 2⟩, k⟩ ∧ 15 * u + 6 ≤ j)) := by
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
      rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩ | ⟨u, rfl, -⟩
      · rw [hvc] at hgen; exact Nat.lt_irrefl _ hgen
      · rw [cert_number] at hx0; exact absurd (number_inj hx0) (by omega)
      · rw [cr_number] at hx0; exact absurd (number_inj hx0) (by omega)
    · rw [blk_view] at hg; exact absurd (view_inj hg) (by omega)
  · obtain rfl := (block_of_view hb).1 hyv
    have hxn : x.data.blockNumber = ⟨u + 1⟩ := by rw [hcert.2]; exact blk_number u
    refine ⟨u, hl, ?_⟩
    rcases hasCert1 hc with rfl | ⟨u', rfl, -⟩ | ⟨u', rfl, hxt⟩
    · exact absurd (number_inj hxn) (by omega)
    · rw [cert_number] at hxn
      obtain rfl : u' = u := by have := number_inj hxn; omega
      refine Or.inl ⟨?_, by omega⟩
      rw [hsig, hvc, hdc, cert_view]; rfl
    · rw [cr_number] at hxn
      obtain rfl : u' = u := by have := number_inj hxn; omega
      refine Or.inr ⟨?_, by omega⟩
      rw [hsig, hvc, hdc]; rfl

include hv in
/-- Every member of a block's committee votes2 for it by the step its payload arrives in. -/
theorem votes2 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ 15 * u + 4
    ∧ Output.send (.vote2 ⟨(C2 u).data, ⟨3 * u + 1⟩, k⟩) ∈ (tr k j).output := by
  refine Classical.byContradiction fun hneg => settled k (15 * u + 4) (.vote2 (certOf (blk u)))
    ⟨⟨blk u, hasCert1_of u (by omega), hasProposal_of u (by omega), ⟨Nat.le_refl _, rfl⟩,
      payload_of u hl (by omega)⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', -, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_vote2 hv hjv
    · rw [cert_view] at hvv
      obtain rfl : u' = u := by have := view_inj hvv; omega
      exact hneg ⟨j, by omega, hjv⟩
    · rw [cert_view] at hvv; have := view_inj hvv; omega
  · rw [cert_view] at hv2
    rcases hasCert2 hc2 with ⟨x, rfl, hlt⟩ | ⟨x, rfl, hlt⟩
    · have : 3 * x + 1 = 3 * u + 1 := view_inj hv2
      omega
    · have : 3 * x + 2 = 3 * u + 1 := view_inj hv2
      omega
  · rw [cert_view]
    rintro (ht | ⟨tc, htc, hle⟩)
    · obtain ⟨y, hle, hy⟩ := timedOut ht
      have : 3 * u + 1 ≤ 3 * y + 3 := hle
      omega
    · obtain ⟨y, rfl, hy⟩ := received_tc htc
      have : 3 * u + 1 ≤ 3 * y + 3 := hle
      omega
  · refine Kit.afterFloor_of input hv (by rw [cert_view]; show 0 < 3 * u + 1; omega) fun x hx => ?_
    rw [cert_view]
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
    · show 0 < 3 * u + 1 + 20; omega
    · rw [blk_view]; show 3 * y + 1 < 3 * u + 1 + 20; omega

include hv in
/-- Every member of a block's committee votes2 on the re-vote's `Cert1` by the step it arrives in. -/
theorem votes2R (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ 15 * u + 6
    ∧ Output.send (.vote2 ⟨(C2R u).data, ⟨3 * u + 2⟩, k⟩) ∈ (tr k j).output := by
  refine Classical.byContradiction fun hneg => settled k (15 * u + 6) (.vote2 (CR u))
    ⟨⟨blk u, hasCR_of u (by omega), hasProposal_of u (by omega),
      ⟨by rw [blk_view]; show 3 * u + 1 ≤ 3 * u + 2; omega, rfl⟩, payload_of u hl (by omega)⟩,
      fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', -, ⟨rfl, -⟩ | ⟨rfl, -⟩⟩ := sent_vote2 hv hjv
    · have := view_inj hvv; omega
    · obtain rfl : u' = u := by have := view_inj hvv; omega
      exact hneg ⟨j, by omega, hjv⟩
  · rcases hasCert2 hc2 with ⟨x, rfl, hlt⟩ | ⟨x, rfl, hlt⟩
    · have : 3 * x + 1 = 3 * u + 2 := view_inj hv2
      omega
    · have : 3 * x + 2 = 3 * u + 2 := view_inj hv2
      omega
  · rintro (ht | ⟨tc, htc, hle⟩)
    · obtain ⟨y, hle, hy⟩ := timedOut ht
      have : 3 * u + 2 ≤ 3 * y + 3 := hle
      omega
    · obtain ⟨y, rfl, hy⟩ := received_tc htc
      have : 3 * u + 2 ≤ 3 * y + 3 := hle
      omega
  · refine Kit.afterFloor_of input hv (by show 0 < 3 * u + 2; omega) fun x hx => ?_
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
    · show 0 < 3 * u + 2 + 20; omega
    · rw [blk_view]; show 3 * y + 1 < 3 * u + 2 + 20; omega

end Sends

/-! ## Time -/

section Time

/--
When step `r` of a block's fifteen happens, from the block's start: one unit a
step, except that the timer for the view after the re-vote fires `τ = 33` after
the nodes entered it, on the re-vote's `Cert1`.
-/
def off (r : Nat) : Nat := if r ≤ 11 then r else r + 27

/-- When step `n` happens. A block takes `42` time units. -/
def tm (n : Nat) : Nat := 42 * (n / 15) + off (n % 15)

/-- The time of step `15u + r`, as facts `omega` reads. -/
theorem tm_facts (u r : Nat) (hr : r < 15) :
    (r ≤ 11 → tm (15 * u + r) = 42 * u + r) ∧ (12 ≤ r → tm (15 * u + r) = 42 * u + r + 27) := by
  simp only [tm, off]
  rw [show (15 * u + r) / 15 = u by omega, show (15 * u + r) % 15 = r by omega]
  exact ⟨fun h => by rw [ite_eq_left h], fun h => by rw [ite_eq_right (by omega)]; omega⟩

theorem tm_zero (u : Nat) : tm (15 * u) = 42 * u := by
  have := (tm_facts u 0 (by omega)).1 (by omega)
  simpa using this

theorem tm_succ (n : Nat) : tm n ≤ tm (n + 1) := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  by_cases h14 : r = 14
  · subst h14
    rw [show 15 * u + 14 + 1 = 15 * (u + 1) by omega, tm_zero]
    have := (tm_facts u 14 (by omega)).2 (by omega)
    omega
  · rw [show 15 * u + r + 1 = 15 * u + (r + 1) by omega]
    have := tm_facts u r hr
    have := tm_facts u (r + 1) (by omega)
    omega

theorem tm_mono {n m : Nat} (h : n ≤ m) : tm n ≤ tm m := Kit.mono_of_succ tm_succ h

theorem tm_ge (n : Nat) : n ≤ tm n := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  have := tm_facts u r hr
  omega

/-- A node enters the view after a re-vote at the step the re-vote's `Cert1` arrives in. -/
theorem vAt_entry {n u : Nat} (h1 : vAt (n + 1) = 3 * u + 3) (h2 : vAt n ≠ 3 * u + 3) : n = 15 * u + 6 := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := vAt_facts u' r hr
  by_cases h14 : r = 14
  · subst h14
    rw [show 15 * u' + 14 + 1 = 15 * (u' + 1) + 0 by omega] at h1
    have := vAt_facts (u' + 1) 0 (by omega)
    omega
  · rw [show 15 * u' + r + 1 = 15 * u' + (r + 1) by omega] at h1
    have := vAt_facts u' (r + 1) (by omega)
    omega

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
theorem backed1R (u : Nat) : Cert1Backed (C := C) (fun k _ => tr k) (CR u) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k, by rw [cr_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes1R hv k u (lag_member hk.1)
  exact ⟨j, hj⟩

include hv in
theorem backed2 (u : Nat) : Cert2Backed (C := C) (fun k _ => tr k) (C2 u) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k,
    by show C.Quorum (certOf (blk u)).data.epoch _; rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes2 hv k u (lag_member hk.1)
  exact ⟨j, hj⟩

include hv in
theorem backed2R (u : Nat) : Cert2Backed (C := C) (fun k _ => tr k) (C2R u) := by
  refine ⟨fun k => C.members ⟨u + 1⟩ k ∧ C.honest ⟨u + 1⟩ k,
    by show C.Quorum (certOf (blk u)).data.epoch _; rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes2R hv k u (lag_member hk.1)
  exact ⟨j, hj⟩

theorem tcBacked (u : Nat) : TimeoutCertBacked (C := C) (fun k _ => tr k) (T u) :=
  ⟨fun k => C.members ⟨u + 2⟩ k ∧ C.honest ⟨u + 2⟩ k, members_quorum _, fun k _ _ =>
    ⟨_, ⟨rfl, rfl, rfl, lockAt_timeout_le u⟩, ⟨15 * u + 12, times_out k u⟩⟩⟩

include hv in
theorem tcChecked (u : Nat) : TimeoutLockChecked (C := C) (fun k _ => tr k) cfg (T u) :=
  ⟨Or.inr (backed1R hv u), by show 3 * u + 2 ≤ 3 * u + 3; omega⟩

/-- The honest nodes running the machine on their schedules. -/
def net : TimedNetwork cfg leader C where
  honestQuorum := members_quorum
  trace k _ := tr k
  safe k _ n := .of_every (protocol hv k n).toSafeHistory
  cert1Genuine k _ n x hc := by
    rcases Input.mem_cert1.mp hc with hin | ⟨c2, p, hin⟩ | ⟨s, p, vid, hin, rfl⟩
    · obtain ⟨u, ⟨-, rfl⟩ | ⟨-, rfl⟩⟩ := input_cert1 hin
      · exact Or.inr (backed1 hv u)
      · exact Or.inr (backed1R hv u)
    · obtain ⟨u, rfl, -, -⟩ := input_epochChange hin
      exact Or.inr (backed1 hv u)
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      rw [blk_parent]
      cases u with
      | zero => exact Or.inl rfl
      | succ u => exact Or.inr (backed1 hv u)
  cert2Genuine k _ n x hc := by
    rcases Input.mem_cert2.mp hc with hin | ⟨c1, p, hin⟩
    · obtain ⟨u, ⟨-, rfl⟩ | ⟨-, rfl⟩⟩ := input_cert2 hin
      · exact backed2 hv u
      · exact backed2R hv u
    · obtain ⟨u, -, -, ⟨-, rfl⟩ | ⟨-, rfl⟩⟩ := input_epochChange hin
      · exact backed2 hv u
      · exact backed2R hv u
  timeoutCertGenuine k _ n tc hc := by
    rcases Input.mem_timeoutCert.mp hc with hin | ⟨s, p, vid, hin, hte⟩ | ⟨s, r, hin, hte⟩
    · obtain ⟨u, -, rfl⟩ := input_tc hin
      exact ⟨tcBacked u, tcChecked hv u⟩
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      cases u with
      | zero => cases hte
      | succ u => cases hte; exact ⟨tcBacked u, tcChecked hv u⟩
    · obtain ⟨u, ⟨-, -, rfl⟩ | ⟨-, -, rfl⟩⟩ := input_revote hin <;> cases hte
  revoteGenuine k _ n _ r hin := by
    obtain ⟨u, ⟨-, -, rfl⟩ | ⟨-, -, rfl⟩⟩ := input_revote hin
    · exact backed1 hv u
    · exact backed1R hv u
  time _ _ n := tm n
  timeMono _ _ n := tm_succ n
  protocol k _ n := .of_every (protocol hv k n)
  timeoutCertCausal k _ n tc hin := by
    obtain ⟨u, rfl, rfl⟩ := input_tc hin
    refine ⟨fun k => C.members ⟨u + 2⟩ k ∧ C.honest ⟨u + 2⟩ k, members_quorum _, fun k' _ _ =>
      ⟨15 * u + 12, _, ⟨rfl, rfl, rfl, lockAt_timeout_le u⟩, times_out k' u, ?_⟩⟩
    have := (tm_facts u 12 (by omega)).2 (by omega)
    have := (tm_facts u 13 (by omega)).2 (by omega)
    omega
  oneHonestCausal k _ n v hin := by
    obtain ⟨u, rfl, rfl⟩ := input_oneHonest hin
    refine ⟨a, _, Or.inl rfl, 15 * u + 12, _, rfl, times_out a u, rfl, ?_⟩
    have := (tm_facts u 12 (by omega)).2 (by omega)
    have := (tm_facts u 14 (by omega)).2 (by omega)
    omega
  authentic k _ n l msg hin _ _ := by
    cases hi : (tr k n).input <;> rw [hi] at hin <;> simp only [Input.sentBy, reduceCtorEq] at hin
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨u, rfl, rfl, rfl⟩ := input_proposal_any hi
      obtain ⟨j, hj, hjp⟩ := proposes hv u
      refine ⟨j, hjp, Nat.lt_of_le_of_lt (tm_mono hj) ?_⟩
      have := tm_zero u
      have := (tm_facts u 1 (by omega)).1 (by omega)
      omega
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨u, ⟨rfl, rfl, rfl⟩ | ⟨rfl, rfl, rfl⟩⟩ := input_revote hi
      · obtain ⟨j, hj, hjr⟩ := revotes hv u
        refine ⟨j, hjr, Nat.lt_of_le_of_lt (tm_mono (show j ≤ 15 * u + 4 by omega)) ?_⟩
        have := (tm_facts u 4 (by omega)).1 (by omega)
        have := (tm_facts u 5 (by omega)).1 (by omega)
        omega
      · obtain ⟨j, hj, hjr⟩ := revotes2 hv u
        refine ⟨j, hjr, Nat.lt_of_le_of_lt (tm_mono hj) ?_⟩
        have := (tm_facts u 6 (by omega)).1 (by omega)
        have := (tm_facts u 9 (by omega)).1 (by omega)
        omega

theorem net_time {k : PubKey} {hk : C.Honest k} {n : Nat} : (net hv).time k hk n = tm n := rfl

theorem by_at {k : PubKey} {hk : C.Honest k} {T m : Nat} {P : History → Prop}
    (hm : 0 < m → tm (m - 1) ≤ T) (hp : P (H k m)) : (net hv).By k hk T P :=
  Kit.by_at (fun _ _ => rfl) (fun _ _ _ => rfl) tm_succ hm hp

theorem sentBy {k : PubKey} {hk : C.Honest k} {t : Nat} {m : Message} (hs : (net hv).SentByTime k hk t m) :
    ∃ j, tm j ≤ t ∧ Output.send m ∈ (tr k j).output :=
  Kit.sentBy (N := net hv) (fun _ _ => rfl) (fun _ _ _ => rfl) hs

include hv in
/-- A header for every view `a` is ready to propose in, while in it, arrives within `Δ`. -/
theorem header_arrives {k : PubKey} {hk : C.Honest k} {n : Nat} {p : Proposal}
    (hin : (H k (n + 1)).InView cfg p.viewNumber) (hready : ProposalReady cfg leader k (H k (n + 1)) p) :
    (net hv).By k hk (max (tm n) 0 + 4) fun hist =>
      ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
  obtain ⟨hlead, ⟨-, hnext, hep, hnum⟩, hj, -, hopen, hsafe, -, -⟩ := hready
  obtain rfl := FiveNodes.leader_eq hlead
  have hmax := Nat.le_max_left (tm n) 0
  have hc := hasCert1_of_certJustified hj
  -- Every header arrives within `Δ` of the node's being able to use it; `m` is when.
  have hdone : ∀ m, m ≤ n + 1 ∨ tm (m - 1) ≤ tm n + 4 → ∀ hdr', hdr'.blockNumber = p.blockHeader.blockNumber →
      (H a m).Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') →
      (net hv).By a hk (max (tm n) 0 + 4) fun hist =>
        ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
          ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
    intro m hm hdr' h1 h2
    rcases hm with hm | hm
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨hdr', h1, received_mono hm h2⟩
    · exact by_at hv (m := m) (fun _ => by omega) ⟨hdr', h1, h2⟩
  rcases hasCert1 hc with hpc | ⟨x, hpc, hx⟩ | ⟨x, hpc, hx⟩
  · -- On genesis: view one, whose header comes first.
    have hpv : p.viewNumber = ⟨1⟩ := by
      rcases hnext with ⟨-, h⟩ | ⟨tc, hte, -⟩
      · rw [← h, hpc]; rfl
      · exfalso
        obtain ⟨y, rfl, -⟩ := (justified_held hj).2 tc hte
        obtain ⟨he, -⟩ := hsafe _ hte
        rw [hep, ← hnum, hpc] at he
        have : y + 2 = 1 := congrArg EpochNumber.toNat he
        omega
    refine hdone 1 (Or.inr (by show tm 0 ≤ _; have : tm 0 = 0 := rfl; omega)) (hdr 1)
      (by rw [← hnum, hpc]; rfl) ?_
    rw [hpv, hpc]; exact recv_at 0 0 (by omega) (by omega)
  · have hnum' : p.blockHeader.blockNumber = ⟨x + 2⟩ := by rw [← hnum, hpc, cert_number]; rfl
    have hen : EntersEpoch cfg p := by
      show IsLastBlock (p.blockHeader.blockNumber - 1) 1
      rw [hnum']; exact last_block (n := x + 1) (by omega)
    rcases hnext with ⟨-, h⟩ | ⟨tc, hte, htv⟩
    · -- On a block, at the view after it: `a` has left it when the `Cert2` arrives.
      exfalso
      have hpv : p.viewNumber = ⟨3 * x + 2⟩ := by rw [← h, hpc, cert_view]; rfl
      have hvn : 3 * x + 2 = vAt (n + 1) := by
        rw [hpv] at hin; exact congrArg ViewNumber.toNat (inView_eq hin)
      obtain ⟨-, c2, hc2, hc2v, hc2d⟩ := hopen hen
      rcases hc2 with hc2 | rfl
      case inr =>
        exfalso
        have h0 := congrArg Vote2Data.blockNumber hc2d
        have h2 : p.parentCert.data.toVote2.blockNumber = ⟨x + 1⟩ := by rw [hpc]; exact cert_number x
        rw [h2] at h0
        exact absurd (congrArg BlockNumber.toNat h0) (by show ¬ (0 : Nat) = x + 1; omega)
      have hn7 : 15 * x + 7 < n + 1 := by
        rcases hasCert2 hc2 with ⟨y, rfl, hy⟩ | ⟨y, rfl, hy⟩
        · have : y = x := by
            have := congrArg Vote2Data.blockNumber hc2d
            have h1 : (C2 y).data.blockNumber = ⟨y + 1⟩ := cert_number y
            have h2 : p.parentCert.data.toVote2.blockNumber = ⟨x + 1⟩ := by rw [hpc]; exact cert_number x
            rw [h1, h2] at this; have := number_inj this; omega
          omega
        · rw [hpv] at hc2v
          have : 3 * y + 2 < 3 * x + 2 := hc2v
          have : y = x := by
            have := congrArg Vote2Data.blockNumber hc2d
            have h1 : (C2R y).data.blockNumber = ⟨y + 1⟩ := cert_number y
            have h2 : p.parentCert.data.toVote2.blockNumber = ⟨x + 1⟩ := by rw [hpc]; exact cert_number x
            rw [h1, h2] at this; have := number_inj this; omega
          omega
      obtain ⟨u', r, hr, hn1⟩ := steps (n + 1)
      have := vAt_facts u' r hr
      rw [hn1] at hvn hn7
      omega
    · -- After a timeout: the next epoch's first view.
      obtain ⟨y, rfl, hy⟩ := (justified_held hj).2 tc hte
      obtain ⟨he, -⟩ := hsafe _ hte
      rw [hep, hnum', show cfg.epochHeight = 1 from rfl, epochOf_one_height (by omega)] at he
      obtain rfl : y = x := by have : y + 2 = x + 2 := congrArg EpochNumber.toNat he; omega
      have hpv : p.viewNumber = ⟨3 * (y + 1) + 1⟩ := by
        rw [← htv]; show (⟨3 * y + 3 + 1⟩ : ViewNumber) = _; congr 1
      have := tm_mono (show 15 * y + 13 ≤ n by omega)
      have := (tm_facts y 13 (by omega)).2 (by omega)
      have := tm_zero (y + 1)
      refine hdone (15 * (y + 1) + 1) (by
          by_cases hn : 15 * (y + 1) + 1 ≤ n + 1
          · exact Or.inl hn
          · exact Or.inr (by show tm (15 * (y + 1)) ≤ _; omega)) (hdr (y + 2)) hnum'.symm ?_
      rw [hpv, hpc]; exact recv_at (y + 1) 0 (by omega) (by omega)
  · -- On a re-vote's `Cert1`: it would open an epoch at a view no block is at.
    exfalso
    have hen : EntersEpoch cfg p := by
      show IsLastBlock (p.blockHeader.blockNumber - 1) 1
      rw [← hnum, hpc, cr_number]; exact last_block (n := x + 1) (by omega)
    obtain ⟨⟨q, hq, hqv, -⟩, -⟩ := hopen hen
    rw [hpc] at hqv
    exact (block_of_view hq).2 hqv

/-- The timer for a view does not fire before `τ` has passed since the node entered it. -/
theorem timer_not_early {k : PubKey} {n m : Nat} {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v)
    (hnot : n = 0 ∨ ¬ (H k n).InView cfg v) (hinm : input k m = .timeout v) : tm n + 33 ≤ tm m := by
  obtain ⟨u, rfl, rfl⟩ := input_timeout hinm
  have h1 : vAt (n + 1) = 3 * u + 3 := (congrArg ViewNumber.toNat (inView_eq hin)).symm
  have h2 : vAt n ≠ 3 * u + 3 := fun h => by
    rcases hnot with rfl | hnot
    · have : vAt 0 = 1 := by decide
      omega
    · exact hnot (by have := inView (k := k) n; rwa [h] at this)
  rw [vAt_entry h1 h2, (tm_facts u 6 (by omega)).1 (by omega), (tm_facts u 12 (by omega)).2 (by omega)]
  omega

/-- The timer for a view fires within `τ` of the node's entering it, unless the node has moved on. -/
theorem timer_fires {k : PubKey} (n : Nat) {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v) :
    ∃ m, n < m ∧ tm m ≤ tm n + 33 ∧ (input k m = .timeout v ∨ ∃ w, v < w ∧ (H k (m + 1)).InView cfg w) := by
  have hv' := inView_eq hin
  subst hv'
  obtain ⟨u, r, hr, hn1⟩ := steps (n + 1)
  rw [hn1]
  obtain ⟨h14, h7, h4, h0⟩ := vAt_facts u r hr
  have hlo : ∀ j, 15 * u + j ≤ n → j ≤ 11 → 42 * u + j ≤ tm n := fun j hj hj11 => by
    have := tm_mono hj
    have := (tm_facts u j (by omega)).1 hj11
    omega
  by_cases r14 : 14 ≤ r
  · -- The view after the timeout certificate: the node moves on with the next block's `Cert1`.
    rw [h14 r14]
    refine ⟨15 * (u + 1) + 3, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
    · rw [(tm_facts (u + 1) 3 (by omega)).1 (by omega), show n = 15 * u + 13 by omega,
        (tm_facts u 13 (by omega)).2 (by omega)]; omega
    · rw [show 15 * (u + 1) + 3 + 1 = 15 * (u + 1) + 4 by omega,
        (vAt_facts (u + 1) 4 (by omega)).2.2.1 (by omega) (by omega)]
      show 3 * u + 4 < 3 * (u + 1) + 2; omega
  by_cases r7 : 7 ≤ r
  · rw [h7 r7 (by omega)]
    by_cases r13 : r = 13
    · -- Timed out: the timeout certificate moves the node on.
      refine ⟨15 * u + 13, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
      · rw [show n = 15 * u + 12 by omega, (tm_facts u 12 (by omega)).2 (by omega),
          (tm_facts u 13 (by omega)).2 (by omega)]; omega
      · rw [(vAt_facts u 14 (by omega)).1 (by omega)]; show 3 * u + 3 < 3 * u + 4; omega
    · refine ⟨15 * u + 12, by omega, ?_, Or.inl ?_⟩
      · have := hlo 6 (by omega) (by omega)
        rw [(tm_facts u 12 (by omega)).2 (by omega)]
        omega
      · rw [input_at k u 12 (by omega)]; rfl
  by_cases r4 : 4 ≤ r
  · -- The re-vote's view: the node moves on with the re-vote's `Cert1`.
    rw [h4 r4 (by omega)]
    refine ⟨15 * u + 6, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
    · have := hlo 3 (by omega) (by omega)
      rw [(tm_facts u 6 (by omega)).1 (by omega)]
      omega
    · rw [show 15 * u + 6 + 1 = 15 * u + 7 by omega, (vAt_facts u 7 (by omega)).2.1 (by omega) (by omega)]
      show 3 * u + 2 < 3 * u + 3; omega
  · -- The view of a block: the node moves on with its `Cert1`.
    rw [h0 (by omega)]
    refine ⟨15 * u + 3, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
    · rw [(tm_facts u 3 (by omega)).1 (by omega)]
      by_cases hr0 : r = 0
      · subst hr0
        obtain ⟨u', rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
        rw [show n = 15 * u' + 14 by omega, (tm_facts u' 14 (by omega)).2 (by omega)]; omega
      · have := hlo 0 (by omega) (by omega)
        omega
    · rw [show 15 * u + 3 + 1 = 15 * u + 4 by omega, (vAt_facts u 4 (by omega)).2.2.1 (by omega) (by omega)]
      show 3 * u + 1 < 3 * u + 2; omega

include hv in
/-- Every delivery within `Δ = 4`, after GST `0`, with view timer `τ = 33`. -/
theorem sync : Synchrony (net hv) 0 4 33 where
  proposal l hl n p hsend _ k hk hmem _ := by
    simp only [net_time hv]
    obtain ⟨rfl, u, rfl, hu⟩ := sent_proposal hv hsend
    rw [blk_epoch] at hmem
    have hl0 := lag_member hmem
    have := tm_mono hu
    have := tm_zero u
    have := (tm_facts u 1 (by omega)).1 (by omega)
    have hmax := Nat.le_max_left (tm n) 0
    refine by_at hv (m := 15 * u + 2) (fun _ => by show tm (15 * u + 1) ≤ _; omega)
      ⟨⟨⟨3 * u + 1⟩, (blk u).payloadCommit⟩, ⟨by rw [blk_view], rfl⟩, ?_⟩
    have := recv_at (k := k) (n := 15 * u + 2) u 1 (by omega) (by omega)
    simp only [phase, hl0, ite_eq_left] at this
    exact this
  revote l hl n r hsend _ k hk _ _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    obtain ⟨rfl, x, ⟨rfl, hx⟩ | ⟨rfl, hx⟩⟩ := sent_revote hv hsend <;> have := tm_mono hx
    · have := (tm_facts x 4 (by omega)).1 (by omega)
      have := (tm_facts x 5 (by omega)).1 (by omega)
      exact by_at hv (m := 15 * x + 6) (fun _ => by show tm (15 * x + 5) ≤ _; omega)
        (recv_at (k := k) x 5 (by omega) (by omega))
    · -- The request on the re-vote's `Cert1`, after the epoch change.
      have := (tm_facts x 6 (by omega)).1 (by omega)
      have := (tm_facts x 9 (by omega)).1 (by omega)
      exact by_at hv (m := 15 * x + 10) (fun _ => by show tm (15 * x + 9) ≤ _; omega)
        (recv_at (k := k) x 9 (by omega) (by omega))
  cert1 q d v t hq hvotes k hk _ := by
    obtain ⟨hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    have hmax := Nat.le_max_left t 0
    obtain ⟨x, ⟨-, heq, hx⟩ | ⟨heq, hx⟩⟩ := sent_vote1 hv hj <;> simp only [Vote.mk.injEq] at heq <;>
      obtain ⟨rfl, rfl, -⟩ := heq <;> have := tm_mono hx
    · have := (tm_facts x 1 (by omega)).1 (by omega)
      have := (tm_facts x 3 (by omega)).1 (by omega)
      refine by_at hv (m := 15 * x + 4) (fun _ => by show tm (15 * x + 3) ≤ _; omega) ?_
      rw [show (⟨(certOf (blk x)).data, ⟨3 * x + 1⟩⟩ : Cert1) = certOf (blk x) by rw [← cert_view x]]
      exact recv_at x 3 (by omega) (by omega)
    · -- The re-vote's `Cert1`.
      have := (tm_facts x 5 (by omega)).1 (by omega)
      have := (tm_facts x 6 (by omega)).1 (by omega)
      exact by_at hv (m := 15 * x + 7) (fun _ => by show tm (15 * x + 6) ≤ _; omega)
        (recv_at (k := k) x 6 (by omega) (by omega))
  cert2 q d v t hq hvotes k hk _ := by
    obtain ⟨hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    have hmax := Nat.le_max_left t 0
    obtain ⟨x, -, ⟨heq, hx⟩ | ⟨heq, hx⟩⟩ := sent_vote2 hv hj <;> simp only [Vote.mk.injEq] at heq <;>
      obtain ⟨rfl, rfl, -⟩ := heq <;> have := tm_mono hx
    · have := (tm_facts x 4 (by omega)).1 (by omega)
      have := (tm_facts x 7 (by omega)).1 (by omega)
      exact by_at hv (m := 15 * x + 8) (fun _ => by show tm (15 * x + 7) ≤ _; omega)
        (recv_at (k := k) x 7 (by omega) (by omega))
    · have := (tm_facts x 6 (by omega)).1 (by omega)
      have := (tm_facts x 10 (by omega)).1 (by omega)
      exact by_at hv (m := 15 * x + 11) (fun _ => by show tm (15 * x + 10) ≤ _; omega)
        (recv_at (k := k) x 10 (by omega) (by omega))
  cert2Spread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    refine by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) ?_
    rcases hasCert2 hc with ⟨x, rfl, hx⟩ | ⟨x, rfl, hx⟩
    · exact hasC2_of x hx
    · exact hasC2R_of x hx
  certSpread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩ | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasCert1_of x hx)
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasCR_of x hx)
  lockSpread c k hk n hc k' hk' hm _ := by
    -- A member rebuilds the payload the step after the block's `Cert1`, and holds it for the re-vote's.
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩ | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · have hl : lag k' x = 0 := lag_member (by rw [cert_epoch] at hm; exact hm)
      by_cases hn : 15 * x + 4 < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
          (lockable_iff.mpr (Or.inr (Or.inl ⟨x, rfl, Or.inl ⟨hl, hn⟩⟩)))
      · have := tm_mono (show 15 * x + 3 ≤ n by omega)
        have := (tm_facts x 3 (by omega)).1 (by omega)
        have := (tm_facts x 4 (by omega)).1 (by omega)
        exact by_at hv (m := 15 * x + 5) (fun _ => by show tm (15 * x + 4) ≤ _; omega)
          (lockable_iff.mpr (Or.inr (Or.inl ⟨x, rfl, Or.inl ⟨hl, by omega⟩⟩)))
    · have hl : lag k' x = 0 := lag_member (by rw [cr_epoch] at hm; exact hm)
      exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        (lockable_iff.mpr (Or.inr (Or.inr ⟨x, rfl, hl, hx⟩)))
  blockSpread c b _ k hk n hcb k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    refine by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) ?_
    rcases hasProposal hcb.2 with rfl | ⟨y, rfl, hy⟩
    · exact Or.inl rfl
    · exact hasProposal_of y hy
  timeoutCert e q v t hq hvotes k hk _ := by
    obtain ⟨hka, L, hsent⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hsent
    obtain ⟨u, rfl, heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
    obtain ⟨⟨rfl, -⟩, rfl, -⟩ := heq
    have := (tm_facts u 12 (by omega)).2 (by omega)
    have := (tm_facts u 13 (by omega)).2 (by omega)
    have hmax := Nat.le_max_left t 0
    exact by_at hv (m := 15 * u + 14) (fun _ => by show tm (15 * u + 13) ≤ _; omega)
      ⟨T u, rfl, rfl, recv_at (k := k) u 13 (by omega) (by omega)⟩
  timeoutCertSpread tc k hk n hin k' hk' _ := by
    refine Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate tc))
      (fun _ h => ⟨tc, rfl, rfl, h⟩) (Kit.by_slower ?_)
    simp only [net_time hv]
    obtain ⟨u, rfl, hu⟩ := received_tc hin
    have hmax := Nat.le_max_left (tm n) 0
    exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
      (recv_at (k := k') u 13 (by omega) hu)
  timeoutLockSpread tc k hk n hin k' hk' hm _ := by
    simp only [net_time hv]
    obtain ⟨u, rfl, hu⟩ := received_tc hin
    have hl : lag k' u = 0 := lag_member (by
      have : (T u).data.lock.data.epoch = ⟨u + 1⟩ := cr_epoch u
      rw [this] at hm; exact hm)
    have hmax := Nat.le_max_left (tm n) 0
    exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
      (lockable_iff.mpr (Or.inr (Or.inr ⟨u, rfl, hl, by omega⟩)))
  epochChange c2 b hcm _ k hk n hbc k' hk' _ := by
    obtain ⟨hb, hc2⟩ := hbc
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    have hbx : ∀ x, c2.data = (certOf (blk x)).data.toVote2 → b = blk x := fun x hd => by
      have hbn : b.blockHeader.blockNumber = ⟨x + 1⟩ := by
        rw [← cert_number x]; exact ((congrArg Vote2Data.blockNumber hcm.2).symm.trans
          (congrArg Vote2Data.blockNumber hd))
      exact (block_of_number hb hbn).1
    rcases hasCert2 hc2 with ⟨x, rfl, hx⟩ | ⟨x, rfl, hx⟩
    · obtain rfl := hbx x rfl
      by_cases hn : 15 * x + 8 < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
          ⟨_, tookEpochChange_of x hn⟩
      · have := tm_mono (show 15 * x + 7 ≤ n by omega)
        have := (tm_facts x 7 (by omega)).1 (by omega)
        have := (tm_facts x 8 (by omega)).1 (by omega)
        exact by_at hv (m := 15 * x + 9) (fun _ => by show tm (15 * x + 8) ≤ _; omega)
          ⟨_, tookEpochChange_of x (by omega)⟩
    · obtain rfl := hbx x rfl
      by_cases hn : 15 * x + 11 < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
          ⟨_, tookEpochChangeR_of x hn⟩
      · have := tm_mono (show 15 * x + 10 ≤ n by omega)
        have := (tm_facts x 10 (by omega)).1 (by omega)
        have := (tm_facts x 11 (by omega)).1 (by omega)
        exact by_at hv (m := 15 * x + 12) (fun _ => by show tm (15 * x + 11) ≤ _; omega)
          ⟨_, tookEpochChangeR_of x (by omega)⟩
  proposalValid _ _ _ p _ _ := hv p
  validatedSound _ _ _ _ _ _ b _ := hv b
  validated k hk n s p vid hin _ _ := by
    simp only [net_time hv]
    obtain ⟨u, rfl, -, -, rfl, -⟩ := input_proposal hin
    have := (tm_facts u 1 (by omega)).1 (by omega)
    have := (tm_facts u 2 (by omega)).1 (by omega)
    have hmax := Nat.le_max_left (tm (15 * u + 1)) 0
    refine by_at hv (m := 15 * u + 3) (fun _ => by show tm (15 * u + 2) ≤ _; omega) ?_
    rw [blk_view]
    exact recv_at (k := k) u 2 (by omega) (by omega)
  header k hk n p _ hin hready := by simp only [net_time hv]; exact header_arrives hv hin hready
  timeUnbounded _ _ T := ⟨T + 1, by have := tm_ge (T + 1); show T < tm (T + 1); omega⟩
  timerNotEarly k hk n m v hin hnot _ hinm := timer_not_early hin hnot hinm
  timerFires k hk n v hin _ := timer_fires n hin

theorem rotation : LeaderRotation C leader := fun _ v => ⟨v, Nat.le_refl _, a, rfl, Or.inl rfl, Or.inl rfl⟩

include hv in
/-- **The liveness premises can be met together, with every re-vote answered.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv) 0 4 33 ∧ Prompt (net hv) 0
      ∧ 8 * 4 + 3 * 0 < 33 ∧ LeaderRotation C leader :=
  ⟨cfg_coherent, sync hv, prompt_of_machine _ 0 (fun k _ => ⟨input k, rfl⟩)
    (fun _ hk => Kit.steady_of_uniform (fun _ _ _ h => h) hk), by decide, rotation⟩

include hv in
/--
And the re-votes are answered: `a` asks for one on every block; every member of
the block's committee votes1 on it again; every node holds the second `Cert2`,
at the re-vote's view, and takes the epoch change with it; the timeout
certificate for the view after it is locked on the re-vote's `Cert1`, while the
node new to the committee times out locked on the block's own; and `a` opens the
next epoch on the block behind that certificate.
-/
theorem revote_answered :
    (∀ u, ∃ j, Output.send (.revote (R u)) ∈ (tr a j).output)
      ∧ (∀ k u, C.members ⟨u + 1⟩ k → C.Honest k →
        ∃ j, Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨3 * u + 2⟩, k⟩) ∈ (tr k j).output)
      ∧ (∀ k u, (H k (15 * u + 12)).HasCert2 (C2R u)
        ∧ (H k (15 * u + 12)).TookEpochChange cfg (certOf (blk u)) (C2R u) (blk u))
      ∧ (∀ u, (T u).data.lock = CR u ∧ lockAt a (15 * u + 12) = CR u
        ∧ lockAt (third ⟨u + 2⟩) (15 * u + 12) = certOf (blk u))
      ∧ (∀ u, (blk (u + 1)).timeoutEvidence = some (T u)
        ∧ (blk (u + 1)).parentCert = certOf (blk u)
        ∧ ∃ j, Output.send (.proposal (blk (u + 1))) ∈ (tr a j).output) :=
  ⟨fun u => let ⟨j, _, hj⟩ := revotes hv u; ⟨j, hj⟩,
    fun k u hm _ => let ⟨j, _, hj⟩ := votes1R hv k u (lag_member hm); ⟨j, hj⟩,
    fun _ u => ⟨hasC2R_of u (by omega), tookEpochChangeR_of u (by omega)⟩,
    fun u => ⟨rfl, by rw [lockAt_timeout, lag_a, ite_eq_left rfl],
      by rw [lockAt_timeout, ite_eq_right (by simp [lag])]⟩,
    fun u => ⟨rfl, rfl, let ⟨j, _, hj⟩ := proposes hv (u + 1); ⟨j, hj⟩⟩⟩

include hv in
/-- So every honest node keeps deciding, every boundary's re-vote answered. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : C.Honest k) :
    (net hv).DecidesAfter k hk t := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met hv
  exact (Liveness.chainGrows (cfg := cfg) (leader := leader) (C := C) (net hv) 0 4 0 33
    hc hcf hs hp hb hr).2 t k hk (Or.inl (Kit.steady_of_uniform (fun _ _ _ h => h) hk))

end Net

end RevoteWitness
end NewProtocolImpl
