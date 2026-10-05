module

public import NewProtocolImpl.WitnessKit
public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.FiveNodes

/-!
# A network the liveness premises hold in, with two blocks to an epoch

`NewProtocolImpl.EpochWitness` has one block to an epoch, so every block ends one,
and a node outside a block's committee always locks on it through the epoch
change. Here an epoch has two blocks, and a node outside the committee never holds
the first block's payload and never locks on it. It follows the views on
certificates alone, and decides the first block from the block and its
certificates (`Synchrony.certSpread`, `Synchrony.blockSpread`).

The nodes and committees are `NewProtocolImpl.FiveNodes`'s: `a`, `b`, `c` and `nw`
honest, `d` faulty and silent, `c` outside the even epochs' committees and `nw`
outside the odd ones'. `a` leads every view. Epoch height is two: block `u + 1`, at
view `u + 1`, is in epoch `u / 2 + 1`, so blocks `2w + 1` and `2w + 2` make epoch
`w + 1`. A node receives eight inputs to a block:

* the header, the proposal with the node's share, the validity report, the
  payload, the `Cert1` and the `Cert2`. A node outside the block's committee gets
  the proposal without a share, and a second validity report
  instead of the payload (`phase`);
* for an epoch's last block, the epoch change, with the block's `Cert1` and `Cert2`,
  through which a node outside the committee locks on the block; then the re-vote
  request `a` sent once it could lock on the block, in view `2w + 3`. It arrives
  after the epoch change, so nobody votes on it (`NotBehind`);
* for an epoch's first block, two more validity reports, which change nothing.

`a` proposes the next epoch's first block in view `2w + 3` too, on the last block's
own `Cert1`, with no timeout evidence, behind its `Cert2` (`OpensEpochJustified`).
The re-vote request is the outgoing epoch's message for the view and the proposal
the incoming one's, so the leader may send both (`ProtocolHistory.proposeOnce`),
and no epoch change costs a view timer (`two_blocks`).

Times: one unit a step. GST is zero, `Δ = 4`, `δ = 0` and `τ = 33`.

`BlockValid` is taken as a hypothesis, being opaque; `decides` also takes
`CollisionFree`.
-/

@[expose] public section

namespace NewProtocolImpl
namespace LongEpochWitness

open NewProtocol History
open FourNodes (hdr)
open Kit (received received_mono upTo_H upTo_self getElem_H tr_step sent_iff epoch_ext view_inj number_inj)
open FiveNodes (a b c nw d third leader C members_quorum third_honest d_faulty quorum_a anchorB certOf lag
  lag_cases lag_member lag_a lag_outside)

/-! ## Blocks and the schedule -/

/-- Two blocks to an epoch. -/
def cfg : Config where
  anchorBlock := anchorB
  anchorCert := certOf anchorB
  decideBuffer := 20
  epochHeight := 2

theorem cfg_coherent : ConfigCoherent cfg where
  anchorCertView := rfl
  anchorBlockEpoch := rfl
  anchorCertBlock := rfl
  anchorCertBlockNumber := rfl
  anchorCertEpoch := rfl

/-- Block `u + 1`, at view `u + 1`, in epoch `u / 2 + 1`, on the one before. -/
def blk : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨1⟩, certOf anchorB, none, ⟨0⟩⟩
  | u + 1 => ⟨hdr (u + 2), ⟨u + 2⟩, ⟨(u + 1) / 2 + 1⟩, certOf (blk u), none, ⟨0⟩⟩

/-- The parent of `blk u`, and the block at height `u`. -/
def parentOf : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

/-- The `Cert2` over `blk u`. -/
def C2 (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨u + 1⟩⟩

/-- The re-vote request `a` sends in view `2w + 3`, once it can lock on epoch `w + 1`'s last block. -/
def R (w : Nat) : RevoteRequest := ⟨certOf (blk (2 * w + 1)), ⟨2 * w + 3⟩, none⟩

theorem blk_view (u : Nat) : (blk u).viewNumber = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_number (u : Nat) : (blk u).blockHeader.blockNumber = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_epoch (u : Nat) : (blk u).epoch = ⟨u / 2 + 1⟩ := by cases u <;> rfl

theorem blk_header (u : Nat) : (blk u).blockHeader = hdr (u + 1) := by cases u <;> rfl

theorem blk_parent (u : Nat) : (blk u).parentCert = certOf (parentOf u) := by cases u <;> rfl

theorem blk_evidence (u : Nat) : (blk u).timeoutEvidence = none := by cases u <;> rfl

theorem cert_view (u : Nat) : (certOf (blk u)).view = ⟨u + 1⟩ := blk_view u

theorem cert_number (u : Nat) : (certOf (blk u)).data.blockNumber = ⟨u + 1⟩ := blk_number u

theorem cert_epoch (u : Nat) : (certOf (blk u)).data.epoch = ⟨u / 2 + 1⟩ := blk_epoch u

theorem R_epoch (w : Nat) : (R w).cert.data.epoch = ⟨w + 1⟩ := by
  show (certOf (blk (2 * w + 1))).data.epoch = _
  rw [cert_epoch]; congr 1; omega

theorem parentOf_view (u : Nat) : (parentOf u).viewNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_view u

theorem parentOf_number (u : Nat) : (parentOf u).blockHeader.blockNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_number u

theorem par (u : Nat) : u % 2 = 0 ∨ u % 2 = 1 := by omega

/-- With two blocks to an epoch, block `u + 1` is in epoch `u / 2 + 1`. -/
theorem epochOf_blk (u : Nat) : epochOf ⟨u + 1⟩ 2 = ⟨u / 2 + 1⟩ := by
  simp only [epochOf_eq]
  rw [ite_eq_right (by decide), ite_eq_right (by show ¬ u + 1 = 0; omega)]
  by_cases h : (u + 1) % 2 = 0
  · rw [ite_eq_left (by show (u + 1) % 2 = 0; exact h)]; congr 1; omega
  · rw [ite_eq_right (by show ¬ (u + 1) % 2 = 0; exact h)]; congr 1; omega

theorem blk_wellFormed (u : Nat) : ProposalWellFormed cfg (blk u) := by
  refine ⟨?_, Or.inl ⟨blk_evidence u, ?_⟩, ?_, ?_⟩
  · rw [blk_parent, blk_view]; show (parentOf u).viewNumber.toNat < u + 1
    rw [parentOf_view]; show u < u + 1; omega
  · rw [blk_parent, blk_view]; show (parentOf u).viewNumber + 1 = _
    rw [parentOf_view]; rfl
  · rw [blk_epoch, blk_number]; exact (epochOf_blk u).symm
  · rw [blk_parent, blk_number]; show (parentOf u).blockHeader.blockNumber + 1 = _
    rw [parentOf_number]; rfl

/-- The last blocks of the epochs are the odd ones. -/
theorem blk_last (u : Nat) : IsLastBlock (blk u).blockHeader.blockNumber cfg.epochHeight ↔ u % 2 = 1 := by
  rw [blk_number]
  rw [isLastBlock_iff]; show (u + 1 ≠ 0 ∧ 2 ≠ 0 ∧ (u + 1) % 2 = 0) ↔ _
  constructor
  · intro h; omega
  · intro h; omega

/-- The first blocks of the epochs after the first are the even ones. -/
theorem enters_iff (u : Nat) : EntersEpoch cfg (blk u) ↔ u ≠ 0 ∧ u % 2 = 0 := by
  show IsLastBlock ((blk u).blockHeader.blockNumber - 1) 2 ↔ _
  rw [blk_number]
  rw [isLastBlock_iff]; show (u + 1 - 1 ≠ 0 ∧ 2 ≠ 0 ∧ (u + 1 - 1) % 2 = 0) ↔ _
  constructor
  · intro h; omega
  · intro h; omega

theorem blk_enters_zero : ¬ EntersEpoch cfg (blk 0) := fun h => h.1 rfl

theorem blk_safe (u : Nat) : SafeParent (blk u) := fun _ h => by rw [blk_evidence] at h; cases h

/--
What the environment hands node `k` in the steps of block `u + 1`. A node outside
the epoch's committee gets the proposal without a share, and no payload. Only the
epoch's last block brings the epoch change and the re-vote request.
-/
def phase (k : PubKey) (u : Nat) : Nat → Input
  | 0 => .headerBuilt ⟨u + 1⟩ (blockHash (parentOf u)) (hdr (u + 1))
  | 1 => if lag k (u / 2) = 0 then .proposal a (blk u) (some ⟨⟨u + 1⟩, (blk u).payloadCommit⟩) else .proposal a (blk u) none
  | 2 => .blockValidated ⟨u + 1⟩ (blockHash (blk u))
  | 3 => if lag k (u / 2) = 0 then .blockReconstructed ⟨u + 1⟩ (blk u).payloadCommit
      else .blockValidated ⟨u + 1⟩ (blockHash (blk u))
  | 4 => .certificate1 (certOf (blk u))
  | 5 => .certificate2 (C2 u)
  | 6 => if u % 2 = 1 then .epochChange (certOf (blk u)) (C2 u) (blk u)
      else .blockValidated ⟨u + 1⟩ (blockHash (blk u))
  | _ => if u % 2 = 1 then .revote a (R (u / 2)) else .blockValidated ⟨u + 1⟩ (blockHash (blk u))

/-- Eight steps to a block. -/
def input (k : PubKey) (n : Nat) : Input := phase k (n / 8) (n % 8)

/-- Node `k` running the machine on its schedule, and its history after `n` steps. -/
local notation "tr" => Kit.tr cfg leader input

local notation "H" => Kit.H cfg leader input

theorem input_at (k : PubKey) (u r : Nat) (hr : r < 8) : input k (8 * u + r) = phase k u r := by
  simp only [input]
  rw [show (8 * u + r) / 8 = u by omega, show (8 * u + r) % 8 = r by omega]

theorem steps (n : Nat) : ∃ u r, r < 8 ∧ n = 8 * u + r :=
  ⟨n / 8, n % 8, Nat.mod_lt _ (by omega), by omega⟩

theorem r8 (r : Nat) (hr : r < 8) :
    r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 ∨ r = 5 ∨ r = 6 ∨ r = 7 := by omega

/-! ## What the nodes receive -/

section Inputs

variable {k : PubKey}

theorem input_phase {n : Nat} {i : Input} (hi : input k n = i) :
    ∃ u r, r < 8 ∧ n = 8 * u + r ∧ phase k u r = i := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  exact ⟨u, r, hr, rfl, by rw [← input_at k u r hr]; exact hi⟩

theorem input_header {n : Nat} {v : ViewNumber} {h : BlockHash} {x : BlockHeader}
    (hi : input k n = .headerBuilt v h x) :
    ∃ u, n = 8 * u ∧ v = ⟨u + 1⟩ ∧ h = blockHash (parentOf u) ∧ x = hdr (u + 1) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal {n : Nat} {s : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal s p (some vid)) :
    ∃ u, n = 8 * u + 1 ∧ lag k (u / 2) = 0 ∧ s = a ∧ p = blk u ∧ vid = ⟨⟨u + 1⟩, (blk u).payloadCommit⟩ := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal_none {n : Nat} {s : PubKey} {p : Proposal} (hi : input k n = .proposal s p none) :
    ∃ u, n = 8 * u + 1 ∧ s = a ∧ p = blk u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.symm⟩

/-- Every proposal a node receives is `a`'s block for its round, with a share or without. -/
theorem input_proposal_any {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ u, n = 8 * u + 1 ∧ s = a ∧ p = blk u := by
  cases share with
  | some vid => obtain ⟨u, h1, -, h2, h3, -⟩ := input_proposal hi; exact ⟨u, h1, h2, h3⟩
  | none => exact input_proposal_none hi

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    ∃ u, n = 8 * u + 4 ∧ x = certOf (blk u) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, he.symm⟩

/-- A payload arrives at step three, at the members of the epoch's committee only. -/
theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = 8 * u + 3 ∧ lag k (u / 2) = 0 ∧ v = ⟨u + 1⟩ ∧ pc = (blk u).payloadCommit := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, hl, he.1.symm, he.2.symm⟩

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, n = 8 * u + 5 ∧ x = C2 u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, he.symm⟩

/-- An epoch change arrives at step six of an epoch's last block. -/
theorem input_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : input k n = .epochChange c1 c2 p) :
    ∃ u, n = 8 * u + 6 ∧ u % 2 = 1 ∧ c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u, rfl, hp, he.1.symm, he.2.1.symm, he.2.2.symm⟩

/-- A re-vote request arrives at the last step of an epoch. -/
theorem input_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hi : input k n = .revote s r) :
    ∃ w, n = 16 * w + 15 ∧ s = a ∧ r = R w := by
  obtain ⟨u, r', hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp] at he
  all_goals exact ⟨u / 2, by omega, he.1.symm, he.2.symm⟩

/-- Nothing here times out. -/
theorem input_quiet {n : Nat} : (∀ v, input k n ≠ .timeout v) ∧ (∀ v, input k n ≠ .timeoutOneHonest v)
    ∧ (∀ tc, input k n ≠ .timeoutCertificate tc) := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  rw [input_at k u r hr]
  rcases lag_cases k (u / 2) with hl | hl <;> rcases par u with hp | hp <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl, hp]

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

theorem recv_at {n : Nat} (u r : Nat) (hr : r < 8) (h : 8 * u + r < n) :
    (H k n).Received (phase k u r) :=
  received.mpr ⟨8 * u + r, h, input_at k u r hr⟩

theorem recv_revote {n : Nat} (w : Nat) (h : 16 * w + 15 < n) : (H k n).Received (.revote a (R w)) := by
  have hr := recv_at (k := k) (2 * w + 1) 7 (by omega) (by omega : 8 * (2 * w + 1) + 7 < n)
  have hp : phase k (2 * w + 1) 7 = .revote a (R w) := by
    show (if (2 * w + 1) % 2 = 1 then _ else _) = _
    rw [ite_eq_left (by omega), show (2 * w + 1) / 2 = w by omega]
  rwa [hp] at hr

theorem no_tc {n : Nat} {tc : TimeoutCert} : ¬ (H k n).Received (.timeoutCertificate tc) := fun hr => by
  obtain ⟨j, -, hj⟩ := received.mp hr; exact input_quiet.2.2 tc hj

theorem received_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hr : (H k n).Received (.revote s r)) :
    ∃ w, s = a ∧ r = R w ∧ 16 * w + 15 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨w, rfl, rfl, rfl⟩ := input_revote hji
  exact ⟨w, rfl, rfl, hj⟩

theorem hasProposal {n : Nat} {x : Block} (hb : (H k n).HasProposal cfg x) :
    x = anchorB ∨ ∃ u, x = blk u ∧ 8 * u + 1 < n := by
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
    obtain ⟨u, rfl, -, -, -, rfl⟩ := input_epochChange hji
    exact Or.inr ⟨u, rfl, by omega⟩

/-- Every node holds the proposal of block `u + 1` from its second step on: with a share, or without. -/
theorem hasProposal_of {n : Nat} (u : Nat) (h : 8 * u + 1 < n) : (H k n).HasProposal cfg (blk u) := by
  have hr := recv_at (k := k) u 1 (by omega) h
  simp only [phase] at hr
  rcases lag_cases k (u / 2) with hl | hl
  · rw [hl, ite_eq_left rfl] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)
  · rw [hl, ite_eq_right (by decide)] at hr; exact Or.inr (Or.inl ⟨a, _, hr⟩)

theorem hasParent_of {n : Nat} (u : Nat) (h : 8 * u < n + 6) : (H k n).HasProposal cfg (parentOf u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u => exact hasProposal_of u (by omega)

/-- Which held proposal a height names. -/
theorem block_of_number {n x : Nat} {y : Block} (hb : (H k n).HasProposal cfg y)
    (hv : y.blockHeader.blockNumber = ⟨x + 1⟩) : y = blk x ∧ 8 * x + 1 < n := by
  rcases hasProposal hb with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [blk_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

theorem hasCert1 {n : Nat} {x : Cert1} (hc : (H k n).HasCert1 cfg x) :
    x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ 8 * u + 4 < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert1 hji
    exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, -, rfl, -, -⟩ := input_epochChange hji
    exact Or.inr ⟨u, rfl, by omega⟩

theorem hasCert1_of {n : Nat} (u : Nat) (h : 8 * u + 4 < n) : (H k n).HasCert1 cfg (certOf (blk u)) :=
  Or.inr (Or.inl (recv_at u 4 (by omega) h))

/-- Which held certificate a height names. -/
theorem cert_of_number {n x : Nat} {y : Cert1} (hc : (H k n).HasCert1 cfg y)
    (hv : y.data.blockNumber = ⟨x + 1⟩) : y = certOf (blk x) ∧ 8 * x + 4 < n := by
  rcases hasCert1 hc with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [cert_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

theorem hasCert2 {n : Nat} {x : Cert2} (hc : (H k n).HasCert2 x) : ∃ u, x = C2 u ∧ 8 * u + 5 < n := by
  rcases hc with hr | ⟨c1, p, hr⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert2 hji
    exact ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, -, -, rfl, -⟩ := input_epochChange hji
    exact ⟨u, rfl, by omega⟩

theorem hasCert2_of {n : Nat} (u : Nat) (h : 8 * u + 5 < n) : (H k n).HasCert2 (C2 u) :=
  Or.inl (recv_at u 5 (by omega) h)

/-- The payload of a block arrives at step three, at the members of its epoch's committee. -/
theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis
      ∨ ∃ u, v = ⟨u + 1⟩ ∧ pc = (blk u).payloadCommit ∧ lag k (u / 2) = 0 ∧ 8 * u + 3 < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, hl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hl, hj⟩

theorem payload_of {n : Nat} (u : Nat) (hl : lag k (u / 2) = 0) (h : 8 * u + 3 < n) :
    (H k n).HasPayload cfg (blk u).viewNumber (blk u).payloadCommit := by
  have hr := recv_at (k := k) u 3 (by omega) h
  simp only [phase, hl, ite_eq_left] at hr
  rw [blk_view]; exact Or.inr hr

theorem epochChange_wellFormed (u : Nat) (hp : u % 2 = 1) :
    EpochChangeWellFormed cfg (certOf (blk u)) (C2 u) (blk u) :=
  ⟨by rw [blk_view]; exact Nat.le_refl _, rfl, rfl, rfl, blk_wellFormed u, (blk_last u).mpr hp⟩

theorem tookEpochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (ht : (H k n).TookEpochChange cfg c1 c2 p) :
    ∃ u, u % 2 = 1 ∧ c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u ∧ 8 * u + 6 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp ht.1
  obtain ⟨u, rfl, hp, rfl, rfl, rfl⟩ := input_epochChange hji
  exact ⟨u, hp, rfl, rfl, rfl, hj⟩

theorem tookEpochChange_of {n : Nat} (u : Nat) (hp : u % 2 = 1) (h : 8 * u + 6 < n) :
    (H k n).TookEpochChange cfg (certOf (blk u)) (C2 u) (blk u) := by
  have hr := recv_at (k := k) u 6 (by omega) h
  simp only [phase, hp, ite_true] at hr
  exact ⟨hr, epochChange_wellFormed u hp⟩

/--
What a node can lock on after `n` steps: genesis, and each block from the step
after its `Cert1`, which comes after its payload, or, an epoch's last block, from
the epoch change.
-/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x ↔ x = certOf anchorB
      ∨ ∃ u, x = certOf (blk u) ∧ ((lag k (u / 2) = 0 ∧ 8 * u + 4 < n) ∨ (u % 2 = 1 ∧ 8 * u + 6 < n)) := by
  constructor
  · rintro (rfl | ⟨hc, y, hy, ⟨-, hcd⟩, hp⟩ | ⟨c2, p, ht⟩)
    · exact Or.inl rfl
    · rcases hasCert1 hc with rfl | ⟨u, rfl, hcu⟩
      · exact Or.inl rfl
      · have hyn : y.blockHeader.blockNumber = ⟨u + 1⟩ := by
          have := congrArg Vote1Data.blockNumber hcd
          rw [← cert_number u]; exact this.symm
        obtain ⟨rfl, -⟩ := block_of_number hy hyn
        rcases hasPayload hp with hg | ⟨u', hv, -, hl, -⟩
        · rw [blk_view] at hg; exact absurd (view_inj hg) (by omega)
        · rw [blk_view] at hv
          obtain rfl : u = u' := by have := view_inj hv; omega
          exact Or.inr ⟨u, rfl, Or.inl ⟨hl, hcu⟩⟩
    · obtain ⟨u, hp, rfl, -, -, hlt⟩ := tookEpochChange ht
      exact Or.inr ⟨u, rfl, Or.inr ⟨hp, hlt⟩⟩
  · rintro (rfl | ⟨u, rfl, ⟨hl, hlt⟩ | ⟨hp, hlt⟩⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hasCert1_of u hlt, blk u, hasProposal_of u (by omega),
        ⟨Nat.le_refl _, rfl⟩, payload_of u hl (by omega)⟩)
    · exact Or.inr (Or.inr ⟨_, _, tookEpochChange_of u hp hlt⟩)

/-! ### The view and the epoch after `n` steps -/

/-- The view every node is in after `n` steps: it moves on with each `Cert1`. -/
def vAt (n : Nat) : Nat := (n + 3) / 8 + 1

theorem vAt_eq (n : Nat) : vAt n = (n + 3) / 8 + 1 := rfl

/-- The epoch every node is in after `n` steps: the next one from each epoch change on. -/
def eAt (n : Nat) : Nat := (n + 1) / 16 + 1

theorem eAt_eq (n : Nat) : eAt n = (n + 1) / 16 + 1 := rfl

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ vAt n := by
  rw [vAt_eq]
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, -⟩ | ⟨c1, c2, p, ht, rfl⟩
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ _; omega
    · show (certOf (blk u)).view.toNat + 1 ≤ _
      rw [cert_view]; show u + 1 + 1 ≤ _; omega
  · exact absurd htc no_tc
  · obtain ⟨u, -, -, rfl, -, hlt⟩ := tookEpochChange ht
    show u + 1 + 1 ≤ _; omega

theorem inView (n : Nat) : (H k n).InView cfg ⟨vAt n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  by_cases h5 : n < 5
  · rw [show vAt n = 1 by rw [vAt_eq]; omega]
    exact Or.inl ⟨certOf anchorB, Or.inl rfl, rfl⟩
  · obtain ⟨u, hu, hn⟩ : ∃ u, vAt n = u + 2 ∧ 8 * u + 4 < n :=
      ⟨(n - 5) / 8, by rw [vAt_eq]; omega, by omega⟩
    rw [hu]
    exact Or.inl ⟨certOf (blk u), hasCert1_of u hn, by rw [cert_view]; rfl⟩

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨vAt n⟩ :=
  Kit.inView_unique hv (inView n)

theorem epochGround {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e.toNat ≤ eAt n := by
  rw [eAt_eq]
  rcases he with rfl | ⟨c1, c2, p, ht, rfl⟩ | ⟨tc, htc, -⟩ | ⟨x, hx, -, rfl⟩
  · show 1 ≤ _; omega
  · obtain ⟨u, hp, -, rfl, -, hlt⟩ := tookEpochChange ht
    show (certOf (blk u)).data.epoch.toNat + 1 ≤ _
    rw [cert_epoch]; show u / 2 + 1 + 1 ≤ _; omega
  · exact absurd htc no_tc
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 1 ≤ _; omega
    · rw [cert_epoch]; show u / 2 + 1 ≤ _; omega

theorem c2_epoch (u : Nat) : (⟨u / 2 + 2⟩ : EpochNumber) = (C2 u).data.epoch + 1 := by
  show _ = (certOf (blk u)).data.epoch + 1
  rw [cert_epoch]; rfl

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨eAt n⟩ := by
  refine ⟨?_, fun e he => epochGround he⟩
  by_cases h : n < 15
  · rw [show eAt n = 1 by rw [eAt_eq]; omega]; exact Or.inl rfl
  · obtain ⟨w, hw⟩ : ∃ w, eAt n = w + 2 := ⟨(n + 1) / 16 - 1, by rw [eAt_eq]; omega⟩
    have ht := tookEpochChange_of (k := k) (2 * w + 1) (by omega) (by rw [eAt_eq] at hw; omega : 8 * (2 * w + 1) + 6 < n)
    rw [hw, show w + 2 = (2 * w + 1) / 2 + 2 by omega]
    exact Or.inr (Or.inl ⟨_, _, _, ht, c2_epoch _⟩)

theorem notBehind {n : Nat} {e : EpochNumber} (h : eAt n ≤ e.toNat) : NotBehind cfg (H k n) e :=
  Kit.notBehind_of (inEpoch n) h

theorem notBehind_le {n : Nat} {e : EpochNumber} (h : NotBehind cfg (H k n) e) : eAt n ≤ e.toNat :=
  Kit.le_of_notBehind (inEpoch n) h

end Holds

/-! ## What the honest nodes send -/

section Sends

theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H k (n + 1)) o :=
  Kit.settled input k n o

/-- No node ever times out: no timer fires and no one-honest indication arrives. -/
theorem no_timeoutVote {k : PubKey} {j : Nat} {vote : TimeoutVote} :
    Output.send (.timeoutVote vote) ∉ (tr k j).output := fun hx => by
  rw [tr_step] at hx
  rcases step_mem hx with hx | ⟨o, out', -, -, ha⟩
  · obtain ⟨v, -, (⟨hi, -⟩ | ⟨hi, -⟩)⟩ := mem_timeoutAnswer hx
    · exact input_quiet.1 v hi
    · exact input_quiet.2.1 v hi
  · exact act_no_timeoutVote ha

theorem not_timedOut {k : PubKey} {n : Nat} {v : ViewNumber} : ¬ (H k n).TimedOut v := fun ⟨_, hs, _⟩ => by
  obtain ⟨j, -, hj⟩ := sent_iff.mp hs
  exact no_timeoutVote hj

theorem not_pastView {k : PubKey} {n : Nat} {v : ViewNumber} : ¬ (H k n).PastView v := by
  rintro (ht | ⟨tc, htc, -⟩)
  · exact not_timedOut ht
  · exact no_tc htc

variable (hv : ∀ b, BlockValid b)

include hv in
theorem protocol (k : PubKey) (n : Nat) : ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H k n) :=
  Kit.protocol input hv k n

/-- With no timeout certificate anywhere, a justified certificate carries no evidence. -/
theorem justified_none {k : PubKey} {n : Nat} {x : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg (H k n) x ev) : ev = none := by
  cases ev with
  | none => rfl
  | some tc =>
    obtain ⟨-, m, hr, -⟩ := hj
    rw [upTo_H] at hr
    exact absurd hr no_tc

include hv in
/--
`a`'s only re-vote requests are for each epoch's last block, in the view after it,
from the step it can lock on the block until the epoch change moves it on.
-/
theorem sent_revote {k : PubKey} {j : Nat} {r : RevoteRequest} (hr : Output.send (.revote r) ∈ (tr k j).output) :
    k = a ∧ ∃ w, r = R w ∧ 16 * w + 12 ≤ j := by
  have hj := (protocol hv k (j + 1)).revoteJustified j r ⟨_, (getElem_H j), hr⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨-, hpath, hlast⟩ := hj.wellFormed
  have hcur := notBehind_le hj.current
  have hev := justified_none hj.justified
  have hl : (H a (j + 1)).Lockable cfg r.cert := hj.lockable hev
  rcases lockable_iff.mp hl with he | ⟨u, he, hlt⟩
  · rw [he] at hlast; exact absurd hlast.1 (by decide)
  · rw [he] at hlast
    have hp : u % 2 = 1 := (blk_last u).mp hlast
    obtain ⟨w, rfl⟩ : ∃ w, u = 2 * w + 1 := ⟨u / 2, by omega⟩
    refine ⟨w, ?_, by omega⟩
    rcases hpath with ⟨-, hv1⟩ | ⟨tc, hte, -⟩
    · obtain ⟨cert, view, ev⟩ := r
      simp only at he hev hv1
      subst he hev
      rw [cert_view] at hv1
      rw [← hv1]; rfl
    · rw [hev] at hte; cases hte

include hv in
/-- A proposal is for the view of a header the node was handed, after it was handed it. -/
theorem proposal_header {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    ∃ u, 8 * u ≤ j ∧ p.viewNumber = ⟨u + 1⟩ ∧ p.blockHeader = hdr (u + 1) := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨i, hi, hii⟩ := received.mp hj.built
  obtain ⟨u, rfl, hvw, -, hx⟩ := input_header hii
  exact ⟨u, by omega, hvw, hx⟩

include hv in
/-- Every proposal a node sends is `a`'s block for its view, sent once the header arrived. -/
theorem sent_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) : k = a ∧ ∃ u, p = blk u ∧ 8 * u ≤ j := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain rfl : k = a := FiveNodes.leader_eq hj.leads
  refine ⟨rfl, ?_⟩
  obtain ⟨u, hju, hvw, hhdr⟩ := proposal_header hv hp
  refine ⟨u, ?_, hju⟩
  obtain ⟨-, -, hep, hnum⟩ := hj.wellFormed
  have hte := justified_none hj.justified
  have hpc := hasCert1_of_certJustified hj.justified
  have hid : p.identity = ⟨0⟩ := by
    rw [tr_step] at hp
    rcases step_mem hp with h0 | ⟨o, out', -, -, ha⟩
    · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer h0; cases hx
    · obtain ⟨_, v, -, hmem, -, -⟩ := act_proposal ha
      exact (mem_proposalCandidates hmem).1
  have hnum' : p.parentCert.data.blockNumber.toNat + 1 = u + 1 := by
    have := congrArg BlockNumber.toNat hnum; rw [hhdr] at this; exact this
  have hepu : p.epoch = ⟨u / 2 + 1⟩ := by rw [hep, hhdr]; exact epochOf_blk u
  have hpcP : p.parentCert = certOf (parentOf u) := by
    cases u with
    | zero =>
      rcases hasCert1 hpc with h | ⟨x, h, -⟩
      · exact h
      · rw [h, cert_number] at hnum'; have : x + 1 + 1 = 0 + 1 := hnum'; omega
    | succ u =>
      exact (cert_of_number hpc (BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = u + 1 by omega))).1
  obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
  simp only at hvw hhdr hid hepu hpcP hte ⊢
  rw [hvw, hhdr, hid, hepu, hpcP, hte]
  cases u <;> rfl

/-- `a` may propose block `u + 1` from the step its header arrives until the block's `Cert1` does. -/
theorem blk_justified (u r : Nat) (h1 : 1 ≤ r) (h3 : r ≤ 3) :
    ProposalJustified cfg leader a (H a (8 * u + r)) (blk u) := by
  refine ⟨⟨rfl, blk_wellFormed u, ?_, ⟨parentOf u, hasParent_of u (by omega),
      by rw [blk_parent]; exact Nat.le_refl _, by rw [blk_parent]; rfl⟩, ?_, blk_safe u, notBehind ?_,
      ⟨_, (inView _).1, ?_⟩⟩, ?_⟩
  · show CertJustified cfg _ (blk u).parentCert (blk u).timeoutEvidence
    rw [blk_parent, blk_evidence]
    cases u with
    | zero => exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
    | succ u =>
      exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inr ⟨u, rfl, Or.inl ⟨lag_a _, by omega⟩⟩))
  · intro hen
    cases u with
    | zero => exact absurd hen blk_enters_zero
    | succ u =>
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), ?_, rfl⟩
      rw [blk_view]; show u + 1 < u + 1 + 1; omega
  · rw [blk_epoch]; show eAt (8 * u + r) ≤ u / 2 + 1; rw [eAt_eq]; omega
  · rw [blk_view]; show u + 1 ≤ vAt (8 * u + r); rw [vAt_eq]; omega
  · rw [blk_view, blk_header, blk_parent]
    exact recv_at u 0 (by omega) (by omega)

include hv in
/-- `a` proposes each block by the step its header arrives in. -/
theorem proposes (u : Nat) : ∃ j, j ≤ 8 * u ∧ Output.send (.proposal (blk u)) ∈ (tr a j).output := by
  refine Classical.byContradiction fun hneg => settled a (8 * u) (.propose (blk u).epoch ⟨u + 1⟩)
    ⟨Or.inl ⟨blk u, blk_justified u 1 (Nat.le_refl _) (by omega), blk_view u, rfl⟩, ?_, not_timedOut, ?_⟩
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv
      obtain rfl : x = u := by have := view_inj hpv; omega
      exact hneg ⟨j, by omega, hjp⟩
    · -- `a`'s re-vote request in that view is of the epoch before.
      obtain ⟨j, -, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, w, rfl, -⟩ := sent_revote hv hjr
      have h1 : 2 * w + 3 = u + 1 := view_inj hrv
      have h2 : w + 1 = u / 2 + 1 :=
        congrArg EpochNumber.toNat ((R_epoch w).symm.trans (hre.trans (blk_epoch u)))
      omega
  · have := inView (k := a) (8 * u + 1)
    rwa [show vAt (8 * u + 1) = u + 1 by rw [vAt_eq]; omega] at this

include hv in
/-- `a` asks for a re-vote on each epoch's last block as soon as it can lock on it. -/
theorem revotes (w : Nat) : ∃ j, j ≤ 16 * w + 12 ∧ Output.send (.revote (R w)) ∈ (tr a j).output := by
  have hlk : (H a (16 * w + 12 + 1)).Lockable cfg (R w).cert :=
    lockable_iff.mpr (Or.inr ⟨2 * w + 1, rfl, Or.inl ⟨lag_a _, by omega⟩⟩)
  refine Classical.byContradiction fun hneg => settled a (16 * w + 12) (.propose ⟨w + 1⟩ ⟨2 * w + 3⟩)
    ⟨Or.inr ⟨R w, ⟨rfl, ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_⟩, Liveness.buildable_of_lockable hlk, fun _ => hlk,
      (fun _ h => by cases h), notBehind ?_, ?_⟩, rfl,
      R_epoch w⟩, ?_, not_timedOut, ?_⟩
  · show (certOf (blk (2 * w + 1))).view < ⟨2 * w + 3⟩
    rw [cert_view]; show 2 * w + 1 + 1 < 2 * w + 3; omega
  · show (certOf (blk (2 * w + 1))).view + 1 = ⟨2 * w + 3⟩; rw [cert_view]; rfl
  · exact (blk_last (2 * w + 1)).mpr (by omega)
  · rw [R_epoch]; show eAt (16 * w + 13) ≤ w + 1; rw [eAt_eq]; omega
  · exact ⟨_, (inView _).1, by show 2 * w + 3 ≤ vAt (16 * w + 13); rw [vAt_eq]; omega⟩
  · rintro (⟨p, hs, hpv, hpe⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv; rw [blk_epoch] at hpe
      have := view_inj hpv; have := congrArg EpochNumber.toNat hpe; simp only at this; omega
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_revote hv hjr
      obtain rfl : x = w := by have : 2 * x + 3 = 2 * w + 3 := view_inj hrv; omega
      exact hneg ⟨j, by omega, hjr⟩
  · have := inView (k := a) (16 * w + 13)
    rwa [show vAt (16 * w + 13) = 2 * w + 3 by rw [vAt_eq]; omega] at this

include hv in
/-- Every vote1 a node sends is for a block of its committee, after its proposal arrived. -/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1} (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(certOf (blk u)).data, ⟨u + 1⟩, k⟩ ∧ lag k (u / 2) = 0 ∧ 8 * u + 1 ≤ j := by
  have hpr := protocol hv k (j + 1)
  obtain ⟨hsig, -⟩ := hpr.vote1Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  obtain ⟨-, ⟨s', p, vid, hrec, -, hfor, -⟩ | ⟨s', r, hrec, -, -, hnb⟩⟩ :=
    hpr.vote1Leader j vote ⟨_, (getElem_H j), hx⟩ trivial
  · rw [upTo_self] at hrec
    obtain ⟨i, hi, hii⟩ := received.mp hrec
    obtain ⟨u, rfl, hl, -, rfl, -⟩ := input_proposal hii
    refine ⟨u, ?_, hl, by omega⟩
    obtain ⟨d, v, sg⟩ := vote
    obtain ⟨hv', hd⟩ := hfor
    simp only at hsig hv' hd
    rw [hsig, hv', hd, blk_view]; rfl
  · -- A re-vote request arrives after the epoch change, which left its block's epoch behind.
    rw [upTo_self] at hrec hnb
    obtain ⟨w, -, rfl, hlt⟩ := received_revote hrec
    have h1 : eAt (j + 1) ≤ w + 1 := by
      have := notBehind_le hnb
      rwa [R_epoch] at this
    rw [eAt_eq] at h1
    omega

include hv in
/-- Every member of an epoch's committee votes1 for its blocks by the step the validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) (hl : lag k (u / 2) = 0) : ∃ j, j ≤ 8 * u + 2
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨u + 1⟩, k⟩) ∈ (tr k j).output := by
  have hopen : OpensEpochJustified cfg (H k (8 * u + 3)) (blk u) := fun he => by
    cases u with
    | zero => exact absurd he blk_enters_zero
    | succ u =>
      exact ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), by rw [blk_view]; show u + 1 < u + 1 + 1; omega, rfl⟩
  refine Classical.byContradiction fun hneg => settled k (8 * u + 2) (.vote1 (blk u))
    ⟨⟨a, _, by
        have := recv_at (k := k) (n := 8 * u + 3) u 1 (by omega) (by omega)
        simp only [phase, hl, ite_true] at this; exact this, rfl, by rw [blk_view], rfl⟩, blk_wellFormed u, ?_, ?_,
      blk_safe u, hopen, notBehind ?_, not_timedOut, fun vote hs _ hvv => ?_, ?_⟩
  · rw [blk_view]; exact recv_at u 2 (by omega) (by omega)
  · -- The parent is genesis, ends the epoch before, or is the epoch's first block, whose payload the node holds.
    cases u with
    | zero => exact Or.inl rfl
    | succ u =>
      rcases par u with hp | hp
      · rw [show (u + 1) / 2 = u / 2 by omega] at hl
        exact Or.inr (Or.inr ⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; exact Nat.le_refl _,
          by rw [blk_parent]; rfl, payload_of u hl (by omega)⟩)
      · exact Or.inr (Or.inl ((enters_iff _).mpr ⟨by omega, by omega⟩))
  · rw [blk_epoch]; show eAt (8 * u + 3) ≤ u / 2 + 1; rw [eAt_eq]; omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -, -⟩ := sent_vote1 hv hjv
    rw [blk_view] at hvv
    obtain rfl : u' = u := by have := view_inj hvv; omega
    exact hneg ⟨j, by omega, hjv⟩
  · rw [blk_view]
    have := inView (k := k) (8 * u + 3)
    rwa [show vAt (8 * u + 3) = u + 1 by rw [vAt_eq]; omega] at this

include hv in
/-- Every vote2 a node sends is on a block, after its payload arrived. -/
theorem sent_vote2 {k : PubKey} {j : Nat} {vote : Vote2} (hx : Output.send (.vote2 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(C2 u).data, ⟨u + 1⟩, k⟩ ∧ 8 * u + 4 ≤ j := by
  obtain ⟨hsig, hgen, x, y, hc, hb, hcert, hpay, hvc, hdc⟩ :=
    (protocol hv k (j + 1)).vote2Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  rw [upTo_self] at hc hb hpay
  obtain ⟨d, v, sg⟩ := vote
  simp only at hsig hvc hdc hgen
  rcases hasPayload hpay with hg | ⟨u, hyv, -, -, h⟩
  · -- At genesis: the anchor, whose certificate is at genesis too.
    exfalso
    rcases hasProposal hb with rfl | ⟨u, rfl, -⟩
    · have hx0 : x.data.blockNumber = ⟨0⟩ := by rw [hcert.2]; rfl
      rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩
      · rw [hvc] at hgen; exact Nat.lt_irrefl _ hgen
      · rw [cert_number] at hx0; exact absurd (number_inj hx0) (by omega)
    · rw [blk_view] at hg; exact absurd (view_inj hg) (by omega)
  · obtain rfl : y = blk u := by
      rcases hasProposal hb with rfl | ⟨u', rfl, -⟩
      · exact absurd (view_inj hyv) (by omega)
      · rw [blk_view] at hyv
        obtain rfl : u' = u := by have := view_inj hyv; omega
        rfl
    obtain ⟨rfl, hcu⟩ := cert_of_number (x := u) hc (by rw [hcert.2]; exact blk_number u)
    refine ⟨u, ?_, by omega⟩
    rw [hsig, hvc, hdc, cert_view]; rfl

include hv in
/-- Every member of an epoch's committee votes2 for its blocks by the step the `Cert1` arrives in. -/
theorem votes2 (k : PubKey) (u : Nat) (hl : lag k (u / 2) = 0) : ∃ j, j ≤ 8 * u + 4
    ∧ Output.send (.vote2 ⟨(C2 u).data, ⟨u + 1⟩, k⟩) ∈ (tr k j).output := by
  refine Classical.byContradiction fun hneg => settled k (8 * u + 4) (.vote2 (certOf (blk u)))
    ⟨⟨blk u, hasCert1_of u (by omega), hasProposal_of u (by omega), ⟨Nat.le_refl _, rfl⟩,
      payload_of u hl (by omega)⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, not_pastView, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -⟩ := sent_vote2 hv hjv
    rw [cert_view] at hvv
    obtain rfl : u' = u := by have := view_inj hvv; omega
    exact hneg ⟨j, by omega, hjv⟩
  · obtain ⟨x, rfl, hlt⟩ := hasCert2 hc2
    rw [cert_view] at hv2
    have : x + 1 = u + 1 := view_inj hv2
    omega
  · refine Kit.afterFloor_of input hv (by rw [cert_view]; show 0 < u + 1; omega) fun x hx => ?_
    rw [cert_view]
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
    · show 0 < u + 1 + 20; omega
    · rw [blk_view]; show y + 1 < u + 1 + 20; omega

end Sends

/-! ## The network -/

section Net

variable (hv : ∀ b, BlockValid b)

include hv in
theorem backed1 (u : Nat) : Cert1Backed (C := C) (fun k _ => tr k) (certOf (blk u)) := by
  refine ⟨fun k => C.members ⟨u / 2 + 1⟩ k ∧ C.honest ⟨u / 2 + 1⟩ k,
    by rw [cert_epoch]; exact members_quorum _, fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes1 hv k u (lag_member hk.1)
  rw [cert_view]
  exact ⟨j, hj⟩

include hv in
theorem backed2 (u : Nat) : Cert2Backed (C := C) (fun k _ => tr k) (C2 u) := by
  refine ⟨fun k => C.members ⟨u / 2 + 1⟩ k ∧ C.honest ⟨u / 2 + 1⟩ k,
    by show C.Quorum (certOf (blk u)).data.epoch _; rw [cert_epoch]; exact members_quorum _,
    fun k hk _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes2 hv k u (lag_member hk.1)
  exact ⟨j, hj⟩

/-- The honest nodes running the machine on their schedules. -/
def net : TimedNetwork cfg leader C where
  honestQuorum := members_quorum
  trace k _ := tr k
  safe k _ n := .of_every (protocol hv k n).toSafeHistory
  cert1Genuine k _ n x hc := by
    rcases Input.mem_cert1.mp hc with hin | ⟨c2, p, hin⟩ | ⟨s, p, vid, hin, rfl⟩
    · obtain ⟨u, -, rfl⟩ := input_cert1 hin
      exact Or.inr (backed1 hv u)
    · obtain ⟨u, -, -, rfl, -, -⟩ := input_epochChange hin
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
    · obtain ⟨u, -, -, -, rfl, -⟩ := input_epochChange hin
      exact backed2 hv u
  timeoutCertGenuine k _ n tc hc := by
    rcases Input.mem_timeoutCert.mp hc with hin | ⟨s, p, vid, hin, hte⟩ | ⟨s, r, hin, hte⟩
    · exact absurd hin (input_quiet.2.2 tc)
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      rw [blk_evidence] at hte; cases hte
    · obtain ⟨w, -, -, rfl⟩ := input_revote hin
      cases hte
  revoteGenuine k _ n s r hin := by
    obtain ⟨w, -, -, rfl⟩ := input_revote hin
    exact backed1 hv (2 * w + 1)
  time _ _ n := n
  timeMono _ _ n := Nat.le_succ n
  protocol k _ n := .of_every (protocol hv k n)
  timeoutCertCausal k _ n tc hin := absurd hin (input_quiet.2.2 tc)
  oneHonestCausal k _ n v hin := absurd hin (input_quiet.2.1 v)
  authentic k _ n l msg hin _ _ := by
    cases hi : (tr k n).input <;> rw [hi] at hin <;> simp only [Input.sentBy, reduceCtorEq] at hin
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨u, rfl, rfl, rfl⟩ := input_proposal_any hi
      obtain ⟨j, hj, hjp⟩ := proposes hv u
      refine ⟨j, hjp, Nat.lt_of_le_of_lt hj ?_⟩
      omega
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨w, rfl, rfl, rfl⟩ := input_revote hi
      obtain ⟨j, hj, hjr⟩ := revotes hv w
      exact ⟨j, hjr, Nat.lt_of_le_of_lt hj (by omega)⟩

theorem net_time {k : PubKey} {hk : C.Honest k} {n : Nat} : (net hv).time k hk n = n := rfl

theorem by_at {k : PubKey} {hk : C.Honest k} {T m : Nat} {P : History → Prop}
    (hm : 0 < m → (m - 1) ≤ T) (hp : P (H k m)) : (net hv).By k hk T P :=
  Kit.by_at (tm := id) (fun _ _ => rfl) (fun _ _ _ => rfl) Nat.le_succ hm hp

theorem sentBy {k : PubKey} {hk : C.Honest k} {t : Nat} {m : Message} (hs : (net hv).SentByTime k hk t m) :
    ∃ j, j ≤ t ∧ Output.send m ∈ (tr k j).output :=
  Kit.sentBy (N := net hv) (tm := id) (fun _ _ => rfl) (fun _ _ _ => rfl) hs

include hv in
/-- A header for every view `a` is ready to propose in arrives within `Δ`. -/
theorem header_arrives {k : PubKey} {hk : C.Honest k} {n : Nat} {p : Proposal}
    (hready : ProposalReady cfg leader k (H k (n + 1)) p) :
    (net hv).By k hk (max n 0 + 4) fun hist =>
      ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
  obtain ⟨hlead, ⟨-, hnext, -, hnum⟩, hj, -, -, -, -, -⟩ := hready
  obtain rfl := FiveNodes.leader_eq hlead
  have hmax := Nat.le_max_left n 0
  have hte := justified_none hj
  have hpv : p.parentCert.view + 1 = p.viewNumber := by
    rcases hnext with ⟨-, h⟩ | ⟨tc, h, -⟩
    · exact h
    · rw [hte] at h; cases h
  have hl : (H a (n + 1)).Buildable cfg p.parentCert := by
    have : CertJustified cfg (H a (n + 1)) p.parentCert p.timeoutEvidence := hj
    rw [hte] at this; exact this
  -- Every header arrives within `Δ` of the node's being able to use it; `m` is when.
  have hdone : ∀ m, m ≤ n + 1 ∨ (m - 1) ≤ n + 4 → ∀ hdr', hdr'.blockNumber = p.blockHeader.blockNumber →
      (H a m).Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') →
      (net hv).By a hk (max n 0 + 4) fun hist =>
        ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
          ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
    intro m hm hdr' h1 h2
    rcases hm with hm | hm
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨hdr', h1, received_mono hm h2⟩
    · exact by_at hv (m := m) (fun _ => by omega) ⟨hdr', h1, h2⟩
  rcases hasCert1 (Liveness.hasCert1_of_buildable hl) with hpc | ⟨x, hpc, hlt⟩
  · -- On genesis: view one, whose header comes first.
    refine hdone 1 (Or.inr (by omega)) (hdr 1)
      (by rw [← hnum, hpc]; rfl) ?_
    rw [← hpv, hpc]; exact recv_at 0 0 (by omega) (by omega)
  · -- On a block: the view after it, once `a` holds the block's certificate.
    refine hdone (8 * (x + 1) + 1) (Or.inr (by show 8 * (x + 1) ≤ _; omega)) (hdr (x + 2))
      (by rw [← hnum, hpc, cert_number]; rfl) ?_
    rw [← hpv, hpc, cert_view]
    exact recv_at (x + 1) 0 (by omega) (by omega)

/-- The timer for a view fires within `τ` of the node's entering it, unless the node has moved on. -/
theorem timer_fires {k : PubKey} (n : Nat) {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v) :
    ∃ m, n < m ∧ m ≤ n + 33 ∧ (input k m = .timeout v ∨ ∃ w, v < w ∧ (H k (m + 1)).InView cfg w) := by
  have hv' := inView_eq hin
  subst hv'
  refine ⟨n + 8, by omega, by omega, Or.inr ⟨_, ?_, inView _⟩⟩
  show vAt (n + 1) < vAt (n + 8 + 1)
  rw [vAt_eq, vAt_eq]; omega

include hv in
/-- Every delivery within `Δ = 4`, after GST `0`, with view timer `τ = 33`. -/
theorem sync : Synchrony (net hv) 0 4 33 where
  proposal l hl n p hsend _ k hk hmem _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left n 0
    obtain ⟨rfl, u, rfl, hu⟩ := sent_proposal hv hsend
    rw [blk_epoch] at hmem
    have hl := lag_member hmem
    refine by_at hv (m := 8 * u + 2) (fun _ => by omega)
      ⟨⟨⟨u + 1⟩, (blk u).payloadCommit⟩, ⟨by rw [blk_view], rfl⟩, ?_⟩
    have := recv_at (k := k) (n := 8 * u + 2) u 1 (by omega) (by omega)
    simp only [phase, hl, ite_true] at this; exact this
  revote l hl n r hsend _ k hk _ _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left n 0
    obtain ⟨rfl, w, rfl, hw⟩ := sent_revote hv hsend
    exact by_at hv (m := 16 * w + 16) (fun _ => by omega) (recv_revote w (by omega))
  cert1 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, heq, -, hx⟩ := sent_vote1 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    have hmax := Nat.le_max_left t 0
    refine by_at hv (m := 8 * x + 5) (fun _ => by omega) ?_
    rw [show (⟨(certOf (blk x)).data, ⟨x + 1⟩⟩ : Cert1) = certOf (blk x) by rw [← cert_view x]]
    exact recv_at x 4 (by omega) (by omega)
  cert2 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, heq, hx⟩ := sent_vote2 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    have hmax := Nat.le_max_left t 0
    exact by_at hv (m := 8 * x + 6) (fun _ => by omega)
      (recv_at (k := k) x 5 (by omega) (by omega))
  cert2Spread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc
    have hmax := Nat.le_max_left n 0
    exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasCert2_of x hx)
  certSpread c k hk n hc k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left n 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasCert1_of x hx)
  lockSpread c k hk n hc k' hk' hmem _ := by
    -- A member of the block's committee can lock on it from the step after its `Cert1`.
    simp only [net_time hv]
    have hmax := Nat.le_max_left n 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · rw [cert_epoch] at hmem
      have hl := lag_member hmem
      by_cases hn : 8 * x + 4 < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
          (lockable_iff.mpr (Or.inr ⟨x, rfl, Or.inl ⟨hl, hn⟩⟩))
      · exact by_at hv (m := 8 * x + 5) (fun _ => by omega)
          (lockable_iff.mpr (Or.inr ⟨x, rfl, Or.inl ⟨hl, by omega⟩⟩))
  blockSpread c b _ k hk n hcb k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left n 0
    refine by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) ?_
    rcases hasProposal hcb.2 with rfl | ⟨y, rfl, hy⟩
    · exact Or.inl rfl
    · exact hasProposal_of y hy
  timeoutCert e q v t hq hvotes k hk _ := by
    obtain ⟨_hka, L, hsent⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, -, hj⟩ := sentBy hv hsent
    exact absurd hj no_timeoutVote
  timeoutCertSpread tc k hk n hin := absurd hin no_tc
  timeoutLockSpread tc k hk n hin := absurd hin no_tc
  epochChange c2 b hcm hlast k hk n hbc k' hk' _ := by
    obtain ⟨hb, hc2⟩ := hbc
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc2
    have hbn : b.blockHeader.blockNumber = ⟨x + 1⟩ := by
      rw [← cert_number x]; exact (congrArg Vote2Data.blockNumber hcm.2).symm
    obtain ⟨rfl, -⟩ := block_of_number hb hbn
    have hp := (blk_last x).mp hlast
    have hmax := Nat.le_max_left n 0
    by_cases hn : 8 * x + 6 < n + 1
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨_, tookEpochChange_of x hp hn⟩
    · exact by_at hv (m := 8 * x + 7) (fun _ => by omega)
        ⟨_, tookEpochChange_of x hp (by omega)⟩
  proposalValid _ _ _ p _ _ := hv p
  validatedSound _ _ _ _ _ _ b _ := hv b
  validated k hk n s p vid hin _ _ := by
    simp only [net_time hv]
    obtain ⟨u, rfl, -, -, rfl, -⟩ := input_proposal hin
    have hmax := Nat.le_max_left (8 * u + 1) 0
    refine by_at hv (m := 8 * u + 3) (fun _ => by omega) ?_
    rw [blk_view]
    exact recv_at (k := k) u 2 (by omega) (by omega)
  header k hk n p _ _ hready := by simp only [net_time hv]; exact header_arrives hv hready
  timeUnbounded _ _ T := ⟨8 * (T + 1), by show T < 8 * (T + 1); omega⟩
  timerNotEarly k hk n m v _ _ _ hinm := absurd hinm (input_quiet.1 v)
  timerFires k hk n v hin _ := timer_fires n hin

theorem rotation : LeaderRotation C leader := fun _ v => ⟨v, Nat.le_refl _, a, rfl, Or.inl rfl, Or.inl rfl⟩

include hv in
/-- **The liveness premises can be met together, with two blocks to an epoch.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv) 0 4 33 ∧ Prompt (net hv) 0
      ∧ 8 * 4 + 3 * 0 < 33 ∧ LeaderRotation C leader :=
  ⟨cfg_coherent, sync hv, prompt_of_machine _ 0 (fun k _ => ⟨input k, rfl⟩)
    (fun _ hk => Kit.steady_of_uniform (fun _ _ _ h => h) hk), by decide, rotation⟩

include hv in
/--
And the epochs take two blocks: every honest node takes every epoch change; the
node outside an epoch's committee holds the epoch's first block with both its
certificates but never can lock on it; and `a` asks for a re-vote on each epoch's
last block and proposes the next epoch's first block in the same view, on the last
block's own `Cert1` and with no timeout evidence.
-/
theorem two_blocks :
    (∀ k w, C.Honest k →
      (H k (16 * w + 15)).TookEpochChange cfg (certOf (blk (2 * w + 1))) (C2 (2 * w + 1)) (blk (2 * w + 1)))
      ∧ (∀ w, ¬ C.members ⟨w + 1⟩ (third ⟨w + 2⟩) ∧ C.honest ⟨w + 2⟩ (third ⟨w + 2⟩))
      ∧ (∀ w, (H (third ⟨w + 2⟩) (16 * w + 6)).HasProposal cfg (blk (2 * w))
        ∧ (H (third ⟨w + 2⟩) (16 * w + 6)).HasCert1 cfg (certOf (blk (2 * w)))
        ∧ (H (third ⟨w + 2⟩) (16 * w + 6)).HasCert2 (C2 (2 * w)))
      ∧ (∀ w n, ¬ (H (third ⟨w + 2⟩) n).Lockable cfg (certOf (blk (2 * w))))
      ∧ (∀ w, (∃ j, Output.send (.revote (R w)) ∈ (tr a j).output)
        ∧ (∃ j, Output.send (.proposal (blk (2 * w + 2))) ∈ (tr a j).output)
        ∧ (R w).view = (blk (2 * w + 2)).viewNumber ∧ EntersEpoch cfg (blk (2 * w + 2))
        ∧ (blk (2 * w + 2)).parentCert = certOf (blk (2 * w + 1))
        ∧ (blk (2 * w + 2)).timeoutEvidence = none) :=
  ⟨fun _ w _ => tookEpochChange_of (2 * w + 1) (by omega) (by omega),
    fun w => ⟨lag_outside (by simp [lag]), third_honest _⟩,
    fun w => ⟨hasProposal_of (2 * w) (by omega), hasCert1_of (2 * w) (by omega), hasCert2_of (2 * w) (by omega)⟩,
    fun w n h => by
      have hl : lag (third ⟨w + 2⟩) w = 2 := by simp [lag]
      rcases lockable_iff.mp h with h0 | ⟨y, hy, hlt⟩
      · have := congrArg (fun x : Cert1 => x.view) h0
        rw [cert_view] at this
        exact absurd (view_inj this) (by omega)
      · have := congrArg (fun x : Cert1 => x.view) hy
        rw [cert_view, cert_view] at this
        obtain rfl : y = 2 * w := by have := view_inj this; omega
        rw [show 2 * w / 2 = w by omega, hl] at hlt
        omega,
    fun w => ⟨let ⟨j, _, hj⟩ := revotes hv w; ⟨j, hj⟩, let ⟨j, _, hj⟩ := proposes hv (2 * w + 2); ⟨j, hj⟩,
      by rw [blk_view]; rfl, (enters_iff _).mpr ⟨by omega, by omega⟩, blk_parent _, blk_evidence _⟩⟩

include hv in
/-- So every honest node keeps deciding, `a`'s re-votes going unanswered at every boundary. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : C.Honest k) :
    (net hv).DecidesAfter k hk t ∧ ∀ w, ∃ j, Output.send (.revote (R w)) ∈ (tr a j).output := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met hv
  exact ⟨(Liveness.chainGrows (cfg := cfg) (leader := leader) (C := C) (net hv) 0 4 0 33
    hc hcf hs hp hb hr).2 t k hk (Or.inl (Kit.steady_of_uniform (fun _ _ _ h => h) hk)),
    fun w => let ⟨j, _, hj⟩ := revotes hv w; ⟨j, hj⟩⟩

end Net

end LongEpochWitness
end NewProtocolImpl
