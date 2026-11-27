module

/-!
# Stake table data

The keys and addresses the stake table holds, a member of the table as it shows it,
the changes L1 makes to it and what each change sets, and the contract events they
come from.

Keys reach this model already decoded and checked: an event carries the keys whose
bytes decoded, and the verdict on its signatures. Keys and addresses are only ever
compared.
-/

@[expose] public section

namespace StakeTable

/-- An Ethereum account: a validator's or a delegator's. -/
structure Address where
  /-- The value. Never read: addresses are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited

/-- A validator's BLS key, the one it signs consensus messages with. -/
structure BlsKey where
  /-- The value. Never read: keys are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited

/-- A validator's Schnorr key, the one it signs light client state with. -/
structure SchnorrKey where
  /-- The value. Never read: keys are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited

/-- A validator's x25519 key, the one its network connections authenticate with. -/
structure X25519Key where
  /-- The value. Never read: keys are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited

/-- A validator's network address. -/
structure NetAddr where
  /-- The address as text. -/
  toString : String
deriving DecidableEq, Repr, Inhabited

/-- Stake amounts are 256-bit unsigned integers on L1 and in the node. -/
def U256 : Nat := 2 ^ 256

/--
The largest commission, in basis points.

A commission is the share of a validator's reward it keeps before its delegators
are paid.
-/
def maxCommission : Nat := 10000

/--
A registered validator, as the stake table shows it.

The keys are `none` where the validator holds none of the kind: one that did not
decode, or a light client key that another validator held before.
-/
structure Member where
  /-- The validator's account, under which it is registered. -/
  account : Address
  /-- Its consensus key. -/
  blsKey : Option BlsKey
  /-- Its light client key. -/
  schnorrKey : Option SchnorrKey
  /-- Its network key. -/
  x25519Key : Option X25519Key
  /-- Its network address. -/
  p2pAddr : Option NetAddr
  /-- The total delegated to it. -/
  stake : Nat
  /-- Its commission, in basis points. -/
  commission : Nat
  /-- Whether its keys are proven to be its own. -/
  authenticated : Bool
deriving DecidableEq, Repr, Inhabited

/-- A change L1 makes to the stake table. -/
inductive Change where
  /-- A validator registers, with the keys it proved, or not, to be its own. -/
  | register (account : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
      (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) (commission : Nat)
      (authenticated : Bool)
  /-- A validator leaves. -/
  | exit (validator : Address)
  /-- A delegator delegates `amount` to a validator. -/
  | delegate (delegator validator : Address) (amount : Nat)
  /-- A delegator withdraws `amount` of its delegation to a validator. -/
  | undelegate (delegator validator : Address) (amount : Nat)
  /-- A validator replaces its consensus and light client keys with ones it proved its own. -/
  | rekey (validator : Address) (bls : BlsKey) (schnorr : SchnorrKey)
  /-- A validator changes its commission. -/
  | setCommission (validator : Address) (commission : Nat)
  /-- A validator changes its network key. -/
  | setX25519 (validator : Address) (key : Option X25519Key)
  /-- A validator changes its network address. -/
  | setP2pAddr (validator : Address) (addr : Option NetAddr)
deriving DecidableEq, Repr

namespace Change

/-- The validator a change is about. -/
def validator : Change → Address
  | register a .. | exit a | delegate _ a _ | undelegate _ a _ | rekey a .. | setCommission a _
  | setX25519 a _ | setP2pAddr a _ => a

/-- What a change moves from delegator `d` to validator `a`: plus a delegation, minus a withdrawal. -/
def flow (d a : Address) : Change → Int
  | delegate d' a' n => if d' = d ∧ a' = a then n else 0
  | undelegate d' a' n => if d' = d ∧ a' = a then -n else 0
  | _ => 0

/-- What a change moves to validator `a`, from any delegator. -/
def inflow (a : Address) : Change → Int
  | delegate _ a' n => if a' = a then n else 0
  | undelegate _ a' n => if a' = a then -n else 0
  | _ => 0

/-- The consensus key a change gives `a`, if it sets one: `some none` if it registers `a` without one. -/
def blsKeyOf (a : Address) : Change → Option (Option BlsKey)
  | register a' k .. => if a' = a then some k else none
  | rekey a' k _ => if a' = a then some (some k) else none
  | _ => none

/-- The light client key a change gives `a`, if it sets one. -/
def schnorrKeyOf (a : Address) : Change → Option (Option SchnorrKey)
  | register a' _ k .. => if a' = a then some k else none
  | rekey a' _ k => if a' = a then some (some k) else none
  | _ => none

/-- The network key a change gives `a`, if it sets one. -/
def x25519KeyOf (a : Address) : Change → Option (Option X25519Key)
  | register a' _ _ k .. => if a' = a then some k else none
  | setX25519 a' k => if a' = a then some k else none
  | _ => none

/-- The network address a change gives `a`, if it sets one. -/
def p2pAddrOf (a : Address) : Change → Option (Option NetAddr)
  | register a' _ _ _ p .. => if a' = a then some p else none
  | setP2pAddr a' p => if a' = a then some p else none
  | _ => none

/-- The commission a change gives `a`, if it sets one. -/
def commissionOf (a : Address) : Change → Option Nat
  | register a' _ _ _ _ c _ => if a' = a then some c else none
  | setCommission a' c => if a' = a then some c else none
  | _ => none

/-- Whether a change proves `a`'s keys its own, if it says: a key update always does. -/
def authenticatedOf (a : Address) : Change → Option Bool
  | register a' _ _ _ _ _ auth => if a' = a then some auth else none
  | rekey a' .. => if a' = a then some true else none
  | _ => none

/-- The consensus keys a change admits. -/
def blsKeys : Change → List BlsKey
  | register _ k .. => k.toList
  | rekey _ k _ => [k]
  | _ => []

/-- The light client keys a change admits. -/
def schnorrKeys : Change → List SchnorrKey
  | register _ _ k .. => k.toList
  | rekey _ _ k => [k]
  | _ => []

/-- The network keys a change admits. -/
def x25519Keys : Change → List X25519Key
  | register _ _ _ k .. => k.toList
  | setX25519 _ k => k.toList
  | _ => []

end Change

/-- The changes the stake table took, in L1 order. -/
abbrev History := List Change

/--
The keys of a registration emitted by `StakeTableV2` or later.

The contract checks the BLS signature itself; the node checks both signatures
again. `verified` means both keys decoded and both signatures verified;
`unverified` carries whichever keys decoded.
-/
inductive RegistrationKeys where
  /-- Both keys decoded and both signatures verified. -/
  | verified (bls : BlsKey) (schnorr : SchnorrKey)
  /-- Not verified: the keys that decoded. -/
  | unverified (bls : Option BlsKey) (schnorr : Option SchnorrKey)
deriving DecidableEq, Repr

/-- The BLS key of a registration, if it decoded. -/
def RegistrationKeys.bls : RegistrationKeys → Option BlsKey
  | .verified k _ => some k
  | .unverified k _ => k

/-- The Schnorr key of a registration, if it decoded. -/
def RegistrationKeys.schnorr : RegistrationKeys → Option SchnorrKey
  | .verified _ k => some k
  | .unverified _ k => k

/-- Whether a registration's keys verified. -/
def RegistrationKeys.isVerified : RegistrationKeys → Bool
  | .verified _ _ => true
  | .unverified _ _ => false

/-- The verdict on the keys of a key update emitted by `StakeTableV2` or later. -/
inductive KeyUpdateKeys where
  /-- Both keys decoded and both signatures verified. -/
  | verified (bls : BlsKey) (schnorr : SchnorrKey)
  /-- The BLS key did not decode. -/
  | invalidBls
  /-- The Schnorr key did not decode. -/
  | invalidSchnorr
  /-- The keys decoded but a signature did not verify. -/
  | failed
deriving DecidableEq, Repr

/--
A stake table event, as the node reads it from L1.

The variants follow the events of the contract's versions: `StakeTable`,
`StakeTableV2` and `StakeTableV3`. A chain's log holds the events of every version
its contract ran, so all are read. Where two versions emit different events with the
same effect, as `StakeTable` and `StakeTableV2` do for exits and undelegations, they
are one variant here.
-/
inductive Event where
  /-- A registration emitted by `StakeTable`. Keys are only decoded, not verified. -/
  | register (account : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey) (commission : Nat)
  /-- A registration emitted by `StakeTableV2`. -/
  | registerV2 (account : Address) (keys : RegistrationKeys) (commission : Nat)
  /-- A registration emitted by `StakeTableV3`, with a network key and address. -/
  | registerV3 (account : Address) (keys : RegistrationKeys) (commission : Nat)
      (x25519 : Option X25519Key) (p2pAddr : Option NetAddr)
  /-- A validator leaves. -/
  | deregister (validator : Address)
  /-- A delegator delegates `amount` to a validator. -/
  | delegate (delegator validator : Address) (amount : Nat)
  /-- A delegator withdraws `amount` of its delegation to a validator. -/
  | undelegate (delegator validator : Address) (amount : Nat)
  /-- A key update emitted by `StakeTable`: the keys if they decoded. -/
  | keyUpdate (validator : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
  /-- A key update emitted by `StakeTableV2` or later. -/
  | keyUpdateV2 (validator : Address) (keys : KeyUpdateKeys)
  /-- A validator changes its commission. -/
  | commissionUpdate (validator : Address) (commission : Nat)
  /-- A validator changes its network key: the key if it decoded. -/
  | x25519Update (validator : Address) (key : Option X25519Key)
  /-- A validator changes its network address: the address if it parsed. -/
  | p2pAddrUpdate (validator : Address) (addr : Option NetAddr)
deriving DecidableEq, Repr

end StakeTable
