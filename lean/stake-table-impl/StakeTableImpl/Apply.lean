module

public import StakeTableImpl.Node

/-!
# Applying an event, as the node does

`StakeTable.apply` follows `StakeTableState::apply_event`
(`crates/espresso/types/src/v0/impls/stake_table.rs`): the same checks in the same
order, and the same error for each. The result has the node's shape: a fatal error,
an expected error that skips the event, or the new state.

The state changes themselves are the specification's (`State.admit`,
`State.update`, `State.remove`); what this adds is the order of the checks and the
errors.
-/

@[expose] public section

namespace StakeTable

open State

/-- An error that stops the replay. The node's `StakeTableError`. -/
inductive StakeTableError where
  /-- A registration under an account that is already registered. -/
  | alreadyRegistered (account : Address)
  /-- An event for a validator that is not registered. -/
  | validatorNotFound (validator : Address)
  /-- An undelegation by a delegator with no delegation to the validator. -/
  | delegatorNotFound (delegator : Address)
  /-- A consensus key that was admitted before. -/
  | blsKeyAlreadyUsed (key : BlsKey)
  /-- A registration under an account whose validator left. -/
  | validatorAlreadyExited (account : Address)
  /-- A delegation of nothing. -/
  | zeroDelegatorStake (delegator : Address)
  /-- An undelegation of more than is delegated. -/
  | insufficientStake
  /-- A key update whose signatures do not verify. -/
  | authenticationFailed
  /-- A commission above `maxCommission`. -/
  | invalidCommission (validator : Address) (commission : Nat)
  /-- A network key that was admitted before. -/
  | x25519KeyAlreadyUsed (key : X25519Key)
  /-- A stake that would not fit in 256 bits. The node panics here instead. -/
  | stakeOverflow
deriving DecidableEq, Repr

/-- An error that skips the event. The node's `ExpectedStakeTableError`. -/
inductive ExpectedError where
  /-- A light client key that was admitted before. -/
  | schnorrKeyAlreadyUsed (key : SchnorrKey)
  /-- A consensus key that does not decode. -/
  | invalidBlsKey
  /-- A light client key that does not decode. -/
  | invalidSchnorrKey
deriving DecidableEq, Repr

/-- What `apply_event` returns: `Result<Result<(), ExpectedStakeTableError>, StakeTableError>`. -/
abbrev ApplyResult := Except StakeTableError (Except ExpectedError State)

/-- The outcome an `ApplyResult` comes to. -/
def ApplyResult.outcome : ApplyResult → Outcome
  | .ok (.ok s) => .applied s
  | .ok (.error _) => .skipped
  | .error _ => .rejected

/-- The checks of a key update once both keys decoded and verified. -/
def State.applyRekey (s : State) (a : Address) (bls : BlsKey) (schnorr : SchnorrKey) :
    ApplyResult :=
  if ¬ s.registered a then .error (.validatorNotFound a)
  else if bls ∈ s.usedBls then .error (.blsKeyAlreadyUsed bls)
  else if schnorr ∈ s.usedSchnorr then .ok (.error (.schnorrKeyAlreadyUsed schnorr))
  else .ok (.ok
    { s.update a (fun v => { v with blsKey := some bls, schnorrKey := some schnorr,
                                    authenticated := true }) with
      usedBls := bls :: s.usedBls
      usedSchnorr := schnorr :: s.usedSchnorr })

/--
Admit a registration emitted by `StakeTableV2` or later, dropping a Schnorr key
admitted before.
-/
def State.admitRegistration (s : State) (a : Address) (keys : RegistrationKeys)
    (commission : Nat) (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) : State :=
  if s.schnorrTaken keys.schnorr then
    s.admit (.new a keys.bls none x25519 p2pAddr commission false)
  else s.admit (.new a keys.bls keys.schnorr x25519 p2pAddr commission keys.isVerified)

/-- Apply one event, with the node's checks in the node's order. -/
def apply (s : State) : Event → ApplyResult
  | .register a bls schnorr commission =>
    if a ∈ s.exits then .error (.validatorAlreadyExited a)
    else if s.registered a then .error (.alreadyRegistered a)
    else if let some k := bls.filter (· ∈ s.usedBls) then .error (.blsKeyAlreadyUsed k)
    else if let some j := schnorr.filter (· ∈ s.usedSchnorr) then
      .ok (.error (.schnorrKeyAlreadyUsed j))
    else .ok (.ok (s.admit (.new a bls schnorr none none commission (bls.isSome && schnorr.isSome))))
  | .registerV2 a keys commission =>
    if a ∈ s.exits then .error (.validatorAlreadyExited a)
    else if s.registered a then .error (.alreadyRegistered a)
    else if let some k := keys.bls.filter (· ∈ s.usedBls) then .error (.blsKeyAlreadyUsed k)
    else .ok (.ok (s.admitRegistration a keys commission none none))
  | .registerV3 a keys commission x25519 p2pAddr =>
    if a ∈ s.exits then .error (.validatorAlreadyExited a)
    else if s.registered a then .error (.alreadyRegistered a)
    else if let some k := keys.bls.filter (· ∈ s.usedBls) then .error (.blsKeyAlreadyUsed k)
    else if let some k := x25519.filter (· ∈ s.usedX25519) then .error (.x25519KeyAlreadyUsed k)
    else .ok (.ok (s.admitRegistration a keys commission x25519 p2pAddr))
  | .deregister a =>
    if ¬ s.registered a then .error (.validatorNotFound a)
    else .ok (.ok (s.remove a))
  | .delegate d a amount =>
    if amount = 0 then .error (.zeroDelegatorStake d)
    else match s.find? a with
      | none => .error (.validatorNotFound a)
      | some v =>
        if U256 ≤ v.stake + amount then .error .stakeOverflow
        else if U256 ≤ amountOf d v.delegators + amount then .error .stakeOverflow
        else .ok (.ok (s.update a fun v =>
          { v with stake := v.stake + amount, delegators := credit d amount v.delegators }))
  | .undelegate d a amount =>
    match s.find? a with
    | none => .error (.validatorNotFound a)
    | some v =>
      if v.stake < amount then .error .insufficientStake
      else if delegates d v.delegators = false then .error (.delegatorNotFound d)
      else if amountOf d v.delegators < amount then .error .insufficientStake
      else .ok (.ok (s.update a fun v =>
        { v with stake := v.stake - amount, delegators := debit d amount v.delegators }))
  | .keyUpdate a bls schnorr =>
    match bls, schnorr with
    | none, _ => .ok (.error .invalidBlsKey)
    | some _, none => .ok (.error .invalidSchnorrKey)
    | some k, some j => s.applyRekey a k j
  | .keyUpdateV2 a keys =>
    match keys with
    | .invalidBls => .ok (.error .invalidBlsKey)
    | .invalidSchnorr => .ok (.error .invalidSchnorrKey)
    | .failed => .error .authenticationFailed
    | .verified k j => s.applyRekey a k j
  | .commissionUpdate a commission =>
    if maxCommission < commission then .error (.invalidCommission a commission)
    else if ¬ s.registered a then .error (.validatorNotFound a)
    else .ok (.ok (s.update a fun v => { v with commission }))
  | .x25519Update a key =>
    if ¬ s.registered a then .error (.validatorNotFound a)
    else if let some k := key.filter (· ∈ s.usedX25519) then .error (.x25519KeyAlreadyUsed k)
    else match key with
      | some k => .ok (.ok { s.update a (fun v => { v with x25519Key := some k }) with
                             usedX25519 := k :: s.usedX25519 })
      | none => .ok (.ok (s.update a fun v => { v with x25519Key := none }))
  | .p2pAddrUpdate a p2pAddr =>
    if ¬ s.registered a then .error (.validatorNotFound a)
    else .ok (.ok (s.update a fun v => { v with p2pAddr }))

end StakeTable
