module

public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.WitnessKit

/-!
# A network whose timeout certificate names a lock some signers lack

In the other witnesses every honest node can lock on whatever the timeout
certificate names. Here one cannot, when it times out, and catches up afterwards.

The four-node committee (`FourNodes`): `a`, `b` and `c` honest, `d` faulty, any three a
quorum, in one epoch. `a` leads every view but view two, which `d` leads and
spends silent. `a` proposes the first block in view one, and its `Cert1` reaches
every node, but `c` does not get the block's payload in time, so it cannot lock
on it. View two times out: `a` and `b` time out locked on the first block, `c` on
the anchor. The timeout certificate names the latest of the three, the first
block's `Cert1`, which `c` could not lock on when it signed (`minority_lock`).

That is only possible before GST. After it, every member can lock on a
certificate within `Δ` of the first (`Synchrony.lockSpread`), so all would lock
before any timer fired. GST is at `c`'s timeout; `c` gets the payload within `Δ`
of it, and from view three `a` builds on the first block behind the timeout
certificate, one block a view.

Steps `0` to `8` run views one and two: the header, the proposal with the node's
share, the validity report, the `Cert1`, the payload (a second validity report at
`c`), the timer for view two, the timeout certificate, the one-honest indication,
and `c`'s payload (a third validity report at the others). From step `9` each view
takes six steps: a header, the proposal, the validity report, the `Cert1`, the
payload and the `Cert2`. Times are the steps, but for the timer: it fires `τ = 25`
after the nodes entered view two.

`BlockValid` is taken as a hypothesis, being opaque; `decides` also takes
`CollisionFree`.
-/

@[expose] public section

namespace NewProtocolImpl
namespace MinorityLockWitness

open NewProtocol History
open FourNodes (cfg certOf anchorB hdr cfg_coherent)
open Kit (getElem_H upTo_self upTo_H sent_iff tr_step)
open FourNodes (a b c d C members_quorum d_faulty quorum_a honest_quorum)

/-- `d` leads view two; `a` every other view. -/
def leader : EpochNumber → ViewNumber → Option PubKey :=
  fun _ v => if v = ⟨2⟩ then some d else some a

/-- The block at height `u + 1`: the first at view one, the rest from view three on. -/
def vOf (u : Nat) : Nat := if u = 0 then 1 else u + 2

/-- The certificate that view two timed out, naming the first block's `Cert1`. -/
def tc2 (b0 : Block) : TimeoutCert := ⟨⟨⟨0⟩, certOf b0⟩, ⟨2⟩⟩

/-- The blocks: the first on the anchor; the second on the first, behind the timeout certificate. -/
def blk : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨0⟩, certOf anchorB, none, ⟨0⟩⟩
  | 1 => ⟨hdr 2, ⟨3⟩, ⟨0⟩, certOf (blk 0), some (tc2 (blk 0)), ⟨0⟩⟩
  | u + 2 => ⟨hdr (u + 3), ⟨u + 4⟩, ⟨0⟩, certOf (blk (u + 1)), none, ⟨0⟩⟩

/-- The timeout certificate of view two. -/
abbrev TC : TimeoutCert := tc2 (blk 0)

/-- The parent of `blk u`. -/
def parentOf : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

theorem blk_parent (u : Nat) : (blk u).parentCert = certOf (parentOf u) := by
  rcases u with _ | _ | u <;> rfl

theorem blk_view (u : Nat) : (blk u).viewNumber = ⟨vOf u⟩ := by
  rcases u with _ | _ | u <;> rfl

theorem blk_number (u : Nat) : (blk u).blockHeader.blockNumber = ⟨u + 1⟩ := by
  rcases u with _ | _ | u <;> rfl

theorem blk_header (u : Nat) : (blk u).blockHeader = hdr (u + 1) := by
  rcases u with _ | _ | u <;> rfl

theorem blk_epoch (u : Nat) : (blk u).epoch = ⟨0⟩ := by
  rcases u with _ | _ | u <;> rfl

theorem certOf_view (u : Nat) : (certOf (blk u)).view = ⟨vOf u⟩ := blk_view u

theorem parentOf_number (u : Nat) : (parentOf u).blockHeader.blockNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_number u

theorem vOf_succ (u : Nat) : vOf (u + 1) = u + 3 := by simp [vOf]

theorem vOf_inj {u u' : Nat} (h : vOf u = vOf u') : u = u' := by
  unfold vOf at h; split at h <;> split at h <;> omega

theorem vOf_ne_two (u : Nat) : vOf u ≠ 2 := by unfold vOf; split <;> omega

/-! ## The schedule -/

/-- What node `k` receives in views one and two. -/
def pre (k : PubKey) : Nat → Input
  | 0 => .headerBuilt ⟨1⟩ (blockHash anchorB) (hdr 1)
  | 1 => .proposal a (blk 0) (some ⟨⟨1⟩, (blk 0).payloadCommit⟩)
  | 2 => .blockValidated ⟨1⟩ (blockHash (blk 0))
  | 3 => .certificate1 (certOf (blk 0))
  | 4 => if k = c then .blockValidated ⟨1⟩ (blockHash (blk 0)) else .blockReconstructed ⟨1⟩ (blk 0).payloadCommit
  | 5 => .timeout ⟨2⟩
  | 6 => .timeoutCertificate TC
  | 7 => .timeoutOneHonest ⟨2⟩
  | _ => if k = c then .blockReconstructed ⟨1⟩ (blk 0).payloadCommit else .blockValidated ⟨1⟩ (blockHash (blk 0))

/-- What every node receives in the steps of the view of `blk (u + 1)`. -/
def phase (u : Nat) : Nat → Input
  | 0 => .headerBuilt ⟨u + 3⟩ (blockHash (blk u)) (hdr (u + 2))
  | 1 => .proposal a (blk (u + 1)) (some ⟨⟨u + 3⟩, (blk (u + 1)).payloadCommit⟩)
  | 2 => .blockValidated ⟨u + 3⟩ (blockHash (blk (u + 1)))
  | 3 => .certificate1 (certOf (blk (u + 1)))
  | 4 => .blockReconstructed ⟨u + 3⟩ (blk (u + 1)).payloadCommit
  | _ => .certificate2 ⟨(certOf (blk (u + 1))).data.toVote2, ⟨u + 3⟩⟩

/-- Views one and two in nine steps, then six steps to a view. -/
def input (k : PubKey) (n : Nat) : Input := if n < 9 then pre k n else phase ((n - 9) / 6) ((n - 9) % 6)

local notation "tr" => Kit.tr cfg leader input

local notation "H" => Kit.H cfg leader input

theorem input_pre {k : PubKey} {n : Nat} (hn : n < 9) : input k n = pre k n := by
  simp [input, hn]

theorem input_at (k : PubKey) (u r : Nat) (hr : r < 6) : input k (9 + 6 * u + r) = phase u r := by
  simp only [input]
  rw [ite_eq_right (by omega), show (9 + 6 * u + r - 9) / 6 = u by omega, show (9 + 6 * u + r - 9) % 6 = r by omega]

/-- Every step is one of views one and two, or a step of a later view. -/
theorem steps (n : Nat) : n < 9 ∨ ∃ u r, r < 6 ∧ n = 9 + 6 * u + r :=
  if h : n < 9 then Or.inl h else Or.inr ⟨(n - 9) / 6, (n - 9) % 6, by omega, by omega⟩

theorem r6 (r : Nat) (hr : r < 6) : r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 ∨ r = 5 := by omega

theorem r9 (n : Nat) (hn : n < 9) :
    n = 0 ∨ n = 1 ∨ n = 2 ∨ n = 3 ∨ n = 4 ∨ n = 5 ∨ n = 6 ∨ n = 7 ∨ n = 8 := by omega

/-- The first step of `blk u`'s view: its header. -/
def sAt (u : Nat) : Nat := if u = 0 then 0 else 9 + 6 * (u - 1)

/-- The step `blk u`'s payload reaches node `k`: late at `c` for the first block. -/
def pAt (k : PubKey) (u : Nat) : Nat := if u = 0 ∧ k = c then 8 else sAt u + 4

theorem sAt_succ (u : Nat) : sAt (u + 1) = 9 + 6 * u := by simp [sAt]

theorem pAt_ge (k : PubKey) (u : Nat) : sAt u + 4 ≤ pAt k u := by
  unfold pAt sAt; split <;> split <;> omega

theorem pAt_le (k : PubKey) (u : Nat) : pAt k u ≤ sAt u + 8 := by
  unfold pAt sAt; split <;> split <;> omega

/-- In views one and two, only the first block's payload has reached anyone, and not `c`. -/
theorem pAt_small {k : PubKey} {u n : Nat} (h : pAt k u < n) (hn : n ≤ 8) : u = 0 ∧ k ≠ c := by
  by_cases hu : u = 0
  · subst hu
    by_cases hk : k = c
    · simp [pAt, hk] at h; omega
    · exact ⟨rfl, hk⟩
  · exfalso
    have : pAt k u = sAt u + 4 := by simp [pAt, hu]
    have : 9 ≤ sAt u := by simp [sAt, hu]
    omega

/-! ## What the nodes receive -/

section Inputs

variable {k : PubKey}

/-- An input is one of views one and two, or a phase of a later view. -/
theorem input_cases {n : Nat} {i : Input} (hi : input k n = i) :
    (n < 9 ∧ pre k n = i) ∨ ∃ u r, r < 6 ∧ n = 9 + 6 * u + r ∧ phase u r = i := by
  rcases steps n with hn | ⟨u, r, hr, rfl⟩
  · exact Or.inl ⟨hn, by rw [← input_pre hn]; exact hi⟩
  · exact Or.inr ⟨u, r, hr, rfl, by rw [← input_at k u r hr]; exact hi⟩

theorem input_timeout {n : Nat} {v : ViewNumber} (hi : input k n = .timeout v) : n = 5 ∧ v = ⟨2⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨rfl, he.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he

theorem input_tc {n : Nat} {tc : TimeoutCert} (hi : input k n = .timeoutCertificate tc) :
    n = 6 ∧ tc = TC := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨rfl, he.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he

theorem input_oneHonest {n : Nat} {v : ViewNumber} (hi : input k n = .timeoutOneHonest v) :
    n = 7 ∧ v = ⟨2⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨rfl, he.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he

theorem input_header {n : Nat} {v : ViewNumber} {h : BlockHash} {x : BlockHeader}
    (hi : input k n = .headerBuilt v h x) :
    ∃ u, n = sAt u ∧ v = ⟨vOf u⟩ ∧ h = blockHash (parentOf u) ∧ x = hdr (u + 1) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨0, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he
    exact ⟨u + 1, by rw [sAt_succ]; rfl, by rw [vOf_succ]; exact he.1.symm, he.2.1.symm, he.2.2.symm⟩

theorem input_proposal {n : Nat} {sn : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal sn p (some vid)) :
    ∃ u, n = sAt u + 1 ∧ sn = a ∧ p = blk u ∧ vid = ⟨⟨vOf u⟩, (blk u).payloadCommit⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨0, rfl, he.1.symm, he.2.1.symm, he.2.2.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he
    exact ⟨u + 1, by rw [sAt_succ], he.1.symm, he.2.1.symm, by rw [vOf_succ]; exact he.2.2.symm⟩

theorem input_validated {n : Nat} {v : ViewNumber} {h : BlockHash}
    (hi : input k n = .blockValidated v h) :
    ∃ u, sAt u + 2 ≤ n ∧ v = ⟨vOf u⟩ ∧ h = blockHash (blk u) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨0, by simp [sAt], he.1.symm, he.2.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he
    exact ⟨u + 1, by rw [sAt_succ]; omega, by rw [vOf_succ]; exact he.1.symm, he.2.symm⟩

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    ∃ u, n = sAt u + 3 ∧ x = certOf (blk u) := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨0, rfl, he.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he
    exact ⟨u + 1, by rw [sAt_succ], he.symm⟩

theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = pAt k u ∧ v = ⟨vOf u⟩ ∧ pc = (blk u).payloadCommit := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    all_goals exact ⟨0, by simp [pAt, sAt, hk], he.1.symm, he.2.symm⟩
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he
    exact ⟨u + 1, by simp [pAt, sAt], by rw [vOf_succ]; exact he.1.symm, he.2.symm⟩

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, n = 9 + 6 * u + 5 ∧ x = ⟨(certOf (blk (u + 1))).data.toVote2, ⟨u + 3⟩⟩ := by
  rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
  · by_cases hk : k = c <;>
      rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
  · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he
    exact ⟨u, rfl, he.symm⟩

/-- Nobody hands over an epoch change, a re-vote request, or a proposal without a share. -/
theorem input_none {n : Nat} : (∀ c1 c2 p, input k n ≠ .epochChange c1 c2 p)
    ∧ (∀ s r, input k n ≠ .revote s r) ∧ (∀ s p, input k n ≠ .proposal s p none) := by
  refine ⟨fun c1 c2 p hi => ?_, fun s r hi => ?_, fun s p hi => ?_⟩ <;>
  · rcases input_cases hi with ⟨hn, he⟩ | ⟨u, r, hr, rfl, he⟩
    · by_cases hk : k = c <;>
        rcases r9 n hn with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [pre, hk] at he
    · rcases r6 r hr with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [phase] at he

/-- Every proposal a node receives comes with its share. -/
theorem input_share {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ vid, share = some vid := by
  cases share with
  | some vid => exact ⟨vid, rfl⟩
  | none => exact absurd hi (input_none.2.2 _ _)

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

theorem received {n : Nat} {i : Input} : (H k n).Received i ↔ ∃ j, j < n ∧ input k j = i := Kit.received

theorem recv {n : Nat} (j : Nat) (h : j < n) : (H k n).Received (input k j) := received.mpr ⟨j, h, rfl⟩

theorem recv_phase {n : Nat} (u r : Nat) (hr : r < 6) (h : 9 + 6 * u + r < n) :
    (H k n).Received (phase u r) := by
  rw [← input_at k u r hr]; exact recv _ h

/-- The proposal of `blk u`, with the node's share. -/
theorem recv_proposal {n : Nat} (u : Nat) (h : sAt u + 1 < n) :
    (H k n).Received (.proposal a (blk u) (some ⟨⟨vOf u⟩, (blk u).payloadCommit⟩)) := by
  cases u with
  | zero => exact recv (k := k) 1 h
  | succ u => rw [sAt_succ] at h; rw [vOf_succ]; exact recv_phase u 1 (by omega) h

theorem recv_validated {n : Nat} (u : Nat) (h : sAt u + 2 < n) :
    (H k n).Received (.blockValidated ⟨vOf u⟩ (blockHash (blk u))) := by
  cases u with
  | zero => exact recv (k := k) 2 h
  | succ u => rw [sAt_succ] at h; rw [vOf_succ]; exact recv_phase u 2 (by omega) h

theorem recv_cert1 {n : Nat} (u : Nat) (h : sAt u + 3 < n) : (H k n).Received (.certificate1 (certOf (blk u))) := by
  cases u with
  | zero => exact recv (k := k) 3 h
  | succ u => rw [sAt_succ] at h; exact recv_phase u 3 (by omega) h

theorem recv_payload {n : Nat} (u : Nat) (h : pAt k u < n) :
    (H k n).Received (.blockReconstructed ⟨vOf u⟩ (blk u).payloadCommit) := by
  cases u with
  | zero =>
    by_cases hk : k = c
    · have : input k 8 = .blockReconstructed ⟨1⟩ (blk 0).payloadCommit := by
        rw [input_pre (by omega)]; simp [pre, hk]
      show (H k n).Received (.blockReconstructed ⟨1⟩ _)
      rw [← this]; exact recv 8 (by simp [pAt, hk] at h; exact h)
    · have : input k 4 = .blockReconstructed ⟨1⟩ (blk 0).payloadCommit := by
        rw [input_pre (by omega)]; simp [pre, hk]
      show (H k n).Received (.blockReconstructed ⟨1⟩ _)
      rw [← this]; exact recv 4 (by simp [pAt, sAt, hk] at h; omega)
  | succ u =>
    have h' : 9 + 6 * u + 4 < n := by simp [pAt, sAt] at h; omega
    rw [vOf_succ]; exact recv_phase u 4 (by omega) h'

theorem recv_tc {n : Nat} (h : 6 < n) : (H k n).Received (.timeoutCertificate TC) := by
  have : input k 6 = .timeoutCertificate TC := by rw [input_pre (by omega)]; rfl
  rw [← this]; exact recv 6 h

theorem hasProposal {n : Nat} {x : Block} (hb : (H k n).HasProposal cfg x) :
    x = anchorB ∨ ∃ u, x = blk u ∧ sAt u + 1 < n := by
  rcases hb with rfl | ⟨s, share, hr⟩ | ⟨c1, c2, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    cases share with
    | some vid =>
      obtain ⟨u, rfl, -, rfl, -⟩ := input_proposal hji
      exact Or.inr ⟨u, rfl, hj⟩
    | none => exact absurd hji (input_none.2.2 _ _)
  · obtain ⟨j, -, hji⟩ := received.mp hr; exact absurd hji (input_none.1 _ _ _)

theorem hasProposal_of {n : Nat} (u : Nat) (h : sAt u + 1 < n) : (H k n).HasProposal cfg (blk u) :=
  Or.inr (Or.inl ⟨a, _, recv_proposal u h⟩)

theorem hasProposal_parent {n : Nat} (u : Nat) (h : u = 0 ∨ sAt (u - 1) + 1 < n) :
    (H k n).HasProposal cfg (parentOf u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u => exact hasProposal_of u (by rcases h with h | h <;> simp_all)

theorem hasCert1 {n : Nat} {x : Cert1} (hc : (H k n).HasCert1 cfg x) :
    x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ sAt u + 3 < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert1 hji
    exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨j, -, hji⟩ := received.mp hr; exact absurd hji (input_none.1 _ _ _)

theorem hasCert2 {n : Nat} {x : Cert2} (hc : (H k n).HasCert2 x) :
    ∃ u, x = ⟨(certOf (blk (u + 1))).data.toVote2, ⟨u + 3⟩⟩ ∧ 9 + 6 * u + 5 < n := by
  rcases hc with hr | ⟨c1, p, hr⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert2 hji
    exact ⟨u, rfl, hj⟩
  · obtain ⟨j, -, hji⟩ := received.mp hr; exact absurd hji (input_none.1 _ _ _)

theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis ∨ ∃ u, v = ⟨vOf u⟩ ∧ pc = (blk u).payloadCommit ∧ pAt k u < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hj⟩

theorem received_tc {n : Nat} {tc : TimeoutCert} (hr : (H k n).Received (.timeoutCertificate tc)) :
    tc = TC ∧ 6 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨rfl, rfl⟩ := input_tc hji
  exact ⟨rfl, hj⟩

theorem no_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal} :
    ¬ (H k n).Received (.epochChange c1 c2 p) :=
  fun hr => by obtain ⟨j, -, hji⟩ := received.mp hr; exact input_none.1 _ _ _ hji

theorem cert_number (u : Nat) : (certOf (blk u)).data.blockNumber = ⟨u + 1⟩ := blk_number u

/-- What a node can lock on after `n` steps: genesis, and each block whose payload has arrived. -/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x ↔ x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ pAt k u < n := by
  constructor
  · rintro (rfl | ⟨hc, b', hb, ⟨-, hcd⟩, hp⟩ | ⟨c2, p, hr, -⟩)
    · exact Or.inl rfl
    · rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩
      · exact Or.inl rfl
      · have hn := congrArg (fun d => d.blockNumber.toNat) hcd
        simp only [cert_number] at hn
        rcases hasProposal hb with rfl | ⟨u', rfl, -⟩
        · simp [anchorB] at hn
        · rw [blk_number] at hn
          obtain rfl : u = u' := by simp at hn; omega
          rcases hasPayload hp with hg | ⟨u'', hv, -, hlt⟩
          · rw [blk_view] at hg
            exact absurd (congrArg ViewNumber.toNat hg) (by unfold vOf; split <;> simp [ViewNumber.genesis])
          · rw [blk_view] at hv
            obtain rfl : u = u'' := vOf_inj (congrArg ViewNumber.toNat hv)
            exact Or.inr ⟨u, rfl, hlt⟩
    · exact absurd hr no_epochChange
  · rintro (rfl | ⟨u, rfl, hlt⟩)
    · exact Or.inl rfl
    · have := pAt_ge k u
      refine Or.inr (Or.inl ⟨Or.inr (Or.inl (recv_cert1 u (by omega))), blk u,
        hasProposal_of u (by omega), ⟨Nat.le_refl _, rfl⟩, Or.inr ?_⟩)
      rw [blk_view]
      exact recv_payload u hlt

/-- The view a node is in after `n` steps: it moves on with each `Cert1`, and with the timeout certificate. -/
def viewAt (n : Nat) : Nat :=
  if n < 4 then 1 else if n < 7 then 2 else if n < 13 then 3 else (n - 13) / 6 + 4

/-- After a block's `Cert1`, the node is past the block's view. -/
theorem view_after (u n : Nat) (h : sAt u + 3 < n) : vOf u + 1 ≤ viewAt n := by
  cases u with
  | zero =>
    simp [sAt] at h
    show 1 + 1 ≤ viewAt n
    unfold viewAt; repeat' split
    all_goals omega
  | succ w =>
    rw [sAt_succ] at h; rw [vOf_succ]
    unfold viewAt; repeat' split
    all_goals omega

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ viewAt n := by
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, ⟨hr, -⟩, -⟩
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ viewAt n
      unfold viewAt; split <;> (try split) <;> (try split) <;> omega
    · show (certOf (blk u)).view.toNat + 1 ≤ viewAt n
      rw [certOf_view]
      exact view_after u n hlt
  · obtain ⟨rfl, hlt⟩ := received_tc htc
    show 2 + 1 ≤ viewAt n
    unfold viewAt; repeat' split
    all_goals omega
  · exact absurd hr no_epochChange

theorem inView (n : Nat) : (H k n).InView cfg ⟨viewAt n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  by_cases h4 : n < 4
  · refine Or.inl ⟨certOf anchorB, Or.inl rfl, ?_⟩
    simp only [viewAt, h4, ↓reduceIte]; rfl
  by_cases h7 : n < 7
  · refine Or.inl ⟨certOf (blk 0), Or.inr (Or.inl (recv_cert1 0 (by simp [sAt]; omega))), ?_⟩
    simp only [viewAt, h4, h7, ↓reduceIte]; rfl
  by_cases h13 : n < 13
  · refine Or.inr (Or.inl ⟨TC, recv_tc (by omega), ?_⟩)
    simp only [viewAt, h4, h7, h13, ↓reduceIte]; rfl
  · obtain ⟨w, hw⟩ : ∃ w, (n - 13) / 6 = w := ⟨_, rfl⟩
    refine Or.inl ⟨certOf (blk (w + 1)), Or.inr (Or.inl (recv_cert1 (w + 1)
      (by rw [sAt_succ]; omega))), ?_⟩
    simp only [viewAt, h4, h7, h13, ↓reduceIte, hw]
    rw [certOf_view, vOf_succ]; rfl

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨viewAt n⟩ :=
  Kit.inView_unique hv (inView n)

/-- In the steps of `blk u`'s view up to its `Cert1`, the node is in that view. -/
theorem viewAt_phase (u r : Nat) (hr : r < 4) : viewAt (sAt u + r) = vOf u := by
  cases u with
  | zero => simp only [sAt, vOf, ↓reduceIte]; unfold viewAt; rw [ite_eq_left (by omega)]
  | succ w =>
    rw [sAt_succ, vOf_succ]
    unfold viewAt; repeat' split
    all_goals omega

/-- The nodes only ever have grounds for the one epoch. -/
theorem epochGround_zero {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e = ⟨0⟩ := by
  rcases he with rfl | ⟨c1, c2, p, ⟨hr, -⟩, -⟩ | ⟨tc, htc, rfl⟩ | ⟨x, hx, -, rfl⟩
  · rfl
  · exact absurd hr no_epochChange
  · obtain ⟨rfl, -⟩ := received_tc htc; rfl
  · rcases hasCert1 hx with rfl | ⟨u, rfl, -⟩
    · rfl
    · exact blk_epoch u

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨0⟩ :=
  ⟨Or.inl rfl, fun _ he => by rw [epochGround_zero he]; exact Nat.le_refl _⟩

theorem notBehind {n u : Nat} : NotBehind cfg (H k n) (blk u).epoch := fun e he => by
  rw [epochGround_zero he.1, blk_epoch]; exact Nat.le_refl _

theorem blk_wellFormed (u : Nat) : ProposalWellFormed cfg (blk u) := by
  rcases u with _ | _ | u
  · exact ⟨by decide, Or.inl ⟨rfl, rfl⟩, rfl, rfl⟩
  · exact ⟨by decide, Or.inr ⟨TC, rfl, rfl⟩, rfl, rfl⟩
  · refine ⟨?_, Or.inl ⟨rfl, ?_⟩, rfl, ?_⟩
    · show (certOf (blk (u + 1))).view.toNat < u + 4
      rw [certOf_view, vOf_succ]; show u + 3 < u + 4; omega
    · show (certOf (blk (u + 1))).view + 1 = ⟨u + 4⟩
      rw [certOf_view, vOf_succ]; rfl
    · show (blk (u + 1)).blockHeader.blockNumber + 1 = ⟨u + 3⟩
      rw [blk_number]; rfl

theorem blk_safe (u : Nat) : SafeParent (blk u) := by
  rcases u with _ | _ | u
  · exact fun _ h => by cases h
  · intro tc h
    cases h
    exact ⟨rfl, Or.inr ⟨rfl, Or.inr rfl⟩⟩
  · exact fun _ h => by cases h

theorem leader_view {u : Nat} : leader ⟨0⟩ ⟨vOf u⟩ = some a := by
  simp only [leader]
  split
  · rename_i h; exact absurd (congrArg ViewNumber.toNat h) (vOf_ne_two u)
  · rfl

theorem viewOf_eq (n : Nat) : viewOf cfg (H k n) = ⟨viewAt n⟩ := viewOf_of_inView (inView n)

theorem epochOf_eq (n : Nat) : epochOfHistory cfg (H k n) = ⟨0⟩ :=
  Liveness.inEpoch_unique (inEpoch_epochOfHistory cfg _) (inEpoch n)

/-- The lock each node carries into view two's timer: `c` cannot lock on the first block. -/
def lock5 (k : PubKey) : Cert1 := if k = c then certOf anchorB else certOf (blk 0)

theorem lockOf_five : lockOf cfg (H k 5) = lock5 k := by
  obtain ⟨hl, hmax⟩ := lockOf_lockedOn (cfg := cfg) (H k 5)
  have hsmall : ∀ u, pAt k u < 5 → u = 0 ∧ k ≠ c := fun u hu => pAt_small hu (by omega)
  by_cases hk : k = c
  · rw [show lock5 k = certOf anchorB from ite_eq_left hk]
    rcases lockable_iff.mp hl with h | ⟨u, -, hu⟩
    · exact h
    · exact absurd hk (hsmall u hu).2
  · rw [show lock5 k = certOf (blk 0) from ite_eq_right hk]
    have h0 := hmax (certOf (blk 0)) (lockable_iff.mpr (Or.inr ⟨0, rfl, by simp [pAt, sAt, hk]⟩))
    rcases lockable_iff.mp hl with h | ⟨u, h, hu⟩
    · exfalso
      rw [h] at h0
      rcases h0 with h0 | ⟨-, h0⟩
      · exact absurd h0 (Nat.lt_irrefl _)
      · exact absurd h0 (by show ¬ (1 : Nat) ≤ 0; omega)
    · rw [h, (hsmall u hu).1]

/-- A block's proposal may go out from the step its header arrives. -/
theorem blk_justified (u n : Nat) (hn : sAt u + 1 ≤ n) :
    ProposalJustified cfg leader a (H a n) (blk u) := by
  refine ⟨⟨by rw [blk_view, blk_epoch]; exact leader_view, blk_wellFormed u, ?_,
    ⟨parentOf u, hasProposal_parent u ?_, by rw [blk_parent]; exact Nat.le_refl _,
      by rw [blk_parent]; rfl⟩, fun he => absurd rfl he.2.1,
    blk_safe u, notBehind, ⟨⟨viewAt n⟩, (inView n).1, ?_⟩⟩, ?_⟩
  · unfold ParentJustified
    rcases u with _ | _ | u
    · exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
    · show (H a n).HasCert1 cfg (certOf (blk 0)) ∧ ∃ m, ((H a n).upTo m).Received (.timeoutCertificate TC)
        ∧ ((∃ l, ((H a n).upTo m).LockedOn cfg l ∧ l.data = (certOf (blk 0)).data)
          ∨ (IsLastBlock (certOf (blk 0)).data.blockNumber cfg.epochHeight
            ∧ ∃ c2, ((H a n).upTo m).HasCert2 c2 ∧ c2.data = (certOf (blk 0)).data.toVote2))
      have hn' : 10 ≤ n := by rw [sAt_succ] at hn; omega
      refine ⟨Or.inr (Or.inl (recv_cert1 0 (by simp [sAt]; omega))), 7, ?_,
        Or.inl ⟨certOf (blk 0), ?_, rfl⟩⟩ <;> rw [upTo_H, show min 7 n = 7 by omega]
      · exact recv_tc (by omega)
      · have h7 : lockOf cfg (H a 7) = certOf (blk 0) := by
          obtain ⟨hl, hmax⟩ := lockOf_lockedOn (cfg := cfg) (H a 7)
          have h0 := hmax (certOf (blk 0)) (lockable_iff.mpr (Or.inr ⟨0, rfl, by simp [pAt, sAt]; decide⟩))
          rcases lockable_iff.mp hl with h | ⟨u', h, hu⟩
          · exfalso; rw [h] at h0
            rcases h0 with h0 | ⟨-, h0⟩
            · exact absurd h0 (Nat.lt_irrefl _)
            · exact absurd h0 (by show ¬ (1 : Nat) ≤ 0; omega)
          · rw [h, (pAt_small hu (by omega)).1]
        rw [← h7]; exact lockOf_lockedOn _
    · show (H a n).Buildable cfg (certOf (blk (u + 1)))
      refine Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inr ⟨u + 1, rfl, ?_⟩))
      have := pAt_le a (u + 1)
      simp only [pAt, sAt] at hn ⊢; simp at hn ⊢; omega
  · rcases u with _ | u
    · exact Or.inl rfl
    · right; simp only [Nat.add_sub_cancel]
      unfold sAt at hn ⊢; split at hn <;> split <;> omega
  · rw [blk_view]
    show vOf u ≤ viewAt n
    rcases u with _ | u
    · unfold viewAt; repeat' split
      all_goals simp [vOf]
      all_goals omega
    · rw [vOf_succ]; rw [sAt_succ] at hn
      unfold viewAt; repeat' split
      all_goals omega
  · rw [blk_view, blk_header, blk_parent]
    have : input a (sAt u) = .headerBuilt ⟨vOf u⟩ (blockHash (parentOf u)) (hdr (u + 1)) := by
      cases u with
      | zero => rw [input_pre (by simp [sAt])]; rfl
      | succ u => rw [sAt_succ, show 9 + 6 * u = 9 + 6 * u + 0 by omega, input_at a u 0 (by omega), vOf_succ]; rfl
    show (H a n).Received (.headerBuilt ⟨vOf u⟩ (blockHash (parentOf u)) (hdr (u + 1)))
    rw [← this]; exact recv _ (by omega)

end Holds

/-! ## What the honest nodes send -/

section Sends

open Lists

variable (hv : ∀ b, BlockValid b)

include hv in
theorem protocol (k : PubKey) (n : Nat) : ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H k n) :=
  Kit.protocol input hv k n

/-- Nothing is owed after a step. -/
theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H k (n + 1)) o :=
  Kit.settled input k n o

/-- The only timeout vote a node sends is its answer to view two's timer, naming its lock then. -/
theorem sent_timeout {k : PubKey} {j : Nat} {vote : TimeoutVote}
    (hx : Output.send (.timeoutVote vote) ∈ (tr k j).output) :
    j = 5 ∧ vote = ⟨⟨⟨0⟩, lock5 k⟩, ⟨2⟩, k⟩ := by
  rw [tr_step] at hx
  rcases step_mem hx with hx | ⟨o, out', -, -, ha⟩
  · obtain ⟨v, hv, (⟨hi, -⟩ | ⟨hi, hle⟩)⟩ := mem_timeoutAnswer hx
    · obtain ⟨rfl, rfl⟩ := input_timeout hi
      simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      exact ⟨rfl, by rw [hv, epochOf_eq, lockOf_five]⟩
    · obtain ⟨rfl, rfl⟩ := input_oneHonest hi
      rw [viewOf_eq] at hle
      exact absurd hle (by show ¬ viewAt 7 ≤ 2; simp [viewAt])
  · exact absurd ha act_no_timeoutVote

/-- Every node times view two out at its timer. -/
theorem times_out (k : PubKey) : Output.send (.timeoutVote ⟨⟨⟨0⟩, lock5 k⟩, ⟨2⟩, k⟩) ∈ (tr k 5).output := by
  rw [tr_step, step_output]
  apply discharge_sup
  have hi : input k 5 = .timeout ⟨2⟩ := by rw [input_pre (by omega)]; rfl
  rw [hi]
  have hv5 : viewOf cfg (H k 5) = ⟨2⟩ := by rw [viewOf_eq]; rfl
  simp only [timeoutAnswer, hv5, epochOf_eq, lockOf_five, ite_true, List.mem_singleton]

theorem not_timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h3 : 3 ≤ v.toNat) :
    ¬ (H k n).TimedOut v := fun ⟨vote, hs, hle⟩ => by
  obtain ⟨j, -, hj⟩ := sent_iff.mp hs
  obtain ⟨-, rfl⟩ := sent_timeout hj
  have : v.toNat ≤ 2 := hle
  omega

/-- Before view two's timer, nothing is timed out. -/
theorem not_timedOut_early {k : PubKey} {n : Nat} {v : ViewNumber} (hn : n ≤ 5) :
    ¬ (H k n).TimedOut v := fun ⟨vote, hs, _⟩ => by
  obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
  obtain ⟨rfl, -⟩ := sent_timeout hjv
  omega

theorem not_pastView {k : PubKey} {n : Nat} {v : ViewNumber} (h3 : 3 ≤ v.toNat) :
    ¬ (H k n).PastView v := by
  rintro (h | ⟨tc, htc, hle⟩)
  · exact not_timedOut h3 h
  · obtain ⟨rfl, -⟩ := received_tc htc
    have : v.toNat ≤ 2 := hle
    omega

include hv in
/-- No honest node asks for a re-vote: no block is the last of an epoch. -/
theorem no_revote (k : PubKey) (j : Nat) (r : RevoteRequest) : Output.send (.revote r) ∉ (tr k j).output :=
  fun hr => by
    have hj := (protocol hv k (j + 1)).revoteJustified j r ⟨_, (getElem_H j), hr⟩ trivial
    exact hj.wellFormed.2.2.2.1 rfl

theorem hdr_number (u : Nat) : (hdr u).blockNumber = ⟨u⟩ := rfl

include hv in
/-- Every proposal an honest node sends is `a`'s, the schedule's block, after its header arrived. -/
theorem sent_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    k = a ∧ ∃ u, p = blk u ∧ sAt u ≤ j := by
  have hj := (protocol hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨⟨hlead, hwf, hpj, -, -, -, -, -⟩, hrec⟩ := hj
  obtain ⟨i, hi, hii⟩ := received.mp hrec
  obtain ⟨u, rfl, hvw, -, hhdr⟩ := input_header hii
  have hka : k = a := by
    rw [hvw, show p.epoch = ⟨0⟩ by rw [hwf.epoch]; rfl, leader_view] at hlead
    exact (Option.some.inj hlead).symm
  refine ⟨hka, u, ?_, by omega⟩
  subst hka
  obtain ⟨hid, hte⟩ : p.identity = ⟨0⟩ ∧ (p.timeoutEvidence = none
      ∨ ∃ tc, p.timeoutEvidence = some tc ∧ (H a (j + 1)).Received (.timeoutCertificate tc)) := by
    rw [tr_step] at hp
    rcases step_mem hp with hx | ⟨o, out', -, -, ha⟩
    · obtain ⟨v, hx, -⟩ := mem_timeoutAnswer hx; cases hx
    · obtain ⟨_, v, rfl, hmem, -, -⟩ := act_proposal ha
      have he : SameInputs (H a j ++ [Step.mk (input a j) out']) (H a (j + 1)) := by
        rw [← Kit.hpre, ← Kit.hpre]
        exact sameInputs_step
      obtain ⟨hid, -, hte⟩ := mem_proposalCandidates hmem
      refine ⟨hid, ?_⟩
      rcases hte with h | ⟨tc, h1, h2⟩
      · exact Or.inl h
      · exact Or.inr ⟨tc, h1, (he.received _).mp h2⟩
  have hep : p.epoch = ⟨0⟩ := by rw [hwf.epoch]; rfl
  have hnum : p.parentCert.data.blockNumber = ⟨u⟩ := by
    have h := hwf.height
    rw [hhdr, hdr_number] at h
    exact BlockNumber.ext (by have := congrArg BlockNumber.toNat h; simp at this; omega)
  unfold ParentJustified at hpj
  have hpc : p.parentCert = certOf (parentOf u) ∧ p.timeoutEvidence = (blk u).timeoutEvidence := by
    rcases hte with hte | ⟨tc, hte, htc⟩
    · rw [hte] at hpj
      have hnext : p.parentCert.view + 1 = p.viewNumber := by
        rcases hwf.covered with ⟨-, h⟩ | ⟨tc, h, -⟩
        · exact h
        · rw [hte] at h; cases h
      rcases hasCert1 (Liveness.hasCert1_of_buildable hpj) with h | ⟨i', h, -⟩
      · rw [h, hvw] at hnext
        have h1 : 0 + 1 = vOf u := congrArg ViewNumber.toNat hnext
        have hu : u = 0 := by unfold vOf at h1; split at h1 <;> omega
        subst hu
        exact ⟨h, by rw [hte]; rfl⟩
      · rw [h, hvw] at hnext
        have h1 : vOf i' + 1 = vOf u := by rw [certOf_view] at hnext; exact congrArg ViewNumber.toNat hnext
        rw [h, cert_number] at hnum
        have hn : i' + 1 = u := congrArg BlockNumber.toNat hnum
        subst hn
        have hi0 : i' ≠ 0 := fun h0 => by subst h0; simp [vOf] at h1
        obtain ⟨i, rfl⟩ : ∃ i, i' = i + 1 := ⟨i' - 1, by omega⟩
        exact ⟨h, by rw [hte]; rfl⟩
    · obtain ⟨rfl, -⟩ := received_tc htc
      have hu : u = 1 := by
        rcases hwf.covered with ⟨hn, -⟩ | ⟨tc', hte', htv⟩
        · rw [hte] at hn; cases hn
        · rw [hte] at hte'; cases hte'
          rw [hvw] at htv
          have : 2 + 1 = vOf u := congrArg ViewNumber.toNat htv
          unfold vOf at this; split at this <;> omega
      subst hu
      rw [hte] at hpj
      rcases hasCert1 hpj.1 with h | ⟨i', h, -⟩
      · rw [h] at hnum; exact absurd (congrArg BlockNumber.toNat hnum) (by decide)
      · rw [h, cert_number] at hnum
        have : i' = 0 := by have := congrArg BlockNumber.toNat hnum; simp at this; omega
        subst this
        exact ⟨h, by rw [hte]; rfl⟩
  obtain ⟨hpc, hte'⟩ := hpc
  obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
  simp only at hvw hhdr hid hep hpc hte' ⊢
  subst hhdr hid hep hpc hte'
  rcases u with _ | _ | u <;> (simp only [vOf] at hvw; subst hvw; rfl)

include hv in
/-- `a` proposes each block by the step its header arrives in. -/
theorem proposes (u : Nat) :
    ∃ j, j ≤ sAt u ∧ Output.send (.proposal (blk u)) ∈ (tr a j).output := by
  have hin := inView (k := a) (sAt u + 1)
  rw [viewAt_phase u 1 (by omega)] at hin
  refine Classical.byContradiction fun hneg => settled a (sAt u) (.propose (blk u).epoch ⟨vOf u⟩)
    ⟨Or.inl ⟨blk u, blk_justified u _ (Nat.le_refl _), blk_view u, rfl⟩, ?_, ?_, hin⟩
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, -⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨-, u', rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv
      obtain rfl : u' = u := vOf_inj (congrArg ViewNumber.toNat hpv)
      exact hneg ⟨j, by omega, hjp⟩
    · obtain ⟨j, -, hjr⟩ := sent_iff.mp hs
      exact no_revote hv a j r hjr
  · rcases u with _ | u
    · exact not_timedOut_early (by simp [sAt])
    · exact not_timedOut (by show 3 ≤ vOf (u + 1); rw [vOf_succ]; omega)

theorem not_pastView_early {k : PubKey} {n : Nat} {v : ViewNumber} (hn : n ≤ 5) :
    ¬ (H k n).PastView v := by
  rintro (h | ⟨tc, htc, -⟩)
  · exact not_timedOut_early hn h
  · obtain ⟨-, h⟩ := received_tc htc
    omega

include hv in
/-- Every vote1 an honest node sends is on the schedule's proposal, after it arrived. -/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1}
    (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(certOf (blk u)).data, ⟨vOf u⟩, k⟩ ∧ sAt u + 1 ≤ j := by
  obtain ⟨hsig, ⟨s, p, vid, hrec, -, -, -, -, hfor⟩ | ⟨s, r, -, hw, -⟩⟩ :=
    (protocol hv k (j + 1)).vote1Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  rotate_left
  · exact absurd rfl hw.2.2.2.1
  rw [upTo_self] at hrec
  obtain ⟨i, hi, hii⟩ := received.mp hrec
  obtain ⟨u, rfl, -, rfl, -⟩ := input_proposal hii
  refine ⟨u, ?_, by omega⟩
  obtain ⟨dt, v, sg⟩ := vote
  obtain ⟨hv', hd⟩ := hfor
  simp only at hsig hv' hd
  rw [hsig, hv', hd, blk_view]
  rfl

/-- The parent's payload reaches every node before its child's validity report. -/
theorem parent_ready {k : PubKey} (u : Nat) : ParentReady cfg (H k (sAt u + 2 + 1)) (blk u) := by
  rcases u with _ | w
  · exact Or.inl rfl
  · refine Or.inr (Or.inr ⟨blk w, hasProposal_of w ?_, ?_, ?_, Or.inr ?_⟩)
    · rw [sAt_succ]; unfold sAt; split <;> omega
    · rw [blk_parent]; exact Nat.le_refl _
    · rw [blk_parent]; rfl
    · rw [blk_view]
      refine recv_payload w ?_
      rw [sAt_succ]; unfold pAt sAt; repeat' split
      all_goals omega

include hv in
/-- Every honest node votes1 for `blk u` by the step its validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) : ∃ j, j ≤ sAt u + 2
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨vOf u⟩, k⟩) ∈ (tr k j).output := by
  have hto : ¬ (H k (sAt u + 2 + 1)).TimedOut (blk u).viewNumber := by
    rcases u with _ | u
    · exact not_timedOut_early (by simp [sAt])
    · exact not_timedOut (by rw [blk_view, vOf_succ]; show 3 ≤ u + 3; omega)
  refine Classical.byContradiction fun hneg => settled k (sAt u + 2) (.vote1 (blk u))
    ⟨⟨a, _, recv_proposal u (by omega), by rw [blk_view, blk_epoch]; exact leader_view,
      by rw [blk_view], rfl⟩, blk_wellFormed u,
      by rw [blk_view]; exact recv_validated u (by omega), parent_ready u, blk_safe u,
      fun he => absurd rfl he.2.1, notBehind, hto, fun vote hs _ hvv => ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -⟩ := sent_vote1 hv hjv
    rw [blk_view] at hvv
    obtain rfl : u' = u := vOf_inj (congrArg ViewNumber.toNat hvv)
    exact hneg ⟨j, by omega, hjv⟩
  · rw [blk_view]
    have := inView (k := k) (sAt u + 3)
    rwa [viewAt_phase u 3 (by omega)] at this

include hv in
/-- Every vote2 an honest node sends is on the schedule's `Cert1`, after it arrived. -/
theorem sent_vote2 {k : PubKey} {j : Nat} {vote : Vote2}
    (hx : Output.send (.vote2 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(certOf (blk u)).data.toVote2, ⟨vOf u⟩, k⟩ ∧ sAt u + 3 ≤ j := by
  obtain ⟨hsig, hgen, x, b', hc, -, -, -, hvc, hdc⟩ := (protocol hv k (j + 1)).vote2Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  rw [upTo_self] at hc
  rcases hasCert1 hc with rfl | ⟨u, rfl, hlt⟩
  · rw [hvc] at hgen; exact absurd hgen (Nat.lt_irrefl _)
  · refine ⟨u, ?_, by omega⟩
    obtain ⟨dt, v, sg⟩ := vote
    simp only at hsig hvc hdc
    rw [hsig, hvc, hdc, certOf_view]

include hv in
/--
Every honest node votes2 for `blk u` by the step its payload arrives in, but `c`
for the first block: its payload comes after view two timed out.
-/
theorem votes2 (k : PubKey) (u : Nat) (hu : u ≠ 0 ∨ k ≠ c) : ∃ j, j ≤ sAt u + 4
    ∧ Output.send (.vote2 ⟨(certOf (blk u)).data.toVote2, ⟨vOf u⟩, k⟩) ∈ (tr k j).output := by
  have hp : pAt k u = sAt u + 4 := by
    unfold pAt
    exact ite_eq_right fun ⟨h0, hk⟩ => hu.elim (· h0) (· hk)
  have hpast : ¬ (H k (sAt u + 4 + 1)).PastView (certOf (blk u)).view := by
    rcases u with _ | u
    · exact not_pastView_early (by simp [sAt])
    · exact not_pastView (by rw [certOf_view, vOf_succ]; show 3 ≤ u + 3; omega)
  refine Classical.byContradiction fun hneg => settled k (sAt u + 4) (.vote2 (certOf (blk u)))
    ⟨⟨blk u, Or.inr (Or.inl (recv_cert1 u (by omega))), hasProposal_of u (by omega),
      ⟨Nat.le_refl _, rfl⟩, Or.inr ?_⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_,
      hpast, Kit.afterFloor_of input hv (by rw [certOf_view]; show 0 < vOf u; unfold vOf; split <;> omega)
        fun x hx => by
          rw [certOf_view]
          rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩
          · show 0 < vOf u + 20; omega
          · rw [blk_view]; show vOf y < vOf u + 20
            rcases y with _ | y <;> rcases u with _ | u <;> simp [vOf, sAt] at hy ⊢ <;> omega⟩
  · rw [blk_view]; exact recv_payload u (by omega)
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -⟩ := sent_vote2 hv hjv
    rw [certOf_view] at hvv
    obtain rfl : u' = u := vOf_inj (congrArg ViewNumber.toNat hvv)
    exact hneg ⟨j, by omega, hjv⟩
  · obtain ⟨x, rfl, hlt⟩ := hasCert2 hc2
    rw [certOf_view] at hv2
    have h := congrArg ViewNumber.toNat hv2
    rw [show x + 3 = vOf (x + 1) from (vOf_succ x).symm] at h
    obtain rfl : x + 1 = u := vOf_inj h
    rw [sAt_succ] at hlt
    omega

include hv in
/-- `c` never votes2 on the first block: its payload comes after `c` timed view two out. -/
theorem c_skips_first {j : Nat} {vote : Vote2} (hx : Output.send (.vote2 vote) ∈ (tr c j).output) :
    vote.view ≠ ⟨1⟩ := fun h1 => by
  obtain ⟨-, hgen, x, b', hc, hb, hcert, hpay, hvc, -⟩ := (protocol hv c (j + 1)).vote2Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  rw [upTo_self] at hc hb hpay
  rcases lockable_iff.mp (Or.inr (Or.inl ⟨hc, b', hb, hcert, hpay⟩)) with rfl | ⟨u, rfl, hu⟩
  · rw [hvc] at hgen; exact absurd hgen (Nat.lt_irrefl _)
  · rw [hvc, certOf_view] at h1
    obtain rfl : u = 0 := vOf_inj (congrArg ViewNumber.toNat h1)
    have hu : 8 < j + 1 := hu
    have := (protocol hv c (j + 1)).vote2BeforeTimeout j vote ⟨_, (getElem_H j), hx⟩ trivial _
      (by rw [upTo_self]; exact sent_iff.mpr ⟨5, by omega, times_out c⟩) trivial
    rw [hvc, certOf_view] at this
    exact absurd this (by show ¬ 2 < vOf 0; simp [vOf])

end Sends

/-! ## The network -/

section Net

variable (hv : ∀ b, BlockValid b)

/-- The time of step `n`: the step, but view two's timer fires `τ = 25` after the nodes entered the view. -/
def tm (n : Nat) : Nat := if n ≤ 4 then n else n + 23

theorem tm_mono (n : Nat) : tm n ≤ tm (n + 1) := by unfold tm; split <;> split <;> omega

theorem tm_lt {i j : Nat} (h : i < j) : tm i < tm j := by unfold tm; split <;> split <;> omega

/-- Three steps after step `j` is within `Δ = 3` of its time, counted from GST `28`. -/
theorem tm_within {i j t : Nat} (hi : i ≤ j + 3) (ht : tm j ≤ t) : tm i ≤ max t 28 + 3 := by
  have := Nat.le_max_left t 28
  have := Nat.le_max_right t 28
  unfold tm at ht ⊢; split at ht <;> split <;> omega

/-- Views one and two end within `Δ` of GST. -/
theorem tm_early {i : Nat} (hi : i ≤ 8) (t : Nat) : tm i ≤ max t 28 + 3 := by
  have := Nat.le_max_right t 28
  unfold tm; split <;> omega

/-- A payload arrives within `Δ` of its block's proposal, or, for the first at `c`, of GST. -/
theorem tm_pAt (k : PubKey) {u j t : Nat} (hj : sAt u + 1 ≤ j) (ht : tm j ≤ t) :
    tm (pAt k u) ≤ max t 28 + 3 := by
  unfold pAt; split
  · exact tm_early (by omega) t
  · exact tm_within (by omega) ht

/-- Each lock is no later than the first block's `Cert1`. -/
theorem lock5_le (k : PubKey) : LockLE (lock5 k) (certOf (blk 0)) := by
  unfold lock5; split
  · exact Or.inr ⟨rfl, Nat.zero_le _⟩
  · exact Or.inr ⟨rfl, Nat.le_refl _⟩

theorem quorum_c {q : PubKey → Prop} (hq : C.Quorum ⟨0⟩ q) (hh : ∀ k, q k → C.Honest k) : q c := by
  rcases hq with ⟨-, -, h3⟩ | ⟨-, -, h3⟩ | ⟨-, h2, -⟩ | ⟨-, h2, -⟩
  · exact h3
  · exact absurd (hh d h3) d_faulty
  · exact h2
  · exact h2

include hv in
theorem backed1 (u : Nat) : Cert1Backed (C := C) (fun k _ => tr k) (certOf (blk u)) :=
  ⟨C.honest ⟨0⟩, honest_quorum, fun k _ _ => by
    obtain ⟨j, -, hj⟩ := votes1 hv k u
    rw [certOf_view]
    exact ⟨j, hj⟩⟩

include hv in
theorem backed2 (u : Nat) :
    Cert2Backed (C := C) (fun k _ => tr k) ⟨(certOf (blk (u + 1))).data.toVote2, ⟨u + 3⟩⟩ :=
  ⟨C.honest ⟨0⟩, honest_quorum, fun k _ _ => by
    obtain ⟨j, -, hj⟩ := votes2 hv k (u + 1) (Or.inl (by omega))
    rw [vOf_succ] at hj
    exact ⟨j, hj⟩⟩

/-- Every node signed view two's timeout with a lock no later than the certificate's. -/
theorem tc_backed : TimeoutCertBacked (C := C) (fun k _ => tr k) TC :=
  ⟨C.honest ⟨0⟩, honest_quorum, fun k _ _ => ⟨_, ⟨rfl, rfl, rfl, lock5_le k⟩, ⟨5, times_out k⟩⟩⟩

/-- The honest nodes running the machine on the schedule. -/
def net : TimedNetwork cfg leader C where
  honestQuorum := members_quorum
  trace k _ := tr k
  safe k _ n := .of_every (protocol hv k n).toSafeHistory
  cert1Genuine _ _ n x hc := by
    rcases Input.mem_cert1.mp hc with hin | ⟨c2, p, hin⟩ | ⟨s, p, vid, hin, rfl⟩
    · obtain ⟨u, -, rfl⟩ := input_cert1 hin
      exact Or.inr (backed1 hv u)
    · exact absurd hin (input_none.1 _ _ _)
    · obtain ⟨v, rfl⟩ := input_share hin
      obtain ⟨u, -, -, rfl, -⟩ := input_proposal hin
      rw [blk_parent]
      cases u with
      | zero => exact Or.inl rfl
      | succ u => exact Or.inr (backed1 hv u)
  cert2Genuine _ _ n x hc := by
    rcases Input.mem_cert2.mp hc with hin | ⟨c1, p, hin⟩
    · obtain ⟨u, -, rfl⟩ := input_cert2 hin
      exact backed2 hv u
    · exact absurd hin (input_none.1 _ _ _)
  timeoutCertGenuine _ _ n tc hc := by
    rcases Input.mem_timeoutCert.mp hc with hin | ⟨s, p, vid, hin, hte⟩ | ⟨s, r, hin, -⟩
    · obtain ⟨-, rfl⟩ := input_tc hin
      exact ⟨tc_backed, Or.inr (backed1 hv 0), by show (1 : Nat) ≤ 2; omega⟩
    · obtain ⟨v, rfl⟩ := input_share hin
      obtain ⟨u, -, -, rfl, -⟩ := input_proposal hin
      rcases u with _ | _ | u
      · cases hte
      · cases hte; exact ⟨tc_backed, Or.inr (backed1 hv 0), by show (1 : Nat) ≤ 2; omega⟩
      · cases hte
    · exact absurd hin (input_none.2.1 _ _)
  revoteGenuine _ _ _ _ _ hin := absurd hin (input_none.2.1 _ _)
  time _ _ n := tm n
  timeMono _ _ n := tm_mono n
  protocol k _ n := .of_every (protocol hv k n)
  timeoutCertCausal _ _ n tc hin := by
    obtain ⟨rfl, rfl⟩ := input_tc hin
    exact ⟨C.honest ⟨0⟩, honest_quorum, fun k _ _ =>
      ⟨5, _, ⟨rfl, rfl, rfl, lock5_le k⟩, times_out k, by show tm 5 < tm 6; decide⟩⟩
  oneHonestCausal _ _ n v hin := by
    obtain ⟨rfl, rfl⟩ := input_oneHonest hin
    exact ⟨a, ⟨0⟩, Or.inl rfl, 5, _, rfl, times_out a, rfl, by show tm 5 < tm 7; decide⟩
  authentic k _ n l msg hin _ _ := by
    cases hi : (tr k n).input <;> rw [hi] at hin <;> simp only [Input.sentBy, reduceCtorEq] at hin
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨v, rfl⟩ := input_share hi
      obtain ⟨u, rfl, rfl, rfl, -⟩ := input_proposal hi
      obtain ⟨j, hj, hjp⟩ := proposes hv u
      exact ⟨j, hjp, tm_lt (by omega)⟩
    · exact absurd hi (input_none.2.1 _ _)

/-- What a node's history holds after `n` steps, it holds by the time of step `n - 1`. -/
theorem by_of {k : PubKey} {hk : C.Honest k} {T n : Nat} {P : History → Prop} (hn : 0 < n → tm (n - 1) ≤ T)
    (hp : P (H k n)) : (net hv).By k hk T P :=
  Kit.by_at (tm := tm) (fun _ _ => rfl) (fun _ _ _ => rfl) tm_mono hn hp

theorem by_within {k : PubKey} {hk : C.Honest k} {j t n : Nat} {P : History → Prop} (hn : n ≤ j + 4)
    (ht : tm j ≤ t) (hp : P (H k n)) : (net hv).By k hk (max t 28 + 3) P :=
  by_of hv (fun _ => tm_within (by omega) ht) hp

theorem by_pAt {k : PubKey} {hk : C.Honest k} {u j t : Nat} {P : History → Prop} (hj : sAt u + 1 ≤ j)
    (ht : tm j ≤ t) (hp : P (H k (pAt k u + 1))) : (net hv).By k hk (max t 28 + 3) P :=
  by_of hv (fun _ => by rw [Nat.add_sub_cancel]; exact tm_pAt k hj ht) hp

theorem sentBy {k : PubKey} {hk : C.Honest k} {t : Nat} {m : Message} (hs : (net hv).SentByTime k hk t m) :
    ∃ j, tm j ≤ t ∧ Output.send m ∈ (tr k j).output :=
  Kit.sentBy (tm := tm) (fun _ _ => rfl) (fun _ _ _ => rfl) hs

include hv in
/-- A header for each view an honest node may propose in arrives within three steps. -/
theorem header_arrives {k : PubKey} {hk : C.Honest k} {n : Nat} {p : Proposal}
    (hready : ProposalReady cfg leader k (H k (n + 1)) p) :
    (net hv).By k hk (max (tm n) 28 + 3) fun hist =>
      ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
  obtain ⟨hlead, ⟨hlt, hnext, hep, hnum⟩, hj, -, -, hsafe, -, -⟩ := hready
  -- An honest leader: not of view two.
  have hv2 : p.viewNumber ≠ ⟨2⟩ := fun h => by
    rw [h] at hlead; simp only [leader, ite_true] at hlead
    exact d_faulty ((Option.some.inj hlead) ▸ hk)
  have hep0 : p.epoch = ⟨0⟩ := by rw [hep]; rfl
  have hc := hasCert1_of_certJustified hj
  have hcase : (p.parentCert = certOf anchorB ∧ p.viewNumber = ⟨1⟩)
      ∨ (p.parentCert = certOf (blk 0) ∧ p.viewNumber = ⟨3⟩ ∧ 6 ≤ n)
      ∨ ∃ i, p.parentCert = certOf (blk (i + 1)) ∧ p.viewNumber = ⟨i + 1 + 3⟩ ∧ sAt (i + 1) + 3 ≤ n := by
    rcases hnext with ⟨-, h⟩ | ⟨tc, hte, htv⟩
    · rcases hasCert1 hc with hpc | ⟨i, hpc, hi⟩ <;> rw [hpc] at h
      · exact Or.inl ⟨hpc, h.symm⟩
      · rcases i with _ | i
        · exact absurd h.symm hv2
        · refine Or.inr (Or.inr ⟨i, hpc, ?_, by omega⟩)
          rw [← h, certOf_view, vOf_succ]; rfl
    · unfold ParentJustified at hj
      rw [hte] at hj
      obtain ⟨-, m, htc, -⟩ := hj
      rw [upTo_H] at htc
      obtain ⟨rfl, h6⟩ := received_tc htc
      obtain ⟨-, hallow⟩ := hsafe TC hte
      have h6 : 6 ≤ n := by omega
      rcases hasCert1 hc with hpc | ⟨i, hpc, -⟩ <;> rw [hpc] at hallow hlt
      · exfalso
        rcases hallow with h | ⟨-, h | h⟩
        · rw [hep0] at h; exact absurd h (Nat.lt_irrefl 0)
        · exact absurd h (by show ¬ (1 : Nat) ≤ 0; omega)
        · have h1 : (1 : Nat) = 0 := congrArg (fun d => d.blockNumber.toNat) h
          omega
      · rw [← htv] at hlt
        have hi : vOf i < 3 := by rw [certOf_view] at hlt; exact hlt
        obtain rfl : i = 0 := by unfold vOf at hi; split at hi <;> omega
        exact Or.inr (Or.inl ⟨hpc, htv.symm, h6⟩)
  rcases hcase with ⟨hpc, hpv⟩ | ⟨hpc, hpv, hn⟩ | ⟨i, hpc, hpv, hi⟩
  · refine by_within hv (j := n) (n := 1) (by omega) (Nat.le_refl _) ⟨hdr 1, ?_, ?_⟩
    · rw [← hnum, hpc]; rfl
    · rw [hpv, hpc]; exact recv (k := k) 0 (by omega)
  · refine by_within hv (j := n) (n := 10) (by omega) (Nat.le_refl _) ⟨hdr 2, ?_, ?_⟩
    · rw [← hnum, hpc]
      show (⟨2⟩ : BlockNumber) = (blk 0).blockHeader.blockNumber + 1
      rw [blk_number]; rfl
    · rw [hpv, hpc]; exact recv_phase 0 0 (by omega) (by omega)
  · refine by_within hv (j := n) (n := sAt (i + 2) + 1) (by rw [sAt_succ]; rw [sAt_succ] at hi; omega)
      (Nat.le_refl _) ⟨hdr (i + 1 + 2), ?_, ?_⟩
    · rw [← hnum, hpc]
      show (⟨i + 1 + 2⟩ : BlockNumber) = (blk (i + 1)).blockHeader.blockNumber + 1
      rw [blk_number]; rfl
    · rw [hpv, hpc, sAt_succ]; exact recv_phase (i + 1) 0 (by omega) (by omega)

include hv in
/-- Every delivery within `Δ = 3` after GST `28`, with view timer `τ = 25`. -/
theorem sync : Synchrony (net hv) 28 3 25 where
  revote l hl n r hsend := absurd hsend (no_revote hv l n r)
  proposal l hl n p hsend _ k hk _ _ := by
    obtain ⟨rfl, u, rfl, hu⟩ := sent_proposal hv hsend
    exact by_within hv (j := n) (n := sAt u + 2) (by omega) (Nat.le_refl _)
      ⟨_, ⟨(blk_view u).symm, rfl⟩, recv_proposal u (by omega)⟩
  cert1 q dd v t hq hvotes k hk _ := by
    obtain ⟨hka, hs⟩ := hvotes a (quorum_a hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨u, heq, hu⟩ := sent_vote1 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    refine by_within hv (n := sAt u + 4) (by omega) hjt ?_
    rw [show (⟨(certOf (blk u)).data, ⟨vOf u⟩⟩ : Cert1) = certOf (blk u) by rw [← certOf_view u]]
    exact recv_cert1 u (by omega)
  cert2 q dd v t hq hvotes k hk _ := by
    obtain ⟨hkc, hs⟩ := hvotes c (quorum_c hq fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨u, heq, hu⟩ := sent_vote2 hv hj
    rcases u with _ | u
    · exact absurd (by rw [heq]; rfl) (c_skips_first hv hj)
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    rw [sAt_succ] at hu
    refine by_within hv (n := 9 + 6 * u + 6) (by omega) hjt ?_
    rw [vOf_succ]
    exact recv_phase u 5 (by omega) (by omega)
  cert2Spread x _ _ n hc _ _ _ := by
    obtain ⟨u, rfl, hu⟩ := hasCert2 hc
    exact by_within hv (j := n) (n := 9 + 6 * u + 6) (by omega) (Nat.le_refl _)
      (Or.inl (recv_phase u 5 (by omega) (by omega)))
  certSpread x _ _ n hc _ _ _ := by
    rcases hasCert1 hc with rfl | ⟨u, rfl, hu⟩
    · exact by_within hv (j := n) (n := 0) (by omega) (Nat.le_refl _) (Or.inl rfl)
    · exact by_within hv (j := n) (n := sAt u + 4) (by omega) (Nat.le_refl _)
        (Or.inr (Or.inl (recv_cert1 u (by omega))))
  lockSpread x _ _ n hc k' _ _ _ := by
    -- `c` can lock on the first block only after GST, but within `Δ` of it.
    rcases hasCert1 hc with rfl | ⟨u, rfl, hu⟩
    · exact by_within hv (j := n) (n := 0) (by omega) (Nat.le_refl _) (Or.inl rfl)
    · exact by_pAt hv (u := u) (j := n) (by omega) (Nat.le_refl _)
        (lockable_iff.mpr (Or.inr ⟨u, rfl, by omega⟩))
  blockSpread _ _ _ _ _ n hc _ _ _ := by
    rcases hasProposal hc.2 with rfl | ⟨u, rfl, hu⟩
    · exact by_within hv (j := n) (n := 0) (by omega) (Nat.le_refl _) (Or.inl rfl)
    · exact by_within hv (j := n) (n := sAt u + 2) (by omega) (Nat.le_refl _) (hasProposal_of u (by omega))
  timeoutCert e q v t hq hvotes k hk _ := by
    obtain ⟨hka, L, hsent⟩ := hvotes a (quorum_a (by cases e; exact hq) fun k hk => .of (hvotes k hk).1)
    obtain ⟨j, hjt, hj⟩ := sentBy hv hsent
    obtain ⟨rfl, heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
    obtain ⟨⟨rfl, -⟩, rfl, -⟩ := heq
    exact by_within hv (n := 7) (by omega) hjt ⟨TC, rfl, rfl, recv_tc (by omega)⟩
  timeoutOneHonest e q v t hq hvotes k hk _ := by
    obtain ⟨k0, hq0, -, hk0⟩ := C.intersect _ q q hq hq
    obtain ⟨L, hs⟩ := hvotes k0 hq0 hk0
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨rfl, heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨-, rfl, -⟩ := heq
    exact Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate TC))
      (fun _ h => Or.inr (Or.inl ⟨TC.view + 1, Or.inr (Or.inl ⟨TC, h, rfl⟩), by decide⟩))
      (by_within hv (j := 5) (n := 7) (by omega) hjt (recv_tc (by omega)))
  timeoutCertForward tc k hk n hin _ _ k' hk' _ := by
    obtain ⟨rfl, hlt⟩ := received_tc (hin ▸ Trace.received_self _ n : (H k (n + 1)).Received _)
    exact Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate TC))
      (fun _ h => ⟨TC.view + 1, Or.inr (Or.inl ⟨TC, h, rfl⟩), by decide⟩)
      (by_within hv (j := n) (n := 7) (by omega) (Nat.le_refl _) (recv_tc (by omega)))
  timeoutCatchUp k hk v hrep :=
    (Kit.catchUp_vacuous (N := net hv) (tm := tm) (fun _ _ => rfl) (fun _ _ _ => rfl) tm_mono 5
      (fun m vote hout _ => Nat.le_of_eq (sent_timeout hout).1) hrep).elim
  timeoutLockSpread tc _ _ n hin k' _ _ _ := by
    -- The lock `c` lacked when it signed: it can lock on it within `Δ` of GST.
    obtain ⟨rfl, hlt⟩ := received_tc hin
    exact by_pAt hv (u := 0) (j := n) (by simp [sAt]; omega) (Nat.le_refl _)
      (lockable_iff.mpr (Or.inr ⟨0, rfl, by omega⟩))
  epochChange _ _ _ hlast := absurd rfl hlast.2.1
  proposalValid _ _ _ p _ _ := hv p
  validatedSound _ _ _ _ _ _ b _ := hv b
  validated k hk n s p vid hin _ _ := by
    obtain ⟨u, rfl, -, rfl, -⟩ := input_proposal hin
    refine by_within hv (j := sAt u + 1) (n := sAt u + 3) (by omega) (Nat.le_refl _) ?_
    rw [blk_view]
    exact recv_validated u (by omega)
  header _ _ _ _ _ _ hready := header_arrives hv hready
  timeUnbounded _ _ T := ⟨T + 1, by show T < tm (T + 1); unfold tm; split <;> omega⟩
  timerNotEarly k _ n m v hin hnot hnm hinp := by
    obtain ⟨rfl, rfl⟩ := input_timeout hinp
    have h1 : 2 = viewAt (n + 1) := congrArg ViewNumber.toNat (inView_eq hin)
    have h2 : viewAt n ≠ 2 := fun h => by
      rcases hnot with rfl | hnot
      · exact absurd h (by decide)
      · exact hnot (by have := inView (k := k) n; rwa [h] at this)
    have hn : n = 3 := by
      revert h1 h2; unfold viewAt; repeat' split
      all_goals (intro h1 h2; omega)
    subst hn
    show tm 3 + 25 ≤ tm 5
    decide
  timerFires k _ n v hin _ := by
    have hv' := inView_eq hin
    subst hv'
    by_cases h3 : n < 3
    · have hlt : viewAt (n + 1) < viewAt 4 := by
        unfold viewAt; repeat' split
        all_goals omega
      exact ⟨3, h3, by show tm 3 ≤ tm n + 25; unfold tm; split <;> split <;> omega,
        Or.inr ⟨⟨viewAt 4⟩, hlt, inView 4⟩⟩
    by_cases h5 : n < 5
    · have h2 : viewAt (n + 1) = 2 := by
        unfold viewAt; repeat' split
        all_goals omega
      refine ⟨5, by omega, by show tm 5 ≤ tm n + 25; unfold tm; split <;> split <;> omega, Or.inl ?_⟩
      rw [h2]; rfl
    · have hlt : viewAt (n + 1) < viewAt (n + 7) := by
        unfold viewAt; repeat' split
        all_goals omega
      exact ⟨n + 6, by omega, by show tm (n + 6) ≤ tm n + 25; unfold tm; split <;> split <;> omega,
        Or.inr ⟨⟨viewAt (n + 7)⟩, hlt, inView (n + 7)⟩⟩

theorem rotation : LeaderRotation C leader := by
  intro _ v
  refine ⟨v + 3, show v.toNat ≤ v.toNat + 3 by omega, a, ?_, Or.inl rfl, Or.inl rfl⟩
  simp only [leader]
  split
  · rename_i h
    have : v.toNat + 3 = 2 := congrArg ViewNumber.toNat h
    omega
  · rfl

include hv in
/-- **The liveness premises can be met together, with a timeout before GST.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv) 28 3 25 ∧ Prompt (net hv) 0
      ∧ 8 * 3 + 3 * 0 < 25 ∧ LeaderRotation C leader :=
  ⟨cfg_coherent, sync hv, prompt_of_machine _ 0 (fun k _ => ⟨input k, rfl⟩)
    (fun _ hk => Kit.steady_of_uniform (fun _ _ _ h => h) hk), by decide, rotation⟩

include hv in
/--
And the certificate does name a lock `c` lacked: `c` times view two out locked on
the anchor, while the certificate names the first block's `Cert1`. Holding the
certificate, `c` still cannot lock on that `Cert1` until the payload arrives;
then it votes1 on the second block, which builds on the first behind the
certificate.
-/
theorem minority_lock :
    Output.send (.timeoutVote ⟨⟨⟨0⟩, certOf anchorB⟩, ⟨2⟩, c⟩) ∈ (tr c 5).output
      ∧ TC.data.lock = certOf (blk 0)
      ∧ (H c 8).Received (.timeoutCertificate TC) ∧ ¬ (H c 8).Lockable cfg TC.data.lock
      ∧ (H c 9).Lockable cfg TC.data.lock
      ∧ (blk 1).timeoutEvidence = some TC ∧ (blk 1).parentCert = TC.data.lock
      ∧ ∃ j, Output.send (.vote1 ⟨(certOf (blk 1)).data, ⟨3⟩, c⟩) ∈ (tr c j).output := by
  refine ⟨times_out c, rfl, recv_tc (by omega), fun h => ?_,
    lockable_iff.mpr (Or.inr ⟨0, rfl, by simp [pAt]⟩), rfl, rfl, ?_⟩
  · rcases lockable_iff.mp h with h | ⟨u, -, hu⟩
    · have h1 : (1 : Nat) = 0 := congrArg (fun x : Cert1 => x.data.blockNumber.toNat) h
      omega
    · exact (pAt_small hu (by omega)).2 rfl
  · obtain ⟨j, -, hj⟩ := votes1 hv c 1
    exact ⟨j, hj⟩

include hv in
/-- So every honest node keeps deciding. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : C.Honest k) :
    (net hv).DecidesAfter k hk t := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met hv
  exact (Liveness.chainGrows (cfg := cfg) (leader := leader) (C := C) (net hv) 28 3 0 25
    hc hcf hs hp hb hr).2 t k hk (Or.inl (Kit.steady_of_uniform (fun _ _ _ h => h) hk))

end Net

end MinorityLockWitness
end NewProtocolImpl
