module

public import StakeTableImpl.Apply
public import StakeTableImpl.Indexed.State

/-!
# Applying an event to the indexed state

`StakeTable.Indexed.apply` makes the checks of `StakeTable.apply`, in the same order
and with the same errors, on the indexed state.
-/

@[expose] public section

namespace StakeTable.Indexed

/--
An event applied, or skipped.

A skipped event hands back the state unchanged, so that the caller holds no other
reference to it and the next event can update it in place.
-/
inductive Applied where
  /-- The event applied, giving this state. -/
  | applied (s : State)
  /-- The event is skipped for this reason; the state is unchanged. -/
  | skipped (err : ExpectedError) (s : State)

/-- What applying an event to the indexed state returns. -/
abbrev ApplyResult := Except StakeTableError Applied

/-- The checks of a key update once both keys decoded and verified. -/
def State.applyRekey (s : State) (a : Address) (bls : BlsKey) (schnorr : SchnorrKey) :
    ApplyResult :=
  if !s.registered a then .error (.validatorNotFound a)
  else if s.usedBls.contains bls then .error (.blsKeyAlreadyUsed bls)
  else if s.usedSchnorr.contains schnorr then .ok (.skipped (.schnorrKeyAlreadyUsed schnorr) s)
  else .ok (.applied
    { s.update a (fun v => { v with blsKey := some bls, schnorrKey := some schnorr,
                                    authenticated := true }) with
      usedBls := s.usedBls.insert bls
      usedSchnorr := s.usedSchnorr.insert schnorr })

/--
Admit a registration emitted by `StakeTableV2` or later, dropping a Schnorr key
admitted before.
-/
def State.admitRegistration (s : State) (a : Address) (keys : RegistrationKeys)
    (commission : Nat) (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) : State :=
  if (keys.schnorr.filter s.usedSchnorr.contains).isSome then
    s.admit (.new a keys.bls none x25519 p2pAddr commission false)
  else s.admit (.new a keys.bls keys.schnorr x25519 p2pAddr commission keys.isVerified)

/-- Apply one event, with the node's checks in the node's order. -/
def apply (s : State) : Event → ApplyResult
  | .register a bls schnorr commission =>
    if s.exits.contains a then .error (.validatorAlreadyExited a)
    else if s.registered a then .error (.alreadyRegistered a)
    else if let some k := bls.filter s.usedBls.contains then .error (.blsKeyAlreadyUsed k)
    else if let some j := schnorr.filter s.usedSchnorr.contains then
      .ok (.skipped (.schnorrKeyAlreadyUsed j) s)
    else .ok (.applied (s.admit (.new a bls schnorr none none commission (bls.isSome && schnorr.isSome))))
  | .registerV2 a keys commission =>
    if s.exits.contains a then .error (.validatorAlreadyExited a)
    else if s.registered a then .error (.alreadyRegistered a)
    else if let some k := keys.bls.filter s.usedBls.contains then .error (.blsKeyAlreadyUsed k)
    else .ok (.applied (s.admitRegistration a keys commission none none))
  | .registerV3 a keys commission x25519 p2pAddr =>
    if s.exits.contains a then .error (.validatorAlreadyExited a)
    else if s.registered a then .error (.alreadyRegistered a)
    else if let some k := keys.bls.filter s.usedBls.contains then .error (.blsKeyAlreadyUsed k)
    else if let some k := x25519.filter s.usedX25519.contains then
      .error (.x25519KeyAlreadyUsed k)
    else .ok (.applied (s.admitRegistration a keys commission x25519 p2pAddr))
  | .deregister a =>
    if !s.registered a then .error (.validatorNotFound a)
    else .ok (.applied (s.remove a))
  | .delegate d a amount =>
    if amount = 0 then .error (.zeroDelegatorStake d)
    else match s.validators[a]? with
      | none => .error (.validatorNotFound a)
      | some v =>
        if U256 ≤ v.stake + amount then .error .stakeOverflow
        else if U256 ≤ v.delegators.getD d 0 + amount then .error .stakeOverflow
        else .ok (.applied (s.update a fun v =>
          { v with stake := v.stake + amount, delegators := credit d amount v.delegators }))
  | .undelegate d a amount =>
    match s.validators[a]? with
    | none => .error (.validatorNotFound a)
    | some v =>
      if v.stake < amount then .error .insufficientStake
      else if v.delegators.contains d = false then .error (.delegatorNotFound d)
      else if v.delegators.getD d 0 < amount then .error .insufficientStake
      else .ok (.applied (s.update a fun v =>
        { v with stake := v.stake - amount, delegators := debit d amount v.delegators }))
  | .keyUpdate a bls schnorr =>
    match bls, schnorr with
    | none, _ => .ok (.skipped .invalidBlsKey s)
    | some _, none => .ok (.skipped .invalidSchnorrKey s)
    | some k, some j => s.applyRekey a k j
  | .keyUpdateV2 a keys =>
    match keys with
    | .invalidBls => .ok (.skipped .invalidBlsKey s)
    | .invalidSchnorr => .ok (.skipped .invalidSchnorrKey s)
    | .failed => .error .authenticationFailed
    | .verified k j => s.applyRekey a k j
  | .commissionUpdate a commission =>
    if maxCommission < commission then .error (.invalidCommission a commission)
    else if !s.registered a then .error (.validatorNotFound a)
    else .ok (.applied (s.update a fun v => { v with commission }))
  | .x25519Update a key =>
    if !s.registered a then .error (.validatorNotFound a)
    else if let some k := key.filter s.usedX25519.contains then .error (.x25519KeyAlreadyUsed k)
    else match key with
      | some k => .ok (.applied { s.update a (fun v => { v with x25519Key := some k }) with
                             usedX25519 := s.usedX25519.insert k })
      | none => .ok (.applied (s.update a fun v => { v with x25519Key := none }))
  | .p2pAddrUpdate a p2pAddr =>
    if !s.registered a then .error (.validatorNotFound a)
    else .ok (.applied (s.update a fun v => { v with p2pAddr }))

/--
Apply events in order: an expected error skips the event, a fatal one stops the
replay.
-/
def applyAll (s : State) : List Event → Except StakeTableError State
  | [] => .ok s
  | e :: es =>
    match apply s e with
    | .ok (.applied s') => applyAll s' es
    | .ok (.skipped _ s') => applyAll s' es
    | .error err => .error err

end StakeTable.Indexed
