module

public import StakeTableSpec.Types

/-!
# Helpers of the specification

List operations and small structures the specification is written with, kept apart
so that `StakeTableSpec.Spec` holds only what the stake table means and what is
claimed of it.
-/

@[expose] public section

namespace StakeTable

/-- Exactly one `x` satisfies `P`. -/
abbrev Unique {α : Sort _} (P : α → Prop) : Prop :=
  ∃ x, P x ∧ ∀ y, P y → y = x

/-- What the last change that says gives, if any change does. -/
def latest {α : Type} (f : Change → Option α) (h : History) : Option α :=
  (h.filterMap f).getLast?

/-- The list without repetitions, keeping each element's last occurrence. -/
def dedup : List Address → List Address
  | [] => []
  | x :: xs => if x ∈ xs then dedup xs else x :: dedup xs

/-- The delegators that ever delegated to or withdrew from `a`, with repetitions. -/
def delegatorsOf (h : History) (a : Address) : List Address :=
  h.filterMap fun
    | .delegate d a' _ | .undelegate d a' _ => if a' = a then some d else none
    | _ => none

/-- Drop each element equal to the one directly before it, as Rust's `Vec::dedup` does. -/
def dedupAdjacent {α : Type} [DecidableEq α] : List α → List α
  | [] => []
  | [a] => [a]
  | a :: b :: l => if a = b then dedupAdjacent (b :: l) else a :: dedupAdjacent (b :: l)

/-- Where a log sits on L1: its block number, and its index within the block. -/
abbrev LogKey := Nat × Nat

/-- `a` comes before `b` on L1: in an earlier block, or earlier in the same block. -/
def LogKey.before (a b : LogKey) : Bool :=
  decide (a.1 < b.1) || (decide (a.1 = b.1) && decide (a.2 < b.2))

/-- A validator as the leader schedule sees it: the bytes of its consensus key, and its stake. -/
structure Entry where
  /-- The consensus key, as bytes. -/
  key : List Nat
  /-- The stake. -/
  stake : Nat
deriving DecidableEq, Repr, Inhabited

/-- The key `xor`ed with the DRB result repeated, byte by byte. -/
def xorKey (drb key : List Nat) : List Nat :=
  key.mapIdx fun i b => b ^^^ drb.getD (i % drb.length) 0

/-- The total stake. -/
def totalStake (es : List Entry) : Nat :=
  (es.map (·.stake)).sum

/-- The stake of the entries before position `i`. -/
def stakeBefore (es : List Entry) (i : Nat) : Nat :=
  totalStake (es.take i)

end StakeTable
