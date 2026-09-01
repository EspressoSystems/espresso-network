module

@[expose] public section

namespace NewProtocol

/-- The index of a view. -/
structure ViewNumber where
  /-- A `ViewNumber` wraps a `Nat`. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

namespace ViewNumber

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

instance (n : Nat) : OfNat EpochNumber n := ⟨⟨n⟩⟩

end EpochNumber

/--
The epoch a block number falls in, under an epoch height of `height`.

The whole of the epoch arithmetic the protocol reads. Blocks are dealt out
`height` to an epoch, so block `n` belongs to epoch `⌈n / height⌉`, and epochs
are numbered from one: the last block of epoch `k` is `k * height`, and the
first of epoch `k + 1` is one past it.

Two edges. Block zero is the genesis block, which precedes every epoch and is
reported as epoch one, the epoch whose blocks follow it. A `height` of zero
means epochs are not in use at all, and every block reports epoch zero; the
rules that read this are written so that such a run has one epoch and no
boundary.

The implementation computes the same function of a block number and the
configured epoch height, and every epoch it looks a committee up under is one
this returns.
-/
def epochOf (blockNumber height : Nat) : EpochNumber :=
  if height = 0 then ⟨0⟩
  else if blockNumber = 0 then ⟨1⟩
  else if blockNumber % height = 0 then ⟨blockNumber / height⟩
  else ⟨blockNumber / height + 1⟩

/--
Whether a block number is the last of its epoch.

Where a boundary falls, and so where the committee changes.
-/
def IsLastBlock (blockNumber height : Nat) : Prop :=
  blockNumber ≠ 0 ∧ height ≠ 0 ∧ blockNumber % height = 0

instance (b h : Nat) : Decidable (IsLastBlock b h) :=
  inferInstanceAs (Decidable (_ ∧ _ ∧ _))

/--
The block after genesis is in genesis's epoch.

The one step `epochOf_succ` does not cover, since it asks for a non-zero block
number. Block zero is the genesis block and reports epoch one; so does the
block after it, whichever epoch height is in force.
-/
theorem epochOf_one (h : Nat) (hh : h ≠ 0) : epochOf 1 h = epochOf 0 h := by
  simp only [epochOf, hh, if_false, if_true]
  by_cases h1 : h = 1
  · subst h1; simp
  · rw [Nat.mod_eq_of_lt (show (1 : Nat) < h by omega)]
    simp
    omega

/--
Epochs are numbered from one, whenever there are epochs at all.

So epoch zero names no block, which is what makes the epoch a quantity the
argument can count back from: there is nothing before epoch one to reach.
-/
theorem epochOf_pos {n h : Nat} (hh : h ≠ 0) : (epochOf n h).toNat ≠ 0 := by
  simp only [epochOf, hh, if_false]
  by_cases hn : n = 0
  · simp [hn]
  · rw [if_neg hn]
    by_cases hmod : n % h = 0
    · rw [if_pos hmod]
      intro hz
      have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
      rw [show n / h = 0 from hz, Nat.mul_zero] at hqr
      omega
    · rw [if_neg hmod]
      exact Nat.succ_ne_zero _

/--
An epoch's last block is at the height its epoch number fixes.

So two last blocks of one epoch are at one height, which is what turns "one
block before the other" into "the same block" when the argument reaches a
boundary.
-/
theorem lastBlock_height {n h e : Nat} (hlast : IsLastBlock n h)
    (he : epochOf n h = ⟨e⟩) : n = e * h := by
  obtain ⟨hn, hh, hmod⟩ := hlast
  have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
  have hdiv : n / h = e := by
    simp only [epochOf, hh, hn, hmod, if_false, if_true] at he
    exact congrArg EpochNumber.toNat he
  rw [← hdiv, Nat.mul_comm]
  omega

/--
No block of an epoch is at a height past the epoch's last.

The counterpart of `lastBlock_height`: together they say an epoch's blocks stop
where its last block is, which is what makes a second last block of one epoch
impossible.
-/
theorem epochOf_height_le {n h : Nat} (hh : h ≠ 0) :
    n ≤ (epochOf n h).toNat * h := by
  have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
  have hr : n % h < h := Nat.mod_lt _ (Nat.pos_of_ne_zero hh)
  simp only [epochOf, hh, if_false]
  by_cases hn : n = 0
  · simp [hn]
  · rw [if_neg hn]
    by_cases hmod : n % h = 0
    · rw [if_pos hmod]
      show n ≤ n / h * h
      have hc : n / h * h = h * (n / h) := Nat.mul_comm _ _
      omega
    · rw [if_neg hmod]
      show n ≤ (n / h + 1) * h
      have : (n / h + 1) * h = h * (n / h) + h := by
        rw [Nat.add_mul, Nat.one_mul, Nat.mul_comm]
      omega

/--
An epoch changes only at a boundary, and then by one.

The fact the epoch-crossing argument runs on: stepping from one block to the
next either stays in the epoch or enters the next one, and which it is is
decided by whether the block before it was its epoch's last. Nothing else can
happen, so a branch cannot slip from one epoch into another without passing
through a last block.
-/
theorem epochOf_succ (n h : Nat) (hh : h ≠ 0) (hn : n ≠ 0) :
    epochOf (n + 1) h = if IsLastBlock n h then epochOf n h + 1 else epochOf n h := by
  have hpos : 0 < h := Nat.pos_of_ne_zero hh
  have hr : n % h < h := Nat.mod_lt _ hpos
  have hqr : h * (n / h) + n % h = n := Nat.div_add_mod n h
  by_cases hz : n % h = 0
  · rw [if_pos (show IsLastBlock n h from ⟨hn, hh, hz⟩)]
    have hnq : n + 1 = h * (n / h) + 1 := by omega
    by_cases h1 : h = 1
    · subst h1
      simp only [epochOf, hh, hn, Nat.mod_one, Nat.div_one, if_false, if_true,
        Nat.succ_ne_zero]
      rfl
    · have hmod : (n + 1) % h = 1 := by
        rw [hnq, Nat.mul_add_mod]
        exact Nat.mod_eq_of_lt (by omega)
      have h1' : 1 < h := by omega
      have hdiv : (n + 1) / h = n / h := by
        rw [hnq, Nat.mul_add_div hpos, Nat.div_eq_of_lt h1']
        omega
      simp only [epochOf, hh, hz, hn, hmod, hdiv, Nat.succ_ne_zero, if_false, if_true,
]
      rfl
  · rw [if_neg (show ¬ IsLastBlock n h from fun hl => hz hl.2.2)]
    by_cases hedge : n % h + 1 = h
    · have hnq : n + 1 = h * (n / h + 1) := by
        rw [Nat.mul_add, Nat.mul_one]; omega
      have hmod : (n + 1) % h = 0 := by rw [hnq, Nat.mul_mod_right]
      have hdiv : (n + 1) / h = n / h + 1 := by
        rw [hnq, Nat.mul_div_cancel_left _ hpos]
      simp only [epochOf, hh, hz, hn, hmod, hdiv, Nat.succ_ne_zero, if_false, if_true]
    · have hnq : n + 1 = h * (n / h) + (n % h + 1) := by omega
      have hlt' : n % h + 1 < h := by omega
      have hmod : (n + 1) % h = n % h + 1 := by
        rw [hnq, Nat.mul_add_mod]
        exact Nat.mod_eq_of_lt hlt'
      have hdiv : (n + 1) / h = n / h := by
        rw [hnq, Nat.mul_add_div hpos, Nat.div_eq_of_lt hlt']
        omega
      simp only [epochOf, hh, hz, hn, hmod, hdiv, Nat.succ_ne_zero, if_false]

end NewProtocol
