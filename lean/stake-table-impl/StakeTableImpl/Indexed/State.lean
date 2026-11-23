module

public import StakeTableImpl.Node
public import Std.Data.HashMap
public import Std.Data.HashSet

/-!
# The indexed state

The stake table state as a node keeps it: validators and delegations in hash
maps, admitted keys and exits in hash sets, and registration order in an array.
Lookups take constant time; removing a validator takes time linear in their
number, as the node's `IndexMap::shift_remove` does.

`StakeTable.Indexed.Rel` (in `StakeTableImpl.Indexed.Refines`) relates it to the
list state of the specification.
-/

@[expose] public section

namespace StakeTable

deriving instance Hashable for Address
deriving instance Hashable for BlsKey
deriving instance Hashable for SchnorrKey
deriving instance Hashable for X25519Key

namespace Indexed

open Std

/-- A registered validator, with its delegations in a hash map. -/
structure Validator where
  /-- The validator's account, under which it is registered. -/
  account : Address
  /-- Its consensus key, if the key decoded. -/
  blsKey : Option BlsKey
  /-- Its light client key, if the key decoded and was not used before. -/
  schnorrKey : Option SchnorrKey
  /-- Its network key, if it registered one that decoded. -/
  x25519Key : Option X25519Key
  /-- Its network address, if it registered one that parsed. -/
  p2pAddr : Option NetAddr
  /-- The total delegated to it. -/
  stake : Nat
  /-- Its commission, in basis points. -/
  commission : Nat
  /-- Each delegator with the amount it delegated. -/
  delegators : HashMap Address Nat
  /-- Whether its keys are proven to be its own. -/
  authenticated : Bool

/-- The stake table state, indexed. -/
structure State where
  /-- The registered validators, by account. -/
  validators : HashMap Address Validator
  /-- The accounts of the registered validators, in the order they registered. -/
  order : Array Address
  /-- The accounts of validators that left. -/
  exits : HashSet Address
  /-- Every consensus key ever admitted. -/
  usedBls : HashSet BlsKey
  /-- Every light client key ever admitted. -/
  usedSchnorr : HashSet SchnorrKey
  /-- Every network key ever admitted. -/
  usedX25519 : HashSet X25519Key

/-- The state before any event. -/
def State.empty : State := ⟨∅, #[], ∅, ∅, ∅, ∅⟩

/-- A validator just registered: no stake, no delegators. -/
def Validator.new (account : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) (commission : Nat)
    (authenticated : Bool) : Validator :=
  { account, blsKey := bls, schnorrKey := schnorr, x25519Key := x25519, p2pAddr, stake := 0,
    commission, delegators := ∅, authenticated }

/-- Add `amount` to `d`'s delegation, creating it if `d` has none. -/
def credit (d : Address) (amount : Nat) (ds : HashMap Address Nat) : HashMap Address Nat :=
  ds.insert d (ds.getD d 0 + amount)

/-- Take `amount` from `d`'s delegation, removing it if nothing is left. -/
def debit (d : Address) (amount : Nat) (ds : HashMap Address Nat) : HashMap Address Nat :=
  if ds.getD d 0 - amount = 0 then ds.erase d else ds.insert d (ds.getD d 0 - amount)

/-- Add a key to a set, if there is one. -/
def insertKey {κ : Type} [BEq κ] [Hashable κ] (set : HashSet κ) : Option κ → HashSet κ
  | some k => set.insert k
  | none => set

namespace State

variable (s : State)

/-- Whether a validator is registered under `a`. -/
def registered (a : Address) : Bool :=
  s.validators.contains a

/-- Add a validator after the others, and admit its keys. -/
def admit (v : Validator) : State :=
  { s with
    validators := s.validators.insert v.account v
    order := s.order.push v.account
    usedBls := insertKey s.usedBls v.blsKey
    usedSchnorr := insertKey s.usedSchnorr v.schnorrKey
    usedX25519 := insertKey s.usedX25519 v.x25519Key }

/-- Change the validator registered under `a`, in place. -/
def update (a : Address) (f : Validator → Validator) : State :=
  { s with validators := s.validators.modify a f }

/-- Remove the validator registered under `a`, and record that it left. Its keys stay admitted. -/
def remove (a : Address) : State :=
  { s with
    validators := s.validators.erase a
    order := s.order.filter (· != a)
    exits := s.exits.insert a }

end State

end Indexed

end StakeTable
