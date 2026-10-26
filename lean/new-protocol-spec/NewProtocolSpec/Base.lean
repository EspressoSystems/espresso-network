module

/-!
# Numbers and epochs

Views, epochs and block heights, and how a block height falls into an epoch
(`epochOf`). The facts about `epochOf` the proofs read come after the
definitions.
-/

@[expose] public section

namespace NewProtocol

/-- The index of a view. -/
structure ViewNumber where
  /-- A `ViewNumber` wraps a `Nat`. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

namespace ViewNumber

/-- The view of the genesis block, before any proposal. -/
def genesis : ViewNumber := ⟨0⟩

instance : HAdd ViewNumber Nat ViewNumber := ⟨fun v n => ⟨v.toNat + n⟩⟩

instance : HSub ViewNumber Nat ViewNumber := ⟨fun v n => ⟨v.toNat - n⟩⟩

instance : LE ViewNumber := ⟨fun a b => a.toNat ≤ b.toNat⟩

instance : LT ViewNumber := ⟨fun a b => a.toNat < b.toNat⟩

instance (a b : ViewNumber) : Decidable (a ≤ b) :=
  inferInstanceAs (Decidable (a.toNat ≤ b.toNat))

instance (a b : ViewNumber) : Decidable (a < b) :=
  inferInstanceAs (Decidable (a.toNat < b.toNat))

instance (n : Nat) : OfNat ViewNumber n := ⟨⟨n⟩⟩

instance : Max ViewNumber := ⟨fun a b => if a ≤ b then b else a⟩

instance : Min ViewNumber := ⟨fun a b => if a ≤ b then a else b⟩

/-- A `ViewNumber` is its `toNat`, so equal indices are equal views. -/
theorem ext {a b : ViewNumber} (h : a.toNat = b.toNat) : a = b := by
  cases a; cases b; simp_all

/-- Two views each no later than the other are one view. -/
theorem le_antisymm {a b : ViewNumber} (h1 : a ≤ b) (h2 : b ≤ a) : a = b :=
  ext (Nat.le_antisymm h1 h2)

end ViewNumber

/-- The index of an epoch. -/
structure EpochNumber where
  /-- An `EpochNumber` wraps a `Nat`. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

namespace EpochNumber

/-- An `EpochNumber` is its `toNat`, so equal indices are equal epochs. -/
theorem ext {a b : EpochNumber} (h : a.toNat = b.toNat) : a = b := by
  cases a; cases b; simp_all

instance : HAdd EpochNumber Nat EpochNumber := ⟨fun e n => ⟨e.toNat + n⟩⟩

instance : LE EpochNumber := ⟨fun a b => a.toNat ≤ b.toNat⟩

instance : LT EpochNumber := ⟨fun a b => a.toNat < b.toNat⟩

instance (a b : EpochNumber) : Decidable (a ≤ b) :=
  inferInstanceAs (Decidable (a.toNat ≤ b.toNat))

instance (a b : EpochNumber) : Decidable (a < b) :=
  inferInstanceAs (Decidable (a.toNat < b.toNat))

instance : Max EpochNumber := ⟨fun a b => if a ≤ b then b else a⟩

instance (n : Nat) : OfNat EpochNumber n := ⟨⟨n⟩⟩

end EpochNumber

/--
The height of a block in the chain.

A distinct type from `ViewNumber`: the two are counted separately and a run has
more views than blocks, so putting them in one type would let a proof confuse them
silently. `Config.epochHeight` stays a `Nat`, since it is how many blocks an epoch
holds, a length rather than a position.
-/
structure BlockNumber where
  /-- A `BlockNumber` wraps a `Nat`. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

namespace BlockNumber

/-- A `BlockNumber` is its `toNat`, so equal heights are equal positions. -/
theorem ext {a b : BlockNumber} (h : a.toNat = b.toNat) : a = b := by
  cases a; cases b; simp_all

instance : HAdd BlockNumber Nat BlockNumber := ⟨fun b n => ⟨b.toNat + n⟩⟩

instance : HSub BlockNumber Nat BlockNumber := ⟨fun b n => ⟨b.toNat - n⟩⟩

instance : LE BlockNumber := ⟨fun a b => a.toNat ≤ b.toNat⟩

instance : LT BlockNumber := ⟨fun a b => a.toNat < b.toNat⟩

instance (a b : BlockNumber) : Decidable (a ≤ b) :=
  inferInstanceAs (Decidable (a.toNat ≤ b.toNat))

instance (a b : BlockNumber) : Decidable (a < b) :=
  inferInstanceAs (Decidable (a.toNat < b.toNat))

instance (n : Nat) : OfNat BlockNumber n := ⟨⟨n⟩⟩

/-- Block zero is the height whose `Nat` is zero. -/
theorem eq_zero_iff (b : BlockNumber) : b = 0 ↔ b.toNat = 0 :=
  ⟨fun h => h ▸ rfl, fun h => ext h⟩

/-- A height counted forward is the `Nat` counted forward, which is what lets `omega` at it. -/
@[simp] theorem toNat_add (b : BlockNumber) (n : Nat) : (b + n).toNat = b.toNat + n := rfl

/--
Stepping forward and back again is where it started.

`EntersEpoch` asks about the block before a proposal's, and a proposal's height
is its parent's plus one, so the two are the same block.
-/
@[simp] theorem add_sub_cancel (b : BlockNumber) (n : Nat) : b + n - n = b := by
  cases b; exact ext (Nat.add_sub_cancel _ _)

end BlockNumber

/--
The epoch a block number falls in, under an epoch height of `height`.

The whole of the epoch arithmetic the protocol reads. Blocks are dealt out
`height` to an epoch, so block `n` belongs to epoch `⌈n / height⌉`, and epochs
are numbered from one: the last block of epoch `k` is `k * height`, and the
first of epoch `k + 1` is one past it.

Block zero and a `height` of zero do not follow that rule. Block zero is the
anchor: it precedes every epoch and is reported as epoch one, the epoch whose
blocks follow it. A `height` of zero means epochs are not in use at all, and every
block reports epoch zero; the rules that read this are written so that such a run
has one epoch and no boundary.

The implementation computes the same function of a block number and the
configured epoch height, and every epoch it looks a committee up under is one
this returns.
-/
def epochOf (blockNumber : BlockNumber) (height : Nat) : EpochNumber :=
  if height = 0 then 0
  else if blockNumber = 0 then 1
  else if blockNumber.toNat % height = 0 then ⟨blockNumber.toNat / height⟩
  else ⟨blockNumber.toNat / height + 1⟩

/--
Whether a block number is the last of its epoch.

Where a boundary falls, and so where the committee changes.
-/
def IsLastBlock (blockNumber : BlockNumber) (height : Nat) : Prop :=
  blockNumber ≠ 0 ∧ height ≠ 0 ∧ blockNumber.toNat % height = 0

instance (b : BlockNumber) (h : Nat) : Decidable (IsLastBlock b h) :=
  inferInstanceAs (Decidable (_ ∧ _ ∧ _))

/-! ## Facts about `epochOf` -/

/-- `epochOf` on the `Nat` of the block number, the form `omega` reads. -/
theorem epochOf_eq (b : BlockNumber) (height : Nat) :
    epochOf b height =
      if height = 0 then ⟨0⟩
      else if b.toNat = 0 then ⟨1⟩
      else if b.toNat % height = 0 then ⟨b.toNat / height⟩
      else ⟨b.toNat / height + 1⟩ := by
  unfold epochOf
  by_cases hb : b.toNat = 0
  · rw [ite_eq_left ((BlockNumber.eq_zero_iff b).mpr hb), ite_eq_left hb]; rfl
  · rw [ite_eq_right (fun h => hb ((BlockNumber.eq_zero_iff b).mp h)), ite_eq_right hb]; rfl

/-- `IsLastBlock` on the `Nat` of the block number. -/
theorem isLastBlock_iff {b : BlockNumber} {height : Nat} :
    IsLastBlock b height ↔ b.toNat ≠ 0 ∧ height ≠ 0 ∧ b.toNat % height = 0 :=
  ⟨fun ⟨h1, h2, h3⟩ => ⟨fun h => h1 ((BlockNumber.eq_zero_iff b).mpr h), h2, h3⟩,
    fun ⟨h1, h2, h3⟩ => ⟨fun h => h1 ((BlockNumber.eq_zero_iff b).mp h), h2, h3⟩⟩

/--
Block one is in the anchor's epoch.

The one step `epochOf_succ` does not cover, since it asks for a non-zero block
number. Block zero is the anchor and reports epoch one; so does the block after
it, whichever epoch height is in force.
-/
theorem epochOf_one (h : Nat) (hh : h ≠ 0) : epochOf 1 h = epochOf 0 h := by
  show epochOf ⟨1⟩ h = epochOf ⟨0⟩ h
  simp only [epochOf_eq, hh, ite_false, ite_true]
  by_cases h1 : h = 1
  · subst h1; simp
  · rw [Nat.mod_eq_of_lt (show (1 : Nat) < h by omega)]
    simp
    omega

/--
Epochs are numbered from one, whenever there are epochs at all.

So epoch zero names no block: there is nothing before epoch one.
-/
theorem epochOf_pos {n : BlockNumber} {h : Nat} (hh : h ≠ 0) :
    (epochOf n h).toNat ≠ 0 := by
  obtain ⟨n⟩ := n
  simp only [epochOf_eq, hh, ite_false]
  by_cases hn : n = 0
  · simp [hn]
  · rw [ite_eq_right hn]
    by_cases hmod : n % h = 0
    · rw [ite_eq_left hmod]
      intro hz
      have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
      rw [show n / h = 0 from hz, Nat.mul_zero] at hqr
      omega
    · rw [ite_eq_right hmod]
      exact Nat.succ_ne_zero _

/--
An epoch's last block is at the height its epoch number fixes.

So two last blocks of one epoch are at one height.
-/
theorem lastBlock_height {n : BlockNumber} {h e : Nat} (hlast : IsLastBlock n h)
    (he : epochOf n h = ⟨e⟩) : n.toNat = e * h := by
  obtain ⟨n⟩ := n
  show n = e * h
  obtain ⟨hn, hh, hmod⟩ := isLastBlock_iff.mp hlast
  have hn : n ≠ 0 := hn
  have hmod : n % h = 0 := hmod
  have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
  have hdiv : n / h = e := by
    simp only [epochOf_eq, hh, hn, hmod, ite_false, ite_true] at he
    exact congrArg EpochNumber.toNat he
  rw [← hdiv, Nat.mul_comm]
  omega

/--
No block of an epoch is at a height past the epoch's last.

The counterpart of `lastBlock_height`: together they say an epoch's blocks stop
where its last block is.
-/
theorem epochOf_height_le {n : BlockNumber} {h : Nat} (hh : h ≠ 0) :
    n.toNat ≤ (epochOf n h).toNat * h := by
  obtain ⟨n⟩ := n
  have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
  have hr : n % h < h := Nat.mod_lt _ (Nat.pos_of_ne_zero hh)
  simp only [epochOf_eq, hh, ite_false]
  by_cases hn : n = 0
  · simp [hn]
  · rw [ite_eq_right hn]
    by_cases hmod : n % h = 0
    · rw [ite_eq_left hmod]
      show n ≤ n / h * h
      have hc : n / h * h = h * (n / h) := Nat.mul_comm _ _
      omega
    · rw [ite_eq_right hmod]
      show n ≤ (n / h + 1) * h
      have : (n / h + 1) * h = h * (n / h) + h := by
        rw [Nat.add_mul, Nat.one_mul, Nat.mul_comm]
      omega

/--
An epoch changes only at a boundary, and then by one.

Stepping from one block to the next either stays in the epoch or enters the next
one, and which it is depends on whether the block before it was its epoch's last.
So a branch cannot pass from one epoch into another without passing through a last
block.
-/
theorem epochOf_succ (n : BlockNumber) (h : Nat) (hh : h ≠ 0) (hn : n.toNat ≠ 0) :
    epochOf (n + 1) h = if IsLastBlock n h then epochOf n h + 1 else epochOf n h := by
  obtain ⟨n⟩ := n
  show epochOf ⟨n + 1⟩ h = if IsLastBlock ⟨n⟩ h then epochOf ⟨n⟩ h + 1 else epochOf ⟨n⟩ h
  have hn : n ≠ 0 := hn
  have hpos : 0 < h := Nat.pos_of_ne_zero hh
  have hr : n % h < h := Nat.mod_lt _ hpos
  have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
  by_cases hz : n % h = 0
  · rw [ite_eq_left (show IsLastBlock ⟨n⟩ h from isLastBlock_iff.mpr ⟨hn, hh, hz⟩)]
    have hnq : n + 1 = h * (n / h) + 1 := by omega
    by_cases h1 : h = 1
    · subst h1
      simp only [epochOf_eq, hh, hn, Nat.mod_one, Nat.div_one, ite_false, ite_true,
        Nat.succ_ne_zero]
      rfl
    · have hmod : (n + 1) % h = 1 := by
        rw [hnq, Nat.mul_add_mod]
        exact Nat.mod_eq_of_lt (by omega)
      have h1' : 1 < h := by omega
      have hdiv : (n + 1) / h = n / h := by
        rw [hnq, Nat.mul_add_div hpos, Nat.div_eq_of_lt h1']
        omega
      simp only [epochOf_eq, hh, hz, hn, hmod, hdiv, Nat.succ_ne_zero, ite_false, ite_true,
]
      rfl
  · rw [ite_eq_right (show ¬ IsLastBlock ⟨n⟩ h from fun hl => hz hl.2.2)]
    by_cases hedge : n % h + 1 = h
    · have hnq : n + 1 = h * (n / h + 1) := by
        rw [Nat.mul_add, Nat.mul_one]; omega
      have hmod : (n + 1) % h = 0 := by rw [hnq, Nat.mul_mod_right]
      have hdiv : (n + 1) / h = n / h + 1 := by
        rw [hnq, Nat.mul_div_cancel_left _ hpos]
      simp only [epochOf_eq, hh, hz, hn, hmod, hdiv, Nat.succ_ne_zero, ite_false, ite_true]
    · have hnq : n + 1 = h * (n / h) + (n % h + 1) := by omega
      have hlt' : n % h + 1 < h := by omega
      have hmod : (n + 1) % h = n % h + 1 := by
        rw [hnq, Nat.mul_add_mod]
        exact Nat.mod_eq_of_lt hlt'
      have hdiv : (n + 1) / h = n / h := by
        rw [hnq, Nat.mul_add_div hpos, Nat.div_eq_of_lt hlt']
        omega
      simp only [epochOf_eq, hh, hz, hn, hmod, hdiv, Nat.succ_ne_zero, ite_false]

end NewProtocol
