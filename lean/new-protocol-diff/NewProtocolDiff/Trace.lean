module

public import NewProtocolDiff.Json
public import NewProtocolSpec.History

/-!
# The trace format

A recorded run as JSON Lines, one `NewProtocolDiff.Event` per line:

```json
{"consensus":{"input":{"certificate1":{"c":{"data":{"blockHash":1,"epoch":0,"blockNumber":0},"view":0}}},"output":[],"t":12}}
```

A trace step is an input with the outputs it drew, and the time it was taken, in
milliseconds since the recorder started (`readTiming`). Outputs released later,
on an input the specification has no counterpart for, follow the step's own, and
a `released` list gives the time and number of each such release. Without its time, a step is
exactly a `NewProtocol.Step`, so the history a trace records is its steps in order
(`historyOf`).

A trace carries inputs and outputs, never state: the specification says nothing
about what a node keeps, so pruning is not recorded.

One object per *line* rather than one array for the file, because a recorder can
append as it goes and a reader can name the line a bad step is on. Blank lines
are skipped, and so are lines starting with `#`, which carry the recorder's
header, leader and dropped-input notes as well as any comment a person writes.

Every field is named, and the names are the specification's own, so reading a
trace needs nothing but `NewProtocolSpec.Interface` beside it. The one field
worth knowing about is `identity` inside a proposal: it is the identity the
recording implementation assigned that block, and carrying it is what lets the
two sides' hash comparisons mean the same thing. A recorder must emit the
commitment it computed, never a digest of the fields beside it.
-/

@[expose] public section

open Lean

namespace NewProtocolDiff

open NewProtocol

/-- One line of a trace: a consensus step. -/
inductive Event where
  /-- The node took `input` and emitted `output`. -/
  | consensus (input : Input) (output : List Output)
deriving DecidableEq, Repr, ToJson, FromJson

/-- The history a trace records: its steps, in order. -/
def historyOf (events : List Event) : History :=
  events.map fun | .consensus input output => Step.mk input output

/-- Whether a line carries no step: blank, or a hand-written comment. -/
def isSkippable (line : String) : Bool :=
  let t := line.trimAscii
  t.isEmpty || t.startsWith "#"

/--
Read a trace.

Errors carry the line number, since the only interesting line in a long trace is
the one that would not parse.
-/
def parseTrace (text : String) : Except String (List Event) := do
  let mut steps : Array Event := #[]
  for (line, n) in text.splitOn "\n" |>.zipIdx do
    unless isSkippable line do
      let json ← (Json.parse line).mapError fun e => s!"line {n + 1}: {e}"
      let step ← (fromJson? json : Except String Event).mapError fun e => s!"line {n + 1}: {e}"
      steps := steps.push step
  return steps.toList

/--
The recorder's `# N outputs had no step to join: …` note, if the trace has one.

A recorder adds the outputs released on a dropped input to the last step written.
Before the first step there is none, and the note names the actions the trace
then has no step for.
-/
def heldNote (text : String) : Option String :=
  (text.splitOn "\n").find? fun l =>
    l.startsWith "# " && (l.splitOn "outputs had no step to join: ").length > 1

/-- A number a recorder wrote in a `# <tag>t=<n>` note, if the trace has one. -/
def notedTime (text : String) (tag : String) : Option Nat :=
  (text.splitOn "\n").findSome? fun l =>
    if l.startsWith ("# " ++ tag ++ "t=") then (l.drop (tag.length + 4)).toString.toNat? else none

/-- When each step of a trace happened, and when the recording stopped. -/
structure Timing where
  /-- Element `n` is when step `n` happened. -/
  steps : Array Nat
  /-- Element `n` is when each later release added to step `n`, and how many outputs. -/
  releases : Array (List (Nat × Nat))
  /-- When the recorder stopped, no earlier than the last step. -/
  ended : Nat

/--
The times a trace records, in milliseconds since its recorder started.

Every step must carry a time, as must the end.
-/
def readTiming (text : String) : Except String Timing := do
  let mut steps : Array Nat := #[]
  let mut releases : Array (List (Nat × Nat)) := #[]
  for (line, n) in text.splitOn "\n" |>.zipIdx do
    unless isSkippable line do
      let json ← (Json.parse line).mapError fun e => s!"line {n + 1}: {e}"
      if let .ok step := json.getObjVal? "consensus" then
        match step.getObjValAs? Nat "t" with
        | .ok t => steps := steps.push t
        | .error _ => throw s!"line {n + 1}: a step without a time"
        let released ← match step.getObjVal? "released" with
          | .error _ => pure []
          | .ok rs => (do
              let rs ← rs.getArr?
              rs.toList.mapM fun (r : Json) => do
                return (← r.getObjValAs? Nat "t", ← r.getObjValAs? Nat "outputs")
            ).mapError fun e => s!"line {n + 1}: released: {e}"
        releases := releases.push released
  let some ended := notedTime text "ended at " | throw "no end time"
  return ⟨steps, releases, ended⟩

/-- Write a trace, one step per line. -/
def renderTrace (steps : List Event) : String :=
  String.join (steps.map fun s => (toJson s).compress ++ "\n")

end NewProtocolDiff
