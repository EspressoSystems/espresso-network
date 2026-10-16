module

public import NewProtocolDiff.Trace
public import NewProtocolDiff.Corpus
public import NewProtocolDiff.PromptCheck
public import NewProtocolDiff.Check
public meta import NewProtocolSpec.Interface
public meta import NewProtocolDiff.Json
public meta import NewProtocolDiff.Trace
public meta import NewProtocolDiff.Corpus
public meta import NewProtocolDiff.PromptCheck
public meta import NewProtocolDiff.Check

/-!
# Checks on the harness itself

A comparison harness that silently mis-reads its input is worse than none, so
the format is checked at build time in both directions:

* the identity injection does not collide and preserves the recorder's text, and
  every constructor round-trips through JSON — which catches an instance the
  deriving got wrong as well as one a future hand-written override breaks;
* the step times are read as written, and the `Prompt` check passes a node that
  votes, and answers its timers, in time and fails one that does not;
* the signing rules refuse a vote whose identity does not match the block, since
  that is how a hash-namespace mistake would show up.
-/

@[expose] public section

namespace NewProtocolDiff
namespace Tests

open Lean NewProtocol

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

/-- A lock for the timeout votes and certificates below. -/
private def lk : Cert1 := ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩

private def prop1 : Proposal :=
  ⟨⟨⟨42⟩, 1⟩, ⟨1⟩, ⟨0⟩, ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨3⟩⟩

private def prop2 : Proposal :=
  ⟨⟨⟨8⟩, 2⟩, ⟨5⟩, ⟨0⟩, ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩, some ⟨⟨⟨0⟩, lk⟩, ⟨4⟩⟩, ⟨9⟩⟩

/-- Every input constructor, with `timeoutEvidence` present and absent. -/
private def inputs : List Input :=
  [ .blockReconstructed ⟨1⟩ ⟨42⟩,
    .certificate1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩,
    .certificate2 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩,
    .headerBuilt ⟨2⟩ ⟨3⟩ ⟨⟨7⟩, 2⟩,
    .proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩),
    .proposal ⟨2⟩ prop2 (some ⟨⟨5⟩, ⟨8⟩⟩),
    .blockValidated ⟨1⟩ ⟨3⟩,
    .timeout ⟨4⟩,
    .timeoutCertificate ⟨⟨⟨0⟩, lk⟩, ⟨4⟩⟩,
    .timeoutOneHonest ⟨6⟩,
    .epochChange ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ prop2,
    .revote ⟨1⟩ ⟨⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩, ⟨6⟩, none⟩,
    .revote ⟨1⟩ ⟨⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩, ⟨7⟩, some ⟨⟨⟨1⟩, lk⟩, ⟨6⟩⟩⟩ ]

/-- Every output constructor. -/
private def outputs : List Output :=
  [ .send (.proposal prop1),
    .send (.vote1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩, ⟨1⟩⟩),
    .send (.vote2 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩, ⟨1⟩⟩),
    .send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨4⟩, ⟨1⟩⟩),
    .send (.timeoutCert ⟨⟨⟨0⟩, lk⟩, ⟨4⟩⟩),
    .send (.cert1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩),
    .send (.cert2 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩),
    .send (.vidShare ⟨⟨1⟩, ⟨42⟩⟩),
    .send (.epochChange ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ prop2),
    .send (.revote ⟨⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩, ⟨7⟩, some ⟨⟨⟨1⟩, lk⟩, ⟨6⟩⟩⟩),
    .decided [] ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩ ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩,
    .decided [prop2, prop1] ⟨⟨⟨9⟩, ⟨0⟩, 2⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩, ⟨0⟩, 2⟩, ⟨5⟩⟩ ]

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

/-- A whole trace survives writing and reading. -/
private def trace : List Event :=
  [ .consensus (.certificate1 ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩)) [.send (.vote1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩, ⟨1⟩⟩)],
    .consensus (.timeout ⟨4⟩)
      [.send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨4⟩, ⟨1⟩⟩), .send (.cert1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩)] ]

/-- info: true -/
#guard_msgs in
#eval match parseTrace (renderTrace trace) with
  | .ok es => es == trace
  | .error _ => false

/-- A 32-byte identity survives the JSON path, which a plain number could not. -/
private def wide : Proposal :=
  ⟨⟨⟨2 ^ 250 + 7⟩, 1⟩, ⟨1⟩, ⟨0⟩, ⟨⟨⟨2 ^ 255 - 1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨2 ^ 248 + 3⟩⟩

/-- info: true -/
#guard_msgs in #eval roundTrips [Input.proposal ⟨2 ^ 200⟩ wide (some ⟨⟨1⟩, ⟨2 ^ 250 + 7⟩⟩)]

-- A hand-written trace may use plain numbers where a recorder writes base64.
/-- info: true -/
#guard_msgs in
#eval match Json.parse "{\"data\":{\"blockHash\":3,\"epoch\":0,\"blockNumber\":1},\"view\":1}" >>=
    (fromJson? · : Json → Except String Cert1) with
  | .ok c => c == (⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩ : Cert1)
  | .error _ => false

/-! ## Acting promptly

A run that owes a vote1 once the block is validated, voting in the same step, later
on a release, or not at all. A vote with an identity that does not match the block ends the obligation
too, since a node votes once per view, so the signing rules are what refuse it: the
namespace mistake a structural hash would have caused everywhere.
-/

private def anchor : Block :=
  ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, ⟨0⟩, ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨1⟩⟩

private def cfg : Config := ⟨anchor, ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, 20, 0⟩
private def me : PubKey := ⟨1⟩

private def opening : List Event :=
  [ .consensus (.certificate1 ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩)) [],
    .consensus (.blockReconstructed ViewNumber.genesis ⟨7⟩) [] ]

private def voteStep (votes : List Output) : Event :=
  .consensus (.blockValidated ⟨1⟩ ⟨3⟩) votes

private def vote (identity : BlockHash) : Output := .send (.vote1 ⟨⟨identity, ⟨0⟩, 1⟩, ⟨1⟩, me⟩)

/--
What a run still owes `δ` after it, with step `n` at time `n`, the last step's
outputs released at `released`, and the recording ending at `ended`.
-/
private def late (es : List Event) (released : List (Nat × Nat)) (ended δ : Nat) : List String :=
  let h := historyOf es
  let ms := moments h (List.range h.length).toArray
    ((List.replicate (h.length - 1) []).toArray.push released)
  (promptFaults cfg (fun _ _ => some me) me ms ended δ).map fun (k, o) =>
    match o, ms[k]? with
    | .vote1 p, some m => s!"vote1 in view {p.viewNumber.toNat} at t={m.time}"
    | _, _ => "another obligation"

/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep [vote ⟨3⟩]]) [] 100 5

/-- info: ["vote1 in view 1 at t=3"] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [] 100 5

-- A vote released after the step is cast when it is released, not when the step was taken.
/-- info: ["vote1 in view 1 at t=3"] -/
#guard_msgs in #eval late (opening ++ [voteStep [vote ⟨3⟩]]) [(50, 1)] 100 5

/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep [vote ⟨3⟩]]) [(50, 1)] 100 60

/-- info: true -/
#guard_msgs in #eval checkSafe cfg me (historyOf (opening ++ [voteStep [vote ⟨3⟩]]))

/-- info: false -/
#guard_msgs in #eval checkSafe cfg me (historyOf (opening ++ [voteStep [vote ⟨999⟩]]))

-- A recording that stopped before the deadline could still have voted.
/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [] 8 5

/-! A timer the node takes in view 1, answered in the same step, on a release, in a later step, or not at all. -/

private def timerStep (outs : List Output) : Event := .consensus (.timeout ⟨1⟩) outs

private def timeoutVote : Output := .send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨1⟩, me⟩)

/-- The timers a run has not answered `δ` after them, timed as in `late`. -/
private def unanswered (es : List Event) (released : List (Nat × Nat)) (ended δ : Nat) : List String :=
  let h := historyOf es
  let ms := moments h (List.range h.length).toArray
    ((List.replicate (h.length - 1) []).toArray.push released)
  (timerFaults cfg me ms ended δ).map fun (k, v) =>
    s!"timeout vote for view {v.toNat} at t={(ms[k]?.map (·.time)).getD 0}"

/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [timeoutVote]]) [] 100 5

/-- info: ["timeout vote for view 1 at t=3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep []]) [] 100 5

/-- info: ["timeout vote for view 1 at t=3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [timeoutVote]]) [(50, 1)] 100 5

/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [timeoutVote]]) [(50, 1)] 100 60

-- The vote may come in a later step, as long as it is within `δ`.
/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [], voteStep [timeoutVote]]) [] 100 5

/-- info: ["timeout vote for view 1 at t=3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [], voteStep [timeoutVote]]) [] 100 0

/-- A recorded step carries its time and its releases, and the trace its end. -/
private def timedTrace : String :=
  "# trace {}\n" ++
  r#"{"consensus":{"input":{"timeout":{"v":4}},"output":[],"t":17}}"# ++ "\n" ++
  r#"{"consensus":{"input":{"timeout":{"v":5}},"output":[],"t":40,"released":[{"t":48,"outputs":2}]}}"# ++
  "\n# ended at t=55\n"

/-- info: "ok #[17, 40] #[[], [(48, 2)]] 55" -/
#guard_msgs in
#eval match readTiming timedTrace with
  | .ok t => s!"ok {t.steps} {t.releases} {t.ended}"
  | .error e => e

/-! ## A boundary of the model is not a failure

A header whose payload commitment is `null` is a block from before a version
boundary. The parser refuses it, and the refusal has to read as out of scope
rather than as a broken trace, or every recorded cutover run fails the build.

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
  r#""share":{"view":1,"payloadCommit":3}}},"output":[]}}"#

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

