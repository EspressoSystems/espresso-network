module

public import StakeTableImpl.Node.Delegations

/-!
# Applying an event

What each event does to the state: whether it applies, is skipped or is rejected,
and the state it gives when it applies. The conditions are stated as a whole; the
order the node checks them in, and which error it reports, are the
implementation's.

Lists stand for the node's maps and sets; only membership and the order of
registration are read from them.
-/

@[expose] public section

namespace StakeTable

/-- A validator just registered: no stake, no delegators. -/
def Validator.new (account : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) (commission : Nat)
    (authenticated : Bool) : Validator :=
  { account, blsKey := bls, schnorrKey := schnorr, x25519Key := x25519, p2pAddr, stake := 0,
    commission, delegators := [], authenticated }

namespace State

variable (s : State)

/-- The validator registered under `a`, if any. -/
def find? (a : Address) : Option Validator :=
  s.validators.find? (·.account == a)

/-- Whether a validator is registered under `a`. -/
def registered (a : Address) : Bool :=
  s.validators.any (·.account == a)

/-- Whether a decoded BLS key was admitted before. -/
def blsTaken : Option BlsKey → Bool
  | some k => decide (k ∈ s.usedBls)
  | none => false

/-- Whether a decoded Schnorr key was admitted before. -/
def schnorrTaken : Option SchnorrKey → Bool
  | some k => decide (k ∈ s.usedSchnorr)
  | none => false

/-- Whether a decoded x25519 key was admitted before. -/
def x25519Taken : Option X25519Key → Bool
  | some k => decide (k ∈ s.usedX25519)
  | none => false

/-- Add a validator after the others, and admit its keys. -/
def admit (v : Validator) : State :=
  { s with
    validators := s.validators ++ [v]
    usedBls := v.blsKey.toList ++ s.usedBls
    usedSchnorr := v.schnorrKey.toList ++ s.usedSchnorr
    usedX25519 := v.x25519Key.toList ++ s.usedX25519 }

/-- Change the validator registered under `a`, in place. -/
def update (a : Address) (f : Validator → Validator) : State :=
  { s with validators := s.validators.map fun v => if v.account = a then f v else v }

/-- Remove the validator registered under `a`, and record that it left. Its keys stay admitted. -/
def remove (a : Address) : State :=
  { s with
    validators := s.validators.filter (·.account != a)
    exits := a :: s.exits }

/--
A registration emitted by `StakeTableV2` or later.

A Schnorr key admitted before does not reject the registration, since `StakeTable`
did not prevent reuse: the validator registers without it, and
unauthenticated.
-/
def registerNew (a : Address) (keys : RegistrationKeys) (commission : Nat)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) : Outcome :=
  if a ∈ s.exits ∨ s.registered a ∨ s.blsTaken keys.bls ∨ s.x25519Taken x25519 then .rejected
  else if s.schnorrTaken keys.schnorr then
    .applied (s.admit (.new a keys.bls none x25519 p2pAddr commission false))
  else .applied (s.admit (.new a keys.bls keys.schnorr x25519 p2pAddr commission keys.isVerified))

/-- Replace a validator's consensus and light client keys with decoded, verified ones. -/
def rekey (a : Address) (bls : BlsKey) (schnorr : SchnorrKey) : Outcome :=
  if ¬ s.registered a ∨ bls ∈ s.usedBls then .rejected
  else if schnorr ∈ s.usedSchnorr then .skipped
  else .applied
    { s.update a (fun v => { v with blsKey := some bls, schnorrKey := some schnorr,
                                    authenticated := true }) with
      usedBls := bls :: s.usedBls
      usedSchnorr := schnorr :: s.usedSchnorr }

end State

open State in
/--
Apply one event.

A delegation that would take a validator's stake to `U256` or beyond is rejected:
the node panics on it.
-/
def step (s : State) : Event → Outcome
  | .register a bls schnorr commission =>
    if a ∈ s.exits ∨ s.registered a ∨ s.blsTaken bls then .rejected
    else if s.schnorrTaken schnorr then .skipped
    else .applied (s.admit (.new a bls schnorr none none commission (bls.isSome && schnorr.isSome)))
  | .registerV2 a keys commission => s.registerNew a keys commission none none
  | .registerV3 a keys commission x25519 p2pAddr => s.registerNew a keys commission x25519 p2pAddr
  | .deregister a => if s.registered a then .applied (s.remove a) else .rejected
  | .delegate d a amount =>
    match s.find? a with
    | none => .rejected
    | some v =>
      if amount = 0 ∨ U256 ≤ v.stake + amount then .rejected
      else .applied (s.update a fun v =>
        { v with stake := v.stake + amount, delegators := credit d amount v.delegators })
  | .undelegate d a amount =>
    match s.find? a with
    | none => .rejected
    | some v =>
      if v.stake < amount ∨ delegates d v.delegators = false ∨ amountOf d v.delegators < amount
      then .rejected
      else .applied (s.update a fun v =>
        { v with stake := v.stake - amount, delegators := debit d amount v.delegators })
  | .keyUpdate a (some bls) (some schnorr) => s.rekey a bls schnorr
  | .keyUpdate _ _ _ => .skipped
  | .keyUpdateV2 a (.verified bls schnorr) => s.rekey a bls schnorr
  | .keyUpdateV2 _ .failed => .rejected
  | .keyUpdateV2 _ _ => .skipped
  | .commissionUpdate a commission =>
    if maxCommission < commission ∨ ¬ s.registered a then .rejected
    else .applied (s.update a fun v => { v with commission })
  | .x25519Update a none =>
    if s.registered a then .applied (s.update a fun v => { v with x25519Key := none })
    else .rejected
  | .x25519Update a (some k) =>
    if ¬ s.registered a ∨ k ∈ s.usedX25519 then .rejected
    else .applied { s.update a (fun v => { v with x25519Key := some k }) with
                    usedX25519 := k :: s.usedX25519 }
  | .p2pAddrUpdate a p2pAddr =>
    if s.registered a then .applied (s.update a fun v => { v with p2pAddr }) else .rejected

/--
Apply events in order, from `s`.

`none` if an event is rejected: the node then has no stake table.
-/
def replay (s : State) : List Event → Option State
  | [] => some s
  | e :: es =>
    match step s e with
    | .applied s' => replay s' es
    | .skipped => replay s es
    | .rejected => none

/-! ## The invariant

What holds of every state the events build, from the empty state on
(`StakeTable.replay_inv`). It needs no assumption about what the contract emits:
an event that would break it is skipped or rejected.
-/

/--
A validator's stake adds up.

Its stake is the sum of its delegations, each delegator appears once with a
positive amount, and the stake fits in 256 bits.
-/
structure Validator.Sound (v : Validator) : Prop where
  /-- Each delegator appears once. -/
  delegatorsNodup : (v.delegators.map (·.1)).Nodup
  /-- Each delegation is positive. -/
  delegationsPositive : ∀ e ∈ v.delegators, 0 < e.2
  /-- The stake is the sum of the delegations. -/
  stakeTotal : v.stake = total v.delegators
  /-- The stake fits in 256 bits. -/
  stakeBound : v.stake < U256

/--
A kind of key is tracked: every key a validator holds was admitted, and no two
validators hold the same one.

`key` reads the kind of key off a validator, and `used` lists the keys admitted.
Since accounts are distinct, two validators with the same account are the same
validator.
-/
structure KeysTracked {κ : Type} (vs : List Validator) (key : Validator → Option κ)
    (used : List κ) : Prop where
  /-- Every key held was admitted. -/
  admitted : ∀ v ∈ vs, ∀ k, key v = some k → k ∈ used
  /-- No two validators hold the same key. -/
  unique : ∀ v ∈ vs, ∀ w ∈ vs, ∀ k, key v = some k → key w = some k → v.account = w.account

/-- The invariant of the stake table state. -/
structure Inv (s : State) : Prop where
  /-- No account is registered twice. -/
  accountsNodup : (s.validators.map (·.account)).Nodup
  /-- No registered validator has left. -/
  notExited : ∀ v ∈ s.validators, v.account ∉ s.exits
  /-- Every validator's stake adds up. -/
  sound : ∀ v ∈ s.validators, v.Sound
  /-- Consensus keys are admitted and distinct. -/
  bls : KeysTracked s.validators (·.blsKey) s.usedBls
  /-- Light client keys are admitted and distinct. -/
  schnorr : KeysTracked s.validators (·.schnorrKey) s.usedSchnorr
  /-- Network keys are admitted and distinct. -/
  x25519 : KeysTracked s.validators (·.x25519Key) s.usedX25519

/-- The accounts of the registered validators, in registration order. -/
def State.accounts (s : State) : List Address :=
  s.validators.map (·.account)

/-- Nothing admitted or recorded as left is forgotten from `s` to `s'`. -/
structure Grows (s s' : State) : Prop where
  /-- Admitted consensus keys stay admitted. -/
  usedBls : s.usedBls ⊆ s'.usedBls
  /-- Admitted light client keys stay admitted. -/
  usedSchnorr : s.usedSchnorr ⊆ s'.usedSchnorr
  /-- Admitted network keys stay admitted. -/
  usedX25519 : s.usedX25519 ⊆ s'.usedX25519
  /-- Validators that left stay recorded. -/
  exits : s.exits ⊆ s'.exits

end StakeTable
