module

public import NewProtocolImpl
public import Lean.Data.Json

/-!
# JSON for the protocol types

`ToJson`/`FromJson` for everything a trace carries, **derived** rather than
written out. That is the point: a hand-rolled encoding drifts from the types one
field at a time, and silently, because both ends of a positional format agree on
being wrong. Derived instances change when the types change.

Three deliberate choices:

* The scalar wrappers get instances by hand. Derived, each would read
  `{"toNat": 3}`, and a single proposal contains five of them.
* Of those, the three cryptographic ones — a block identity, a payload
  commitment, a key — travel as a **string**, because a real one is 32 bytes or
  more and a recorder's JSON writer will not emit a 78-digit integer. The string
  is whatever text the recorder identifies the value by, base64 or hex or
  anything else: nothing here interprets an identity, every rule only ever
  *compares* one, so all the model needs of the string is that distinct values
  give distinct numbers (`identToNat`, which reads the UTF-8 bytes and so
  accepts any string at all). A plain number is still accepted on the way in, so
  a trace can be written by hand with small values.
  `NewProtocol.ViewNumber` stays a number, being one.
* Everything else is derived, including the one wart that brings: a constructor
  with a single argument keeps that argument's name as a key, so
  `NewProtocol.Output.send` reads `{"send": {"m": …}}`. Removing it would mean
  writing out instances for `Input`, `Output` and `Message` — thirty
  constructors of exactly the code this file exists to avoid.

The instances live in this package's namespace, not the specification's: they
are the harness's business, and the specification has no reason to depend on
`Lean`. Only the derived instances take their names from the type's namespace,
which is Lean's doing.
-/

@[expose] public section

open Lean

namespace NewProtocolDiff

open NewProtocol

instance : ToJson ViewNumber := ⟨fun v => toJson v.toNat⟩
instance : FromJson ViewNumber := ⟨fun j => ViewNumber.mk <$> fromJson? j⟩

instance : ToJson EpochNumber := ⟨fun e => toJson e.toNat⟩
instance : FromJson EpochNumber := ⟨fun j => EpochNumber.mk <$> fromJson? j⟩

/-!
### Identities

A recorder names each cryptographic value with a string; the model stores a
number. All that is asked of the correspondence is that it be injective, since
`blockHash` images are only ever compared — which is also exactly what
`CollisionFree` assumes of them (`NewProtocolSpec.Assumptions`).

The correspondence is *bijective* base 256 over the string's UTF-8 bytes: a
string is a numeral whose digits are `byte + 1`, so no digit is zero and no two
strings share a number however their lengths differ. Reading the bytes rather
than the characters makes this total — any string at all names a number, whatever
encoding a recorder identifies its values with.

The inverse is where the partiality lands instead, since not every byte sequence
is valid UTF-8. It costs nothing: a number that names no string goes out as a
number, which is a form the reader accepts, so every number still survives the
round trip. In a real trace the case does not arise, every number there having
come from a string in the first place.

Bijective rather than ordinary positional numeration so that the inverse exists
at all — under which a trace this package re-renders is textually the one it
read, and can be diffed against the recorder's own.
-/

/-- The number of digits, which for a bijective numeration is also the base. -/
def identBase : Nat := 256

/-- A number for an identity string. Injective, and total for any string. -/
def identToNat (s : String) : Nat :=
  s.toUTF8.foldl (fun acc b => acc * identBase + b.toNat + 1) 0

/-- And back to the same string, when the bytes are text. -/
def natToIdent (n : Nat) : Option String :=
  let rec go (fuel n : Nat) (acc : List UInt8) : List UInt8 :=
    match fuel, n with
    | 0, _ => acc
    | _, 0 => acc
    | fuel + 1, n =>
      let digit := (n - 1) % identBase + 1
      go fuel ((n - digit) / identBase) (UInt8.ofNat (digit - 1) :: acc)
  String.fromUTF8? ⟨(go n n []).toArray⟩

/-- A cryptographic value goes out as the text it came in as, or as a number. -/
def cryptoToJson (n : Nat) : Json :=
  match natToIdent n with
  | some s => toJson s
  | none => toJson n

/-- And comes in as that text, or as a plain number if a human wrote the trace. -/
def cryptoFromJson : Json → Except String Nat
  | .str s => .ok (identToNat s)
  | j => fromJson? j

instance : ToJson BlockHash := ⟨fun h => cryptoToJson h.toNat⟩
instance : FromJson BlockHash := ⟨fun j => BlockHash.mk <$> cryptoFromJson j⟩

instance : ToJson PayloadCommit := ⟨fun c => cryptoToJson c.toNat⟩
instance : FromJson PayloadCommit := ⟨fun j => PayloadCommit.mk <$> cryptoFromJson j⟩

instance : ToJson PubKey := ⟨fun k => cryptoToJson k.toNat⟩
instance : FromJson PubKey := ⟨fun j => PubKey.mk <$> cryptoFromJson j⟩

deriving instance ToJson, FromJson for Vote1Data, Vote2Data, TimeoutData, Certificate

instance : ToJson BlockHeader :=
  ⟨fun h => Json.mkObj
    [("payloadCommit", toJson h.payloadCommit), ("blockNumber", toJson h.blockNumber)]⟩

/--
The mark a parse error carries when a trace is *outside* the specification rather
than broken.

`FromJson` fixes the error type to `String`, so the distinction has no type to
travel in and has to travel in the message. `NewProtocolDiff.Corpus` reads it back
out: a trace refused for a reason so marked is out of scope, which is what the
reason already said in prose, and not a failure. Without the mark the two are
indistinguishable to a caller, and a boundary of the model fails the run.
-/
def outOfScopeMark : String := "outside the model: "

/--
A header without a payload commitment is refused, and the refusal is marked.

A recorder writes `null` there for a header carrying a commitment of a kind this
protocol does not accept, which in a real run means a block inherited from an
earlier protocol at a version boundary. That is outside what the specification
covers, so such a trace is out of scope rather than a disagreement, and
`outOfScopeMark` is what carries that difference to whoever reads the error.
-/
instance : FromJson BlockHeader := ⟨fun j => do
  let pc ← j.getObjVal? "payloadCommit"
  match pc with
  | .null => throw (outOfScopeMark ++ "a header with no payload commitment is a \
      block from before a version boundary, which this model does not cover")
  | _ => BlockHeader.mk <$> fromJson? pc <*> (do fromJson? (← j.getObjVal? "blockNumber"))⟩
deriving instance ToJson, FromJson for Proposal, VidShare, Vote, CatchupEvidence
deriving instance ToJson, FromJson for Message, Input, Output, Event

end NewProtocolDiff
