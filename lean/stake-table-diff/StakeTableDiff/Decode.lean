module

public import StakeTableImpl
public import Lean.Data.Json

/-!
# Reading the node's cases

The node's corpus test writes one JSON object per line (`case.rs` under
`crates/espresso/types/src/v0/impls/stake_table/corpus` describes the format). Byte
strings are `0x`-prefixed hex, amounts and hashes decimal strings.

A key or an address becomes the number of its bytes read big-endian after a leading 1
(`StakeTableDiff.natOfBytes`), so that distinct byte strings give distinct numbers:
the implementation only ever compares them.
-/

@[expose] public section

open Lean StakeTable

namespace StakeTableDiff

/-- The number for a byte string: big-endian, after a leading 1. -/
def natOfBytes (b : List UInt8) : Nat :=
  b.foldl (fun n x => n * 256 + x.toNat) 1

def hexDigit (c : Char) : Except String Nat :=
  if '0' ≤ c ∧ c ≤ '9' then pure (c.toNat - '0'.toNat)
  else if 'a' ≤ c ∧ c ≤ 'f' then pure (c.toNat - 'a'.toNat + 10)
  else if 'A' ≤ c ∧ c ≤ 'F' then pure (c.toNat - 'A'.toNat + 10)
  else throw s!"not a hex digit: {c}"

/-- The bytes of a `0x`-prefixed hex string. -/
def hexBytes (s : String) : Except String (List UInt8) := do
  let some digits := s.dropPrefix? "0x" | throw s!"not 0x-prefixed: {s}"
  let rec go : List Char → Except String (List UInt8)
    | [] => pure []
    | [_] => throw s!"odd number of hex digits: {s}"
    | a :: b :: rest => do
      let byte := (← hexDigit a) * 16 + (← hexDigit b)
      return byte.toUInt8 :: (← go rest)
  go digits.toString.toList

/-- Field `k` of an object. -/
def field (j : Json) (k : String) : Except String Json :=
  (j.getObjVal? k).mapError (s!"field {k}: " ++ ·)

def str (j : Json) (k : String) : Except String String := do (← field j k).getStr?

def num (j : Json) (k : String) : Except String Nat := do (← field j k).getNat?

def bool (j : Json) (k : String) : Except String Bool := do (← field j k).getBool?

/-- A decimal string as a number. -/
def decimal (s : String) : Except String Nat :=
  match s.toNat? with
  | some n => pure n
  | none => throw s!"not a decimal number: {s}"

def amount (j : Json) (k : String) : Except String Nat := do decimal (← str j k)

/-- Field `k`, read by `f`, or none if it is `null`. -/
def opt {α : Type} (j : Json) (k : String) (f : Json → Except String α) :
    Except String (Option α) := do
  match ← field j k with
  | .null => pure none
  | v => some <$> f v

/-- The elements of array field `k`, each read by `f`. -/
def arr {α : Type} (j : Json) (k : String) (f : Json → Except String α) :
    Except String (List α) := do
  (← (← field j k).getArr?).toList.mapM f

/-- A pair `[a, b]`. -/
def pair {α β : Type} (f : Json → Except String α) (g : Json → Except String β) (j : Json) :
    Except String (α × β) := do
  match (← j.getArr?).toList with
  | [a, b] => return (← f a, ← g b)
  | _ => throw s!"not a pair: {j.compress}"

/-- A byte string, as the number the implementation compares. -/
def ident (j : Json) : Except String Nat := do natOfBytes <$> hexBytes (← j.getStr?)

def decimalJ (j : Json) : Except String Nat := do decimal (← j.getStr?)

def address (j : Json) (k : String) : Except String Address := do Address.mk <$> ident (← field j k)

def blsKey (j : Json) : Except String BlsKey := BlsKey.mk <$> ident j

def schnorrKey (j : Json) : Except String SchnorrKey := SchnorrKey.mk <$> ident j

def x25519Key (j : Json) : Except String X25519Key := X25519Key.mk <$> ident j

def netAddr (j : Json) : Except String NetAddr := NetAddr.mk <$> j.getStr?

def registrationKeys (j : Json) : Except String RegistrationKeys := do
  match ← str j "verdict" with
  | "verified" => return .verified (← blsKey (← field j "bls")) (← schnorrKey (← field j "schnorr"))
  | "unverified" => return .unverified (← opt j "bls" blsKey) (← opt j "schnorr" schnorrKey)
  | v => throw s!"unknown registration verdict {v}"

def keyUpdateKeys (j : Json) : Except String KeyUpdateKeys := do
  match ← str j "verdict" with
  | "verified" => return .verified (← blsKey (← field j "bls")) (← schnorrKey (← field j "schnorr"))
  | "invalidBls" => return .invalidBls
  | "invalidSchnorr" => return .invalidSchnorr
  | "failed" => return .failed
  | v => throw s!"unknown key update verdict {v}"

def event (j : Json) : Except String Event := do
  match ← str j "type" with
  | "register" =>
    return .register (← address j "account") (← opt j "bls" blsKey) (← opt j "schnorr" schnorrKey)
      (← num j "commission")
  | "registerV2" =>
    return .registerV2 (← address j "account") (← registrationKeys (← field j "keys"))
      (← num j "commission")
  | "registerV3" =>
    return .registerV3 (← address j "account") (← registrationKeys (← field j "keys"))
      (← num j "commission") (← opt j "x25519" x25519Key) (← opt j "p2pAddr" netAddr)
  | "deregister" => return .deregister (← address j "validator")
  | "delegate" => return .delegate (← address j "delegator") (← address j "validator") (← amount j "amount")
  | "undelegate" =>
    return .undelegate (← address j "delegator") (← address j "validator") (← amount j "amount")
  | "keyUpdate" =>
    return .keyUpdate (← address j "validator") (← opt j "bls" blsKey) (← opt j "schnorr" schnorrKey)
  | "keyUpdateV2" => return .keyUpdateV2 (← address j "validator") (← keyUpdateKeys (← field j "keys"))
  | "commissionUpdate" => return .commissionUpdate (← address j "validator") (← num j "commission")
  | "x25519Update" => return .x25519Update (← address j "validator") (← opt j "key" x25519Key)
  | "p2pAddrUpdate" => return .p2pAddrUpdate (← address j "validator") (← opt j "addr" netAddr)
  | t => throw s!"unknown event type {t}"

/--
A stake table state in a form both sides give alike: validators in registration
order, keys and addresses as numbers, and every set sorted.
-/
structure StateView where
  validators : List (Nat × Option Nat × Option Nat × Option Nat × Option String × Nat × Nat × Bool
    × List (Nat × Nat))
  usedBls : List Nat
  usedSchnorr : List Nat
  usedX25519 : List Nat
  exits : List Nat
deriving BEq, Repr

def sortNats (l : List Nat) : List Nat := l.mergeSort (· ≤ ·)

def sortPairs (l : List (Nat × Nat)) : List (Nat × Nat) := l.mergeSort (·.1 ≤ ·.1)

/-- The node's state as the corpus records it. -/
def stateView (j : Json) : Except String StateView := do
  let validator (v : Json) := do
    return (← ident (← field v "account"), ← opt v "bls" ident, ← opt v "schnorr" ident,
      ← opt v "x25519" ident, ← opt v "p2pAddr" Json.getStr?, ← amount v "stake",
      ← num v "commission", ← bool v "authenticated",
      sortPairs (← arr v "delegators" (pair ident decimalJ)))
  return {
    validators := ← arr j "validators" validator
    usedBls := sortNats (← arr j "usedBls" ident)
    usedSchnorr := sortNats (← arr j "usedSchnorr" ident)
    usedX25519 := sortNats (← arr j "usedX25519" ident)
    exits := sortNats (← arr j "exits" ident) }

/-- The implementation's indexed state in the same form. -/
def viewOf (s : Indexed.State) : StateView where
  validators := s.order.toList.filterMap fun a => s.validators[a]?.map fun v =>
    (v.account.toNat, v.blsKey.map (·.toNat), v.schnorrKey.map (·.toNat),
      v.x25519Key.map (·.toNat), v.p2pAddr.map (·.toString), v.stake, v.commission,
      v.authenticated, sortPairs (v.delegators.toList.map fun (d, n) => (d.toNat, n)))
  usedBls := sortNats (s.usedBls.toList.map (·.toNat))
  usedSchnorr := sortNats (s.usedSchnorr.toList.map (·.toNat))
  usedX25519 := sortNats (s.usedX25519.toList.map (·.toNat))
  exits := sortNats (s.exits.toList.map (·.toNat))

end StakeTableDiff
