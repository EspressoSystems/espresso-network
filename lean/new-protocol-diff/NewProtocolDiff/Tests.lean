module

public import NewProtocolDiff.Trace
public import NewProtocolDiff.Replay
public import NewProtocolDiff.Corpus
public meta import NewProtocolSpec.Interface
public meta import NewProtocolImpl.Protocol
public meta import NewProtocolDiff.Json
public meta import NewProtocolDiff.Trace
public meta import NewProtocolDiff.Replay
public meta import NewProtocolDiff.Corpus

/-!
# Checks on the harness itself

A comparison harness that silently mis-reads its input is worse than none, so
the format is checked at build time in both directions:

* the identity injection does not collide and preserves the recorder's text, and
  every constructor round-trips through JSON — which catches an instance the
  deriving got wrong as well as one a future hand-written override breaks;
* the comparison accepts a run the machine agrees with, and rejects one it does
  not — including the case that matters most, a recorded identity that does not
  match the block, since that is how a hash-namespace mistake would show up.
-/

@[expose] public section

namespace NewProtocolDiff
namespace Tests

open Lean NewProtocol

-- `NewProtocol.Event` carries no equality of its own, nothing in the
-- specification comparing two events. The round-trip check needs one, so it is
-- derived here rather than the specification growing an instance for a test.
deriving instance DecidableEq for Event

/-! ## Identities

The injection a recorder's strings go through. Checked on the shapes a recorder
actually emits — base64 and hex of a 32-byte value — and on the property that
matters, which is that different text never collides, including across lengths
and outside ASCII.
-/

private def identityTexts : List String :=
  [ "", "A", "AA", "AAA", "aA", "Aa", "0", "/", "+", "café",
    "3q2+7wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    "deadbeef00000000000000000000000000000000000000000000000000000000" ]

/-- info: true -/
#guard_msgs in
#eval
  let nats := identityTexts.map identToNat
  (nats.eraseDups).length == identityTexts.length

-- Text in, the same text out: a trace this package re-renders is the one it read.
/-- info: true -/
#guard_msgs in
#eval identityTexts.all fun t => natToIdent (identToNat t) == some t

-- A number naming no string still survives, going out as a number instead.
/-- info: true -/
#guard_msgs in
#eval
  let n := 2 ^ 200
  match natToIdent n with
  | some s => identToNat s == n
  | none => match cryptoFromJson (cryptoToJson n) with
    | .ok m => m == n
    | .error _ => false

/-! ## Round trip -/

private def prop1 : Proposal :=
  ⟨⟨⟨42⟩⟩, ⟨1⟩, ⟨⟨⟨1⟩⟩, ViewNumber.genesis⟩, none, ⟨3⟩⟩

private def prop2 : Proposal :=
  ⟨⟨⟨8⟩⟩, ⟨5⟩, ⟨⟨⟨3⟩⟩, ⟨1⟩⟩, some ⟨(), ⟨4⟩⟩, ⟨9⟩⟩

/-- Every input constructor, with `timeoutEvidence` present and absent. -/
private def inputs : List Input :=
  [ .blockReconstructed ⟨1⟩ ⟨42⟩,
    .certificate1 ⟨⟨⟨3⟩⟩, ⟨1⟩⟩,
    .certificate2 ⟨⟨⟨3⟩⟩, ⟨1⟩⟩,
    .advanceView ⟨⟨⟨1⟩⟩, ViewNumber.genesis⟩,
    .headerBuilt ⟨2⟩ ⟨3⟩ ⟨⟨7⟩⟩,
    .proposal ⟨1⟩ prop1 ⟨⟨1⟩, ⟨42⟩⟩,
    .proposal ⟨2⟩ prop2 ⟨⟨5⟩, ⟨8⟩⟩,
    .blockValidated ⟨1⟩ ⟨3⟩,
    .timeout ⟨4⟩,
    .timeoutCertificate ⟨(), ⟨4⟩⟩,
    .timeoutOneHonest ⟨6⟩ ]

/-- Every output constructor, including all three shapes of catchup evidence. -/
private def outputs : List Output :=
  [ .send (.proposal prop1),
    .send (.vote1 ⟨⟨⟨3⟩⟩, ⟨1⟩, ⟨1⟩⟩),
    .send (.vote2 ⟨⟨⟨3⟩⟩, ⟨1⟩, ⟨1⟩⟩),
    .send (.timeoutVote ⟨(), ⟨4⟩, ⟨1⟩⟩ none),
    .send (.timeoutVote ⟨(), ⟨4⟩, ⟨1⟩⟩ (some (.cert1 ⟨⟨⟨3⟩⟩, ⟨1⟩⟩))),
    .send (.timeoutVote ⟨(), ⟨4⟩, ⟨1⟩⟩ (some (.timeout ⟨(), ⟨3⟩⟩))),
    .send (.timeoutCert ⟨(), ⟨4⟩⟩ ⟨5⟩),
    .send (.cert1 ⟨⟨⟨3⟩⟩, ⟨1⟩⟩),
    .send (.cert2 ⟨⟨⟨3⟩⟩, ⟨1⟩⟩),
    .send (.vidShare ⟨⟨1⟩, ⟨42⟩⟩),
    .decided [] ⟨⟨⟨3⟩⟩, ⟨1⟩⟩ ⟨⟨⟨3⟩⟩, ⟨1⟩⟩,
    .decided [prop2, prop1] ⟨⟨⟨9⟩⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩⟩, ⟨5⟩⟩ ]

/-- Through `toJson`, the text, and back — the whole path a trace file takes. -/
private def roundTrips {α : Type} [DecidableEq α] [ToJson α] [FromJson α]
    (xs : List α) : Bool :=
  xs.all fun x =>
    match Json.parse (toJson x).compress >>= (fromJson? · : Json → Except String α) with
    | .ok y => y == x
    | .error _ => false

private def inputRoundTrips : Bool := roundTrips inputs

private def outputRoundTrips : Bool := roundTrips outputs

/-- info: true -/
#guard_msgs in #eval inputRoundTrips

/-- info: true -/
#guard_msgs in #eval outputRoundTrips

/-- A whole trace survives writing and reading, collections and all. -/
private def trace : List Event :=
  [ .consensus (.advanceView ⟨⟨⟨1⟩⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 ⟨⟨1⟩, ⟨42⟩⟩) [.send (.vote1 ⟨⟨⟨3⟩⟩, ⟨1⟩, ⟨1⟩⟩)],
    .collect,
    .consensus (.timeout ⟨4⟩)
      [.send (.timeoutVote ⟨(), ⟨4⟩, ⟨1⟩⟩ none), .send (.cert1 ⟨⟨⟨3⟩⟩, ⟨1⟩⟩)] ]

/-- info: true -/
#guard_msgs in
#eval match parseTrace (renderTrace trace) with
  | .ok es => es == trace
  | .error _ => false

/-- A 32-byte identity survives the JSON path, which a plain number could not. -/
private def wide : Proposal :=
  ⟨⟨⟨2 ^ 250 + 7⟩⟩, ⟨1⟩, ⟨⟨⟨2 ^ 255 - 1⟩⟩, ViewNumber.genesis⟩, none, ⟨2 ^ 248 + 3⟩⟩

/-- info: true -/
#guard_msgs in #eval roundTrips [Input.proposal ⟨2 ^ 200⟩ wide ⟨⟨1⟩, ⟨2 ^ 250 + 7⟩⟩]

-- A hand-written trace may use plain numbers where a recorder writes base64.
/-- info: true -/
#guard_msgs in
#eval match Json.parse "{\"data\":{\"blockHash\":3},\"view\":1}" >>=
    (fromJson? · : Json → Except String Cert1) with
  | .ok c => c == (⟨⟨⟨3⟩⟩, ⟨1⟩⟩ : Cert1)
  | .error _ => false

/-! ## The comparison

The same run three ways: as the machine plays it, missing the validity report
the vote depends on, and with an identity that does not match the block. The
last is the namespace mistake a structural hash would have caused everywhere.
-/

private def anchor : Block :=
  ⟨⟨⟨0⟩⟩, ViewNumber.genesis, ⟨⟨⟨1⟩⟩, ViewNumber.genesis⟩, none, ⟨1⟩⟩

private def cfg : Config := ⟨anchor, ⟨⟨⟨1⟩⟩, ViewNumber.genesis⟩, 20⟩
private def me : PubKey := ⟨1⟩

private def run (es : List Event) : String :=
  (replay cfg (fun _ => some me) me es).report

private def opening : List Event :=
  [ .consensus (.advanceView ⟨⟨⟨1⟩⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 ⟨⟨1⟩, ⟨42⟩⟩) [],
    .consensus (.blockReconstructed ViewNumber.genesis ⟨7⟩) [] ]

private def voteStep (identity : BlockHash) : Event :=
  .consensus (.blockValidated ⟨1⟩ identity)
    [.send (.vote1 ⟨⟨⟨3⟩⟩, ⟨1⟩, me⟩), .send (.vidShare ⟨⟨1⟩, ⟨42⟩⟩)]

/-- info: "OK: 4 steps, marks agree exactly" -/
#guard_msgs in #eval run (opening ++ [voteStep ⟨3⟩])

/-- info: "DIVERGED after 3 steps\n  step 3: the recording voted1 in view 1, the machine did not" -/
#guard_msgs in
#eval run (opening ++ [.consensus (.timeout ⟨9⟩) [.send (.vote1 ⟨⟨⟨3⟩⟩, ⟨1⟩, me⟩)]])

/-- info: "DIVERGED after 3 steps\n  step 3: the recording voted1 in view 1, the machine did not" -/
#guard_msgs in #eval run (opening ++ [voteStep ⟨999⟩])

/-! ### Collecting

A collection prunes the machine and must leave what the run still owes. Nothing
records one — see `NewProtocolDiff.Trace` — so without a trace written here the
`collect` arm of `replay` would be reached by nothing at all, and could rot
unnoticed.
-/

/-- info: "OK: 5 steps, marks agree exactly" -/
#guard_msgs in #eval run (opening ++ [.collect, voteStep ⟨3⟩])

/-! ## A boundary of the model is not a failure

A header whose payload commitment is `null` is a block from before a version
boundary. The parser refuses it, and the refusal has to read as out of scope
rather than as a broken trace: it was the second for a while, which made every
recorded cutover run a red build waiting to happen.

The two lines below are written out rather than encoded from the types, because
neither is a value the types admit — that is what makes them worth testing. Kept
on one line each: `parseTrace` splits on newlines, so a step broken across two
would be malformed for the wrong reason and the first test would pass without
touching the case it names.
-/

/-- A step delivering a proposal whose header carries no payload commitment. -/
private def preCutoverLine : String :=
  r#"{"consensus":{"input":{"proposal":{"sender":1,"p":{"blockHeader":"# ++
  r#"{"payloadCommit":null},"viewNumber":1,"parentCert":{"data":"# ++
  r#"{"blockHash":7},"view":0},"timeoutEvidence":null,"identity":9},"# ++
  r#""vid":{"view":1,"payloadCommit":3}}},"output":[]}}"#

/-- A step that is simply incomplete. -/
private def brokenLine : String :=
  r#"{"consensus":{"input":{"proposal":{"sender":1}}}}"#

/-- How a parse failure classifies: the boundary of the model, or a bad recording. -/
private def classify (line : String) : String :=
  match parseTrace line with
  | .ok _ => "parsed, which it should not"
  | .error e => if (parseOutOfScope e).isSome then "out of scope" else "malformed"

/-- info: "out of scope" -/
#guard_msgs in #eval classify preCutoverLine

/-- info: "malformed" -/
#guard_msgs in #eval classify brokenLine

end Tests
end NewProtocolDiff
