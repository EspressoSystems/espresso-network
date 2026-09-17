module

public import NewProtocolDiff.Json

/-!
# The trace format

A recorded run as JSON Lines, one `NewProtocol.Event` per line:

```json
{"consensus":{"input":{"advanceView":{"c":{"data":{"blockHash":1},"view":0}}},"output":[]}}
{"collect":{}}
```

The specification already has the type a trace step wants — an input with the
outputs it drew, or a collection — so a trace *is* a list of `Event`s and this
module only reads and writes them.

A collection is read but never recorded, and that is the stronger choice rather
than a gap. A trace carries inputs and outputs, never state, so a `.collect` step
could not be checked against `GcSpec` anyway — it would only tell the machine it
may prune too. Left out, the machine keeps every mark for the whole replay, and an
implementation that pruned one it still owed is caught the moment it acts twice.
That is how the duplicate vote2 was found.

One object per *line* rather than one array for the file, because a recorder can
append as it goes and a reader can name the line a bad step is on. Blank lines
are skipped, and so are lines starting with `#` — not JSON, but traces get
written by hand while a recorder is being built, and a comment is worth more
then than strict conformance.

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

/-- Write a trace, one step per line. -/
def renderTrace (steps : List Event) : String :=
  String.join (steps.map fun s => (toJson s).compress ++ "\n")

/-- Write a trace indented, for a human reading a divergence rather than a machine. -/
def renderTracePretty (steps : List Event) : String :=
  String.join (steps.map fun s => (toJson s).pretty ++ "\n")

end NewProtocolDiff
