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

end NewProtocol
