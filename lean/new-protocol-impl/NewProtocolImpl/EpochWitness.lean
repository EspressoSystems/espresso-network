module

public import NewProtocolImpl.WitnessKit
public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.FiveNodes

/-!
# A network the liveness premises hold in, across epochs

In a run with a single epoch the premises about epoch changes and re-votes hold
only because they never apply. Here every block ends an epoch, and the committee
changes with it.

The nodes and committees are `NewProtocolImpl.FiveNodes`'s: `a`, `b`, `c` and `nw`
honest, `d` faulty and silent, `c` outside the even epochs' committees and `nw`
outside the odd ones'. `a` leads every view. Epoch height is one, so block `u + 1`,
at view `u + 1`, is the only block of epoch `u + 1`. A node receives eight inputs to
a block:

* the header, the proposal with the node's share, the validity report, the
  `Cert1`, the payload and the `Cert2`. A node outside the block's committee gets
  the proposal without a share, and no payload (`phase`);
* the epoch change, with the block's `Cert1` and `Cert2`. A node outside the
  committee locks on the block through it, and votes for the next block without
  the payload of its parent (`ParentReady`);
* the re-vote request `a` sent once it could lock on the block, before the `Cert2`
  could arrive, in view `u + 2`. It arrives after the epoch change, so nobody votes
  on it (`NotBehind`).

`a` then proposes the next epoch's first block in that same view, on the block's
own `Cert1` and behind its `Cert2` (`OpensEpochJustified`). The re-vote request is
the outgoing epoch's message for the view and the proposal the incoming one's, so
the leader may send both (`ProtocolHistory.proposeOnce`), and no epoch change costs
a view timer (`epochs_change`).

Times: one unit a step. GST is zero, `Δ = 4`, `δ = 0` and `τ = 33`.

`BlockValid` is taken as a hypothesis, being opaque; `decides` also takes
`CollisionFree`.
-/

@[expose] public section

namespace NewProtocolImpl
namespace EpochWitness

open NewProtocol History
open FourNodes (hdr)
open Kit (received received_mono upTo_H upTo_self getElem_H tr_step sent_iff epoch_ext view_inj number_inj
  lockLE_antisymm)
open FiveNodes (a b c nw d third leader C members_quorum d_faulty quorum_a anchorB certOf cfg cfg_coherent
  last_block epochOf_one_height lag lag_cases lag_member lag_a lag_outside)

/-! ## Blocks and the schedule -/

/-- Block `u + 1`, at view `u + 1`, the only block of epoch `u + 1`, on the one before. -/
def blk : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨1⟩, certOf anchorB, none, ⟨0⟩⟩
  | u + 1 => ⟨hdr (u + 2), ⟨u + 2⟩, ⟨u + 2⟩, certOf (blk u), none, ⟨0⟩⟩

/-- The parent of `blk u`. -/
def parentOf : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

/-- The `Cert2` over `blk u`. -/
def C2 (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨u + 1⟩⟩

/-- The re-vote request `a` sends in view `u + 2`, once it can lock on `blk u`. -/
def R (u : Nat) : RevoteRequest := ⟨certOf (blk u), ⟨u + 2⟩, none⟩

theorem blk_view (u : Nat) : (blk u).viewNumber = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_number (u : Nat) : (blk u).blockHeader.blockNumber = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_epoch (u : Nat) : (blk u).epoch = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_header (u : Nat) : (blk u).blockHeader = hdr (u + 1) := by cases u <;> rfl

theorem blk_parent (u : Nat) : (blk u).parentCert = certOf (parentOf u) := by cases u <;> rfl

theorem blk_evidence (u : Nat) : (blk u).timeoutEvidence = none := by cases u <;> rfl

theorem cert_view (u : Nat) : (certOf (blk u)).view = ⟨u + 1⟩ := blk_view u

theorem cert_number (u : Nat) : (certOf (blk u)).data.blockNumber = ⟨u + 1⟩ := blk_number u

theorem cert_epoch (u : Nat) : (certOf (blk u)).data.epoch = ⟨u + 1⟩ := blk_epoch u

theorem parentOf_view (u : Nat) : (parentOf u).viewNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_view u

theorem parentOf_number (u : Nat) : (parentOf u).blockHeader.blockNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_number u

theorem blk_wellFormed (u : Nat) : ProposalWellFormed cfg (blk u) := by
  refine ⟨?_, Or.inl ⟨blk_evidence u, ?_⟩, ?_, ?_⟩
  · rw [blk_parent, blk_view]; show (parentOf u).viewNumber.toNat < u + 1
    rw [parentOf_view]; show u < u + 1; omega
  · rw [blk_parent, blk_view]; show (parentOf u).viewNumber + 1 = _
    rw [parentOf_view]; rfl
  · rw [blk_epoch, blk_number]; exact (epochOf_one_height (by omega)).symm
  · rw [blk_parent, blk_number]; show (parentOf u).blockHeader.blockNumber + 1 = _
    rw [parentOf_number]; rfl

theorem blk_enters (u : Nat) : EntersEpoch cfg (blk (u + 1)) := by
  show IsLastBlock ((blk (u + 1)).blockHeader.blockNumber - 1) 1
  rw [blk_number]
  exact last_block (n := u + 1) (by omega)

theorem blk_enters_zero : ¬ EntersEpoch cfg (blk 0) := fun h => h.1 rfl

theorem blk_safe (u : Nat) : SafeParent (blk u) := fun _ h => by rw [blk_evidence] at h; cases h

/--
What the environment hands node `k` in the steps of block `u + 1`. A node outside
the block's committee gets the proposal without a share, and no payload.
-/
def phase (k : PubKey) (u : Nat) : Nat → Input
  | 0 => .headerBuilt ⟨u + 1⟩ (blockHash (parentOf u)) (hdr (u + 1))
  | 1 => if lag k u = 0 then .proposal a (blk u) (some ⟨⟨u + 1⟩, (blk u).payloadCommit⟩) else .proposal a (blk u) none
  | 2 => .blockValidated ⟨u + 1⟩ (blockHash (blk u))
  | 3 => .certificate1 (certOf (blk u))
  | 4 => if lag k u = 0 then .blockReconstructed ⟨u + 1⟩ (blk u).payloadCommit
      else .blockValidated ⟨u + 1⟩ (blockHash (blk u))
  | 5 => .certificate2 (C2 u)
  | 6 => .epochChange (certOf (blk u)) (C2 u) (blk u)
  | _ => .revote a (R u)

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
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal {n : Nat} {s : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal s p (some vid)) :
    ∃ u, n = 8 * u + 1 ∧ lag k u = 0 ∧ s = a ∧ p = blk u ∧ vid = ⟨⟨u + 1⟩, (blk u).payloadCommit⟩ := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, hl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal_none {n : Nat} {s : PubKey} {p : Proposal} (hi : input k n = .proposal s p none) :
    ∃ u, n = 8 * u + 1 ∧ s = a ∧ p = blk u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.symm⟩

/-- Every proposal a node receives is `a`'s block for its round, with a share or without. -/
theorem input_proposal_any {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ u, n = 8 * u + 1 ∧ s = a ∧ p = blk u := by
  cases share with
  | some vid => obtain ⟨u, h1, -, h2, h3, -⟩ := input_proposal hi; exact ⟨u, h1, h2, h3⟩
  | none => exact input_proposal_none hi

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    ∃ u, n = 8 * u + 3 ∧ x = certOf (blk u) := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.symm⟩

/-- A payload arrives at step four, at the members of the block's committee only. -/
theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = 8 * u + 4 ∧ lag k u = 0 ∧ v = ⟨u + 1⟩ ∧ pc = (blk u).payloadCommit := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, hl, he.1.symm, he.2.symm⟩

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, n = 8 * u + 5 ∧ x = C2 u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.symm⟩

theorem input_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : input k n = .epochChange c1 c2 p) :
    ∃ u, n = 8 * u + 6 ∧ c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u := by
  obtain ⟨u, r, hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hi : input k n = .revote s r) :
    ∃ u, n = 8 * u + 7 ∧ s = a ∧ r = R u := by
  obtain ⟨u, r', hr, rfl, he⟩ := input_phase hi
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl] at he
  all_goals exact ⟨u, rfl, he.1.symm, he.2.symm⟩

/-- Nothing here times out. -/
theorem input_quiet {n : Nat} : (∀ v, input k n ≠ .timeout v) ∧ (∀ v, input k n ≠ .timeoutOneHonest v)
    ∧ (∀ tc, input k n ≠ .timeoutCertificate tc) := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  rw [input_at k u r hr]
  rcases lag_cases k u with hl | hl <;>
    rcases r8 _ hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase, hl]

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

theorem recv_at {n : Nat} (u r : Nat) (hr : r < 8) (h : 8 * u + r < n) :
    (H k n).Received (phase k u r) :=
  received.mpr ⟨8 * u + r, h, input_at k u r hr⟩

theorem no_tc {n : Nat} {tc : TimeoutCert} : ¬ (H k n).Received (.timeoutCertificate tc) := fun hr => by
  obtain ⟨j, -, hj⟩ := received.mp hr; exact input_quiet.2.2 tc hj

theorem received_revote {n : Nat} {s : PubKey} {r : RevoteRequest} (hr : (H k n).Received (.revote s r)) :
    ∃ u, s = a ∧ r = R u ∧ 8 * u + 7 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨u, rfl, rfl, rfl⟩ := input_revote hji
  exact ⟨u, rfl, rfl, hj⟩

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
    obtain ⟨u, rfl, -, -, rfl⟩ := input_epochChange hji
    exact Or.inr ⟨u, rfl, by omega⟩

/-- Every node holds the proposal of block `u + 1` from its second step on: with a share, or without. -/
theorem hasProposal_of {n : Nat} (u : Nat) (h : 8 * u + 1 < n) : (H k n).HasProposal cfg (blk u) := by
  have hr := recv_at (k := k) u 1 (by omega) h
  simp only [phase] at hr
  rcases lag_cases k u with hl | hl
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
    x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ 8 * u + 3 < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert1 hji
    exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl, -, -⟩ := input_epochChange hji
    exact Or.inr ⟨u, rfl, by omega⟩

theorem hasCert1_of {n : Nat} (u : Nat) (h : 8 * u + 3 < n) : (H k n).HasCert1 cfg (certOf (blk u)) :=
  Or.inr (Or.inl (recv_at u 3 (by omega) h))

/-- Which held certificate a height names. -/
theorem cert_of_number {n x : Nat} {y : Cert1} (hc : (H k n).HasCert1 cfg y)
    (hv : y.data.blockNumber = ⟨x + 1⟩) : y = certOf (blk x) ∧ 8 * x + 3 < n := by
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
    obtain ⟨u, rfl, -, rfl, -⟩ := input_epochChange hji
    exact ⟨u, rfl, by omega⟩

theorem hasCert2_of {n : Nat} (u : Nat) (h : 8 * u + 5 < n) : (H k n).HasCert2 (C2 u) :=
  Or.inl (recv_at u 5 (by omega) h)

/-- The payload of a block arrives at step four, at the members of its committee. -/
theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis
      ∨ ∃ u, v = ⟨u + 1⟩ ∧ pc = (blk u).payloadCommit ∧ lag k u = 0 ∧ 8 * u + 4 < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, hl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hl, hj⟩

theorem payload_of {n : Nat} (u : Nat) (hl : lag k u = 0) (h : 8 * u + 4 < n) :
    (H k n).HasPayload cfg (blk u).viewNumber (blk u).payloadCommit := by
  have hr := recv_at (k := k) u 4 (by omega) h
  simp only [phase, hl, ite_eq_left] at hr
  rw [blk_view]; exact Or.inr hr

theorem epochChange_wellFormed (u : Nat) : EpochChangeWellFormed cfg (certOf (blk u)) (C2 u) (blk u) := by
  refine ⟨by rw [blk_view]; exact Nat.le_refl _, rfl, rfl, rfl, blk_wellFormed u, ?_⟩
  rw [blk_number]; exact last_block (by omega)

theorem tookEpochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (ht : (H k n).TookEpochChange cfg c1 c2 p) :
    ∃ u, c1 = certOf (blk u) ∧ c2 = C2 u ∧ p = blk u ∧ 8 * u + 6 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp ht.1
  obtain ⟨u, rfl, rfl, rfl, rfl⟩ := input_epochChange hji
  exact ⟨u, rfl, rfl, rfl, hj⟩

theorem tookEpochChange_of {n : Nat} (u : Nat) (h : 8 * u + 6 < n) :
    (H k n).TookEpochChange cfg (certOf (blk u)) (C2 u) (blk u) :=
  ⟨recv_at u 6 (by omega) h, epochChange_wellFormed u⟩

/--
What a node can lock on after `n` steps: genesis, and each block from the step
after its payload, or, outside the block's committee, from the epoch change.
-/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x ↔ x = certOf anchorB
      ∨ ∃ u, x = certOf (blk u) ∧ ((lag k u = 0 ∧ 8 * u + 4 < n) ∨ 8 * u + 6 < n) := by
  constructor
  · rintro (rfl | ⟨hc, y, hy, ⟨-, hcd⟩, hp⟩ | ⟨c2, p, ht⟩)
    · exact Or.inl rfl
    · rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩
      · exact Or.inl rfl
      · have hyn : y.blockHeader.blockNumber = ⟨u + 1⟩ := by
          have := congrArg Vote1Data.blockNumber hcd
          rw [← cert_number u]; exact this.symm
        obtain ⟨rfl, -⟩ := block_of_number hy hyn
        rcases hasPayload hp with hg | ⟨u', hv, -, hl, h⟩
        · rw [blk_view] at hg; exact absurd (view_inj hg) (by omega)
        · rw [blk_view] at hv
          obtain rfl : u = u' := by have := view_inj hv; omega
          exact Or.inr ⟨u, rfl, Or.inl ⟨hl, h⟩⟩
    · obtain ⟨u, rfl, -, -, hlt⟩ := tookEpochChange ht
      exact Or.inr ⟨u, rfl, Or.inr hlt⟩
  · rintro (rfl | ⟨u, rfl, ⟨hl, hlt⟩ | hlt⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hasCert1_of u (by omega), blk u, hasProposal_of u (by omega),
        ⟨Nat.le_refl _, rfl⟩, payload_of u hl hlt⟩)
    · exact Or.inr (Or.inr ⟨_, _, tookEpochChange_of u hlt⟩)

/-! ### The view, the epoch and the lock after `n` steps -/

/-- The view every node is in after `n` steps: it moves on with each `Cert1`. -/
def vAt (n : Nat) : Nat := if 4 ≤ n % 8 then n / 8 + 2 else n / 8 + 1

theorem vAt_facts (u r : Nat) (hr : r < 8) :
    (4 ≤ r → vAt (8 * u + r) = u + 2) ∧ (r < 4 → vAt (8 * u + r) = u + 1) := by
  simp only [vAt]
  rw [show (8 * u + r) / 8 = u by omega, show (8 * u + r) % 8 = r by omega]
  exact ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

/-- The epoch every node is in after `n` steps: the next one from the epoch change on. -/
def eAt (n : Nat) : Nat := if 7 ≤ n % 8 then n / 8 + 2 else n / 8 + 1

theorem eAt_facts (u r : Nat) (hr : r < 8) :
    (7 ≤ r → eAt (8 * u + r) = u + 2) ∧ (r < 7 → eAt (8 * u + r) = u + 1) := by
  simp only [eAt]
  rw [show (8 * u + r) / 8 = u by omega, show (8 * u + r) % 8 = r by omega]
  exact ⟨fun h => ite_eq_left h, fun h => ite_eq_right (by omega)⟩

theorem eAt_ge {n u : Nat} (h : 8 * u + 7 ≤ n) : u + 2 ≤ eAt n := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := eAt_facts u' r hr
  omega

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ vAt n := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := vAt_facts u' r hr
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, -⟩ | ⟨c1, c2, p, ht, rfl⟩
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ _; omega
    · show (certOf (blk u)).view.toNat + 1 ≤ _
      rw [cert_view]; show u + 1 + 1 ≤ _; omega
  · exact absurd htc no_tc
  · obtain ⟨u, -, rfl, -, hlt⟩ := tookEpochChange ht
    show u + 1 + 1 ≤ _; omega

theorem inView (n : Nat) : (H k n).InView cfg ⟨vAt n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  obtain ⟨u, r, hr, rfl⟩ := steps n
  obtain ⟨h4, h0⟩ := vAt_facts u r hr
  by_cases r4 : 4 ≤ r
  · rw [h4 r4]
    exact Or.inl ⟨certOf (blk u), hasCert1_of u (by omega), by rw [cert_view]; rfl⟩
  rw [h0 (by omega)]
  cases u with
  | zero => exact Or.inl ⟨certOf anchorB, Or.inl rfl, rfl⟩
  | succ u => exact Or.inl ⟨certOf (blk u), hasCert1_of u (by omega), by rw [cert_view]; rfl⟩

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨vAt n⟩ :=
  Kit.inView_unique hv (inView n)

theorem viewOf_eq (n : Nat) : viewOf cfg (H k n) = ⟨vAt n⟩ := viewOf_of_inView (inView n)

theorem epochGround {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e.toNat ≤ eAt n := by
  obtain ⟨u', r, hr, rfl⟩ := steps n
  have := eAt_facts u' r hr
  rcases he with rfl | ⟨c1, c2, p, ht, rfl⟩ | ⟨tc, htc, -⟩ | ⟨x, hx, -, rfl⟩
  · show 1 ≤ _; omega
  · obtain ⟨u, -, rfl, -, hlt⟩ := tookEpochChange ht
    show (certOf (blk u)).data.epoch.toNat + 1 ≤ _
    rw [cert_epoch]; show u + 1 + 1 ≤ _; omega
  · exact absurd htc no_tc
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 1 ≤ _; omega
    · rw [cert_epoch]; show u + 1 ≤ _; omega

theorem c2_epoch (u : Nat) : (⟨u + 2⟩ : EpochNumber) = (C2 u).data.epoch + 1 := by
  show _ = (certOf (blk u)).data.epoch + 1
  rw [cert_epoch]; rfl

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨eAt n⟩ := by
  refine ⟨?_, fun e he => epochGround he⟩
  obtain ⟨u, r, hr, rfl⟩ := steps n
  obtain ⟨h7, h0⟩ := eAt_facts u r hr
  by_cases r7 : 7 ≤ r
  · rw [h7 r7]; exact Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of u (by omega), c2_epoch u⟩)
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

/--
The lock after `n` steps: each block from the step after its payload, outside its
committee from the epoch change.
-/
def lockAt (k : PubKey) (n : Nat) : Cert1 :=
  if (lag k (n / 8) = 0 ∧ 5 ≤ n % 8) ∨ 7 ≤ n % 8 then certOf (blk (n / 8)) else certOf (parentOf (n / 8))

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
  · rw [cert_view] at hv; exact absurd (view_inj hv) (by omega)
  · rw [cert_view] at hv; exact absurd (view_inj hv) (by omega)
  · rw [cert_view, cert_view] at hv
    obtain rfl : j = m := by have := view_inj hv; omega
    rfl

theorem lockedOn_lockAt (n : Nat) : (H k n).LockedOn cfg (lockAt k n) := by
  obtain ⟨u, r, hr, rfl⟩ := steps n
  unfold lockAt
  rw [show (8 * u + r) / 8 = u by omega, show (8 * u + r) % 8 = r by omega]
  by_cases hcond : (lag k u = 0 ∧ 5 ≤ r) ∨ 7 ≤ r
  · rw [ite_eq_left hcond]
    refine ⟨lockable_iff.mpr (Or.inr ⟨u, rfl, ?_⟩), fun x hx => ?_⟩
    · rcases hcond with ⟨hka, h⟩ | h
      · exact Or.inl ⟨hka, by omega⟩
      · exact Or.inr (by omega)
    rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hlt⟩
    · exact lockLE_anchor u
    · exact lockLE_blk (by rcases hlt with ⟨-, h⟩ | h <;> omega)
  · rw [ite_eq_right hcond]
    refine ⟨?_, fun x hx => ?_⟩
    · cases u with
      | zero => exact lockable_iff.mpr (Or.inl rfl)
      | succ u => exact lockable_iff.mpr (Or.inr ⟨u, rfl, Or.inr (by omega)⟩)
    · rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hlt⟩
      · cases u with
        | zero => exact Or.inr ⟨rfl, Nat.le_refl _⟩
        | succ u => exact lockLE_anchor u
      · have hj : j < u := by
          by_cases hju : j = u
          · subst hju
            exact absurd (by rcases hlt with ⟨hka, h⟩ | h
                             · exact Or.inl ⟨hka, by omega⟩
                             · exact Or.inr (by omega)) hcond
          · rcases hlt with ⟨-, h⟩ | h <;> omega
        obtain ⟨u', rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
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
`a`'s only re-vote requests are for each block, in the view after it, from the step
it can lock on the block until the epoch change moves it on.
-/
theorem sent_revote {k : PubKey} {j : Nat} {r : RevoteRequest} (hr : Output.send (.revote r) ∈ (tr k j).output) :
    k = a ∧ ∃ u, r = R u ∧ 8 * u + 4 ≤ j := by
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
  · rw [he, cert_epoch] at hcur
    have hle : 8 * u + 4 ≤ j := by
      have := eAt_ge (n := j + 1) (u := u)
      rcases hlt with ⟨-, h⟩ | h <;> omega
    refine ⟨u, ?_, hle⟩
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
  have hepu : p.epoch = ⟨u + 1⟩ := by rw [hep, hhdr]; exact epochOf_one_height (by omega)
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
  have hvf := vAt_facts u r (by omega)
  have hef := eAt_facts u r (by omega)
  refine ⟨⟨rfl, blk_wellFormed u, ?_, ⟨parentOf u, hasParent_of u (by omega),
      by rw [blk_parent]; exact Nat.le_refl _, by rw [blk_parent]; rfl⟩, ?_, blk_safe u, notBehind ?_,
      ⟨_, (inView _).1, ?_⟩⟩, ?_⟩
  · show CertJustified cfg _ (blk u).parentCert (blk u).timeoutEvidence
    rw [blk_parent, blk_evidence]
    cases u with
    | zero => exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
    | succ u => exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inr ⟨u, rfl, Or.inr (by omega)⟩))
  · intro hen
    cases u with
    | zero => exact absurd hen blk_enters_zero
    | succ u =>
      refine ⟨⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
        C2 u, Or.inl <| hasCert2_of u (by omega), ?_, rfl⟩
      rw [blk_view]; show u + 1 < u + 1 + 1; omega
  · rw [blk_epoch]; show eAt (8 * u + r) ≤ u + 1; omega
  · rw [blk_view]; show u + 1 ≤ vAt (8 * u + r); omega
  · rw [blk_view, blk_header, blk_parent]
    exact recv_at u 0 (by omega) (by omega)

include hv in
/-- `a` proposes each block by the step its header arrives in. -/
theorem proposes (u : Nat) : ∃ j, j ≤ 8 * u ∧ Output.send (.proposal (blk u)) ∈ (tr a j).output := by
  have hvf := vAt_facts u 1 (by omega)
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
      obtain ⟨-, x, rfl, -⟩ := sent_revote hv hjr
      rw [blk_epoch] at hre
      have h1 : x + 2 = u + 1 := view_inj hrv
      have h2 : x + 1 = u + 1 := congrArg EpochNumber.toNat ((cert_epoch x).symm.trans hre)
      omega
  · have := inView (k := a) (8 * u + 1)
    rwa [show vAt (8 * u + 1) = u + 1 by omega] at this

include hv in
/-- `a` asks for a re-vote on each block as soon as it can lock on it. -/
theorem revotes (u : Nat) : ∃ j, j ≤ 8 * u + 4 ∧ Output.send (.revote (R u)) ∈ (tr a j).output := by
  have hvf := vAt_facts u 4 (by omega)
  have hef := eAt_facts u 4 (by omega)
  have hvf5 := vAt_facts u 5 (by omega)
  have hef5 := eAt_facts u 5 (by omega)
  have hlk : (H a (8 * u + 4 + 1)).Lockable cfg (R u).cert :=
    lockable_iff.mpr (Or.inr ⟨u, rfl, Or.inl ⟨lag_a u, by omega⟩⟩)
  refine Classical.byContradiction fun hneg => settled a (8 * u + 4) (.propose ⟨u + 1⟩ ⟨u + 2⟩)
    ⟨Or.inr ⟨R u, ⟨rfl, ⟨?_, Or.inl ⟨rfl, ?_⟩, ?_⟩, Liveness.buildable_of_lockable hlk, fun _ => hlk,
      (fun _ h => by cases h), notBehind ?_, ?_⟩, rfl,
      cert_epoch u⟩, ?_, not_timedOut, ?_⟩
  · show (certOf (blk u)).view < ⟨u + 2⟩; rw [cert_view]; show u + 1 < u + 2; omega
  · show (certOf (blk u)).view + 1 = ⟨u + 2⟩; rw [cert_view]; rfl
  · show IsLastBlock (certOf (blk u)).data.blockNumber 1; rw [cert_number]; exact last_block (by omega)
  · rw [show (R u).cert.data.epoch = ⟨u + 1⟩ from cert_epoch u]; show eAt (8 * u + 5) ≤ u + 1; omega
  · exact ⟨_, (inView _).1, by show u + 2 ≤ vAt (8 * u + 5); omega⟩
  · rintro (⟨p, hs, hpv, hpe⟩ | ⟨r, hs, hrv, hre⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv; rw [blk_epoch] at hpe
      have := view_inj hpv; have := congrArg EpochNumber.toNat hpe; simp only at this; omega
    · obtain ⟨j, hj, hjr⟩ := sent_iff.mp hs
      obtain ⟨-, x, rfl, -⟩ := sent_revote hv hjr
      obtain rfl : x = u := by have := view_inj hrv; omega
      exact hneg ⟨j, by omega, hjr⟩
  · have := inView (k := a) (8 * u + 5)
    rwa [show vAt (8 * u + 5) = u + 2 by omega] at this

include hv in
/-- Every vote1 a node sends is for a block, after its proposal arrived. -/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1} (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(certOf (blk u)).data, ⟨u + 1⟩, k⟩ ∧ lag k u = 0 ∧ 8 * u + 1 ≤ j := by
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
    obtain ⟨u, -, rfl, hlt⟩ := received_revote hrec
    have h1 : eAt (j + 1) ≤ u + 1 := by
      have := notBehind_le hnb
      rwa [show (R u).cert.data.epoch = ⟨u + 1⟩ from cert_epoch u] at this
    have := eAt_ge (n := j + 1) (u := u) (by omega)
    omega

include hv in
/-- Every member of a block's committee votes1 for it by the step its validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ 8 * u + 2
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨u + 1⟩, k⟩) ∈ (tr k j).output := by
  have hvf := vAt_facts u 3 (by omega)
  have hef := eAt_facts u 3 (by omega)
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
  · cases u with
    | zero => exact Or.inl rfl
    | succ u => exact Or.inr (Or.inl (blk_enters u))
  · rw [blk_epoch]; show eAt (8 * u + 3) ≤ u + 1; omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -, -⟩ := sent_vote1 hv hjv
    rw [blk_view] at hvv
    obtain rfl : u' = u := by have := view_inj hvv; omega
    exact hneg ⟨j, by omega, hjv⟩
  · rw [blk_view]
    have := inView (k := k) (8 * u + 3)
    rwa [show vAt (8 * u + 3) = u + 1 by omega] at this

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
    obtain ⟨rfl, -⟩ := cert_of_number (x := u) hc (by rw [hcert.2]; exact blk_number u)
    refine ⟨u, ?_, by omega⟩
    rw [hsig, hvc, hdc, cert_view]; rfl

include hv in
/-- Every member of a block's committee votes2 for it by the step its payload arrives in. -/
theorem votes2 (k : PubKey) (u : Nat) (hl : lag k u = 0) : ∃ j, j ≤ 8 * u + 4
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
    · exact absurd hin (input_quiet.2.2 tc)
    · obtain ⟨u, -, -, rfl⟩ := input_proposal_any hin
      rw [blk_evidence] at hte; cases hte
    · obtain ⟨u, -, -, rfl⟩ := input_revote hin
      cases hte
  revoteGenuine k _ n s r hin := by
    obtain ⟨u, -, -, rfl⟩ := input_revote hin
    exact backed1 hv u
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
      obtain ⟨u, rfl, rfl, rfl⟩ := input_revote hi
      obtain ⟨j, hj, hjr⟩ := revotes hv u
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
  obtain ⟨hlead, ⟨-, hnext, -, hnum⟩, hj, -, hop, -, -, -⟩ := hready
  obtain rfl := FiveNodes.leader_eq hlead
  have hmax := Nat.le_max_left n 0
  have hte := justified_none hj
  have hpv : p.parentCert.view + 1 = p.viewNumber := by
    rcases hnext with ⟨-, h⟩ | ⟨tc, h, -⟩
    · exact h
    · rw [hte] at h; cases h
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
  rcases hasCert1 (hasCert1_of_certJustified hj) with hpc | ⟨x, hpc, -⟩
  · -- On genesis: view one, whose header comes first.
    refine hdone 1 (Or.inr (by omega)) (hdr 1)
      (by rw [← hnum, hpc]; rfl) ?_
    rw [← hpv, hpc]; exact recv_at 0 0 (by omega) (by omega)
  · -- On a block: the view after it, which opens the next epoch, once `a` holds the block's `Cert2`.
    have hpn : p.blockHeader.blockNumber = ⟨x + 2⟩ := by rw [← hnum, hpc, cert_number]; rfl
    have hen : EntersEpoch cfg p := by
      show IsLastBlock (p.blockHeader.blockNumber - 1) 1
      rw [hpn]; exact last_block (n := x + 1) (by omega)
    obtain ⟨-, c2, hc2, -, hd⟩ := hop hen
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
    refine hdone (8 * (y + 1) + 1) (by
        by_cases hn : 8 * (y + 1) + 1 ≤ n + 1
        · exact Or.inl hn
        · exact Or.inr (by show (8 * (y + 1)) ≤ _; omega)) (hdr (y + 2))
      (by rw [← hnum, hpc, cert_number]; rfl) ?_
    rw [← hpv, hpc, cert_view]
    exact recv_at (y + 1) 0 (by omega) (by omega)

/-- The timer for a view fires within `τ` of the node's entering it, unless the node has moved on. -/
theorem timer_fires {k : PubKey} (n : Nat) {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v) :
    ∃ m, n < m ∧ m ≤ n + 33 ∧ (input k m = .timeout v ∨ ∃ w, v < w ∧ (H k (m + 1)).InView cfg w) := by
  have hv' := inView_eq hin
  subst hv'
  obtain ⟨u, r, hr, hn1⟩ := steps (n + 1)
  rw [hn1]
  obtain ⟨h4, h0⟩ := vAt_facts u r hr
  by_cases r4 : 4 ≤ r
  · -- The view after a block: the node moves on with the next block's `Cert1`.
    rw [h4 r4]
    refine ⟨8 * (u + 1) + 3, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
    · omega
    · rw [show 8 * (u + 1) + 3 + 1 = 8 * (u + 1) + 4 by omega, (vAt_facts (u + 1) 4 (by omega)).1 (by omega)]
      show u + 2 < u + 1 + 2; omega
  · -- The view of a block: the node moves on with its `Cert1`.
    rw [h0 (by omega)]
    refine ⟨8 * u + 3, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
    · omega
    · rw [show 8 * u + 3 + 1 = 8 * u + 4 by omega, (vAt_facts u 4 (by omega)).1 (by omega)]
      show u + 1 < u + 2; omega

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
    obtain ⟨rfl, u, rfl, hu⟩ := sent_revote hv hsend
    exact by_at hv (m := 8 * u + 8) (fun _ => by omega) (recv_at (k := k) u 7 (by omega) (by omega))
  cert1 q d v t hq hvotes k hk _ := by
    obtain ⟨_hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, heq, -, hx⟩ := sent_vote1 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    have hmax := Nat.le_max_left t 0
    refine by_at hv (m := 8 * x + 4) (fun _ => by omega) ?_
    rw [show (⟨(certOf (blk x)).data, ⟨x + 1⟩⟩ : Cert1) = certOf (blk x) by rw [← cert_view x]]
    exact recv_at x 3 (by omega) (by omega)
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
  lockSpread c k hk n hc k' hk' _ _ := by
    -- Every node can lock on a block by the epoch change, three steps after its `Cert1`.
    simp only [net_time hv]
    have hmax := Nat.le_max_left n 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · by_cases hn : 8 * x + 6 < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
          (lockable_iff.mpr (Or.inr ⟨x, rfl, Or.inr hn⟩))
      · exact by_at hv (m := 8 * x + 7) (fun _ => by omega)
          (lockable_iff.mpr (Or.inr ⟨x, rfl, Or.inr (by omega)⟩))
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
  timeoutOneHonest e q v t hq hvotes _ _ _ := by
    exfalso
    obtain ⟨k0, hq0, -, hk0⟩ := C.intersect _ q q hq hq
    obtain ⟨L, hs⟩ := hvotes k0 hq0 hk0
    obtain ⟨j, -, hj⟩ := sentBy hv hs
    rcases Kit.timeout_vote_input input hv hj with h | h
    · exact input_quiet.1 _ h
    · exact input_quiet.2.1 _ h
  timeoutCertForward tc k hk n hin := absurd (hin ▸ Trace.received_self _ n) no_tc
  timeoutCatchUp k hk v hrep := by
    exfalso
    obtain ⟨m, vote, -, -, hout⟩ := hrep 0
    rcases Kit.timeout_vote_input input hv hout with h | h
    · exact input_quiet.1 _ h
    · exact input_quiet.2.1 _ h
  timeoutLockSpread tc k hk n hin := absurd hin no_tc
  epochChange c2 b hcm _ k hk n hbc k' hk' _ := by
    obtain ⟨hb, hc2⟩ := hbc
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc2
    have hbn : b.blockHeader.blockNumber = ⟨x + 1⟩ := by
      rw [← cert_number x]; exact (congrArg Vote2Data.blockNumber hcm.2).symm
    obtain ⟨rfl, -⟩ := block_of_number hb hbn
    have hmax := Nat.le_max_left n 0
    by_cases hn : 8 * x + 6 < n + 1
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨_, tookEpochChange_of x hn⟩
    · exact by_at hv (m := 8 * x + 7) (fun _ => by omega)
        ⟨_, tookEpochChange_of x (by omega)⟩
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
/-- **The liveness premises can be met together, across epoch changes.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv) 0 4 33 ∧ Prompt (net hv) 0
      ∧ 8 * 4 + 3 * 0 < 33 ∧ LeaderRotation C leader :=
  ⟨cfg_coherent, sync hv, prompt_of_machine _ 0 (fun k _ => ⟨input k, rfl⟩)
    (fun _ hk => Kit.steady_of_uniform (fun _ _ _ h => h) hk), by decide, rotation⟩

include hv in
/--
And the epochs do change, without a view timer: the committee changes with every
epoch, `a` asks for a re-vote on every epoch's last block and proposes the next
epoch's first block in the same view, on the last block's own `Cert1` and with no
timeout evidence, and nobody times out.
-/
theorem epochs_change :
    (∀ u, ¬ C.members ⟨u + 1⟩ (third ⟨u + 2⟩) ∧ C.honest ⟨u + 2⟩ (third ⟨u + 2⟩))
      ∧ (∀ u, (∃ j, Output.send (.revote (R u)) ∈ (tr a j).output)
        ∧ (∃ j, Output.send (.proposal (blk (u + 1))) ∈ (tr a j).output)
        ∧ (R u).view = (blk (u + 1)).viewNumber ∧ EntersEpoch cfg (blk (u + 1))
        ∧ (blk (u + 1)).parentCert = certOf (blk u) ∧ (blk (u + 1)).timeoutEvidence = none)
      ∧ (∀ k j (vote : TimeoutVote), Output.send (.timeoutVote vote) ∉ (tr k j).output) :=
  ⟨fun u => ⟨lag_outside (by simp [lag]), FiveNodes.third_honest _⟩,
    fun u => ⟨let ⟨j, _, hj⟩ := revotes hv u; ⟨j, hj⟩, let ⟨j, _, hj⟩ := proposes hv (u + 1); ⟨j, hj⟩,
      by rw [blk_view]; rfl, blk_enters u, blk_parent (u + 1), blk_evidence _⟩,
    fun _ _ _ => no_timeoutVote⟩

include hv in
/-- So every honest node keeps deciding, and no epoch change costs a view timer. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : C.Honest k) :
    (net hv).DecidesAfter k hk t := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met hv
  exact (Liveness.chainGrows (cfg := cfg) (leader := leader) (C := C) (net hv) 0 4 0 33
    hc hcf hs hp hb hr).2 t k hk (Or.inl (Kit.steady_of_uniform (fun _ _ _ h => h) hk))

end Net

end EpochWitness
end NewProtocolImpl
