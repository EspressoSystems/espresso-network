module

public import NewProtocolImpl.FourNodes
public import NewProtocolImpl.WitnessKit

/-!
# A network with a faulty leader that equivocates

The four-node committee (`FourNodes`), `a`, `b` and `c` honest and `d` faulty, any three a
quorum, in every epoch. With epochs, `d` is honest in the first epoch, where it
leads no view, and is a member that votes like the others; it is faulty in every
later epoch (`honest_once`). So it is held to the protocol in the first epoch only
(`settledIn`), and is not promised to decide. The leaders rotate: `a`, `b`, `c` and `d` lead views
`4w + 1` to `4w + 4`, and round `w + 1` holds three honest blocks, at views
`4w + 1` to `4w + 3`. In its view `d` equivocates: it proposes one block to `a`
and another to `b`, both on the round's last block, and proposes nothing to `c`.
`a` and `b` each vote for the block they were sent, so no `Cert1` forms, and the
view times out (`equivocation`). The next leader builds on the round's last
block, with the timeout certificate as evidence.

The run comes in two forms, chosen by `B`. Without epochs (`B = false`) all of it
is one epoch. With epochs (`B = true`) each round is an epoch of three blocks, so
`d`'s view comes right after each epoch's last block and its blocks would open the
next epoch (`boundary`). `a` and `b` still vote for them, since they hold the
`Cert2` over the parent (`OpensEpochJustified`). Every node takes the epoch change
before its timer fires, so the timeout votes and the timeout certificate are of
the new epoch, and the next leader opens the epoch behind the certificate. No
honest node asks for a re-vote: the only view it could ask in is the one after
`d`'s, and by then it has left the epoch of the last block (`NotBehind`).

Each of `d`'s blocks reaches only the node it was proposed to, and `c` holds
neither. `blockSpread` asks nothing for them: with distinct block hashes
(`CollisionFree`, taken as a hypothesis), no `Cert1` a node holds is over one.

`c` is slow: every `Cert1` reaches it two steps after the others, and every
payload one step earlier, so it enters each view later and its timer for `d`'s
view fires later. Before it does, the one-honest indication from `a`'s and `b`'s
timeout votes arrives, and `c` answers it (`ProtocolHistory.timeoutAnswered`);
that vote completes the quorum behind the timeout certificate.

Each honest leader's view takes the header, the proposal with the node's share,
the validity report, the `Cert1`, the payload, a second validity report, and the
`Cert2`, with `c`'s `Cert1` and payload moved as above. `d`'s view takes the
proposals with their validity reports at `a` and `b`, the epoch change (with
epochs; without, a step with nothing new), a step with nothing new, the timers of
`a` and `b`, the one-honest indication, `c`'s timer, and the timeout certificate.

The definitions that depend on `B` are written with a prime and taking `B`, and
used through local notations that supply it, so that the proofs read the same for
both forms.

Times (`tm`): one unit a step, except that the timers for `d`'s view fire
`τ = 33` after each node entered it. GST is zero, `Δ = 4` and `δ = 0`.

`BlockValid` is taken as a hypothesis, being opaque.
-/

@[expose] public section

namespace NewProtocolImpl
namespace ByzantineWitness

open NewProtocol History
open FourNodes (hdr certOf)
open Kit (received received_mono upTo_H upTo_self getElem_H tr_step sent_iff view_inj number_inj lockLE_antisymm)
open FourNodes (a b c d)

variable {B : Bool}

/-- The epoch of honest block `u`: with epochs, `u / 3 + 1`, three to an epoch; without, zero. -/
def ep : Bool → Nat → Nat
  | false, _ => 0
  | true, u => u / 3 + 1

/-- Blocks to an epoch: three, or none, for a single epoch. -/
def height : Bool → Nat
  | false => 0
  | true => 3

theorem ep_mono {j m : Nat} (h : j ≤ m) : ep B j ≤ ep B m := by cases B <;> simp only [ep] <;> omega

/-- The blocks of a round are of one epoch. -/
theorem ep_round (w i : Nat) (hi : i < 3) : ep B (3 * w + i) = ep B (3 * w) := by
  cases B <;> simp only [ep] <;> omega

theorem ep_of (hB : B = true) (u : Nat) : ep B u = u / 3 + 1 := by subst hB; rfl

/-- The anchor, in the first epoch. -/
def anchorB' (B : Bool) : Block :=
  ⟨⟨⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis, ⟨ep B 0⟩, ⟨⟨⟨0⟩, ⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis⟩, none, ⟨7⟩⟩

set_option hygiene false in
local notation "anchorB" => anchorB' B

def cfg' (B : Bool) : Config where
  anchorBlock := anchorB
  anchorCert := certOf anchorB
  decideBuffer := 20
  epochHeight := height B

set_option hygiene false in
local notation "cfg" => cfg' B

theorem cfg_coherent : ConfigCoherent cfg where
  anchorCertView := rfl
  anchorBlockEpoch := rfl
  anchorCertBlock := rfl
  anchorCertBlockNumber := rfl
  anchorCertEpoch := by cases B <;> rfl

/-! ## Nodes and committees -/

/--
The four-node committee (`FourNodes`), any three of `a`, `b`, `c` and `d` a quorum in every
epoch. With epochs, `d` is honest in the first epoch, where it leads no view, and
faulty in every later one, where it equivocates.
-/
def C' (B : Bool) : Committee where
  honest e k := FourNodes.C.honest e k ∨ (B = true ∧ k = d ∧ e = ⟨1⟩)
  members := FourNodes.C.members
  Quorum := FourNodes.C.Quorum
  intersect e q q' hq hq' :=
    let ⟨k, h1, h2, hk⟩ := FourNodes.C.intersect e q q' hq hq'
    ⟨k, h1, h2, Or.inl hk⟩
  honestFinite := ⟨[a, b, c, d], fun _ k hk => by
    rcases hk with (rfl | rfl | rfl) | ⟨-, rfl, -⟩ <;> simp⟩

/-- The honest members of every epoch's committee are a quorum. -/
theorem members_quorum : ∀ e, (C' B).Quorum e fun k => (C' B).members e k ∧ (C' B).honest e k := fun _ =>
  Or.inl ⟨⟨Or.inl rfl, Or.inl (Or.inl rfl)⟩, ⟨Or.inr (Or.inl rfl), Or.inl (Or.inr (Or.inl rfl))⟩,
    ⟨Or.inr (Or.inr (Or.inl rfl)), Or.inl (Or.inr (Or.inr rfl))⟩⟩

/-- `d` is honest only in the first epoch, and only with epochs. -/
theorem d_honest {e : EpochNumber} (h : (C' B).honest e d) : B = true ∧ e = ⟨1⟩ := by
  rcases h with (h | h | h) | ⟨hB, -, he⟩
  · cases h
  · cases h
  · cases h
  · exact ⟨hB, he⟩

/-- Every node but `d` is honest in every epoch it is honest in any. -/
theorem steady {k : PubKey} (hk : (C' B).Honest k) (hd : k ≠ d) : (C' B).Steady k := fun _ => by
  obtain ⟨_, h | ⟨-, h, -⟩⟩ := hk
  · exact Or.inl h
  · exact absurd h hd

/-- A quorum of honest members holds `c`, or holds `a` and is of the first epoch. -/
theorem quorum_voter {e : EpochNumber} {q : PubKey → Prop} (hq : (C' B).Quorum e q) (hh : ∀ k, q k → (C' B).honest e k) :
    q c ∨ (q a ∧ e = ⟨1⟩) := by
  rcases hq with ⟨-, -, h⟩ | ⟨h1, -, h3⟩ | ⟨-, h, -⟩ | ⟨-, h, -⟩
  · exact Or.inl h
  · exact Or.inr ⟨h1, (d_honest (hh d h3)).2⟩
  · exact Or.inl h
  · exact Or.inl h

/-! ## Leaders and blocks -/

/-- The leader of view `v`: `a`, `b`, `c` and `d` in turn. -/
def ldr (v : Nat) : PubKey := if v % 4 = 1 then a else if v % 4 = 2 then b else if v % 4 = 3 then c else d

/--
The leaders in turn, except that `d` leads no view of the first epoch: with epochs,
it is honest there (`C'`).
-/
def leader : EpochNumber → ViewNumber → Option PubKey := fun e v =>
  if ldr v.toNat = d ∧ e = ⟨1⟩ then none else some (ldr v.toNat)

theorem leader_some {e : EpochNumber} {v : ViewNumber} {k : PubKey} (h : leader e v = some k) :
    k = ldr v.toNat ∧ (k = d → e ≠ ⟨1⟩) := by
  unfold leader at h
  split at h
  · cases h
  · rename_i hn
    obtain rfl := Option.some.inj h
    exact ⟨rfl, fun hd he => hn ⟨hd, he⟩⟩

theorem leader_of {e : EpochNumber} {v : Nat} (h : ldr v ≠ d ∨ e ≠ ⟨1⟩) : leader e ⟨v⟩ = some (ldr v) := by
  unfold leader
  rw [ite_eq_right_iff.mpr fun ⟨h1, h2⟩ => absurd h1 (h.elim id fun he => absurd h2 he)]

/-- The view of honest block `u`: three to a round of four views, the fourth `d`'s. -/
def vOf (u : Nat) : Nat := 4 * (u / 3) + u % 3 + 1

theorem vOf_at (w i : Nat) (hi : i < 3) : vOf (3 * w + i) = 4 * w + i + 1 := by unfold vOf; omega

/--
Honest block `u`, at height `u + 1`, each on the one before. The first of a round
after the first follows `d`'s view, behind its timeout certificate; with epochs, it
opens its epoch.
-/
def blk' (B : Bool) : Nat → Block
  | 0 => ⟨hdr 1, ⟨1⟩, ⟨ep B 0⟩, certOf anchorB, none, ⟨0⟩⟩
  | u + 1 => ⟨hdr (u + 2), ⟨vOf (u + 1)⟩, ⟨ep B (u + 1)⟩, certOf (blk' B u),
      if (u + 1) % 3 = 0 then some ⟨⟨⟨ep B (u + 1)⟩, certOf (blk' B u)⟩, ⟨vOf (u + 1) - 1⟩⟩ else none, ⟨0⟩⟩

set_option hygiene false in
local notation "blk" => blk' B

/-- The parent of `blk u`. -/
def parentOf' (B : Bool) : Nat → Block
  | 0 => anchorB
  | u + 1 => blk u

set_option hygiene false in
local notation "parentOf" => parentOf' B

/--
The timeout certificate for `d`'s view of round `w + 1`, locked on the round's last
block, and of the next round's epoch.
-/
def T' (B : Bool) (w : Nat) : TimeoutCert := ⟨⟨⟨ep B (3 * w + 3)⟩, certOf (blk (3 * w + 2))⟩, ⟨4 * w + 4⟩⟩

set_option hygiene false in
local notation "T" => T' B

/-- The `Cert2` over `blk u`. -/
def C2' (B : Bool) (u : Nat) : Cert2 := ⟨(certOf (blk u)).data.toVote2, ⟨vOf u⟩⟩

set_option hygiene false in
local notation "C2" => C2' B

/--
`d`'s two blocks for its view of round `w + 1`, both on the round's last block, and
of the next round's epoch: one for `a`, one for `b`.
-/
def E1' (B : Bool) (w : Nat) : Block :=
  ⟨hdr (3 * w + 4), ⟨4 * w + 4⟩, ⟨ep B (3 * w + 3)⟩, certOf (blk (3 * w + 2)), none, ⟨1⟩⟩

set_option hygiene false in
local notation "E1" => E1' B

def E2' (B : Bool) (w : Nat) : Block :=
  ⟨hdr (3 * w + 4), ⟨4 * w + 4⟩, ⟨ep B (3 * w + 3)⟩, certOf (blk (3 * w + 2)), none, ⟨2⟩⟩

set_option hygiene false in
local notation "E2" => E2' B

theorem blk_view (u : Nat) : (blk u).viewNumber = ⟨vOf u⟩ := by cases u <;> rfl

theorem blk_number (u : Nat) : (blk u).blockHeader.blockNumber = ⟨u + 1⟩ := by cases u <;> rfl

theorem blk_epoch (u : Nat) : (blk u).epoch = ⟨ep B u⟩ := by cases u <;> rfl

theorem blk_header (u : Nat) : (blk u).blockHeader = hdr (u + 1) := by cases u <;> rfl

theorem blk_parent (u : Nat) : (blk u).parentCert = certOf (parentOf u) := by cases u <;> rfl

theorem blk_evidence_none {u : Nat} (h : u = 0 ∨ u % 3 ≠ 0) : (blk u).timeoutEvidence = none := by
  cases u with
  | zero => rfl
  | succ u =>
    show (if (u + 1) % 3 = 0 then _ else none) = none
    rw [ite_eq_right (by omega)]

theorem blk_evidence (w : Nat) : (blk (3 * w + 3)).timeoutEvidence = some (T w) := by
  show (if (3 * w + 2 + 1) % 3 = 0 then _ else none) = _
  rw [ite_eq_left (by omega)]
  unfold T'
  rw [show vOf (3 * w + 2 + 1) - 1 = 4 * w + 4 by unfold vOf; omega]

theorem cert_view (u : Nat) : (certOf (blk u)).view = ⟨vOf u⟩ := blk_view u

theorem cert_number (u : Nat) : (certOf (blk u)).data.blockNumber = ⟨u + 1⟩ := blk_number u

theorem cert_epoch (u : Nat) : (certOf (blk u)).data.epoch = ⟨ep B u⟩ := blk_epoch u

/-- The epoch a height falls in: three to an epoch. -/
theorem epochOf_number (u : Nat) : epochOf ⟨u + 1⟩ (cfg).epochHeight = ⟨ep B u⟩ := by
  cases B
  · rfl
  · show epochOf ⟨u + 1⟩ 3 = _
    simp only [epochOf_eq, ep]
    repeat' split
    all_goals first | (exfalso; omega) | (congr 1; omega)

/-- A height is the last of an epoch only with epochs, and every third one then. -/
theorem last_iff {x : Nat} : IsLastBlock ⟨x⟩ (cfg).epochHeight ↔ B = true ∧ x ≠ 0 ∧ x % 3 = 0 := by
  cases B
  · exact ⟨fun h => absurd rfl h.2.1, fun h => absurd h.1 (by decide)⟩
  · rw [isLastBlock_iff]
    show (x ≠ 0 ∧ 3 ≠ 0 ∧ x % 3 = 0) ↔ (true = true ∧ x ≠ 0 ∧ x % 3 = 0)
    exact ⟨fun ⟨h1, _, h3⟩ => ⟨rfl, h1, h3⟩, fun ⟨_, h1, h3⟩ => ⟨h1, by decide, h3⟩⟩

/-- With epochs, the first block of every round but the first opens its epoch. -/
theorem blk_enters {u : Nat} : EntersEpoch cfg (blk u) ↔ B = true ∧ u ≠ 0 ∧ u % 3 = 0 := by
  show IsLastBlock ((blk u).blockHeader.blockNumber - 1) (cfg).epochHeight ↔ _
  rw [blk_number]
  exact last_iff

/-- With epochs, the last block of each epoch is the third of each round. -/
theorem blk_last {u : Nat} : IsLastBlock (blk u).blockHeader.blockNumber (cfg).epochHeight ↔ B = true ∧ u % 3 = 2 := by
  rw [blk_number, last_iff]
  exact ⟨fun ⟨hB, _, h⟩ => ⟨hB, by omega⟩, fun ⟨hB, h⟩ => ⟨hB, by omega, by omega⟩⟩

theorem parentOf_number (u : Nat) : (parentOf u).blockHeader.blockNumber = ⟨u⟩ := by
  cases u with
  | zero => rfl
  | succ u => exact blk_number u

theorem ldr_at (w : Nat) : ldr (4 * w + 1) = a ∧ ldr (4 * w + 2) = b ∧ ldr (4 * w + 3) = c ∧ ldr (4 * w + 4) = d := by
  unfold ldr
  refine ⟨ite_eq_left (by omega), ?_, ?_, ?_⟩
  · rw [ite_eq_right (by omega), ite_eq_left (by omega)]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega), ite_eq_left (by omega)]
  · rw [ite_eq_right (by omega), ite_eq_right (by omega), ite_eq_right (by omega)]

/-- The proposer of honest block `u`: `a`, `b` or `c`. -/
theorem ldr_blk (u : Nat) : (ldr (vOf u) = a ∨ ldr (vOf u) = b ∨ ldr (vOf u) = c) ∧ ldr (vOf u) ≠ d := by
  obtain ⟨h1, h2, h3, -⟩ := ldr_at (u / 3)
  rw [show vOf u = 4 * (u / 3) + u % 3 + 1 from rfl]
  rcases (show u % 3 = 0 ∨ u % 3 = 1 ∨ u % 3 = 2 by omega) with h | h | h <;> rw [h]
  · exact ⟨Or.inl h1, by rw [h1]; decide⟩
  · exact ⟨Or.inr (Or.inl h2), by rw [h2]; decide⟩
  · exact ⟨Or.inr (Or.inr h3), by rw [h3]; decide⟩

/-- An honest block's view is led by its proposer, in every epoch. -/
theorem leader_blk (e : EpochNumber) (u : Nat) : leader e ⟨vOf u⟩ = some (ldr (vOf u)) :=
  leader_of (Or.inl (ldr_blk u).2)

/-! ## The schedule -/

/-- `c` gets each `Cert1` two steps late. -/
def slow (k : PubKey) : Prop := k = c

instance (k : PubKey) : Decidable (slow k) := inferInstanceAs (Decidable (k = c))

/-- What node `k` receives at step `r` of honest block `u`'s view. -/
def viewPhase' (B : Bool) (k : PubKey) (u : Nat) : Nat → Input
  | 0 => .headerBuilt ⟨vOf u⟩ (blockHash (parentOf u)) (hdr (u + 1))
  | 1 => .proposal (ldr (vOf u)) (blk u) (some ⟨⟨vOf u⟩, (blk u).payloadCommit⟩)
  | 2 => .blockValidated ⟨vOf u⟩ (blockHash (blk u))
  | 3 => if slow k then .blockReconstructed ⟨vOf u⟩ (blk u).payloadCommit else .certificate1 (certOf (blk u))
  | 4 => if slow k then .blockValidated ⟨vOf u⟩ (blockHash (blk u))
      else .blockReconstructed ⟨vOf u⟩ (blk u).payloadCommit
  | 5 => if slow k then .certificate1 (certOf (blk u)) else .blockValidated ⟨vOf u⟩ (blockHash (blk u))
  | _ => .certificate2 (C2 u)

set_option hygiene false in
local notation "viewPhase" => viewPhase' B

/-- A validity report again, of the round's last block: what `c` gets while `d` equivocates. -/
def again' (B : Bool) (w : Nat) : Input := .blockValidated ⟨4 * w + 3⟩ (blockHash (blk (3 * w + 2)))

set_option hygiene false in
local notation "again" => again' B

/--
What node `k` receives at step `s` of `d`'s view in round `w + 1`: its proposal
and validity report at `a` and `b`, nothing new at `c`, then the epoch change,
the timers, the one-honest indication and the timeout certificate.
-/
def faultyPhase' (B : Bool) (k : PubKey) (w : Nat) : Nat → Input
  | 0 => if k = a then .proposal d (E1 w) (some ⟨⟨4 * w + 4⟩, (E1 w).payloadCommit⟩)
      else if k = b then .proposal d (E2 w) (some ⟨⟨4 * w + 4⟩, (E2 w).payloadCommit⟩) else again w
  | 1 => if k = a then .blockValidated ⟨4 * w + 4⟩ (blockHash (E1 w))
      else if k = b then .blockValidated ⟨4 * w + 4⟩ (blockHash (E2 w)) else again w
  | 2 => if B then .epochChange (certOf (blk (3 * w + 2))) (C2 (3 * w + 2)) (blk (3 * w + 2)) else again w
  | 3 => again w
  | 4 => if slow k then again w else .timeout ⟨4 * w + 4⟩
  | 5 => .timeoutOneHonest ⟨4 * w + 4⟩
  | 6 => if slow k then .timeout ⟨4 * w + 4⟩ else again w
  | _ => .timeoutCertificate (T w)

set_option hygiene false in
local notation "faultyPhase" => faultyPhase' B

/-- The base step of honest block `u`'s view. -/
def sAt (u : Nat) : Nat := 29 * (u / 3) + 7 * (u % 3)

/-- Twenty-nine steps to a round: three honest views of seven, then `d`'s of eight. -/
def input' (B : Bool) (k : PubKey) (n : Nat) : Input :=
  if n % 29 < 21 then viewPhase k (3 * (n / 29) + n % 29 / 7) (n % 29 % 7) else faultyPhase k (n / 29) (n % 29 - 21)

set_option hygiene false in
local notation "input" => input' B

set_option hygiene false in
local notation "tr" => Kit.tr cfg leader input

set_option hygiene false in
local notation "H" => Kit.H cfg leader input

/-- A step of an honest view, or of `d`'s. -/
theorem steps (n : Nat) : (∃ u r, r < 7 ∧ n = sAt u + r) ∨ ∃ w s, s < 8 ∧ n = 29 * w + 21 + s := by
  by_cases h : n % 29 < 21
  · exact Or.inl ⟨3 * (n / 29) + n % 29 / 7, n % 29 % 7, Nat.mod_lt _ (by omega), by unfold sAt; omega⟩
  · exact Or.inr ⟨n / 29, n % 29 - 21, by omega, by omega⟩

theorem sAt_at (w i : Nat) (hi : i < 3) : sAt (3 * w + i) = 29 * w + 7 * i := by unfold sAt; omega

/-- Division by the round's length, which `omega` does not always see through. -/
theorem split29 (w x : Nat) (hx : x < 29) : (29 * w + x) / 29 = w ∧ (29 * w + x) % 29 = x :=
  ⟨by rw [Nat.mul_add_div (by decide), Nat.div_eq_of_lt hx, Nat.add_zero],
    by rw [Nat.mul_add_mod, Nat.mod_eq_of_lt hx]⟩

theorem input_view {k : PubKey} (u r : Nat) (hr : r < 7) : input k (sAt u + r) = viewPhase k u r := by
  obtain ⟨w, i, hi, rfl⟩ : ∃ w i, i < 3 ∧ u = 3 * w + i := ⟨u / 3, u % 3, Nat.mod_lt _ (by omega), by omega⟩
  rw [sAt_at w i hi, Nat.add_assoc]
  obtain ⟨h1, h2⟩ := split29 w (7 * i + r) (by omega)
  unfold input'
  rw [h1, h2, ite_eq_left (by omega), show (7 * i + r) / 7 = i by omega, show (7 * i + r) % 7 = r by omega]

theorem input_faulty {k : PubKey} (w s : Nat) (hs : s < 8) : input k (29 * w + 21 + s) = faultyPhase k w s := by
  rw [Nat.add_assoc]
  obtain ⟨h1, h2⟩ := split29 w (21 + s) (by omega)
  unfold input'
  rw [h1, h2, ite_eq_right (by omega), show 21 + s - 21 = s by omega]

/-- Honest block `u` as the `i`-th of round `w + 1`: its base step and view, without division. -/
theorem sAt_vOf (u : Nat) : ∃ w i, i < 3 ∧ u = 3 * w + i ∧ sAt u = 29 * w + 7 * i ∧ vOf u = 4 * w + i + 1 :=
  ⟨u / 3, u % 3, Nat.mod_lt _ (by omega), by omega, by unfold sAt; omega, by unfold vOf; omega⟩

/-- Steps and views grow with the block, as facts `omega` does not find on its own. -/
theorem sAt_succ (u : Nat) : sAt u + 7 ≤ sAt (u + 1) := by
  obtain ⟨w, i, hi, rfl, hs, -⟩ := sAt_vOf u
  obtain ⟨w', i', hi', hu', hs', -⟩ := sAt_vOf (3 * w + i + 1)
  rw [hs, hs']
  by_cases h : i = 2
  · subst h; obtain rfl : w' = w + 1 := by omega
    omega
  · obtain rfl : w' = w := by omega
    omega

theorem vOf_mono {j u : Nat} (h : j ≤ u) : vOf j ≤ vOf u := by
  obtain ⟨wj, ij, hij, rfl, -, hvj⟩ := sAt_vOf j
  obtain ⟨w, i, hi, rfl, -, hv⟩ := sAt_vOf u
  rw [hvj, hv]
  by_cases h1 : wj < w
  · omega
  · by_cases h2 : w < wj
    · omega
    · obtain rfl : wj = w := by omega
      omega

theorem sAt_mono {j u : Nat} (h : j ≤ u) : sAt j ≤ sAt u :=
  Kit.mono_of_succ (f := sAt) (fun n => by have := sAt_succ n; omega) h

theorem vOf_succ (u : Nat) : vOf u + 1 ≤ vOf (u + 1) := by unfold vOf; omega

/-- An honest block whose view starts before `d`'s view of round `w + 1` ends is of that round or earlier. -/
theorem idx_round {j w : Nat} (h : sAt j < 29 * w + 29) : j ≤ 3 * w + 2 := by
  obtain ⟨wj, ij, hij, rfl, hsj, -⟩ := sAt_vOf j
  rw [hsj] at h
  by_cases h1 : wj ≤ w
  · omega
  · omega

/-- And one whose view ends after `d`'s view of round `w + 1` starts is of a later round. -/
theorem round_idx {w u : Nat} (h : 29 * w + 21 ≤ sAt u + 6) : 3 * w + 3 ≤ u := by
  obtain ⟨wu, iu, hiu, rfl, hsu, -⟩ := sAt_vOf u
  rw [hsu] at h
  by_cases h1 : w < wu
  · omega
  · omega

theorem r7 (r : Nat) (h : r < 7) : r = 0 ∨ r = 1 ∨ r = 2 ∨ r = 3 ∨ r = 4 ∨ r = 5 ∨ r = 6 := by omega

theorem r8 (s : Nat) (h : s < 8) : s = 0 ∨ s = 1 ∨ s = 2 ∨ s = 3 ∨ s = 4 ∨ s = 5 ∨ s = 6 ∨ s = 7 := by omega

/-- The four kinds of node: `a`, `b`, `c`, and any other. -/
theorem kinds (k : PubKey) : (k = a ∧ ¬ slow k) ∨ (k = b ∧ k ≠ a ∧ ¬ slow k) ∨ (k = c ∧ k ≠ a ∧ k ≠ b ∧ slow k)
    ∨ (k ≠ a ∧ k ≠ b ∧ ¬ slow k) := by
  by_cases ha : k = a
  · subst ha; exact Or.inl ⟨rfl, by decide⟩
  by_cases hb : k = b
  · subst hb; exact Or.inr (Or.inl ⟨rfl, by decide, by decide⟩)
  by_cases hc : k = c
  · subst hc; exact Or.inr (Or.inr (Or.inl ⟨rfl, by decide, by decide, rfl⟩))
  · exact Or.inr (Or.inr (Or.inr ⟨ha, hb, hc⟩))

/-- When a node gets an honest block's `Cert1` and payload, from the base of its view. -/
def c1Off (k : PubKey) : Nat := if slow k then 5 else 3

def pOff (k : PubKey) : Nat := if slow k then 3 else 4

/-- When a node's timer for `d`'s view fires, from the base of that view. -/
def tOff (k : PubKey) : Nat := if slow k then 6 else 4

theorem offs (k : PubKey) : (slow k ∧ c1Off k = 5 ∧ pOff k = 3 ∧ tOff k = 6)
    ∨ (¬ slow k ∧ c1Off k = 3 ∧ pOff k = 4 ∧ tOff k = 4) := by
  by_cases h : slow k
  · exact Or.inl ⟨h, by simp [c1Off, h], by simp [pOff, h], by simp [tOff, h]⟩
  · exact Or.inr ⟨h, by simp [c1Off, h], by simp [pOff, h], by simp [tOff, h]⟩

/-- The offsets alone, as facts `omega` reads. -/
theorem offs_nat (k : PubKey) : (c1Off k = 5 ∧ pOff k = 3 ∧ tOff k = 6) ∨ (c1Off k = 3 ∧ pOff k = 4 ∧ tOff k = 4) := by
  rcases offs k with ⟨-, h⟩ | ⟨-, h⟩
  · exact Or.inl h
  · exact Or.inr h

/-! ## What the nodes receive -/

section Inputs

variable {k : PubKey}

theorem input_header {n : Nat} {v : ViewNumber} {h : BlockHash} {x : BlockHeader}
    (hi : input k n = .headerBuilt v h x) :
    ∃ u, n = sAt u ∧ v = ⟨vOf u⟩ ∧ h = blockHash (parentOf u) ∧ x = hdr (u + 1) := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
    all_goals exact ⟨u, rfl, hi.1.symm, hi.2.1.symm, hi.2.2.symm⟩
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi

theorem input_proposal {n : Nat} {s : PubKey} {p : Proposal} {vid : VidShare}
    (hi : input k n = .proposal s p (some vid)) :
    (∃ u, n = sAt u + 1 ∧ s = ldr (vOf u) ∧ p = blk u ∧ vid = ⟨⟨vOf u⟩, (blk u).payloadCommit⟩)
      ∨ ∃ w, n = 29 * w + 21 ∧ s = d
        ∧ ((k = a ∧ p = E1 w ∧ vid = ⟨⟨4 * w + 4⟩, (E1 w).payloadCommit⟩)
          ∨ (k = b ∧ p = E2 w ∧ vid = ⟨⟨4 * w + 4⟩, (E2 w).payloadCommit⟩)) := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s', hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
    all_goals exact Or.inl ⟨u, rfl, hi.1.symm, hi.2.1.symm, hi.2.2.symm⟩
  · rw [input_faulty w s' hs] at hi
    rcases r8 s' hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi
    · split at hi
      · rename_i ha
        simp only [Input.proposal.injEq, Option.some.injEq] at hi
        exact Or.inr ⟨w, rfl, hi.1.symm, Or.inl ⟨ha, hi.2.1.symm, hi.2.2.symm⟩⟩
      · split at hi
        · rename_i hb
          simp only [Input.proposal.injEq, Option.some.injEq] at hi
          exact Or.inr ⟨w, rfl, hi.1.symm, Or.inr ⟨hb, hi.2.1.symm, hi.2.2.symm⟩⟩
        · simp at hi
    all_goals ((try split at hi) <;> (try split at hi) <;> simp at hi)

theorem input_cert1 {n : Nat} {x : Cert1} (hi : input k n = .certificate1 x) :
    ∃ u, n = sAt u + c1Off k ∧ x = certOf (blk u) := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
    all_goals exact ⟨u, by simp [c1Off, hk], hi.symm⟩
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi

theorem input_payload {n : Nat} {v : ViewNumber} {pc : PayloadCommit}
    (hi : input k n = .blockReconstructed v pc) :
    ∃ u, n = sAt u + pOff k ∧ v = ⟨vOf u⟩ ∧ pc = (blk u).payloadCommit := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
    all_goals exact ⟨u, by simp [pOff, hk], hi.1.symm, hi.2.symm⟩
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi

theorem input_cert2 {n : Nat} {x : Cert2} (hi : input k n = .certificate2 x) :
    ∃ u, n = sAt u + 6 ∧ x = C2 u := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
    all_goals exact ⟨u, rfl, hi.symm⟩
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi

theorem input_timeout {n : Nat} {v : ViewNumber} (hi : input k n = .timeout v) :
    ∃ w, n = 29 * w + 21 + tOff k ∧ v = ⟨4 * w + 4⟩ := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi
    all_goals (rename_i hk; exact ⟨w, by simp [tOff, hk], hi.symm⟩)

theorem input_oneHonest {n : Nat} {v : ViewNumber} (hi : input k n = .timeoutOneHonest v) :
    ∃ w, n = 29 * w + 21 + 5 ∧ v = ⟨4 * w + 4⟩ := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi
    all_goals exact ⟨w, rfl, hi.symm⟩

theorem input_tc {n : Nat} {tc : TimeoutCert} (hi : input k n = .timeoutCertificate tc) :
    ∃ w, n = 29 * w + 21 + 7 ∧ tc = T w := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi
    all_goals exact ⟨w, rfl, hi.symm⟩

/-- No proposal arrives without a share. -/

theorem input_proposal_none {n : Nat} {s : PubKey} {p : Proposal} (hi : input k n = .proposal s p none) : False := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi

/-- Every proposal a node receives comes with its share. -/
theorem input_share {n : Nat} {s : PubKey} {p : Proposal} {share : Option VidShare}
    (hi : input k n = .proposal s p share) : ∃ vid, share = some vid := by
  cases share with
  | some vid => exact ⟨vid, rfl⟩
  | none => exact (input_proposal_none hi).elim

theorem input_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hi : input k n = .epochChange c1 c2 p) :
    ∃ w, B = true ∧ n = 29 * w + 21 + 2 ∧ c1 = certOf (blk (3 * w + 2)) ∧ c2 = C2 (3 * w + 2)
      ∧ p = blk (3 * w + 2) := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr] at hi
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [viewPhase', hk] at hi
  · rw [input_faulty w s hs] at hi
    rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp only [faultyPhase', again'] at hi <;>
      (try split at hi) <;> (try split at hi) <;> simp at hi
    all_goals exact ⟨w, by assumption, rfl, hi.1.symm, hi.2.1.symm, hi.2.2.symm⟩

/-- Nobody hands over a re-vote request. -/
theorem input_none {n : Nat} : ∀ s r, input k n ≠ .revote s r := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [input_view u r hr]
    by_cases hk : slow k <;> rcases r7 r hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [viewPhase', hk]
  · rw [input_faulty w s hs]
    by_cases ha : k = a <;> by_cases hb : k = b <;> by_cases hk : slow k <;> cases B <;>
      rcases r8 s hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      simp [faultyPhase', again', ha, hb, hk, show ¬ slow a by decide, show ¬ slow b by decide, show b ≠ a by decide]

end Inputs

/-! ## What the nodes hold -/

section Holds

open Lists

variable {k : PubKey}

theorem blk_wellFormed (u : Nat) : ProposalWellFormed cfg (blk u) := by
  cases u with
  | zero => exact ⟨show (0 : Nat) < 1 by omega, Or.inl ⟨rfl, rfl⟩, (epochOf_number 0).symm, rfl⟩
  | succ u =>
    refine ⟨?_, ?_, by rw [blk_epoch, blk_number]; exact (epochOf_number (u + 1)).symm, ?_⟩
    · show (certOf (blk u)).view.toNat < vOf (u + 1)
      rw [cert_view]; show vOf u < vOf (u + 1); have := vOf_succ u; omega
    · by_cases h : (u + 1) % 3 = 0
      · obtain ⟨w, hw⟩ : ∃ w, u + 1 = 3 * w + 3 := ⟨(u + 1) / 3 - 1, by omega⟩
        refine Or.inr ⟨T w, by rw [hw, blk_evidence], ?_⟩
        show (⟨4 * w + 4 + 1⟩ : ViewNumber) = ⟨vOf (u + 1)⟩
        congr 1; unfold vOf; omega
      · refine Or.inl ⟨blk_evidence_none (Or.inr h), ?_⟩
        show (certOf (blk u)).view + 1 = ⟨vOf (u + 1)⟩
        rw [cert_view]; show (⟨vOf u + 1⟩ : ViewNumber) = _
        congr 1; unfold vOf; omega
    · show (certOf (blk u)).data.blockNumber + 1 = ⟨u + 2⟩
      rw [cert_number]; rfl

theorem blk_safe (u : Nat) : SafeParent (blk u) := by
  intro tc h
  by_cases hu : u = 0 ∨ u % 3 ≠ 0
  · rw [blk_evidence_none hu] at h; cases h
  · obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 3 := ⟨u / 3 - 1, by omega⟩
    rw [blk_evidence] at h
    cases h
    -- The lock is the round's last block: of the epoch before, with epochs, and the parent itself.
    refine ⟨rfl, ?_⟩
    cases B
    · exact Or.inr ⟨rfl, Or.inl (by rw [blk_parent]; exact Nat.le_refl _)⟩
    · exact Or.inl (show (3 * w + 2) / 3 + 1 < (3 * w + 2 + 1) / 3 + 1 by omega)

theorem recv_view {n : Nat} (u r : Nat) (hr : r < 7) (h : sAt u + r < n) : (H k n).Received (viewPhase k u r) :=
  received.mpr ⟨_, h, input_view u r hr⟩

theorem recv_faulty {n : Nat} (w s : Nat) (hs : s < 8) (h : 29 * w + 21 + s < n) :
    (H k n).Received (faultyPhase k w s) :=
  received.mpr ⟨_, h, input_faulty w s hs⟩

theorem no_revote {n : Nat} {s : PubKey} {r : RevoteRequest} : ¬ (H k n).Received (.revote s r) := fun hr => by
  obtain ⟨j, -, hj⟩ := received.mp hr; exact input_none s r hj

/-- The only epoch changes, with epochs: each epoch's last block, in `d`'s view after it. -/
theorem recv_epochChange {n : Nat} {c1 : Cert1} {c2 : Cert2} {p : Proposal}
    (hr : (H k n).Received (.epochChange c1 c2 p)) :
    ∃ w, B = true ∧ c1 = certOf (blk (3 * w + 2)) ∧ c2 = C2 (3 * w + 2) ∧ p = blk (3 * w + 2)
      ∧ 29 * w + 23 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨w, hB, rfl, rfl, rfl, rfl⟩ := input_epochChange hji
  exact ⟨w, hB, rfl, rfl, rfl, hj⟩

theorem vOf_inj {u u' : Nat} (h : vOf u = vOf u') : u = u' := by unfold vOf at h; omega

/-- No honest block is at `d`'s view. -/
theorem vOf_ne (u w : Nat) : vOf u ≠ 4 * w + 4 := by unfold vOf; omega

theorem hasProposal {n : Nat} {x : Block} (hb : (H k n).HasProposal cfg x) :
    x = anchorB ∨ (∃ u, x = blk u ∧ sAt u + 1 < n)
      ∨ ∃ w, ((k = a ∧ x = E1 w) ∨ (k = b ∧ x = E2 w)) ∧ 29 * w + 21 < n := by
  rcases hb with rfl | ⟨s, share, hr⟩ | ⟨c1, c2, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    cases share with
    | some vid =>
      rcases input_proposal hji with ⟨u, rfl, -, rfl, -⟩ | ⟨w, rfl, -, ⟨hk, rfl, -⟩ | ⟨hk, rfl, -⟩⟩
      · exact Or.inr (Or.inl ⟨u, rfl, hj⟩)
      · exact Or.inr (Or.inr ⟨w, Or.inl ⟨hk, rfl⟩, hj⟩)
      · exact Or.inr (Or.inr ⟨w, Or.inr ⟨hk, rfl⟩, hj⟩)
    | none =>
      exact (input_proposal_none hji).elim
  · obtain ⟨w, -, -, -, rfl, hw⟩ := recv_epochChange hr
    exact Or.inr (Or.inl ⟨3 * w + 2, rfl, by rw [sAt_at w 2 (by omega)]; omega⟩)

theorem hasProposal_of {n : Nat} (u : Nat) (h : sAt u + 1 < n) : (H k n).HasProposal cfg (blk u) :=
  Or.inr (Or.inl ⟨_, _, recv_view u 1 (by omega) h⟩)

theorem hasParent_of {n : Nat} (u : Nat) (h : sAt u < n) : (H k n).HasProposal cfg (parentOf u) := by
  cases u with
  | zero => exact Or.inl rfl
  | succ u => exact hasProposal_of u (by unfold sAt at h ⊢; omega)

theorem hasCert1 {n : Nat} {x : Cert1} (hc : (H k n).HasCert1 cfg x) :
    x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ sAt u + c1Off k < n := by
  rcases hc with rfl | hr | ⟨c2, p, hr⟩
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert1 hji
    exact Or.inr ⟨u, rfl, hj⟩
  · obtain ⟨w, -, rfl, -, -, hw⟩ := recv_epochChange hr
    refine Or.inr ⟨3 * w + 2, rfl, ?_⟩
    rw [sAt_at w 2 (by omega)]
    rcases offs_nat k with ⟨h, -⟩ | ⟨h, -⟩ <;> omega

theorem hasCert1_of {n : Nat} (u : Nat) (h : sAt u + c1Off k < n) : (H k n).HasCert1 cfg (certOf (blk u)) := by
  rcases offs k with ⟨hk, h5, -⟩ | ⟨hk, h3, -⟩
  · have := recv_view (B := B) (k := k) u 5 (by omega) (by omega : sAt u + 5 < n)
    simp only [viewPhase', hk, ite_true] at this
    exact Or.inr (Or.inl this)
  · have := recv_view (B := B) (k := k) u 3 (by omega) (by omega : sAt u + 3 < n)
    simp only [viewPhase', hk, ite_false] at this
    exact Or.inr (Or.inl this)

/-- Which held certificate a height names. -/
theorem cert_of_number {n x : Nat} {y : Cert1} (hc : (H k n).HasCert1 cfg y)
    (hv : y.data.blockNumber = ⟨x + 1⟩) : y = certOf (blk x) ∧ sAt x + c1Off k < n := by
  rcases hasCert1 hc with rfl | ⟨u, rfl, hu⟩
  · exact absurd (number_inj hv) (by omega)
  · rw [cert_number] at hv
    obtain rfl : u = x := by have := number_inj hv; omega
    exact ⟨rfl, hu⟩

theorem hasCert2 {n : Nat} {x : Cert2} (hc : (H k n).HasCert2 x) : ∃ u, x = C2 u ∧ sAt u + 6 < n := by
  rcases hc with hr | ⟨c1, p, hr⟩
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl⟩ := input_cert2 hji
    exact ⟨u, rfl, hj⟩
  · obtain ⟨w, -, -, rfl, -, hw⟩ := recv_epochChange hr
    exact ⟨3 * w + 2, rfl, by rw [sAt_at w 2 (by omega)]; omega⟩

theorem hasCert2_of {n : Nat} (u : Nat) (h : sAt u + 6 < n) : (H k n).HasCert2 (C2 u) :=
  Or.inl (recv_view u 6 (by omega) h)

theorem c2_epoch (u : Nat) : (C2 u).data.epoch = ⟨ep B u⟩ := blk_epoch u

theorem epochChange_wellFormed (hB : B = true) (w : Nat) :
    EpochChangeWellFormed cfg (certOf (blk (3 * w + 2))) (C2 (3 * w + 2)) (blk (3 * w + 2)) :=
  ⟨by rw [blk_view]; exact Nat.le_refl _, rfl, rfl, rfl, blk_wellFormed _, (blk_last (u := 3 * w + 2)).mpr ⟨hB, by omega⟩⟩

theorem tookEpochChange_of {n : Nat} (hB : B = true) (w : Nat) (h : 29 * w + 23 < n) :
    (H k n).TookEpochChange cfg (certOf (blk (3 * w + 2))) (C2 (3 * w + 2)) (blk (3 * w + 2)) :=
  ⟨received.mpr ⟨29 * w + 21 + 2, h, by rw [input_faulty w 2 (by omega)]; subst hB; rfl⟩,
    epochChange_wellFormed hB w⟩

theorem hasPayload {n : Nat} {v : ViewNumber} {pc : PayloadCommit} (hp : (H k n).HasPayload cfg v pc) :
    v = ViewNumber.genesis ∨ ∃ u, v = ⟨vOf u⟩ ∧ pc = (blk u).payloadCommit ∧ sAt u + pOff k < n := by
  rcases hp with ⟨rfl, -⟩ | hr
  · exact Or.inl rfl
  · obtain ⟨j, hj, hji⟩ := received.mp hr
    obtain ⟨u, rfl, rfl, rfl⟩ := input_payload hji
    exact Or.inr ⟨u, rfl, rfl, hj⟩

theorem payload_of {n : Nat} (u : Nat) (h : sAt u + pOff k < n) :
    (H k n).HasPayload cfg (blk u).viewNumber (blk u).payloadCommit := by
  rw [blk_view]
  rcases offs k with ⟨hk, -, h3, -⟩ | ⟨hk, -, h4, -⟩
  · have := recv_view (B := B) (k := k) u 3 (by omega) (by omega : sAt u + 3 < n)
    simp only [viewPhase', hk, ite_true] at this
    exact Or.inr this
  · have := recv_view (B := B) (k := k) u 4 (by omega) (by omega : sAt u + 4 < n)
    simp only [viewPhase', hk, ite_false] at this
    exact Or.inr this

theorem received_tc {n : Nat} {tc : TimeoutCert} (hr : (H k n).Received (.timeoutCertificate tc)) :
    ∃ w, tc = T w ∧ 29 * w + 28 < n := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  obtain ⟨w, rfl, rfl⟩ := input_tc hji
  exact ⟨w, rfl, by omega⟩

theorem recv_tc {n : Nat} (w : Nat) (h : 29 * w + 28 < n) : (H k n).Received (.timeoutCertificate (T w)) :=
  recv_faulty w 7 (by omega) (by omega)

/-- When a node can lock on an honest block: once it holds both its `Cert1` and payload. -/
def lkOff (k : PubKey) : Nat := if slow k then 5 else 4

theorem lkOff_facts (k : PubKey) : c1Off k ≤ lkOff k ∧ pOff k ≤ lkOff k ∧ (lkOff k = 4 ∨ lkOff k = 5)
    ∧ (lkOff k = c1Off k ∨ lkOff k = pOff k) := by
  rcases offs k with ⟨hk, h5, h3, -⟩ | ⟨hk, h3, h4, -⟩ <;> simp [lkOff, hk] <;> omega

/-- What a node can lock on: genesis, and each honest block once it holds its `Cert1` and payload. -/
theorem lockable_iff {n : Nat} {x : Cert1} :
    (H k n).Lockable cfg x ↔ x = certOf anchorB ∨ ∃ u, x = certOf (blk u) ∧ sAt u + lkOff k < n := by
  have hf := lkOff_facts k
  have ho := offs k
  constructor
  · rintro (rfl | ⟨hc, y, hy, ⟨-, hcd⟩, hp⟩ | ⟨c2, p, ht⟩)
    · exact Or.inl rfl
    · rcases hasCert1 hc with rfl | ⟨u, rfl, hu⟩
      · exact Or.inl rfl
      · have hyn : y.blockHeader.blockNumber = ⟨u + 1⟩ := by
          have := congrArg Vote1Data.blockNumber hcd
          rw [← cert_number u]; exact this.symm
        refine Or.inr ⟨u, rfl, ?_⟩
        -- The block is honest block `u`: `d`'s are at a view with no payload.
        rcases hasProposal hy with rfl | ⟨u', rfl, -⟩ | ⟨w, hE, -⟩
        · exact absurd (number_inj hyn) (by omega)
        · rw [blk_number] at hyn
          obtain rfl : u = u' := by have := number_inj hyn; omega
          rcases hasPayload hp with hg | ⟨u'', hv, -, hlt⟩
          · rw [blk_view] at hg; exact absurd (view_inj hg) (by unfold vOf; omega)
          · rw [blk_view] at hv
            obtain rfl : u = u'' := vOf_inj (view_inj hv)
            omega
        · have hEv : y.viewNumber = ⟨4 * w + 4⟩ := by rcases hE with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
          rcases hasPayload hp with hg | ⟨u'', hv, -, -⟩
          · rw [hEv] at hg; exact absurd (view_inj hg) (by omega)
          · rw [hEv] at hv; exact absurd (view_inj hv).symm (vOf_ne u'' w)
    · obtain ⟨w, -, rfl, -, -, hw⟩ := recv_epochChange ht.1
      refine Or.inr ⟨3 * w + 2, rfl, ?_⟩
      rw [sAt_at w 2 (by omega)]; omega
  · rintro (rfl | ⟨u, rfl, h⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hasCert1_of u (by omega), blk u, hasProposal_of u (by omega), ⟨Nat.le_refl _, rfl⟩,
        payload_of u (by omega)⟩)

/-! ### The view, the epoch and the lock after `n` steps -/

/-- The view node `k` is in: it moves on with each honest block's `Cert1`, and with each timeout certificate. -/
def vAt (k : PubKey) (n : Nat) : Nat :=
  if n % 29 < 21 then vOf (3 * (n / 29) + n % 29 / 7) + (if c1Off k < n % 29 % 7 then 1 else 0) else 4 * (n / 29) + 4

theorem vAt_view (u r : Nat) (hr : r < 7) :
    (c1Off k < r → vAt k (sAt u + r) = vOf u + 1) ∧ (r ≤ c1Off k → vAt k (sAt u + r) = vOf u) := by
  obtain ⟨w, i, hi, rfl⟩ : ∃ w i, i < 3 ∧ u = 3 * w + i := ⟨u / 3, u % 3, Nat.mod_lt _ (by omega), by omega⟩
  rw [sAt_at w i hi, Nat.add_assoc]
  obtain ⟨h1, h2⟩ := split29 w (7 * i + r) (by omega)
  unfold vAt
  rw [h1, h2, ite_eq_left (by omega), show (7 * i + r) / 7 = i by omega, show (7 * i + r) % 7 = r by omega]
  exact ⟨fun h => by rw [ite_eq_left h], fun h => by rw [ite_eq_right (by omega), Nat.add_zero]⟩

theorem vAt_faulty (w s : Nat) (hs : s < 8) : vAt k (29 * w + 21 + s) = 4 * w + 4 := by
  rw [Nat.add_assoc]
  obtain ⟨h1, h2⟩ := split29 w (21 + s) (by omega)
  unfold vAt
  rw [h1, h2, ite_eq_right (by omega)]

/-- `vAt` against the steps a ground arrives at, as facts `omega` reads. -/
theorem vAt_ge (n : Nat) : 1 ≤ vAt k n ∧ (∀ u, sAt u + c1Off k < n → vOf u + 1 ≤ vAt k n)
    ∧ (∀ w, 29 * w + 28 < n → 4 * w + 5 ≤ vAt k n) := by
  have ho := offs k
  rcases steps n with ⟨u', r, hr, rfl⟩ | ⟨w', s, hs, rfl⟩
  · have hv := vAt_view (k := k) u' r hr
    have h1 : 1 ≤ vOf u' := by unfold vOf; omega
    refine ⟨by omega, fun u hu => ?_, fun w hw => ?_⟩
    · -- An earlier block's view, or this one's past its `Cert1`.
      by_cases he : u = u'
      · subst he; omega
      · have hlt : u < u' := by
          by_cases hl : u < u'
          · exact hl
          · have := sAt_mono (show u' + 1 ≤ u by omega)
            have := sAt_succ u'
            omega
        have := vOf_mono (show u + 1 ≤ u' by omega)
        have := vOf_succ u
        omega
    · have := round_idx (w := w) (u := u') (by omega)
      have := vOf_mono this
      have : vOf (3 * w + 3) = 4 * w + 5 := by unfold vOf; omega
      omega
  · have := vAt_faulty (k := k) w' s hs
    refine ⟨by omega, fun u hu => ?_, fun w hw => by omega⟩
    have := vOf_mono (idx_round (j := u) (w := w') (by omega))
    rw [vOf_at w' 2 (by omega)] at this
    omega

theorem viewGround {n : Nat} {v : ViewNumber} (hv : (H k n).ViewGround cfg v) : v.toNat ≤ vAt k n := by
  obtain ⟨h1, hc, ht⟩ := vAt_ge (k := k) n
  rcases hv with ⟨x, hx, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, hte, rfl⟩
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hlt⟩
    · show 0 + 1 ≤ _; omega
    · show (certOf (blk u)).view.toNat + 1 ≤ _
      rw [cert_view]; exact hc u hlt
  · obtain ⟨w, rfl, hlt⟩ := received_tc htc
    exact ht w hlt
  · obtain ⟨w, -, -, rfl, -, hw⟩ := recv_epochChange hte.1
    show vOf (3 * w + 2) + 1 ≤ _
    refine hc _ ?_
    rw [sAt_at w 2 (by omega)]
    rcases offs_nat k with ⟨h, -⟩ | ⟨h, -⟩ <;> omega

theorem inView (n : Nat) : (H k n).InView cfg ⟨vAt k n⟩ := by
  refine ⟨?_, fun v hv => viewGround hv⟩
  have ho := offs k
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · obtain ⟨hhi, hlo⟩ := vAt_view (k := k) u r hr
    by_cases hc : c1Off k < r
    · rw [hhi hc]
      exact Or.inl ⟨certOf (blk u), hasCert1_of u (by omega), by rw [cert_view]; rfl⟩
    · rw [hlo (by omega)]
      -- The view of block `u`: on the previous block's `Cert1`, or on `d`'s view's timeout certificate.
      obtain ⟨w, i, hi, rfl⟩ : ∃ w i, i < 3 ∧ u = 3 * w + i := ⟨u / 3, u % 3, Nat.mod_lt _ (by omega), by omega⟩
      rw [vOf_at w i hi]
      have hs := sAt_at w i hi
      by_cases hi0 : i = 0
      · subst hi0
        cases w with
        | zero => exact Or.inl ⟨certOf anchorB, Or.inl rfl, rfl⟩
        | succ w =>
          exact Or.inr (Or.inl ⟨T w, recv_tc w (by omega), rfl⟩)
      · refine Or.inl ⟨certOf (blk (3 * w + i - 1)), hasCert1_of _ ?_, ?_⟩
        · rw [show 3 * w + i - 1 = 3 * w + (i - 1) by omega, sAt_at w (i - 1) (by omega)]; omega
        · rw [cert_view, show 3 * w + i - 1 = 3 * w + (i - 1) by omega, vOf_at w (i - 1) (by omega)]
          show (⟨4 * w + i + 1⟩ : ViewNumber) = ⟨4 * w + (i - 1) + 1 + 1⟩
          congr 1; omega
  · rw [vAt_faulty w s hs]
    exact Or.inl ⟨certOf (blk (3 * w + 2)), hasCert1_of _ (by rw [sAt_at w 2 (by omega)]; omega),
      by rw [cert_view, vOf_at w 2 (by omega)]; rfl⟩

theorem inView_eq {n : Nat} {v : ViewNumber} (hv : (H k n).InView cfg v) : v = ⟨vAt k n⟩ :=
  Kit.inView_unique hv (inView n)

theorem viewOf_eq (n : Nat) : viewOf cfg (H k n) = ⟨vAt k n⟩ := viewOf_of_inView (inView n)

/--
The epoch a node is in after `n` steps: its round's and, with epochs, the next from
the epoch change in `d`'s view.
-/
def eAt' : Bool → Nat → Nat
  | false, _ => 0
  | true, n => if 24 ≤ n % 29 then n / 29 + 2 else n / 29 + 1

set_option hygiene false in
local notation "eAt" => eAt' B

theorem eAt_at (w x : Nat) (hx : x < 29) :
    (x < 24 → eAt (29 * w + x) = ep B (3 * w)) ∧ (24 ≤ x → eAt (29 * w + x) = ep B (3 * w + 3)) := by
  obtain ⟨h1, h2⟩ := split29 w x hx
  cases B
  · exact ⟨fun _ => rfl, fun _ => rfl⟩
  · simp only [eAt', ep]; rw [h1, h2]
    exact ⟨fun h => by rw [ite_eq_right (by omega)]; omega, fun h => by rw [ite_eq_left h]; omega⟩

theorem eAt_view (u r : Nat) (hr : r < 7) : eAt (sAt u + r) = ep B u := by
  obtain ⟨w, i, hi, rfl, hs, -⟩ := sAt_vOf u
  rw [hs, Nat.add_assoc, (eAt_at w (7 * i + r) (by omega)).1 (by omega), ep_round w i hi]

theorem eAt_faulty (w s : Nat) (hs : s < 8) :
    (s < 3 → eAt (29 * w + 21 + s) = ep B (3 * w + 2)) ∧ (3 ≤ s → eAt (29 * w + 21 + s) = ep B (3 * w + 3)) := by
  rw [Nat.add_assoc]
  exact ⟨fun h => by rw [(eAt_at w (21 + s) (by omega)).1 (by omega), ep_round w 2 (by omega)],
    fun h => (eAt_at w (21 + s) (by omega)).2 (by omega)⟩

/-- Once a node holds a timeout certificate, it is in the certificate's epoch or a later one. -/
theorem eAt_tc {n w : Nat} (h : 29 * w + 28 < n) : ep B (3 * w + 3) ≤ eAt n := by
  cases B <;> simp only [eAt', ep] <;> (try split) <;> omega

theorem epochGround {n : Nat} {e : EpochNumber} (he : (H k n).EpochGround cfg e) : e.toNat ≤ eAt n := by
  have hb : ∀ u, sAt u ≤ n → ep B u ≤ eAt n := fun u hu => by
    obtain ⟨w, i, hi, rfl, hs, -⟩ := sAt_vOf u
    rw [hs] at hu
    cases B <;> simp only [eAt', ep] <;> (try split) <;> omega
  rcases he with rfl | ⟨c1, c2, p, ht, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨x, hx, -, rfl⟩
  · exact hb 0 (Nat.zero_le _)
  · obtain ⟨w, hB, -, rfl, -, hw⟩ := recv_epochChange ht.1
    rw [c2_epoch]
    subst hB
    show (3 * w + 2) / 3 + 1 + 1 ≤ eAt' true n
    simp only [eAt']; split <;> omega
  · obtain ⟨w, rfl, hw⟩ := received_tc htc
    exact eAt_tc (by omega)
  · rcases hasCert1 hx with rfl | ⟨u, rfl, hu⟩
    · exact hb 0 (Nat.zero_le _)
    · rw [cert_epoch]; exact hb u (by omega)

theorem inEpoch (n : Nat) : (H k n).InEpoch cfg ⟨eAt n⟩ := by
  refine ⟨?_, fun _ he => epochGround he⟩
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · rw [eAt_view u r hr]
    obtain ⟨w, i, hi, rfl, hsu, -⟩ := sAt_vOf u
    rw [ep_round w i hi]
    cases w with
    | zero => exact Or.inl rfl
    | succ w => exact Or.inr (Or.inr (Or.inl ⟨T w, recv_tc w (by omega), rfl⟩))
  · by_cases hs3 : s < 3
    · rw [(eAt_faulty w s hs).1 hs3]
      refine Or.inr (Or.inr (Or.inr ⟨certOf (blk (3 * w + 2)), hasCert1_of _ ?_, ?_, ?_⟩))
      · rw [sAt_at w 2 (by omega)]; rcases offs_nat k with ⟨h, -⟩ | ⟨h, -⟩ <;> omega
      · rw [cert_epoch, cert_number]; exact (epochOf_number _).symm
      · rw [cert_epoch]
    · rw [(eAt_faulty w s hs).2 (by omega)]
      cases B
      · exact Or.inl rfl
      · refine Or.inr (Or.inl ⟨_, _, _, tookEpochChange_of rfl w (by omega), ?_⟩)
        rw [c2_epoch]; show (⟨(3 * w + 3) / 3 + 1⟩ : EpochNumber) = ⟨(3 * w + 2) / 3 + 1 + 1⟩; congr 1; omega

theorem epochOf_eq (n : Nat) : epochOfHistory cfg (H k n) = ⟨eAt n⟩ :=
  Liveness.inEpoch_unique (inEpoch_epochOfHistory cfg _) (inEpoch n)

theorem notBehind {n : Nat} {e : EpochNumber} (h : eAt n ≤ e.toNat) : NotBehind cfg (H k n) e :=
  Kit.notBehind_of (inEpoch n) h

theorem le_of_notBehind_eAt {n : Nat} {e : EpochNumber} (h : NotBehind cfg (H k n) e) : eAt n ≤ e.toNat :=
  Kit.le_of_notBehind (inEpoch n) h

/-- The lock a node carries into block `u`'s view: the block before's, or genesis. -/
def prevC' (B : Bool) : Nat → Cert1
  | 0 => certOf anchorB
  | u + 1 => certOf (blk u)

set_option hygiene false in
local notation "prevC" => prevC' B

/-- The lock after `n` steps: each honest block, once the node holds its `Cert1` and payload. -/
def lockAt' (B : Bool) (k : PubKey) (n : Nat) : Cert1 :=
  if n % 29 < 21 then
    (if lkOff k < n % 29 % 7 then certOf (blk (3 * (n / 29) + n % 29 / 7)) else prevC (3 * (n / 29) + n % 29 / 7))
  else certOf (blk (3 * (n / 29) + 2))

set_option hygiene false in
local notation "lockAt" => lockAt' B

theorem lockAt_view (u r : Nat) (hr : r < 7) :
    (lkOff k < r → lockAt k (sAt u + r) = certOf (blk u)) ∧ (r ≤ lkOff k → lockAt k (sAt u + r) = prevC u) := by
  obtain ⟨w, i, hi, rfl⟩ : ∃ w i, i < 3 ∧ u = 3 * w + i := ⟨u / 3, u % 3, Nat.mod_lt _ (by omega), by omega⟩
  rw [sAt_at w i hi, Nat.add_assoc]
  obtain ⟨h1, h2⟩ := split29 w (7 * i + r) (by omega)
  unfold lockAt'
  rw [h1, h2, ite_eq_left (by omega), show (7 * i + r) / 7 = i by omega, show (7 * i + r) % 7 = r by omega]
  exact ⟨fun h => by rw [ite_eq_left h], fun h => by rw [ite_eq_right (by omega)]⟩

theorem lockAt_faulty (w s : Nat) (hs : s < 8) : lockAt k (29 * w + 21 + s) = certOf (blk (3 * w + 2)) := by
  rw [Nat.add_assoc]
  obtain ⟨h1, h2⟩ := split29 w (21 + s) (by omega)
  unfold lockAt'
  rw [h1, h2, ite_eq_right (by omega)]

/-- Lock order on honest blocks is chain order: epochs and views both grow along the chain. -/
theorem lockLE_blk {j m : Nat} (h : j ≤ m) : LockLE (certOf (blk j)) (certOf (blk m)) := by
  rw [LockLE, cert_epoch, cert_epoch, cert_view, cert_view]
  by_cases he : ep B j = ep B m
  · exact Or.inr ⟨by rw [he], vOf_mono h⟩
  · exact Or.inl (show ep B j < ep B m by have := ep_mono (B := B) h; omega)

theorem lockLE_anchor (m : Nat) : LockLE (certOf anchorB) (certOf (blk m)) := by
  rw [LockLE, cert_epoch, cert_view]
  by_cases he : ep B 0 = ep B m
  · exact Or.inr ⟨by show (⟨ep B 0⟩ : EpochNumber) = _; rw [he], Nat.zero_le _⟩
  · exact Or.inl (show ep B 0 < ep B m by have := ep_mono (B := B) (Nat.zero_le m); omega)

theorem lockable_ext {n : Nat} {x y : Cert1} (hx : (H k n).Lockable cfg x) (hy : (H k n).Lockable cfg y)
    (hv : x.view = y.view) : x = y := by
  rcases lockable_iff.mp hx with rfl | ⟨j, rfl, -⟩ <;> rcases lockable_iff.mp hy with rfl | ⟨m, rfl, -⟩
  · rfl
  · rw [cert_view] at hv; exact absurd (view_inj hv) (by unfold vOf; omega)
  · rw [cert_view] at hv; exact absurd (view_inj hv) (by unfold vOf; omega)
  · rw [cert_view, cert_view] at hv
    obtain rfl : j = m := vOf_inj (view_inj hv)
    rfl

theorem lockedOn_lockAt (n : Nat) : (H k n).LockedOn cfg (lockAt k n) := by
  have hf := lkOff_facts k
  -- Every lockable certificate is no later than block `m`'s, given its blocks are no later than `m`.
  have hle : ∀ m, (∀ j, sAt j + lkOff k < n → j ≤ m) → ∀ x, (H k n).Lockable cfg x →
      LockLE x (certOf (blk m)) := fun m hm x hx => by
    rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hj⟩
    · exact lockLE_anchor m
    · exact lockLE_blk (hm j hj)
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · obtain ⟨hhi, hlo⟩ := lockAt_view (k := k) u r hr
    by_cases hc : lkOff k < r
    · rw [hhi hc]
      refine ⟨lockable_iff.mpr (Or.inr ⟨u, rfl, by omega⟩), hle u fun j hj => ?_⟩
      by_cases hl : j ≤ u
      · exact hl
      · have := sAt_mono (show u + 1 ≤ j by omega); have := sAt_succ u; omega
    · rw [hlo (by omega)]
      cases u with
      | zero =>
        refine ⟨lockable_iff.mpr (Or.inl rfl), fun x hx => ?_⟩
        rcases lockable_iff.mp hx with rfl | ⟨j, rfl, hj⟩
        · exact Or.inr ⟨rfl, Nat.le_refl _⟩
        · exact absurd hj (by have : sAt 0 = 0 := rfl; omega)
      | succ u =>
        have := sAt_succ u
        refine ⟨lockable_iff.mpr (Or.inr ⟨u, rfl, by omega⟩), hle u fun j hj => ?_⟩
        by_cases hl : j ≤ u
        · exact hl
        · have := sAt_mono (show u + 1 ≤ j by omega); omega
  · rw [lockAt_faulty w s hs]
    have h2 := sAt_at w 2 (by omega)
    refine ⟨lockable_iff.mpr (Or.inr ⟨3 * w + 2, rfl, by omega⟩), hle _ fun j hj => idx_round (by omega)⟩

theorem lockedOn_eq {n : Nat} {L : Cert1} (hL : (H k n).LockedOn cfg L) : L = lockAt k n := by
  have h0 := lockedOn_lockAt (B := B) (k := k) n
  exact lockable_ext hL.1 h0.1 (lockLE_antisymm (h0.2 _ hL.1) (hL.2 _ h0.1)).2

theorem lockOf_eq (n : Nat) : lockOf cfg (H k n) = lockAt k n := lockedOn_eq (lockOf_lockedOn _)

end Holds

set_option hygiene false in
local notation "eAt" => eAt' B

set_option hygiene false in
local notation "prevC" => prevC' B

set_option hygiene false in
local notation "lockAt" => lockAt' B

/-! ## What the honest nodes send -/

section Sends

/-- A view no later than `d`'s of round `w + 1` belongs to a block of that round or earlier. -/
theorem view_le_round {u w : Nat} (h : vOf u ≤ 4 * w + 4) : sAt u ≤ 29 * w + 14 := by
  obtain ⟨wu, iu, hiu, rfl, hs, hv⟩ := sAt_vOf u
  rw [hs]; rw [hv] at h
  by_cases h1 : wu ≤ w
  · omega
  · omega

theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H k (n + 1)) o :=
  Kit.settled input k n o

/-- A node's timeout votes, all in `d`'s views: at its timer, and answering the one-honest indication. -/
theorem sent_timeout {k : PubKey} {j : Nat} {vote : TimeoutVote}
    (hx : Output.send (.timeoutVote vote) ∈ (tr k j).output) :
    ∃ w, (j = 29 * w + 21 + tOff k ∨ j = 29 * w + 21 + 5)
      ∧ vote = ⟨⟨⟨ep B (3 * w + 3)⟩, certOf (blk (3 * w + 2))⟩, ⟨4 * w + 4⟩, k⟩ := by
  have ho := offs k
  rw [tr_step] at hx
  rcases step_mem hx with hx | ⟨o, out', -, -, ha⟩
  · obtain ⟨v, hv, (⟨hi, -⟩ | ⟨hi, -⟩)⟩ := mem_timeoutAnswer hx
    · obtain ⟨w, rfl, rfl⟩ := input_timeout hi
      simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      refine ⟨w, Or.inl rfl, ?_⟩
      rw [hv, epochOf_eq, lockOf_eq, lockAt_faulty w _ (by omega), (eAt_faulty w _ (by omega)).2 (by omega)]
    · obtain ⟨w, rfl, rfl⟩ := input_oneHonest hi
      simp only [Output.send.injEq, Message.timeoutVote.injEq] at hv
      refine ⟨w, Or.inr rfl, ?_⟩
      rw [hv, epochOf_eq, lockOf_eq, lockAt_faulty w _ (by omega), (eAt_faulty w _ (by omega)).2 (by omega)]
  · exact absurd ha act_no_timeoutVote

/-- A node times out `d`'s view at its timer, and again on the one-honest indication. -/
theorem times_out (k : PubKey) (w : Nat) {j : Nat} (hj : j = 29 * w + 21 + tOff k ∨ j = 29 * w + 21 + 5) :
    Output.send (.timeoutVote ⟨⟨⟨ep B (3 * w + 3)⟩, certOf (blk (3 * w + 2))⟩, ⟨4 * w + 4⟩, k⟩) ∈ (tr k j).output := by
  have ho := offs k
  have hs : j - (29 * w + 21) < 8 := by omega
  have hj' : j = 29 * w + 21 + (j - (29 * w + 21)) := by omega
  rw [tr_step, step_output]
  apply discharge_sup
  have hv := vAt_faulty (k := k) w _ hs
  have hl := lockAt_faulty (B := B) (k := k) w _ hs
  have he := (eAt_faulty (B := B) w _ hs).2 (by omega)
  rw [← hj'] at hv hl he
  rcases hj with rfl | rfl
  · have hi : input k (29 * w + 21 + tOff k) = .timeout ⟨4 * w + 4⟩ := by
      rw [input_faulty w _ (by omega)]
      rcases ho with ⟨hk, -, -, ht⟩ | ⟨hk, -, -, ht⟩ <;> rw [ht] <;> simp [faultyPhase', hk]
    rw [hi]
    simp only [timeoutAnswer, viewOf_eq, epochOf_eq, lockOf_eq, hv, hl, he, ite_true, List.mem_singleton]
  · have hi : input k (29 * w + 21 + 5) = .timeoutOneHonest ⟨4 * w + 4⟩ := by
      rw [input_faulty w 5 (by omega)]; rfl
    rw [hi]
    simp only [timeoutAnswer, viewOf_eq, epochOf_eq, lockOf_eq, hv, hl, he]
    rw [ite_eq_left (show (⟨4 * w + 4⟩ : ViewNumber) ≤ ⟨4 * w + 4⟩ from Nat.le_refl _)]
    exact List.mem_singleton_self _

theorem timedOut {k : PubKey} {n : Nat} {v : ViewNumber} (h : (H k n).TimedOut v) :
    ∃ w, v.toNat ≤ 4 * w + 4 ∧ 29 * w + 25 < n := by
  obtain ⟨vote, hs, hle⟩ := h
  obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
  obtain ⟨w, hjw, rfl⟩ := sent_timeout hjv
  have := offs k
  exact ⟨w, hle, by omega⟩

/-- Nothing times an honest block's view out before its view is over. -/
theorem not_timedOut {k : PubKey} {n u : Nat} (h : (H k n).TimedOut ⟨vOf u⟩) (hn : n ≤ sAt u + 7) : False := by
  obtain ⟨w, hle, hlt⟩ := timedOut h
  have := view_le_round (u := u) (w := w) hle
  omega

variable (hv : ∀ b, BlockValid b)

include hv in
theorem protocol (k : PubKey) (n : Nat) : ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H k n) := Kit.protocol input hv k n

/-- What a justified proposal names, the node holds. -/
theorem justified_held {k : PubKey} {n : Nat} {x : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg (H k n) x ev) :
    (H k n).HasCert1 cfg x ∧ ∀ tc, ev = some tc → ∃ w, tc = T w ∧ 29 * w + 28 < n := by
  refine ⟨hasCert1_of_certJustified hj, fun tc hte => ?_⟩
  subst hte
  obtain ⟨-, m, hr, -⟩ := hj
  rw [upTo_H] at hr
  obtain ⟨w, rfl, hw⟩ := received_tc hr
  exact ⟨w, rfl, by omega⟩

include hv in
/--
No honest node asks for a re-vote. It would be for an epoch's last block, in the
view after it, which `d` leads, or after the timeout certificate for that view;
by then the node has left the block's epoch (`NotBehind`).
-/
theorem no_revoteSent {k : PubKey} {j : Nat} {r : RevoteRequest} (hk : k ≠ d) : Output.send (.revote r) ∉ (tr k j).output :=
  fun hr => by
    obtain ⟨hlead, ⟨hlt, hpath, hlast⟩, hjust, -, -, hcur, -⟩ :=
      (protocol hv k (j + 1)).revoteJustified j r ⟨_, (getElem_H j), hr⟩ trivial
    rw [upTo_self] at hjust hcur
    rcases hasCert1 (hasCert1_of_certJustified hjust) with h | ⟨u, h, -⟩
    · rw [h] at hlast; exact hlast.1 rfl
    · rw [h, cert_number] at hlast
      obtain ⟨hB, -, hmod⟩ := last_iff.mp hlast
      obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 2 := ⟨u / 3, by omega⟩
      have hcv : r.cert.view = ⟨4 * w + 3⟩ := by rw [h, cert_view, vOf_at w 2 (by omega)]
      rcases hpath with ⟨-, hv1⟩ | ⟨tc, hte, htv⟩
      · -- The view after the block: `d`'s.
        rw [hcv] at hv1
        rw [← hv1, h, cert_epoch] at hlead
        have hl : ldr (4 * w + 3 + 1) = k := (leader_some hlead).1.symm
        rw [show 4 * w + 3 + 1 = 4 * w + 4 by omega, (ldr_at w).2.2.2] at hl
        exact hk hl.symm
      · -- After a timeout certificate, which put the node in a later epoch.
        have hj' := hjust; rw [hte] at hj'
        obtain ⟨-, m, htc, -⟩ := hj'
        rw [upTo_H] at htc
        obtain ⟨y, rfl, hy⟩ := received_tc htc
        have he := le_of_notBehind_eAt hcur
        rw [h, cert_epoch] at he
        have h1 : 4 * y + 4 + 1 = r.view.toNat := congrArg ViewNumber.toNat htv
        rw [hcv] at hlt
        have h2 : 4 * w + 3 < r.view.toNat := hlt
        have h3 : eAt (j + 1) ≤ ep B (3 * w + 2) := he
        have h4 := eAt_tc (B := B) (n := j + 1) (w := y) (by omega)
        rw [ep_of hB] at h3 h4
        omega

include hv in
/-- A proposal is for the view of a header the node was handed, after it was handed it. -/
theorem proposal_header {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) :
    ∃ u, sAt u ≤ j ∧ p.viewNumber = ⟨vOf u⟩ ∧ p.blockHeader = hdr (u + 1) := by
  have hj := (protocol (B := B) hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨i, hi, hii⟩ := received.mp hj.built
  obtain ⟨u, rfl, hvw, -, hx⟩ := input_header hii
  exact ⟨u, by omega, hvw, hx⟩

/-- An epoch's first block names its parent at the parent's view, behind the parent's `Cert2`. -/
theorem blk_opens {k : PubKey} {n : Nat} (u : Nat) (hn : sAt u ≤ n) : OpensEpochJustified cfg (H k n) (blk u) :=
  fun he => by
    obtain ⟨hu0, hu3⟩ := blk_enters.mp he
    obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 3 := ⟨u / 3 - 1, by omega⟩
    have hs := sAt_at (w + 1) 0 (by omega)
    have hs2 := sAt_at w 2 (by omega)
    rw [show 3 * (w + 1) + 0 = 3 * w + 3 by omega] at hs
    refine ⟨⟨blk (3 * w + 2), hasProposal_of _ (by omega), by rw [blk_parent]; rfl, by rw [blk_parent]; rfl⟩,
      C2 (3 * w + 2), Or.inl <| hasCert2_of _ (by omega), ?_, by rw [blk_parent]; rfl⟩
    rw [blk_view]; show vOf (3 * w + 2) < vOf (3 * w + 2 + 1); have := vOf_succ (3 * w + 2); omega

/-- The leader of block `u`'s view may propose it from the step its header arrives until its `Cert1` does. -/
theorem blk_justified (u r : Nat) (h1 : 1 ≤ r) (h3 : r ≤ 3) :
    ProposalJustified cfg leader (ldr (vOf u)) (H (ldr (vOf u)) (sAt u + r)) (blk u) := by
  have ho := offs (ldr (vOf u))
  have hlk := lkOff_facts (ldr (vOf u))
  refine ⟨⟨by rw [blk_view]; exact leader_blk _ u, blk_wellFormed u, ?_, ⟨parentOf u, hasParent_of u (by omega),
      by rw [blk_parent]; exact Nat.le_refl _, by rw [blk_parent]; rfl⟩, blk_opens u (by omega),
      blk_safe u, notBehind (by rw [blk_epoch, eAt_view u r (by omega)]; exact Nat.le_refl _),
      ⟨_, (inView _).1, ?_⟩⟩, ?_⟩
  · unfold ParentJustified
    by_cases hu : u = 0 ∨ u % 3 ≠ 0
    · rw [blk_evidence_none hu, blk_parent]
      cases u with
      | zero => exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inl rfl))
      | succ u =>
        have := sAt_succ u
        exact Liveness.buildable_of_lockable (lockable_iff.mpr (Or.inr ⟨u, rfl, by omega⟩))
    · -- After `d`'s view: the lock behind the timeout certificate is the round's last block.
      obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 3 := ⟨u / 3 - 1, by omega⟩
      rw [blk_evidence, blk_parent]
      have hs := sAt_at (w + 1) 0 (by omega)
      rw [show 3 * (w + 1) + 0 = 3 * w + 3 by omega] at hs
      show CertJustified cfg _ (certOf (blk (3 * w + 2))) (some (T w))
      refine ⟨hasCert1_of _ (by have := sAt_at w 2 (by omega); omega), sAt (3 * w + 3) + r, ?_,
        Or.inl ⟨certOf (blk (3 * w + 2)), ?_, rfl⟩⟩ <;> rw [upTo_self]
      · exact recv_tc w (by omega)
      · have := lockedOn_lockAt (B := B) (k := ldr (vOf (3 * w + 3))) (sAt (3 * w + 3) + r)
        rwa [(lockAt_view (3 * w + 3) r (by omega)).2 (by omega)] at this
  · rw [blk_view, (vAt_view u r (by omega)).2 (by omega)]; exact Nat.le_refl _
  · rw [blk_view, blk_header, blk_parent]
    exact recv_view u 0 (by omega) (by omega)

include hv in
/-- Every proposal an honest node sends is the honest block of a view it leads, once the header arrived. -/
theorem sent_proposal {k : PubKey} {j : Nat} {p : Proposal}
    (hp : Output.send (.proposal p) ∈ (tr k j).output) : ∃ u, k = ldr (vOf u) ∧ p = blk u ∧ sAt u ≤ j := by
  have hj := (protocol (B := B) hv k (j + 1)).proposeJustified j p ⟨_, (getElem_H j), hp⟩ trivial
  rw [upTo_self] at hj
  obtain ⟨u, hju, hvw, hhdr⟩ := proposal_header hv hp
  have hk : k = ldr (vOf u) := by
    have := hj.leads; rw [hvw] at this; exact (leader_some this).1
  refine ⟨u, hk, ?_, hju⟩
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
  have hepu : p.epoch = ⟨ep B u⟩ := by rw [hep, hhdr]; exact epochOf_number u
  -- The parent is the block before, or genesis.
  have hpcP : p.parentCert = certOf (parentOf u) := by
    cases u with
    | zero =>
      rcases hasCert1 hpc with h | ⟨x, h, -⟩
      · exact h
      · rw [h, cert_number] at hnum'; have : x + 1 + 1 = 0 + 1 := hnum'; omega
    | succ u =>
      exact (cert_of_number hpc (BlockNumber.ext (show p.parentCert.data.blockNumber.toNat = u + 1 by omega))).1
  -- The evidence is the timeout certificate for `d`'s view, right after it, and none otherwise.
  have hte : p.timeoutEvidence = (blk u).timeoutEvidence := by
    by_cases hu : u = 0 ∨ u % 3 ≠ 0
    · rw [blk_evidence_none hu]
      rcases hnext with ⟨h0, -⟩ | ⟨tc, htc, htv⟩
      · exact h0
      · exfalso
        obtain ⟨w, rfl, -⟩ := hev tc htc
        rw [hvw] at htv
        have : 4 * w + 4 + 1 = vOf u := congrArg ViewNumber.toNat htv
        unfold vOf at this; omega
    · obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 3 := ⟨u / 3 - 1, by omega⟩
      rw [blk_evidence]
      rcases hnext with ⟨-, hn⟩ | ⟨tc, htc, htv⟩
      · exfalso
        rw [hpcP, hvw] at hn
        have : (certOf (blk (3 * w + 2))).view.toNat + 1 = vOf (3 * w + 3) := congrArg ViewNumber.toNat hn
        rw [cert_view] at this
        have h1 : vOf (3 * w + 2) = 4 * w + 3 := vOf_at w 2 (by omega)
        have h2 : vOf (3 * w + 3) = 4 * w + 5 := by unfold vOf; omega
        simp only at this; omega
      · obtain ⟨w', rfl, -⟩ := hev tc htc
        rw [hvw] at htv
        have : 4 * w' + 4 + 1 = vOf (3 * w + 3) := congrArg ViewNumber.toNat htv
        have h2 : vOf (3 * w + 3) = 4 * w + 5 := by unfold vOf; omega
        obtain rfl : w' = w := by omega
        exact htc
  obtain ⟨hd, vw, ep, pc, te, idt⟩ := p
  simp only at hvw hhdr hid hepu hpcP hte ⊢
  rw [hvw, hhdr, hid, hepu, hpcP, hte]
  cases u <;> rfl

include hv in
/-- The leader of each honest block's view proposes it by the step its header arrives in. -/
theorem proposes (u : Nat) :
    ∃ j, j ≤ sAt u ∧ Output.send (.proposal (blk u)) ∈ (tr (ldr (vOf u)) j).output := by
  have ho := offs (ldr (vOf u))
  refine Classical.byContradiction fun hneg => settled (ldr (vOf u)) (sAt u) (.propose (blk u).epoch ⟨vOf u⟩)
    ⟨Or.inl ⟨blk u, blk_justified u 1 (Nat.le_refl _) (by omega), blk_view u, rfl⟩, ?_,
      fun ht => not_timedOut ht (by omega), ?_⟩
  · rintro (⟨p, hs, hpv, -⟩ | ⟨r, hs, -⟩)
    · obtain ⟨j, hj, hjp⟩ := sent_iff.mp hs
      obtain ⟨x, -, rfl, -⟩ := sent_proposal hv hjp
      rw [blk_view] at hpv
      obtain rfl : x = u := vOf_inj (view_inj hpv)
      exact hneg ⟨j, by omega, hjp⟩
    · obtain ⟨j, -, hjr⟩ := sent_iff.mp hs
      exact no_revoteSent hv (ldr_blk u).2 hjr
  · have := inView (B := B) (k := ldr (vOf u)) (sAt u + 1)
    rwa [(vAt_view u 1 (by omega)).2 (by omega)] at this

include hv in
/-- Every vote1 a node sends is for an honest block, or, at `a` and `b`, for the block `d` sent it. -/
theorem sent_vote1 {k : PubKey} {j : Nat} {vote : Vote1} (hx : Output.send (.vote1 vote) ∈ (tr k j).output) :
    (∃ u, vote = ⟨(certOf (blk u)).data, ⟨vOf u⟩, k⟩ ∧ sAt u + 1 ≤ j)
      ∨ ∃ w, ((k = a ∧ vote = ⟨(certOf (E1 w)).data, ⟨4 * w + 4⟩, a⟩)
        ∨ (k = b ∧ vote = ⟨(certOf (E2 w)).data, ⟨4 * w + 4⟩, b⟩)) ∧ 29 * w + 21 ≤ j := by
  have hpr := protocol (B := B) hv k (j + 1)
  obtain ⟨hsig, -⟩ := hpr.vote1Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  obtain ⟨-, ⟨s', p, vid, hrec, -, hfor, -⟩ | ⟨s', r, hrec, -⟩⟩ := hpr.vote1Leader j vote ⟨_, (getElem_H j), hx⟩ trivial
  · rw [upTo_self] at hrec
    obtain ⟨i, hi, hii⟩ := received.mp hrec
    rcases input_proposal hii with ⟨u, rfl, -, rfl, -⟩ | ⟨w, rfl, -, ⟨hk, rfl, -⟩ | ⟨hk, rfl, -⟩⟩
    · refine Or.inl ⟨u, ?_, by omega⟩
      obtain ⟨d', v, sg⟩ := vote
      obtain ⟨hv', hd⟩ := hfor
      simp only at hsig hv' hd
      rw [hsig, hv', hd, blk_view]; rfl
    · obtain ⟨d', v, sg⟩ := vote
      obtain ⟨hv', hd⟩ := hfor
      simp only at hsig hv' hd
      refine Or.inr ⟨w, Or.inl ⟨hk, ?_⟩, by omega⟩
      rw [hsig, hv', hd, hk]; rfl
    · obtain ⟨d', v, sg⟩ := vote
      obtain ⟨hv', hd⟩ := hfor
      simp only at hsig hv' hd
      refine Or.inr ⟨w, Or.inr ⟨hk, ?_⟩, by omega⟩
      rw [hsig, hv', hd, hk]; rfl
  · rw [upTo_self] at hrec; exact absurd hrec no_revote

include hv in
/-- Every honest node votes1 for each honest block by the step its validity report arrives in. -/
theorem votes1 (k : PubKey) (u : Nat) : ∃ j, j ≤ sAt u + 2
    ∧ Output.send (.vote1 ⟨(certOf (blk u)).data, ⟨vOf u⟩, k⟩) ∈ (tr k j).output := by
  have ho := offs k
  have hprop : (H k (sAt u + 3)).Received (.proposal (ldr (vOf u)) (blk u) (some ⟨⟨vOf u⟩, (blk u).payloadCommit⟩)) :=
    recv_view u 1 (by omega) (by omega)
  have hready : ParentReady cfg (H k (sAt u + 3)) (blk u) := by
    cases u with
    | zero => exact Or.inl rfl
    | succ u =>
      have := sAt_succ u
      exact Or.inr (Or.inr ⟨blk u, hasProposal_of u (by omega), by rw [blk_parent]; exact Nat.le_refl _,
        by rw [blk_parent]; rfl, payload_of u (by omega)⟩)
  have hcur : NotBehind cfg (H k (sAt u + 2 + 1)) (blk u).epoch :=
    notBehind (by rw [blk_epoch, show sAt u + 2 + 1 = sAt u + 3 from rfl, eAt_view u 3 (by omega)]; exact Nat.le_refl _)
  refine Classical.byContradiction fun hneg => settled k (sAt u + 2) (.vote1 (blk u))
    ⟨⟨_, _, hprop, by rw [blk_view]; exact leader_blk _ u, by rw [blk_view], rfl⟩, blk_wellFormed u, ?_, hready, blk_safe u,
      blk_opens u (by omega), hcur,
      fun ht => not_timedOut (by rwa [blk_view] at ht) (by omega), fun vote hs _ hvv => ?_, ?_⟩
  · rw [blk_view]; exact recv_view u 2 (by omega) (by omega)
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    rw [blk_view] at hvv
    rcases sent_vote1 hv hjv with ⟨u', rfl, -⟩ | ⟨w, ⟨-, rfl⟩ | ⟨-, rfl⟩, -⟩
    · obtain rfl : u' = u := vOf_inj (view_inj hvv)
      exact hneg ⟨j, by omega, hjv⟩
    · exact vOf_ne u w (view_inj hvv).symm
    · exact vOf_ne u w (view_inj hvv).symm
  · rw [blk_view]
    have := inView (B := B) (k := k) (sAt u + 3)
    rwa [(vAt_view u 3 (by omega)).2 (by omega)] at this

/-- A proposal a node holds, at an honest block's view, is that block's. -/
theorem block_of_view {k : PubKey} {n u : Nat} {y : Block} (hb : (H k n).HasProposal cfg y)
    (hv : y.viewNumber = ⟨vOf u⟩) : y = blk u := by
  rcases hasProposal hb with rfl | ⟨u', rfl, -⟩ | ⟨w, hE, -⟩
  · exact absurd (view_inj hv) (by unfold vOf; omega)
  · rw [blk_view] at hv
    obtain rfl : u' = u := vOf_inj (view_inj hv)
    rfl
  · have hEv : y.viewNumber = ⟨4 * w + 4⟩ := by rcases hE with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
    rw [hEv] at hv; exact absurd (view_inj hv).symm (vOf_ne u w)

include hv in
/-- Every vote2 a node sends is on an honest block, once it holds its `Cert1` and payload. -/
theorem sent_vote2 {k : PubKey} {j : Nat} {vote : Vote2} (hx : Output.send (.vote2 vote) ∈ (tr k j).output) :
    ∃ u, vote = ⟨(C2 u).data, ⟨vOf u⟩, k⟩ ∧ sAt u + lkOff k ≤ j := by
  obtain ⟨hsig, hgen, x, y, hc, hb, hcert, hpay, hvc, hdc⟩ :=
    (protocol hv k (j + 1)).vote2Justified j vote ⟨_, (getElem_H j), hx⟩ trivial
  rw [upTo_self] at hc hb hpay
  obtain ⟨d', v, sg⟩ := vote
  simp only at hsig hvc hdc hgen
  have hlk := lkOff_facts k
  rcases hasPayload hpay with hg | ⟨u, hyv, -, hlt⟩
  · -- At genesis: the anchor, whose certificate is at genesis too.
    exfalso
    rcases hasProposal hb with rfl | ⟨u, rfl, -⟩ | ⟨w, hE, -⟩
    · have hx0 : x.data.blockNumber = ⟨0⟩ := by rw [hcert.2]; rfl
      rcases hasCert1 hc with rfl | ⟨u, rfl, -⟩
      · rw [hvc] at hgen; exact Nat.lt_irrefl _ hgen
      · rw [cert_number] at hx0; exact absurd (number_inj hx0) (by omega)
    · rw [blk_view] at hg; exact absurd (view_inj hg) (by unfold vOf; omega)
    · have hEv : y.viewNumber = ⟨4 * w + 4⟩ := by rcases hE with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
      rw [hEv] at hg; exact absurd (view_inj hg) (by omega)
  · obtain rfl := block_of_view hb hyv
    obtain ⟨rfl, hct⟩ := cert_of_number (x := u) hc (by rw [hcert.2]; exact blk_number u)
    refine ⟨u, ?_, by omega⟩
    rw [hsig, hvc, hdc, cert_view]; rfl

include hv in
/-- Every honest node votes2 for each honest block once it holds its `Cert1` and payload. -/
theorem votes2 (k : PubKey) (u : Nat) : ∃ j, j ≤ sAt u + lkOff k
    ∧ Output.send (.vote2 ⟨(C2 u).data, ⟨vOf u⟩, k⟩) ∈ (tr k j).output := by
  have hlk := lkOff_facts k
  refine Classical.byContradiction fun hneg => settled k (sAt u + lkOff k) (.vote2 (certOf (blk u)))
    ⟨⟨blk u, hasCert1_of u (by omega), hasProposal_of u (by omega), ⟨Nat.le_refl _, rfl⟩,
      payload_of u (by omega)⟩, fun vote hs _ hvv => ?_, fun c2 hc2 _ hv2 => ?_, ?_, ?_⟩
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    obtain ⟨u', rfl, -⟩ := sent_vote2 hv hjv
    rw [cert_view] at hvv
    obtain rfl : u' = u := vOf_inj (view_inj hvv)
    exact hneg ⟨j, by omega, hjv⟩
  · obtain ⟨x, rfl, hlt⟩ := hasCert2 hc2
    rw [cert_view] at hv2
    obtain rfl : x = u := vOf_inj (view_inj hv2)
    omega
  · rw [cert_view]
    rintro (ht | ⟨tc, htc, hle⟩)
    · exact not_timedOut ht (by omega)
    · obtain ⟨w, rfl, hw⟩ := received_tc htc
      have := view_le_round (u := u) (w := w) hle
      omega
  · refine Kit.afterFloor_of input hv (by rw [cert_view]; show 0 < vOf u; unfold vOf; omega) fun x hx => ?_
    rw [cert_view]
    have := lkOff_facts k
    rcases hasProposal hx with rfl | ⟨y, rfl, hy⟩ | ⟨w, ⟨-, rfl⟩ | ⟨-, rfl⟩, hw⟩
    · show 0 < vOf u + 20; omega
    · rw [blk_view]; show vOf y < vOf u + 20; unfold vOf sAt at *; omega
    · show 4 * w + 4 < vOf u + 20; unfold vOf sAt at *; omega
    · show 4 * w + 4 < vOf u + 20; unfold vOf sAt at *; omega

end Sends

/-! ## Time -/

section Time

/-- When step `r` of a round happens, from the round's start: the timers for `d`'s view `τ = 33` after each node entered it. -/
def off (r : Nat) : Nat := if r ≤ 24 then r else r + 25

/-- When step `n` happens. A round takes `54` time units. -/
def tm (n : Nat) : Nat := 54 * (n / 29) + off (n % 29)

theorem tm_at (w x : Nat) (hx : x < 29) : (x ≤ 24 → tm (29 * w + x) = 54 * w + x)
    ∧ (25 ≤ x → tm (29 * w + x) = 54 * w + x + 25) := by
  obtain ⟨h1, h2⟩ := split29 w x hx
  unfold tm off
  rw [h1, h2]
  exact ⟨fun h => by rw [ite_eq_left h], fun h => by rw [ite_eq_right (by omega)]; omega⟩

/-- The time of a step of an honest view: one unit a step from its base. -/
theorem tm_view (u r : Nat) (hr : r < 7) : tm (sAt u + r) = tm (sAt u) + r := by
  obtain ⟨w, i, hi, rfl, hs, -⟩ := sAt_vOf u
  rw [hs, Nat.add_assoc, (tm_at w (7 * i + r) (by omega)).1 (by omega)]
  rw [show 29 * w + 7 * i = 29 * w + (7 * i) by omega, (tm_at w (7 * i) (by omega)).1 (by omega)]
  omega

theorem tm_succ (n : Nat) : tm n ≤ tm (n + 1) := by
  have h := Nat.div_add_mod n 29
  obtain ⟨w, x, hx, rfl⟩ : ∃ w x, x < 29 ∧ n = 29 * w + x := ⟨n / 29, n % 29, Nat.mod_lt _ (by omega), by omega⟩
  by_cases h28 : x = 28
  · subst h28
    rw [show 29 * w + 28 + 1 = 29 * (w + 1) + 0 by omega]
    have := (tm_at w 28 (by omega)).2 (by omega)
    have := (tm_at (w + 1) 0 (by omega)).1 (by omega)
    omega
  · rw [show 29 * w + x + 1 = 29 * w + (x + 1) by omega]
    have := tm_at w x hx
    have := tm_at w (x + 1) (by omega)
    omega

theorem tm_mono {n m : Nat} (h : n ≤ m) : tm n ≤ tm m := Kit.mono_of_succ tm_succ h

theorem tm_ge (n : Nat) : n ≤ tm n := by
  obtain ⟨w, x, hx, rfl⟩ : ∃ w x, x < 29 ∧ n = 29 * w + x := ⟨n / 29, n % 29, Nat.mod_lt _ (by omega), by omega⟩
  have := tm_at w x hx
  omega

/-- A node enters `d`'s view of round `w + 1` at the step the round's last block's `Cert1` arrives in. -/
theorem vAt_entry {k : PubKey} {n w : Nat} (h1 : vAt k (n + 1) = 4 * w + 4) (h2 : vAt k n ≠ 4 * w + 4) :
    n = 29 * w + 14 + c1Off k := by
  have ho := offs k
  rcases steps (n + 1) with ⟨u, r, hr, hn⟩ | ⟨w', s, hs, hn⟩
  · obtain ⟨hhi, hlo⟩ := vAt_view (k := k) u r hr
    rw [hn] at h1
    obtain ⟨wu, iu, hiu, rfl, hsu, hvu⟩ := sAt_vOf u
    by_cases hc : c1Off k < r
    · rw [hhi hc, hvu] at h1
      obtain ⟨rfl, rfl⟩ : w = wu ∧ iu = 2 := by omega
      -- Entered at this step, so the step before was still in view `4w + 3`.
      by_cases hr1 : c1Off k + 1 = r
      · omega
      · exfalso; apply h2
        rw [show n = sAt (3 * w + 2) + (r - 1) by omega]
        rw [(vAt_view (3 * w + 2) (r - 1) (by omega)).1 (by omega), hvu]
    · rw [hlo (by omega), hvu] at h1; omega
  · rw [hn, vAt_faulty w' s hs] at h1
    obtain rfl : w = w' := by omega
    exfalso; apply h2
    by_cases hs0 : s = 0
    · subst hs0
      rw [show n = sAt (3 * w + 2) + 6 by rw [sAt_at w 2 (by omega)]; omega,
        (vAt_view (3 * w + 2) 6 (by omega)).1 (by omega), vOf_at w 2 (by omega)]
    · rw [show n = 29 * w + 21 + (s - 1) by omega, vAt_faulty w _ (by omega)]

end Time

/-! ## What `d` owes

With epochs, `d` is honest in the first epoch and owes what a node owes there. It
runs the machine, and before its first timeout vote, step 25, it signs only for
the first epoch: what it owes is then what the machine owes, which is nothing.
After that it owes nothing for the first epoch anyway: it has moved past the
first epoch's views and holds its `Cert2`s, and it decided each of its blocks.
-/

section Owes

variable (hv : ∀ b, BlockValid b)

theorem ep_first (hB : B = true) {u : Nat} (h : sAt u < 25) : (⟨ep B u⟩ : EpochNumber) = ⟨1⟩ := by
  rw [ep_of hB]; unfold sAt at h; congr 1; omega

theorem ep_ne_one (w : Nat) : (⟨ep B (3 * w + 3)⟩ : EpochNumber) ≠ ⟨1⟩ := fun h => by
  have := congrArg EpochNumber.toNat h
  cases B <;> simp only [ep] at this <;> omega

theorem honest_first (hB : B = true) : (C' B).honest ⟨1⟩ d := Or.inr ⟨hB, rfl, rfl⟩

include hv in
/-- Before step 25, `d` signs only for the first epoch. -/
theorem d_early (hB : B = true) {j : Nat} (hj : j < 25) {o : Output} (ho : o ∈ (tr d j).output) :
    o.SignedIn ((C' B).honest · d) := by
  have hfirst : ∀ u, sAt u < 25 → (C' B).honest ⟨ep B u⟩ d := fun u hu => by
    rw [ep_first hB hu]; exact honest_first hB
  cases o with
  | send m =>
    cases m with
    | proposal p =>
      obtain ⟨u, hk, -⟩ := sent_proposal hv ho
      exact absurd hk.symm (ldr_blk u).2
    | revote r =>
      obtain ⟨hlead, ⟨-, -, hlast⟩, hjust, -⟩ :=
        (protocol hv d (j + 1)).revoteJustified j r ⟨_, (getElem_H j), ho⟩ trivial
      rw [upTo_self] at hjust
      rcases hasCert1 (hasCert1_of_certJustified hjust) with h | ⟨u, h, hu⟩
      · rw [h] at hlast; exact absurd rfl hlast.1
      · have he : r.cert.data.epoch = ⟨1⟩ := by
          rw [h, cert_epoch, ep_first hB (by have := offs_nat d; omega)]
        exact absurd he ((leader_some hlead).2 rfl)
    | vote1 vote =>
      rcases sent_vote1 hv ho with ⟨x, rfl, hx⟩ | ⟨w, ⟨hka, -⟩ | ⟨hkb, -⟩, -⟩
      · show (C' B).honest (certOf (blk x)).data.epoch d
        rw [cert_epoch]; exact hfirst x (by omega)
      · exact absurd hka (by decide)
      · exact absurd hkb (by decide)
    | vote2 vote =>
      obtain ⟨x, rfl, hx⟩ := sent_vote2 hv ho
      show (C' B).honest (C2 x).data.epoch d
      rw [c2_epoch]; exact hfirst x (by have := lkOff_facts d; omega)
    | timeoutVote vote =>
      obtain ⟨w, hjw, -⟩ := sent_timeout ho
      have : tOff d = 4 := rfl
      omega
    | _ => trivial
  | decided blocks c1 c2 =>
    obtain ⟨-, -, -, hc2, -⟩ := (protocol hv d (j + 1)).decideJustified j _ blocks c1 c2 (getElem_H j) ho trivial
    rw [upTo_self] at hc2
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc2
    show (C' B).honest (C2 x).data.epoch d
    rw [c2_epoch]; exact hfirst x (by omega)

include hv in
theorem d_restrict (hB : B = true) {n : Nat} (hn : n ≤ 25) : (H d n).restrict ((C' B).honest · d) = H d n :=
  restrict_eq_self fun st hst o ho => by
    obtain ⟨j, hj, rfl⟩ := (Trace.mem_history _).mp hst
    exact d_early hv hB (by omega) ho

include hv in
/-- Before step 22, `d` decides only views up to three. -/
theorem d_decided_small {n : Nat} (hn : n ≤ 22) {w : ViewNumber} (hd : (H d n).DecidedView w) : w.toNat ≤ 3 := by
  obtain ⟨st, hst, blocks, c1, c2, x, hm, hx, rfl⟩ := hd
  obtain ⟨j, hj, rfl⟩ := (Trace.mem_history _).mp hst
  obtain ⟨-, -, -, -, -, -, -, -, hheld⟩ := (protocol hv d (j + 1)).decideJustified j _ blocks c1 c2 (getElem_H j) hm trivial
  obtain ⟨hb, hgen⟩ := hheld x hx
  rw [upTo_self] at hb
  rcases hasProposal hb with rfl | ⟨u, rfl, hu⟩ | ⟨w, ⟨hka, -⟩ | ⟨hkb, -⟩, -⟩
  · exact absurd hgen (Nat.lt_irrefl 0)
  · rw [blk_view]; show vOf u ≤ 3; unfold sAt at hu; unfold vOf; omega
  · exact absurd hka (by decide)
  · exact absurd hkb (by decide)

include hv in
/-- `d` decides each block of the first epoch by step 25. -/
theorem d_decided {u : Nat} (hu : u ≤ 2) : (H d 25).DecidedView ⟨vOf u⟩ := by
  have hs : sAt u + 7 ≤ 22 := by unfold sAt; omega
  refine Classical.byContradiction fun hneg => settled d (sAt u + 6) (.decide (C2 u))
    ⟨blk u, certOf (blk u), hasCert2_of u (by omega), hasProposal_of u (by omega),
      ⟨by rw [blk_view]; exact Nat.le_refl _, rfl⟩, hasCert1_of u (by have := offs_nat d; omega),
      ⟨by rw [cert_view, blk_view]; exact Nat.le_refl _, rfl⟩,
      fun hd => hneg (by rw [blk_view] at hd; exact Liveness.decidedView_mono _ (by omega) hd),
      by rw [blk_view]; show 0 < vOf u; unfold vOf; omega,
      fun w hw => by
        have := d_decided_small hv (by omega) hw
        rw [blk_view]; show w.toNat - 20 < vOf u; unfold vOf; omega⟩

include hv in
/-- From step 25 on, `d` owes nothing for the first epoch. -/
theorem d_late (hB : B = true) (n : Nat) (hn : 25 ≤ n) (o : Obligation)
    (hf : (C' B).honest o.epoch d) :
    ¬ Owed cfg leader d ((H d (n + 1)).restrict ((C' B).honest · d)) o := by
  have he := sameInputs_restrict (P := ((C' B).honest · d)) (h := H d (n + 1))
  have hfirst : ∀ u, (C' B).honest ⟨ep B u⟩ d → u ≤ 2 := fun u h => by
    have := congrArg EpochNumber.toNat (d_honest h).2
    rw [ep_of hB] at this; simp only at this; omega
  cases o with
  | vote1 p =>
    rintro ⟨⟨s, vid, hr, -, -⟩, -, -, -, -, -, -, -, -, hin⟩
    obtain ⟨j, -, hji⟩ := received.mp ((he.received _).mp hr)
    have hview := inView_eq ((he.inView _).mp hin)
    have hge := (vAt_ge (k := d) (n + 1)).2.1 2 (by have := offs_nat d; unfold sAt; omega)
    rcases input_proposal hji with ⟨u, -, -, rfl, -⟩ | ⟨w, -, -, ⟨hka, -⟩ | ⟨hkb, -⟩⟩
    · have hu := hfirst u (by rw [← blk_epoch]; exact hf)
      rw [blk_view] at hview
      have := congrArg ViewNumber.toNat hview
      simp only at this; unfold vOf at this hge; omega
    · exact absurd hka (by decide)
    · exact absurd hkb (by decide)
  | vote1Again r =>
    rintro ⟨⟨s, hr, -⟩, -⟩
    exact no_revote ((he.received _).mp hr)
  | vote2 c =>
    rintro ⟨⟨x, hc, -⟩, -, hc2, -, hgen⟩
    rcases hasCert1 ((he.hasCert1 _).mp hc) with rfl | ⟨u, rfl, -⟩
    · exact absurd hgen.1 (Nat.lt_irrefl 0)
    · have hu := hfirst u (by rw [← cert_epoch]; exact hf)
      exact hc2 (C2 u) ((he.hasCert2 _).mpr (hasCert2_of u (by unfold sAt; omega))) rfl (by rw [cert_view]; rfl)
  | decide c =>
    rintro ⟨x, c1, hc, hb, hcm, -, -, hdec, -⟩
    obtain ⟨u, rfl, -⟩ := hasCert2 ((he.hasCert2 _).mp hc)
    have hu := hfirst u (by rw [← c2_epoch]; exact hf)
    have hxn : x.blockHeader.blockNumber = ⟨u + 1⟩ := by
      rw [← cert_number u]; exact (congrArg Vote2Data.blockNumber hcm.2).symm
    obtain rfl : x = blk u := by
      rcases hasProposal ((he.hasProposal _).mp hb) with rfl | ⟨u', rfl, -⟩ | ⟨w, ⟨hka, -⟩ | ⟨hkb, -⟩, -⟩
      · exact absurd (number_inj hxn) (by omega)
      · rw [blk_number] at hxn; obtain rfl : u' = u := by have := number_inj hxn; omega
        rfl
      · exact absurd hka (by decide)
      · exact absurd hkb (by decide)
    have h25 : ((H d (n + 1)).restrict ((C' B).honest · d)).upTo 25 = H d 25 := by
      rw [restrict_upTo, upTo_H, Nat.min_eq_left (by omega), d_restrict hv hB (Nat.le_refl _)]
    obtain ⟨st, hst, rest⟩ := d_decided hv hu
    rw [← h25] at hst
    exact hdec ⟨st, List.mem_of_mem_take hst, by rw [blk_view]; exact rest⟩
  | propose e v =>
    rintro ⟨hj, -⟩
    have he1 := (d_honest hf).2
    rcases hj with ⟨p, hp, -, hpe⟩ | ⟨r, hr, -, hre⟩
    · exact (leader_some hp.leads).2 rfl (hpe.trans he1)
    · exact (leader_some hr.leads).2 rfl (hre.trans he1)

include hv in
/-- After every step, no node owes anything for the epochs it is honest in. -/
theorem settledIn {k : PubKey} (hk : (C' B).Honest k) (n : Nat) (o : Obligation) :
    ¬ OwedIn cfg leader k ((C' B).honest · k) (H k (n + 1)) o := by
  rintro ⟨hf, ho⟩
  by_cases hkd : k = d
  · subst hkd
    obtain ⟨e, he⟩ := hk
    have hB := (d_honest he).1
    by_cases hn : n + 1 ≤ 25
    · rw [d_restrict hv hB hn] at ho; exact settled d n o ho
    · exact d_late hv hB n (by omega) o hf ho
  · rw [restrict_all (steady hk hkd)] at ho; exact settled k n o ho

end Owes

/-! ## The network -/

section Net

variable (hv : ∀ b, BlockValid b) (hcf : CollisionFree)

include hv in
theorem backed1 (u : Nat) : Cert1Backed (C := C' B) (fun k _ => tr k) (certOf (blk u)) := by
  refine ⟨fun k => (C' B).members ⟨0⟩ k ∧ (C' B).honest ⟨0⟩ k, members_quorum _, fun k _ _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes1 hv k u
  rw [cert_view]
  exact ⟨j, hj⟩

include hv in
theorem backed2 (u : Nat) : Cert2Backed (C := C' B) (fun k _ => tr k) (C2 u) := by
  refine ⟨fun k => (C' B).members ⟨0⟩ k ∧ (C' B).honest ⟨0⟩ k, members_quorum _, fun k _ _ => ?_⟩
  obtain ⟨j, -, hj⟩ := votes2 hv k u
  exact ⟨j, hj⟩

theorem tcBacked (w : Nat) : TimeoutCertBacked (C := C' B) (fun k _ => tr k) (T w) :=
  ⟨fun k => (C' B).members ⟨0⟩ k ∧ (C' B).honest ⟨0⟩ k, members_quorum _, fun k _ _ =>
    ⟨_, ⟨rfl, rfl, rfl, Or.inr ⟨rfl, Nat.le_refl _⟩⟩, ⟨_, times_out k w (Or.inl rfl)⟩⟩⟩

include hv in
theorem tcChecked (w : Nat) : TimeoutLockChecked (C := C' B) (fun k _ => tr k) cfg (T w) :=
  ⟨Or.inr (backed1 hv (3 * w + 2)), by
    show (certOf (blk (3 * w + 2))).view.toNat ≤ 4 * w + 4
    rw [cert_view, vOf_at w 2 (by omega)]; show 4 * w + 2 + 1 ≤ 4 * w + 4; omega⟩

/-- The honest nodes running the machine on their schedules. -/
def net (B : Bool) : TimedNetwork cfg leader (C' B) where
  honestQuorum := members_quorum
  trace k _ := tr k
  safe k _ n := .of_every (protocol hv k n).toSafeHistory
  cert1Genuine k _ n x hc := by
    rcases Input.mem_cert1.mp hc with hin | ⟨c2, p, hin⟩ | ⟨s, p, vid, hin, rfl⟩
    · obtain ⟨u, -, rfl⟩ := input_cert1 hin
      exact Or.inr (backed1 hv u)
    · obtain ⟨w, -, -, rfl, -, -⟩ := input_epochChange hin
      exact Or.inr (backed1 hv _)
    · obtain ⟨v, rfl⟩ := input_share hin
      rcases input_proposal hin with ⟨u, -, -, rfl, -⟩ | ⟨w, -, -, ⟨-, rfl, -⟩ | ⟨-, rfl, -⟩⟩
      · rw [blk_parent]
        cases u with
        | zero => exact Or.inl rfl
        | succ u => exact Or.inr (backed1 hv u)
      · exact Or.inr (backed1 hv _)
      · exact Or.inr (backed1 hv _)
  cert2Genuine k _ n x hc := by
    rcases Input.mem_cert2.mp hc with hin | ⟨c1, p, hin⟩
    · obtain ⟨u, -, rfl⟩ := input_cert2 hin
      exact backed2 hv u
    · obtain ⟨w, -, -, -, rfl, -⟩ := input_epochChange hin
      exact backed2 hv _
  timeoutCertGenuine k _ n tc hc := by
    rcases Input.mem_timeoutCert.mp hc with hin | ⟨s, p, vid, hin, hte⟩ | ⟨s, r, hin, -⟩
    · obtain ⟨w, -, rfl⟩ := input_tc hin
      exact ⟨tcBacked w, tcChecked hv w⟩
    · obtain ⟨v, rfl⟩ := input_share hin
      rcases input_proposal hin with ⟨u, -, -, rfl, -⟩ | ⟨w, -, -, ⟨-, rfl, -⟩ | ⟨-, rfl, -⟩⟩
      · by_cases hu : u = 0 ∨ u % 3 ≠ 0
        · rw [blk_evidence_none hu] at hte; cases hte
        · obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 3 := ⟨u / 3 - 1, by omega⟩
          rw [blk_evidence] at hte; cases hte
          exact ⟨tcBacked w, tcChecked hv w⟩
      · cases hte
      · cases hte
    · exact absurd hin (input_none _ _)
  revoteGenuine k _ n s r hin := absurd hin (input_none s r)
  time _ _ n := tm n
  timeMono _ _ n := tm_succ n
  protocol k _ n := .of_every (protocol hv k n)
  timeoutCertCausal k _ n tc hin := by
    obtain ⟨w, rfl, rfl⟩ := input_tc hin
    refine ⟨fun k => (C' B).members ⟨0⟩ k ∧ (C' B).honest ⟨0⟩ k, members_quorum _, fun k' _ _ =>
      ⟨_, _, ⟨rfl, rfl, rfl, Or.inr ⟨rfl, Nat.le_refl _⟩⟩, times_out k' w (Or.inl rfl), ?_⟩⟩
    have := offs k'
    have e1 := (tm_at w (21 + tOff k') (by omega)).2 (by omega)
    rw [← Nat.add_assoc] at e1
    have e2 := (tm_at w 28 (by omega)).2 (by omega)
    show tm (29 * w + 21 + tOff k') < tm (29 * w + 28)
    omega
  oneHonestCausal k _ n v hin := by
    obtain ⟨w, rfl, rfl⟩ := input_oneHonest hin
    refine ⟨a, _, Or.inl (Or.inl rfl), _, _, rfl, times_out a w (Or.inl rfl), rfl, ?_⟩
    have := (tm_at w 25 (by omega)).2 (by omega)
    have := (tm_at w 26 (by omega)).2 (by omega)
    show tm (29 * w + 25) < tm (29 * w + 26)
    omega
  authentic k _ n l msg hin hl hsg := by
    cases hi : (tr k n).input <;> rw [hi] at hin <;> simp only [Input.sentBy, reduceCtorEq] at hin
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hin)
      obtain ⟨v, rfl⟩ := input_share hi
      rcases input_proposal hi with ⟨u, rfl, rfl, rfl, rfl⟩ | ⟨w, -, rfl, hp⟩
      · obtain ⟨j, hj, hjp⟩ := proposes hv u
        refine ⟨j, hjp, Nat.lt_of_le_of_lt (tm_mono hj) ?_⟩
        show tm (sAt u) < tm (sAt u + 1)
        rw [tm_view u 1 (by omega)]; omega
      · -- `d`'s blocks are of an epoch it is not honest in.
        rcases hp with ⟨-, rfl, -⟩ | ⟨-, rfl, -⟩ <;> exact absurd (d_honest hsg).2 (ep_ne_one w)
    · exact absurd hi (input_none _ _)

theorem net_time {k : PubKey} {hk : (C' B).Honest k} {n : Nat} : (net hv B).time k hk n = tm n := rfl

theorem by_at {k : PubKey} {hk : (C' B).Honest k} {t₀ m : Nat} {P : History → Prop}
    (hm : 0 < m → tm (m - 1) ≤ t₀) (hp : P (H k m)) : (net hv B).By k hk t₀ P :=
  Kit.by_at (fun _ _ => rfl) (fun _ _ _ => rfl) tm_succ hm hp

/-- What holds after step `j`, holds by its time. -/
theorem by_step {k : PubKey} {hk : (C' B).Honest k} {t₀ : Nat} {P : History → Prop} (j : Nat)
    (hj : tm j ≤ t₀) (hp : P (H k (j + 1))) : (net hv B).By k hk t₀ P :=
  by_at hv (fun _ => by simp only [Nat.add_sub_cancel]; exact hj) hp

theorem sentBy {k : PubKey} {hk : (C' B).Honest k} {t : Nat} {m : Message} (hs : (net hv B).SentByTime k hk t m) :
    ∃ j, tm j ≤ t ∧ Output.send m ∈ (tr k j).output :=
  Kit.sentBy (N := net hv B) (fun _ _ => rfl) (fun _ _ _ => rfl) hs

/-- After `d`'s view of round `y + 1`, every node is locked on that round's last block or a later one. -/
theorem lockAt_index {k : PubKey} {n y : Nat} (h : 29 * y + 28 < n) :
    ∃ j, lockAt k n = certOf (blk j) ∧ 3 * y + 2 ≤ j := by
  rcases steps n with ⟨u, r, hr, rfl⟩ | ⟨w, s, hs, rfl⟩
  · have hu := round_idx (w := y) (u := u) (by omega)
    obtain ⟨hhi, hlo⟩ := lockAt_view (k := k) u r hr
    by_cases hc : lkOff k < r
    · exact ⟨u, hhi hc, by omega⟩
    · obtain ⟨u', rfl⟩ : ∃ u', u = u' + 1 := ⟨u - 1, by omega⟩
      exact ⟨u', hlo (by omega), by omega⟩
  · exact ⟨3 * w + 2, lockAt_faulty w s hs, by omega⟩

/-- The time of honest view bases one after another in a round. -/
theorem tm_next (u : Nat) (hu : u % 3 ≠ 2) : sAt (u + 1) = sAt u + 7 ∧ tm (sAt (u + 1)) = tm (sAt u) + 7 := by
  obtain ⟨w, i, hi, rfl, hs, -⟩ := sAt_vOf u
  have hi2 : i < 2 := by omega
  have hs' := sAt_at w (i + 1) (by omega)
  rw [show 3 * w + i + 1 = 3 * w + (i + 1) by omega, hs', hs]
  refine ⟨by omega, ?_⟩
  rw [(tm_at w (7 * (i + 1)) (by omega)).1 (by omega), (tm_at w (7 * i) (by omega)).1 (by omega)]
  omega

/-- The step before an honest view's base is one time unit earlier. -/
theorem tm_before (u : Nat) (hu : 1 ≤ u) : tm (sAt u) ≤ tm (sAt u - 1) + 1 := by
  obtain ⟨w, i, hi, rfl, hs, -⟩ := sAt_vOf u
  rw [hs]
  by_cases hi0 : i = 0
  · subst hi0
    obtain ⟨w', rfl⟩ : ∃ w', w = w' + 1 := ⟨w - 1, by omega⟩
    rw [show 29 * (w' + 1) + 7 * 0 - 1 = 29 * w' + 28 by omega, show 29 * (w' + 1) + 7 * 0 = 29 * (w' + 1) + 0 by omega,
      (tm_at (w' + 1) 0 (by omega)).1 (by omega), (tm_at w' 28 (by omega)).2 (by omega)]
    omega
  · rw [show 29 * w + 7 * i - 1 = 29 * w + (7 * i - 1) by omega, (tm_at w (7 * i) (by omega)).1 (by omega),
      (tm_at w (7 * i - 1) (by omega)).1 (by omega)]
    omega

include hv in
/-- A header for every view an honest leader is ready to propose in arrives within `Δ`. -/
theorem header_arrives {k : PubKey} {hk : (C' B).Honest k} {n : Nat} {p : Proposal}
    (hpe : (C' B).honest p.epoch k) (hready : ProposalReady cfg leader k (H k (n + 1)) p) :
    (net hv B).By k hk (max (tm n) 0 + 4) fun hist =>
      ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
  obtain ⟨hlead, ⟨hlt, hnext, hep, hnum⟩, hj, -, -, hsafe, -, -⟩ := hready
  have hkl : k = ldr p.viewNumber.toNat := (leader_some hlead).1
  have hmax := Nat.le_max_left (tm n) 0
  have hc := hasCert1_of_certJustified hj
  -- Every header arrives within `Δ` of the node's being able to use it; `m` is when.
  have hdone : ∀ m, m ≤ n + 1 ∨ tm (m - 1) ≤ tm n + 4 → ∀ hdr', hdr'.blockNumber = p.blockHeader.blockNumber →
      (H k m).Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') →
      (net hv B).By k hk (max (tm n) 0 + 4) fun hist =>
        ∃ hdr', hdr'.blockNumber = p.blockHeader.blockNumber
          ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr') := by
    intro m hm hdr' h1 h2
    rcases hm with hm | hm
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
        ⟨hdr', h1, received_mono hm h2⟩
    · exact by_at hv (m := m) (fun _ => by omega) ⟨hdr', h1, h2⟩
  -- After a timeout, the parent is what the node was locked on, so the round's last block.
  have hlocked : ∀ tc, p.timeoutEvidence = some tc → ∃ y, tc = T y ∧ 29 * y + 28 < n + 1
      ∧ ∃ j, p.parentCert = certOf (blk j) ∧ 3 * y + 2 ≤ j := fun tc hte => by
    have hj' := hj; unfold ParentJustified at hj'; rw [hte] at hj'
    obtain ⟨-, m, hr, ⟨l, hl, hld⟩ | ⟨hlast, -⟩⟩ := hj'
    · rw [upTo_H] at hr hl
      obtain ⟨y, rfl, hy⟩ := received_tc hr
      obtain ⟨j, hlj, hjy⟩ := lockAt_index (k := k) (y := y) hy
      rw [← lockedOn_eq hl] at hlj
      subst hlj
      refine ⟨y, rfl, by omega, j, ?_, hjy⟩
      rcases hasCert1 hc with h | ⟨x, h, -⟩
      · exfalso; rw [h] at hld
        have := congrArg Vote1Data.blockNumber hld; rw [cert_number] at this
        exact absurd (number_inj this) (by omega)
      · rw [h] at hld ⊢
        have := congrArg Vote1Data.blockNumber hld; rw [cert_number, cert_number] at this
        obtain rfl : j = x := by have := number_inj this; omega
        rfl
    · -- Or an epoch's last block, which the certificate's epoch, the proposal's, names as the one before.
      rw [upTo_H] at hr
      obtain ⟨y, rfl, hy⟩ := received_tc hr
      refine ⟨y, rfl, by omega, ?_⟩
      obtain ⟨hep', -⟩ := hsafe _ hte
      rcases hasCert1 hc with h | ⟨x, h, -⟩
      · rw [h] at hlast; exact absurd hlast.1 (fun h' => h' rfl)
      · refine ⟨x, h, ?_⟩
        have hpn : p.blockHeader.blockNumber = ⟨x + 1 + 1⟩ := by rw [← hnum, h, cert_number]; rfl
        rw [hep, hpn, epochOf_number] at hep'
        have h1 : ep B (3 * y + 3) = ep B (x + 1) := congrArg EpochNumber.toNat hep'
        rw [h, cert_number] at hlast
        obtain ⟨hB, -, h2⟩ := last_iff.mp hlast
        rw [ep_of hB, ep_of hB] at h1
        omega
  rcases hasCert1 hc with hpc | ⟨x, hpc, hx⟩
  · -- On genesis: view one, whose header comes first.
    have hpv : p.viewNumber = ⟨1⟩ := by
      rcases hnext with ⟨-, h⟩ | ⟨tc, hte, -⟩
      · rw [← h, hpc]; rfl
      · exfalso
        obtain ⟨y, -, -, j, hj2, -⟩ := hlocked tc hte
        rw [hpc] at hj2
        have := congrArg (·.data.blockNumber.toNat) hj2; simp only [cert_number] at this
        exact absurd this (by show ¬ 0 = j + 1; omega)
    refine hdone 1 (Or.inr (by show tm 0 ≤ _; have : tm 0 = 0 := rfl; omega)) (hdr 1)
      (by rw [← hnum, hpc]; rfl) ?_
    rw [hpv, hpc]; exact recv_view 0 0 (by omega) (by decide)
  · have hnum' : p.blockHeader.blockNumber = ⟨x + 2⟩ := by rw [← hnum, hpc, cert_number]; rfl
    rcases hnext with ⟨hte, h⟩ | ⟨tc, hte, htv⟩
    · -- On a block: the view after it, which an honest node leads only within a round.
      have hpv : p.viewNumber = ⟨vOf x + 1⟩ := by rw [← h, hpc, cert_view]; rfl
      have hx2 : x % 3 ≠ 2 := fun h2 => by
        obtain ⟨w, rfl⟩ : ∃ w, x = 3 * w + 2 := ⟨x / 3, by omega⟩
        rw [hpv, vOf_at w 2 (by omega)] at hkl
        have hd := (ldr_at w).2.2.2
        rw [show 4 * w + 2 + 1 + 1 = 4 * w + 4 by omega, hd] at hkl
        exact (leader_some hlead).2 hkl (d_honest (hkl ▸ hpe)).2
      have hvx : vOf x + 1 = vOf (x + 1) := by unfold vOf; omega
      obtain ⟨hsn, htn⟩ := tm_next x hx2
      -- Ready only once it holds the parent's certificate.
      have hlk : sAt x + c1Off k < n + 1 := by
        have hl := hj; unfold ParentJustified at hl; rw [hte, hpc] at hl
        rcases hasCert1 (Liveness.hasCert1_of_buildable hl) with h0 | ⟨x', h0, hlt'⟩
        · have := congrArg (·.data.blockNumber.toNat) h0; simp only [cert_number] at this
          exact absurd this (by show ¬ x + 1 = 0; omega)
        · have := congrArg (·.data.blockNumber.toNat) h0; simp only [cert_number] at this
          obtain rfl : x' = x := by omega
          exact hlt'
      have hf := offs_nat k
      have := tm_mono (show sAt x + 3 ≤ n by omega)
      rw [tm_view x 3 (by omega)] at this
      refine hdone (sAt (x + 1) + 1) (by
          by_cases hn : sAt (x + 1) + 1 ≤ n + 1
          · exact Or.inl hn
          · exact Or.inr (by simp only [Nat.add_sub_cancel]; omega)) (hdr (x + 2)) hnum'.symm ?_
      rw [hpv, hpc, hvx]
      exact recv_view (x + 1) 0 (by omega) (by omega)
    · -- After a timeout: the block after the round's last one.
      obtain ⟨y, rfl, hy, j, hj2, hjy⟩ := hlocked _ hte
      rw [hpc] at hj2
      obtain rfl : x = j := by
        have := congrArg (·.data.blockNumber.toNat) hj2; simp only [cert_number] at this; omega
      have hpv : p.viewNumber = ⟨4 * y + 5⟩ := by
        rw [← htv]; show (⟨4 * y + 4 + 1⟩ : ViewNumber) = _; rfl
      have hxy : x = 3 * y + 2 := by
        have h1 : (certOf (blk x)).view.toNat < p.viewNumber.toNat := by rw [← hpc]; exact hlt
        rw [cert_view, hpv] at h1
        have h1' : vOf x < 4 * y + 5 := h1
        by_cases hge : 3 * y + 3 ≤ x
        · have := vOf_mono hge
          have : vOf (3 * y + 3) = 4 * y + 5 := by unfold vOf; omega
          omega
        · omega
      subst hxy
      have hs3 : sAt (3 * y + 3) = 29 * (y + 1) + 0 := by unfold sAt; omega
      have := tm_mono (show 29 * y + 28 ≤ n by omega)
      have := (tm_at y 28 (by omega)).2 (by omega)
      have := (tm_at (y + 1) 0 (by omega)).1 (by omega)
      refine hdone (sAt (3 * y + 3) + 1) (by
          by_cases hn : sAt (3 * y + 3) + 1 ≤ n + 1
          · exact Or.inl hn
          · refine Or.inr ?_
            simp only [Nat.add_sub_cancel]
            rw [hs3]
            omega) (hdr (3 * y + 2 + 2)) hnum'.symm ?_
      rw [hpv, hpc]
      have h45 : (⟨4 * y + 5⟩ : ViewNumber) = ⟨vOf (3 * y + 3)⟩ := by congr 1; unfold vOf; omega
      rw [h45]
      exact recv_view (3 * y + 3) 0 (by omega) (by omega)

/-- The timer for `d`'s view does not fire before `τ` has passed since the node entered it. -/
theorem timer_not_early {k : PubKey} {n m : Nat} {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v)
    (hnot : n = 0 ∨ ¬ (H k n).InView cfg v) (hinm : input k m = .timeout v) : tm n + 33 ≤ tm m := by
  obtain ⟨w, rfl, rfl⟩ := input_timeout hinm
  have h1 : vAt k (n + 1) = 4 * w + 4 := (congrArg ViewNumber.toNat (inView_eq hin)).symm
  have h2 : vAt k n ≠ 4 * w + 4 := fun h => by
    rcases hnot with rfl | hnot
    · have : vAt k 0 = 1 := by simp [vAt, vOf]
      omega
    · exact hnot (by have := inView (B := B) (k := k) n; rwa [h] at this)
  rw [vAt_entry h1 h2]
  have ho := offs k
  have e1 := (tm_at w (14 + c1Off k) (by omega)).1 (by omega)
  have e2 := (tm_at w (21 + tOff k) (by omega)).2 (by omega)
  rw [← Nat.add_assoc] at e1 e2
  omega

/-- The timer for a view fires within `τ` of the node's entering it, unless the node has moved on. -/
theorem timer_fires {k : PubKey} (n : Nat) {v : ViewNumber} (hin : (H k (n + 1)).InView cfg v) :
    ∃ m, n < m ∧ tm m ≤ tm n + 33 ∧ (input k m = .timeout v ∨ ∃ w, v < w ∧ (H k (m + 1)).InView cfg w) := by
  have hv' := inView_eq hin
  subst hv'
  have ho := offs k
  have hon := offs_nat k
  rcases steps (n + 1) with ⟨u, r, hr, hn1⟩ | ⟨w, s, hs, hn1⟩
  · obtain ⟨hhi, hlo⟩ := vAt_view (k := k) u r hr
    rw [hn1]
    by_cases hc : c1Off k < r
    · rw [hhi hc]
      by_cases hu2 : u % 3 = 2
      · -- Entered `d`'s view: its timer fires `τ` later.
        obtain ⟨w, rfl⟩ : ∃ w, u = 3 * w + 2 := ⟨u / 3, by omega⟩
        have hs2 := sAt_at w 2 (by omega)
        refine ⟨29 * w + 21 + tOff k, by omega, ?_, Or.inl ?_⟩
        · have := tm_mono (show sAt (3 * w + 2) + c1Off k ≤ n by omega)
          rw [tm_view _ _ (by omega)] at this
          have e0 : tm (29 * w + 7 * 2) = 54 * w + 14 := (tm_at w 14 (by omega)).1 (by omega)
          have e2 : tm (29 * w + 21 + tOff k) = 54 * w + 46 + tOff k := by
            have := (tm_at w (21 + tOff k) (by omega)).2 (by omega)
            rw [← Nat.add_assoc] at this; omega
          rw [hs2] at this; omega
        · rw [input_faulty w _ (by omega), vOf_at w 2 (by omega)]
          rcases ho with ⟨hk, -, -, ht⟩ | ⟨hk, -, -, ht⟩ <;> rw [ht] <;> simp [faultyPhase', hk]
      · -- The next block's `Cert1` moves the node on.
        obtain ⟨hsn, htn⟩ := tm_next u hu2
        refine ⟨sAt (u + 1) + c1Off k, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
        · have := tm_mono (show sAt u ≤ n by omega)
          rw [tm_view _ _ (by omega)]; omega
        · rw [show sAt (u + 1) + c1Off k + 1 = sAt (u + 1) + (c1Off k + 1) by omega,
            (vAt_view (k := k) (u + 1) (c1Off k + 1) (by omega)).1 (by omega)]
          have := vOf_succ u
          show vOf u + 1 < vOf (u + 1) + 1; omega
    · -- The block's own `Cert1` moves the node on.
      rw [hlo (by omega)]
      refine ⟨sAt u + c1Off k, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
      · rw [tm_view _ _ (by omega)]
        by_cases hr0 : r = 0
        · have := tm_before u (by
            cases u with
            | zero => exact absurd hn1 (by rw [show sAt 0 = 0 from rfl]; omega)
            | succ u => omega)
          rw [show n = sAt u - 1 by omega]; omega
        · have := tm_mono (show sAt u ≤ n by omega); omega
      · rw [show sAt u + c1Off k + 1 = sAt u + (c1Off k + 1) by omega,
          (vAt_view (k := k) u (c1Off k + 1) (by omega)).1 (by omega)]
        show vOf u < vOf u + 1; omega
  · rw [hn1, vAt_faulty w s hs]
    by_cases hst : s ≤ tOff k
    · refine ⟨29 * w + 21 + tOff k, by omega, ?_, Or.inl ?_⟩
      · have := tm_mono (show 29 * w + 20 ≤ n by omega)
        have e0 := (tm_at w 20 (by omega)).1 (by omega)
        have e2 : tm (29 * w + 21 + tOff k) = 54 * w + 46 + tOff k := by
          have := (tm_at w (21 + tOff k) (by omega)).2 (by omega)
          rw [← Nat.add_assoc] at this; omega
        omega
      · rw [input_faulty w _ (by omega)]
        rcases ho with ⟨hk, -, -, ht⟩ | ⟨hk, -, -, ht⟩ <;> rw [ht] <;> simp [faultyPhase', hk]
    · -- Timed out: the timeout certificate moves the node on.
      refine ⟨29 * w + 28, by omega, ?_, Or.inr ⟨_, ?_, inView _⟩⟩
      · have := tm_mono (show 29 * w + 21 + tOff k ≤ n by omega)
        have e2 : tm (29 * w + 21 + tOff k) = 54 * w + 46 + tOff k := by
          have := (tm_at w (21 + tOff k) (by omega)).2 (by omega)
          rw [← Nat.add_assoc] at this; omega
        have e3 := (tm_at w 28 (by omega)).2 (by omega)
        omega
      · have hs3 : 29 * w + 28 + 1 = sAt (3 * (w + 1)) + 0 := by unfold sAt; omega
        rw [hs3, (vAt_view _ 0 (by omega)).2 (by omega)]
        show 4 * w + 4 < vOf (3 * (w + 1)); unfold vOf; omega

include hv hcf in
/-- Every delivery within `Δ = 4`, after GST `0`, with view timer `τ = 33`. -/
theorem sync : Synchrony (net hv B) 0 4 33 where
  proposal l hl n p hsend _ k hk _ _ := by
    simp only [net_time hv]
    obtain ⟨u, rfl, rfl, hu⟩ := sent_proposal hv hsend
    have := tm_mono hu
    have hmax := Nat.le_max_left (tm n) 0
    refine by_step hv (sAt u + 1) (by rw [tm_view u 1 (by omega)]; omega)
      ⟨⟨⟨vOf u⟩, (blk u).payloadCommit⟩, ⟨by rw [blk_view], rfl⟩, recv_view u 1 (by omega) (by omega)⟩
  revote l hl n r hsend hre := by
    by_cases hld : l = d
    · -- `d` leads no view of the first epoch, the one it is honest in.
      subst hld
      obtain ⟨hlead, -⟩ := (protocol hv d (n + 1)).revoteJustified n r ⟨_, (getElem_H n), hsend⟩ trivial
      exact absurd (d_honest hre).2 ((leader_some hlead).2 rfl)
    · exact absurd hsend (no_revoteSent hv hld)
  cert1 q d' v t hq hvotes k hk _ := by
    obtain ⟨k0, hq0, hk0⟩ : ∃ k0, q k0 ∧ (k0 = c ∨ (k0 = a ∧ d'.epoch = ⟨1⟩)) :=
      (quorum_voter hq fun k hk => (hvotes k hk).1).elim (fun h => ⟨c, h, Or.inl rfl⟩)
        fun ⟨h, he⟩ => ⟨a, h, Or.inr ⟨rfl, he⟩⟩
    obtain ⟨hkc, hs⟩ := hvotes k0 hq0
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    have ho := offs_nat k
    have hmax := Nat.le_max_left t 0
    rcases sent_vote1 hv hj with ⟨x, heq, hx⟩ | ⟨w, ⟨hca, heq⟩ | ⟨hcb, -⟩, -⟩
    · simp only [Vote.mk.injEq] at heq
      obtain ⟨rfl, rfl, -⟩ := heq
      have := tm_mono hx
      rw [tm_view x 1 (by omega)] at this
      refine by_step hv (sAt x + c1Off k) (by rw [tm_view x _ (by omega)]; omega) ?_
      rw [show (⟨(certOf (blk x)).data, ⟨vOf x⟩⟩ : Cert1) = certOf (blk x) by rw [← cert_view x]]
      rcases offs k with ⟨hk', h5, -⟩ | ⟨hk', h3, -⟩
      · have := recv_view (B := B) (k := k) (n := sAt x + 5 + 1) x 5 (by omega) (by omega)
        simp only [viewPhase', hk', ite_true] at this; rw [h5]; exact this
      · have := recv_view (B := B) (k := k) (n := sAt x + 3 + 1) x 3 (by omega) (by omega)
        simp only [viewPhase', hk', ite_false] at this; rw [h3]; exact this
    · rcases hk0 with rfl | ⟨-, he⟩
      · exact absurd hca (by decide)
      · simp only [Vote.mk.injEq] at heq
        rw [heq.1] at he
        exact absurd he (ep_ne_one w)
    · rcases hk0 with rfl | ⟨rfl, -⟩ <;> exact absurd hcb (by decide)
  cert2 q d' v t hq hvotes k hk _ := by
    obtain ⟨k0, hq0, hk0⟩ : ∃ k0, q k0 ∧ (k0 = c ∨ (k0 = a ∧ d'.epoch = ⟨1⟩)) :=
      (quorum_voter hq fun k hk => (hvotes k hk).1).elim (fun h => ⟨c, h, Or.inl rfl⟩)
        fun ⟨h, he⟩ => ⟨a, h, Or.inr ⟨rfl, he⟩⟩
    obtain ⟨hkc, hs⟩ := hvotes k0 hq0
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨x, heq, hx⟩ := sent_vote2 hv hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨rfl, rfl, -⟩ := heq
    have hmax := Nat.le_max_left t 0
    have := tm_mono hx
    rcases (lkOff_facts k0).2.2 with h | h <;> rw [h, tm_view x _ (by omega)] at this <;>
      exact by_step hv (sAt x + 6) (by rw [tm_view x 6 (by omega)]; omega) (recv_view x 6 (by omega) (by omega))
  cert2Spread c' k hk n hc k' hk' _ := by
    simp only [net_time hv]
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc
    have hmax := Nat.le_max_left (tm n) 0
    exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasCert2_of x hx)
  certSpread c' k hk n hc k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · have := offs_nat k; have := offs_nat k'
      by_cases hn : sAt x + c1Off k' < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasCert1_of x hn)
      · have := tm_mono (show sAt x + 3 ≤ n by omega)
        rw [tm_view x 3 (by omega)] at this
        exact by_step hv (sAt x + c1Off k') (by rw [tm_view x _ (by omega)]; omega) (hasCert1_of x (by omega))
  lockSpread c' k hk n hc k' hk' _ _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    rcases hasCert1 hc with rfl | ⟨x, rfl, hx⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (lockable_iff.mpr (Or.inl rfl))
    · have := offs_nat k; have hf := lkOff_facts k'
      by_cases hn : sAt x + lkOff k' < n + 1
      · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
          (lockable_iff.mpr (Or.inr ⟨x, rfl, hn⟩))
      · have := tm_mono (show sAt x + 3 ≤ n by omega)
        rw [tm_view x 3 (by omega)] at this
        exact by_step hv (sAt x + lkOff k') (by rw [tm_view x _ (by omega)]; omega)
          (lockable_iff.mpr (Or.inr ⟨x, rfl, by omega⟩))
  blockSpread c' b' hcert k hk n hcb k' hk' _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    rcases hasProposal hcb.2 with rfl | ⟨y, rfl, hy⟩ | ⟨w, hE, -⟩
    · exact by_at hv (m := 0) (fun h => absurd h (by omega)) (Or.inl rfl)
    · exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (hasProposal_of y hy)
    · -- Every `Cert1` a node holds is over the anchor or an honest block, none with the hash of one of `d`'s.
      exfalso
      have hEv : b'.viewNumber = ⟨4 * w + 4⟩ := by rcases hE with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
      have hh := congrArg Vote1Data.blockHash hcert.2
      rcases hasCert1 hcb.1 with rfl | ⟨x, rfl, -⟩
      · have := congrArg (·.viewNumber) (hcf _ _ hh)
        rw [hEv] at this; exact absurd (view_inj this) (by omega)
      · have := congrArg (·.viewNumber) (hcf _ _ hh)
        rw [hEv, blk_view] at this; exact vOf_ne x w (view_inj this)
  timeoutCert e q v t hq hvotes k hk _ := by
    obtain ⟨k0, hq0, hk0⟩ : ∃ k0, q k0 ∧ (k0 = c ∨ (k0 = a ∧ e = ⟨1⟩)) :=
      (quorum_voter hq fun k hk => (hvotes k hk).1).elim (fun h => ⟨c, h, Or.inl rfl⟩)
        fun ⟨h, he⟩ => ⟨a, h, Or.inr ⟨rfl, he⟩⟩
    obtain ⟨_hm, L, hsent⟩ := hvotes k0 hq0
    obtain ⟨j, hjt, hj⟩ := sentBy hv hsent
    rcases hk0.symm with ⟨rfl, he⟩ | rfl
    · -- `a`'s timeout votes are of later epochs than the first.
      obtain ⟨w, -, heq⟩ := sent_timeout hj
      simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
      exact absurd (heq.1.1.symm.trans he) (ep_ne_one w)
    obtain ⟨w, hjw, heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq, TimeoutData.mk.injEq] at heq
    obtain ⟨⟨rfl, -⟩, rfl, -⟩ := heq
    have htc : tOff c = 6 := rfl
    have := tm_mono (show 29 * w + 26 ≤ j by omega)
    have e6 := (tm_at w 26 (by omega)).2 (by omega)
    have e8 := (tm_at w 28 (by omega)).2 (by omega)
    have hmax := Nat.le_max_left t 0
    exact by_at hv (m := 29 * w + 29) (fun _ => by show tm (29 * w + 28) ≤ _; omega)
      ⟨T w, rfl, rfl, recv_tc w (by omega)⟩
  timeoutOneHonest e q v t hq hvotes k hk _ := by
    obtain ⟨k0, hq0, -, hk0⟩ := (C' B).intersect _ q q hq hq
    obtain ⟨L, hs⟩ := hvotes k0 hq0 hk0
    obtain ⟨j, hjt, hj⟩ := sentBy hv hs
    obtain ⟨w, hjw, heq⟩ := sent_timeout hj
    simp only [Vote.mk.injEq] at heq
    obtain ⟨-, rfl, -⟩ := heq
    have ho := offs_nat k0
    have h28 := (tm_at w 28 (by omega)).2 (by omega)
    have hj' : tm (29 * w + 25) ≤ tm j := tm_mono (by omega)
    have h25 := (tm_at w 25 (by omega)).2 (by omega)
    refine Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate (T w)))
      (fun _ h => Or.inr (Or.inl ⟨(T w).view + 1, Or.inr (Or.inl ⟨T w, h, rfl⟩),
        show 4 * w + 4 < 4 * w + 4 + 1 by omega⟩))
      (by_at hv (m := 29 * w + 29) (fun _ => by rw [show 29 * w + 29 - 1 = 29 * w + 28 by omega]; omega)
        (recv_tc w (by omega)))
  timeoutCertForward tc k hk n hin _ _ k' hk' _ := by
    refine Kit.by_imp (P := fun hist => hist.Received (.timeoutCertificate tc))
      (fun _ h => ⟨tc.view + 1, Or.inr (Or.inl ⟨tc, h, rfl⟩),
        show tc.view.toNat < tc.view.toNat + 1 by omega⟩) ?_
    simp only [net_time hv]
    obtain ⟨w, rfl, hw⟩ := received_tc (hin ▸ Trace.received_self _ n : (H k (n + 1)).Received _)
    have hmax := Nat.le_max_left (tm n) 0
    exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega) (recv_tc w hw)
  timeoutCatchUp k hk v hrep := by
    refine (Kit.catchUp_vacuous (N := net hv B) (tm := tm) (fun _ _ => rfl) (fun _ _ _ => rfl) tm_succ
      (29 * v.toNat + 27) (fun m vote hout hvv => ?_) hrep).elim
    obtain ⟨w, hmw, rfl⟩ := sent_timeout hout
    have : v.toNat = 4 * w + 4 := (congrArg ViewNumber.toNat hvv).symm
    have := offs_nat k
    omega
  timeoutLockSpread tc k hk n hin k' hk' _ _ := by
    simp only [net_time hv]
    obtain ⟨w, rfl, hw⟩ := received_tc hin
    have hmax := Nat.le_max_left (tm n) 0
    have hf := lkOff_facts k'
    have hs2 := sAt_at w 2 (by omega)
    exact by_at hv (m := n + 1) (fun _ => by simp only [Nat.add_sub_cancel]; omega)
      (lockable_iff.mpr (Or.inr ⟨3 * w + 2, rfl, by omega⟩))
  epochChange c2 b' hcm hlast k hk n hbc k' hk' _ := by
    -- The last block's `Cert2` arrives in its view, and the epoch change three steps later.
    simp only [net_time hv]
    obtain ⟨hb, hc2⟩ := hbc
    obtain ⟨x, rfl, hx⟩ := hasCert2 hc2
    have hbn : b'.blockHeader.blockNumber = ⟨x + 1⟩ := by
      rw [← cert_number x]; exact (congrArg Vote2Data.blockNumber hcm.2).symm
    rw [hbn] at hlast
    obtain ⟨hB, -, hx1⟩ := last_iff.mp hlast
    have hx2 : x % 3 = 2 := by omega
    obtain ⟨w, rfl⟩ : ∃ w, x = 3 * w + 2 := ⟨x / 3, by omega⟩
    rw [sAt_at w 2 (by omega)] at hx
    have hmax := Nat.le_max_left (tm n) 0
    have e0 := (tm_at w 20 (by omega)).1 (by omega)
    have e3 := (tm_at w 23 (by omega)).1 (by omega)
    have := tm_mono (show 29 * w + 20 ≤ n by omega)
    have hb' : b' = blk (3 * w + 2) := by
      rcases hasProposal hb with rfl | ⟨u, rfl, -⟩ | ⟨w', hE, -⟩
      · exact absurd (number_inj hbn) (by omega)
      · rw [blk_number] at hbn
        obtain rfl : u = 3 * w + 2 := by have := number_inj hbn; omega
        rfl
      · exfalso
        have hEn : b'.blockHeader.blockNumber = ⟨3 * w' + 4⟩ := by rcases hE with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
        rw [hbn] at hEn; have := number_inj hEn; omega
    subst hb'
    exact by_step hv (29 * w + 23) (by omega) ⟨_, tookEpochChange_of hB w (by omega)⟩
  proposalValid _ _ _ p _ _ := hv p
  validatedSound _ _ _ _ _ _ b _ := hv b
  validated k hk n s' p vid hin _ _ := by
    simp only [net_time hv]
    have hmax := Nat.le_max_left (tm n) 0
    rcases input_proposal hin with ⟨u, rfl, -, rfl, -⟩ | ⟨w, rfl, -, ⟨rfl, rfl, -⟩ | ⟨rfl, rfl, -⟩⟩
    · rw [blk_view]
      exact by_step hv (sAt u + 2) (by rw [tm_view u 2 (by omega), tm_view u 1 (by omega)] at *; omega)
        (recv_view u 2 (by omega) (by omega))
    · have e1 := (tm_at w 21 (by omega)).1 (by omega)
      have e2 : tm (29 * w + 21 + 1) = 54 * w + 22 := (tm_at w 22 (by omega)).1 (by omega)
      exact by_step hv (29 * w + 21 + 1) (by omega) (recv_faulty w 1 (by omega) (by omega))
    · have e1 := (tm_at w 21 (by omega)).1 (by omega)
      have e2 : tm (29 * w + 21 + 1) = 54 * w + 22 := (tm_at w 22 (by omega)).1 (by omega)
      have := recv_faulty (B := B) (k := b) (n := 29 * w + 21 + 1 + 1) w 1 (by omega) (by omega)
      simp only [faultyPhase', show b ≠ a by decide, ite_false, ite_true] at this
      exact by_step hv (29 * w + 21 + 1) (by omega) this
  header k hk n p hpe _ hready := by simp only [net_time hv]; exact header_arrives hv hpe hready
  timeUnbounded _ _ t₀ := ⟨t₀ + 1, by have := tm_ge (t₀ + 1); show t₀ < tm (t₀ + 1); omega⟩
  timerNotEarly k hk n m v hin hnot _ hinm := timer_not_early hin hnot hinm
  timerFires k hk n v hin _ := timer_fires n hin

theorem rotation : LeaderRotation (C' B) leader := fun _ v =>
  ⟨⟨4 * v.toNat + 1⟩, show v.toNat ≤ 4 * v.toNat + 1 by omega, a,
    by rw [leader_of (Or.inl (by rw [(ldr_at v.toNat).1]; decide)), (ldr_at v.toNat).1],
    Or.inl (Or.inl rfl), Or.inl rfl⟩

include hv hcf in
/-- **The liveness premises can be met together, with a leader that equivocates.** -/
theorem premises_met :
    ConfigCoherent cfg ∧ Synchrony (net hv B) 0 4 33 ∧ Prompt (net hv B) 0
      ∧ 8 * 4 + 3 * 0 < 33 ∧ LeaderRotation (C' B) leader :=
  ⟨cfg_coherent, sync hv hcf, fun _ hk n o ho => absurd ho (settledIn hv hk n o), by decide, rotation⟩

theorem ldr_d (w : Nat) : ldr (4 * w + 4) = d := by
  simp [ldr, show (4 * w + 4) % 4 = 0 by omega]

include hv in
/-- `a` votes1 for the block `d` sent it, and `b` for the one `d` sent it, by the step its validity report arrives in. -/
theorem votesE (w : Nat) {k : PubKey} {E : Block} (hk : (k = a ∧ E = E1 w) ∨ (k = b ∧ E = E2 w)) :
    ∃ j, j ≤ 29 * w + 22 ∧ Output.send (.vote1 ⟨(certOf E).data, ⟨4 * w + 4⟩, k⟩) ∈ (tr k j).output := by
  have hEv : E.viewNumber = ⟨4 * w + 4⟩ := by rcases hk with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
  have hEp : E.parentCert = certOf (blk (3 * w + 2)) := by rcases hk with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
  have hEe : E.epoch = ⟨ep B (3 * w + 3)⟩ := by rcases hk with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> rfl
  have hprop : (H k (29 * w + 23)).Received (.proposal d E (some ⟨⟨4 * w + 4⟩, E.payloadCommit⟩)) := by
    rcases hk with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
    · have := recv_faulty (B := B) (k := a) (n := 29 * w + 23) w 0 (by omega) (by omega)
      simpa [faultyPhase'] using this
    · have := recv_faulty (B := B) (k := b) (n := 29 * w + 23) w 0 (by omega) (by omega)
      simpa [faultyPhase', show b ≠ a by decide] using this
  have hval : (H k (29 * w + 23)).Received (.blockValidated ⟨4 * w + 4⟩ (blockHash E)) := by
    rcases hk with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
    · have := recv_faulty (B := B) (k := a) (n := 29 * w + 23) w 1 (by omega) (by omega)
      simpa [faultyPhase'] using this
    · have := recv_faulty (B := B) (k := b) (n := 29 * w + 23) w 1 (by omega) (by omega)
      simpa [faultyPhase', show b ≠ a by decide] using this
  have hwf : ProposalWellFormed cfg E := by
    rcases hk with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;>
      exact ⟨show vOf (3 * w + 2) < 4 * w + 4 by unfold vOf; omega,
        Or.inl ⟨rfl, by
          rw [hEp, cert_view]; show (⟨vOf (3 * w + 2) + 1⟩ : ViewNumber) = ⟨4 * w + 4⟩; unfold vOf; congr 1; omega⟩,
        (epochOf_number (3 * w + 3)).symm, rfl⟩
  have hsafe : SafeParent E := fun tc h => by rcases hk with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> cases h
  have hs2 := sAt_at w 2 (by omega)
  have ho := offs_nat k
  have hready : ParentReady cfg (H k (29 * w + 23)) E :=
    Or.inr (Or.inr ⟨blk (3 * w + 2), hasProposal_of _ (by omega), by rw [hEp]; exact Nat.le_refl _,
      by rw [hEp]; rfl, payload_of _ (by omega)⟩)
  have hopens : OpensEpochJustified cfg (H k (29 * w + 23)) E := fun _ =>
    ⟨⟨blk (3 * w + 2), hasProposal_of _ (by omega), by rw [hEp]; rfl, by rw [hEp]; rfl⟩,
      C2 (3 * w + 2), Or.inl <| hasCert2_of _ (by omega),
      by rw [hEv]; show vOf (3 * w + 2) < 4 * w + 4; unfold vOf; omega, by rw [hEp]; rfl⟩
  have hcur : NotBehind cfg (H k (29 * w + 23)) E.epoch := notBehind (by
    rw [hEe, show 29 * w + 23 = 29 * w + 21 + 2 by omega, (eAt_faulty w 2 (by omega)).1 (by omega)]
    exact ep_mono (by omega))
  refine Classical.byContradiction fun hneg => settled k (29 * w + 22) (.vote1 E)
    ⟨⟨_, _, hprop, by rw [hEv, leader_of (Or.inr (hEe ▸ ep_ne_one w)), ldr_d w], by rw [hEv], rfl⟩, hwf, ?_, hready, hsafe,
      hopens, hcur, fun ht => ?_, fun vote hs _ hvv => ?_, ?_⟩
  · rw [hEv]; exact hval
  · rw [hEv] at ht
    obtain ⟨w', hle, hlt⟩ := timedOut ht
    have hle' : 4 * w + 4 ≤ 4 * w' + 4 := hle
    omega
  · obtain ⟨j, hj, hjv⟩ := sent_iff.mp hs
    rw [hEv] at hvv
    rcases sent_vote1 hv hjv with ⟨u', rfl, -⟩ | ⟨w', ⟨hka, rfl⟩ | ⟨hkb, rfl⟩, -⟩
    · exact vOf_ne u' w (view_inj hvv)
    · obtain rfl : w' = w := by have := view_inj hvv; omega
      rcases hk with ⟨-, rfl⟩ | ⟨rfl, -⟩
      · exact hneg ⟨j, by omega, by subst hka; exact hjv⟩
      · exact absurd hka (by decide)
    · obtain rfl : w' = w := by have := view_inj hvv; omega
      rcases hk with ⟨rfl, -⟩ | ⟨-, rfl⟩
      · exact absurd hkb (by decide)
      · exact hneg ⟨j, by omega, by subst hkb; exact hjv⟩
  · rw [hEv]
    have := inView (B := B) (k := k) (29 * w + 23)
    rwa [show 29 * w + 23 = 29 * w + 21 + 2 by omega, vAt_faulty w 2 (by omega)] at this

/-- `c` never holds either of `d`'s blocks. -/
theorem c_lacks {n w : Nat} {E : Block} (hE : E = E1 w ∨ E = E2 w) : ¬ (H c n).HasProposal cfg E := fun h => by
  have hEv : E.viewNumber = ⟨4 * w + 4⟩ := by rcases hE with rfl | rfl <;> rfl
  rcases hasProposal h with rfl | ⟨u, rfl, -⟩ | ⟨w', ⟨hk, -⟩ | ⟨hk, -⟩, -⟩
  · exact absurd (view_inj hEv) (by omega)
  · rw [blk_view] at hEv; exact vOf_ne u w (view_inj hEv)
  · exact absurd hk (by decide)
  · exact absurd hk (by decide)

include hv in
/--
And the network survives the equivocation: `d` sends `a` and `b` different blocks
for its view, and each votes1 for its own; `c` holds neither; no node ever holds
a `Cert1` for that view; `c` answers the one-honest indication with a timeout vote
before its own timer fires; and the next block, from the honest leader after `d`,
names the timeout certificate as evidence.
-/
theorem equivocation (w : Nat) :
    E1 w ≠ E2 w
      ∧ (∃ j, Output.send (.vote1 ⟨(certOf (E1 w)).data, ⟨4 * w + 4⟩, a⟩) ∈ (tr a j).output)
      ∧ (∃ j, Output.send (.vote1 ⟨(certOf (E2 w)).data, ⟨4 * w + 4⟩, b⟩) ∈ (tr b j).output)
      ∧ (∀ n, ¬ (H c n).HasProposal cfg (E1 w) ∧ ¬ (H c n).HasProposal cfg (E2 w))
      ∧ (∀ k n x, (H k n).HasCert1 cfg x → x.view ≠ ⟨4 * w + 4⟩)
      ∧ input c (29 * w + 26) = .timeoutOneHonest ⟨4 * w + 4⟩
      ∧ Output.send (.timeoutVote ⟨⟨⟨ep B (3 * w + 3)⟩, certOf (blk (3 * w + 2))⟩, ⟨4 * w + 4⟩, c⟩)
          ∈ (tr c (29 * w + 26)).output
      ∧ input c (29 * w + 27) = .timeout ⟨4 * w + 4⟩
      ∧ ldr (vOf (3 * w + 3)) = a ∧ (blk (3 * w + 3)).timeoutEvidence = some (T w) := by
  refine ⟨fun h => by simp [E1', E2'] at h, ?_, ?_, fun n => ⟨fun h => ?_, fun h => ?_⟩, fun k n x hx => ?_, ?_,
    times_out c w (Or.inr rfl), ?_, ?_, blk_evidence w⟩
  · obtain ⟨j, -, hj⟩ := votesE hv w (Or.inl ⟨rfl, rfl⟩); exact ⟨j, hj⟩
  · obtain ⟨j, -, hj⟩ := votesE hv w (Or.inr ⟨rfl, rfl⟩); exact ⟨j, hj⟩
  · exact c_lacks (Or.inl rfl) h
  · exact c_lacks (Or.inr rfl) h
  · rcases hasCert1 hx with rfl | ⟨u, rfl, -⟩
    · intro h; exact absurd (view_inj h) (by omega)
    · rw [cert_view]; intro h; exact vOf_ne u w (view_inj h)
  · rw [input_faulty w 5 (by omega)]; rfl
  · rw [input_faulty w 6 (by omega)]; simp [faultyPhase', slow]
  · rw [show 3 * w + 3 = 3 * (w + 1) + 0 by omega, vOf_at (w + 1) 0 (by omega)]
    simp [ldr, show (4 * (w + 1) + 0 + 1) % 4 = 1 by omega]

include hv in
/--
With epochs, `d`'s view is at a boundary: the round's last block ends its epoch,
`d`'s blocks would open the next, the timeout certificate is of the next epoch,
and the next block opens it behind the certificate, with no honest node asking
for a re-vote.
-/
theorem boundary (hB : B = true) (w : Nat) :
    IsLastBlock (blk (3 * w + 2)).blockHeader.blockNumber (cfg).epochHeight
      ∧ EntersEpoch cfg (E1 w) ∧ EntersEpoch cfg (E2 w) ∧ (T w).data.epoch = ⟨w + 2⟩
      ∧ EntersEpoch cfg (blk (3 * w + 3))
      ∧ ∀ k j r, k ≠ d → Output.send (.revote r) ∉ (tr k j).output := by
  have hE : EntersEpoch cfg (E1 w) := by
    show IsLastBlock ⟨3 * w + 4 - 1⟩ (cfg).epochHeight
    exact last_iff.mpr ⟨hB, by omega, by omega⟩
  refine ⟨blk_last.mpr ⟨hB, by omega⟩, hE, hE, by rw [show (T w).data.epoch = ⟨ep B (3 * w + 3)⟩ from rfl, ep_of hB]; congr 1; omega,
    blk_enters.mpr ⟨hB, by omega, by omega⟩, fun k j r hk => no_revoteSent hv hk⟩

include hv in
/--
With epochs, `d` is honest in the first epoch only. It votes there for the first
block, as a member of the committee, and in every later epoch it equivocates as a
leader: what it sends there binds it to nothing, since it is not honest in the
blocks' epoch.
-/
theorem honest_once (hB : B = true) :
    (C' B).Honest d ∧ ¬ (C' B).Steady d
      ∧ (∃ j, Output.send (.vote1 ⟨(certOf (blk 0)).data, ⟨vOf 0⟩, d⟩) ∈ (tr d j).output)
      ∧ ∀ w, ¬ (C' B).honest (E1 w).epoch d ∧ ¬ (C' B).honest (E2 w).epoch d :=
  ⟨⟨⟨1⟩, honest_first hB⟩, fun h => absurd (d_honest (h ⟨2⟩)).2 (by decide),
    let ⟨j, _, hj⟩ := votes1 hv d 0; ⟨j, hj⟩,
    fun w => ⟨fun h => ep_ne_one w (d_honest h).2, fun h => ep_ne_one w (d_honest h).2⟩⟩

include hv in
/-- So every honest node but `d` keeps deciding, whatever `d` sends. -/
theorem decides (hcf : CollisionFree) (t : Nat) (k : PubKey) (hk : (C' B).Honest k) (hkd : k ≠ d) :
    (net hv B).DecidesAfter k hk t := by
  obtain ⟨hc, hs, hp, hb, hr⟩ := premises_met (B := B) hv hcf
  exact (Liveness.chainGrows (leader := leader) (C := C' B) (net hv B) 0 4 0 33
    hc hcf hs hp hb hr).2 t k hk (Or.inl (steady hk hkd))

end Net

end ByzantineWitness
end NewProtocolImpl
