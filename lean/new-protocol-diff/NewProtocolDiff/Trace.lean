module

public import NewProtocolDiff.Json
public import NewProtocolSpec.History

/-!
# The trace format

A recorded run as JSON Lines, one `NewProtocol.Event` per line:

```json
{"consensus":{"input":{"certificate1":{"c":{"data":{"blockHash":1,"epoch":0,"blockNumber":0},"view":0}}},"output":[]}}
{"collect":{}}
```

A trace step is an input with the outputs it drew, or a collection. A consensus
step is exactly a `NewProtocol.Step`, so the history a trace records is its
consensus steps in order (`historyOf`).

A collection is read but never recorded, and that is the stronger choice rather
than a gap. A trace carries inputs and outputs, never state, and the specification
says nothing about what a node keeps, so a `.collect` step has nothing to be
checked against; it would only tell the machine it may forget too. Left out, the
machine keeps its whole history for the replay, and an implementation that forgot
a vote it cast is caught the moment it votes twice.

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

/-- One line of a trace: a consensus step, or a collection. -/
inductive Event where
  /-- The node took `input` and emitted `output`. -/
  | consensus (input : Input) (output : List Output)
  /-- The node pruned its state. -/
  | collect
deriving DecidableEq, Repr, ToJson, FromJson

/-- The history a trace records: its consensus steps, in order. Collections leave no step. -/
def historyOf (events : List Event) : History :=
  events.filterMap fun e => match e with
    | .consensus input output => some (Step.mk input output)
    | .collect => none

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
The outputs a recorder held back for want of a later step, from its
`# N outputs had no later step to ride on: …` note.

The node took these actions after its last recorded step, so they belong to the
history as that step's outputs.
-/
def heldOutputs (text : String) : Except String (List Output) := do
  let tag := "outputs had no later step to ride on: "
  match (text.splitOn "\n").find? fun l => l.startsWith "# " && (l.splitOn tag).length > 1 with
  | none => return []
  | some line =>
    let json ← Json.parse ("[" ++ (line.splitOn tag).getLast! ++ "]")
    (fromJson? json : Except String (List Output))

/-- The events of a trace, with the held outputs appended to the last step. -/
def withHeld (events : List Event) (held : List Output) : List Event :=
  if held.isEmpty then events
  else
    match events.reverse.findIdx? (fun e => match e with | .consensus .. => true | _ => false) with
    | none => events
    | some k =>
      let idx := events.length - 1 - k
      events.mapIdx fun j e => if j = idx then
        (match e with | .consensus i out => .consensus i (out ++ held) | e => e) else e

/-- Write a trace, one step per line. -/
def renderTrace (steps : List Event) : String :=
  String.join (steps.map fun s => (toJson s).compress ++ "\n")

/-- Write a trace indented, for a human reading a divergence rather than a machine. -/
def renderTracePretty (steps : List Event) : String :=
  String.join (steps.map fun s => (toJson s).pretty ++ "\n")

end NewProtocolDiff
