module

public import StakeTableSpec

/-!
# The node's stake table data

The state the node builds from the L1 stake table events, and what applying an event
comes to.
-/

@[expose] public section

namespace StakeTable

/--
A registered validator.

`stake` is the sum of what its delegators delegated to it, and `delegators` maps
each delegator to its amount, each delegator once. A key is `none` when its bytes
did not decode, or, for a Schnorr key, when another validator already used it.
-/
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
  delegators : List (Address × Nat)
  /-- Whether its keys are proven to be its own. -/
  authenticated : Bool
deriving DecidableEq, Repr, Inhabited

/--
The stake table state.

`validators` is in registration order, as the node's map keeps it; active set
selection does not depend on the order. The used key lists remember every key a validator ever
held, including validators that have since left, so no key is admitted twice.
-/
structure State where
  /-- The registered validators, in the order they registered. -/
  validators : List Validator
  /-- The accounts of validators that left. -/
  exits : List Address
  /-- Every consensus key ever admitted. -/
  usedBls : List BlsKey
  /-- Every light client key ever admitted. -/
  usedSchnorr : List SchnorrKey
  /-- Every network key ever admitted. -/
  usedX25519 : List X25519Key
deriving DecidableEq, Repr, Inhabited

/-- The validator as a member of the table: all but its delegations. -/
def Validator.toMember (v : Validator) : Member :=
  { account := v.account, blsKey := v.blsKey, schnorrKey := v.schnorrKey, x25519Key := v.x25519Key,
    p2pAddr := v.p2pAddr, stake := v.stake, commission := v.commission,
    authenticated := v.authenticated }

/-- The state before any event. -/
def State.empty : State := ⟨[], [], [], [], []⟩

/--
What applying an event comes to.

A skipped event leaves the state as it was, and the events after it still apply. A
rejected one also leaves the state as it was, but stops the replay: the node cannot
build the stake table. Rejection is for events the contract should never emit.
-/
inductive Outcome where
  /-- The event applied, giving this state. -/
  | applied (s : State)
  /-- The event is ignored. -/
  | skipped
  /-- The event is invalid; the replay stops. -/
  | rejected
deriving DecidableEq, Repr

end StakeTable
