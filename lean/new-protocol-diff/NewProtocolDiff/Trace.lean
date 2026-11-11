module

public import NewProtocolDiff.Json
public import NewProtocolSpec.History

/-!
# The trace format

A recorded run as JSON Lines, one `NewProtocolDiff.Event` per line:

```json
{"consensus":{"input":{"certificate1":{"c":{"data":{"blockHash":1,"epoch":0,"blockNumber":0},"view":0}}},"output":[],"notes":{"joined":false,"held":[],"drbRequested":[],"stateCert":false}}}
{"dropped":{"kind":{"stored":{"w":{"action":{"v":1,"kind":"vote"}}}},"output":[],"notes":{"joined":false,"held":[],"drbRequested":[],"stateCert":false}}}
```

A step is an input with the outputs it drew: exactly a `NewProtocol.Step`. A
dropped input is one the specification has no counterpart for, with the outputs
the node emitted on it. The specification saw nothing between a step and a
dropped input after it, so the history a trace records is its steps in order,
each with the outputs of the dropped inputs that follow it (`historyOf`).

Each line also says what the implementation did there that the outputs do not
show (`Notes`): the actions it held back until storage confirms a write, with the
writes each waits for, the DRB results it asked for, whether a step was taken
with the next, and whether a state certificate arrived.

A trace carries inputs and outputs, never state: the specification says nothing
about what a node keeps, so pruning is not recorded.

One object per *line* rather than one array for the file, because a recorder can
append as it goes and a reader can name the line a bad step is on. Blank lines
are skipped, and so are lines starting with `#`, which carry the recorder's
header and leader notes as well as any comment a person writes.

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

/-- What an implementation records an action in storage as, before it sends it. -/
inductive ActionKind where
  | vote
  | propose
  | timeout
deriving DecidableEq, Repr, ToJson, FromJson

/-- A write an implementation asks its storage for, and storage confirms. -/
inductive Write where
  /-- The action of `kind` in view `v`. -/
  | action (v : ViewNumber) (kind : ActionKind)
  /-- The proposal of the block `c` for view `v`. -/
  | proposal (v : ViewNumber) (c : BlockHash)
  /-- The node's VID share for view `v`. -/
  | share (v : ViewNumber)
  /-- A lock on the block certified at view `v` in epoch `e`, or a later one. -/
  | lock (v : ViewNumber) (e : EpochNumber)
deriving DecidableEq, Repr, ToJson, FromJson

/-- What an action held back for storage is. -/
inductive HeldKind where
  | vote1
  | vote2
  /-- A proposal, or a re-vote request. -/
  | proposal
  | timeout
deriving DecidableEq, Repr, ToJson, FromJson

/--
An action the implementation decided on and holds back until storage confirms the
writes in `waits`: where it decided on an output it releases later.
-/
structure Held where
  kind : HeldKind
  view : ViewNumber
  epoch : EpochNumber
  waits : List Write
deriving DecidableEq, Repr, ToJson, FromJson

/-- An input the specification has no counterpart for. -/
inductive Dropped where
  /-- Storage confirmed the write `w`. -/
  | stored (w : Write)
  /-- The DRB result of `epoch`, which names its leaders. -/
  | drbResult (epoch : EpochNumber)
  /-- The state of the block `h` at view `v` was found invalid. -/
  | stateValidationFailed (v : ViewNumber) (h : BlockHash)
  /-- The block the node built to propose in view `v` is ready. -/
  | blockBuilt (v : ViewNumber)
  /-- Any other, by the implementation's name for it. -/
  | other (name : String)
deriving DecidableEq, Repr, ToJson, FromJson

/--
A request for the DRB result of `epoch`, with the view and number of the block
the implementation wanted it for, if it made the request for one: a proposal it
received, or a header it built to propose.
-/
structure DrbRequest where
  epoch : EpochNumber
  view : Option ViewNumber
  block : Option BlockNumber
deriving DecidableEq, Repr, ToJson, FromJson

/-- What a line says of the implementation beyond its input and outputs. -/
structure Notes where
  /-- Whether the step was taken with the next: a certificate inside its message. -/
  joined : Bool := false
  /-- The actions held back here. -/
  held : List Held := []
  /-- The DRB results the implementation asked for here. -/
  drbRequested : List DrbRequest := []
  /-- Whether a light-client state certificate the node did not hold arrived here. -/
  stateCert : Bool := false
deriving DecidableEq, Repr, ToJson, FromJson

/-- One line of a trace. -/
inductive Event where
  /-- The node took `input` and emitted `output`. -/
  | consensus (input : Input) (output : List Output) (notes : Notes := {})
  /--
  The node took an input the specification has no counterpart for, and emitted
  `output`. The specification sees `output` as part of the step before.
  -/
  | dropped (kind : Dropped) (output : List Output) (notes : Notes := {})
deriving DecidableEq, Repr, ToJson

/--
Read by the kind the line names, so that an error is the one of that kind: the
parser's mark of a block outside the model (`outOfScopeMark`) has to reach the
caller, and trying each kind in turn would report the last one's failure.
-/
instance : FromJson Event where
  fromJson? j :=
    match j.getObjVal? "consensus", j.getObjVal? "dropped" with
    | .ok body, _ => do
      return .consensus (← body.getObjValAs? Input "input")
        (← body.getObjValAs? (List Output) "output") (← notes body)
    | _, .ok body => do
      return .dropped (← body.getObjValAs? Dropped "kind")
        (← body.getObjValAs? (List Output) "output") (← notes body)
    | _, _ => .error "neither a consensus step nor a dropped input"
where
  /-- The notes of a line, which a line written by hand may leave out. -/
  notes (body : Json) : Except String Notes :=
    match body.getObjVal? "notes" with
    | .ok j => fromJson? j
    | .error _ => pure {}

/--
The history a trace records: its steps, in order. What a dropped input drew joins
the step before it: the specification saw nothing between the two, so that step's
inputs are all that justify it. Before the first step there is none to join
(`unjoined`).
-/
def historyOf (events : List Event) : History :=
  (events.foldl (init := #[]) fun (h : Array Step) e =>
    match e with
    | .consensus input output _ => h.push ⟨input, output⟩
    | .dropped _ output _ =>
      match h.back? with
      | some st => h.pop.push ⟨st.input, st.output ++ output⟩
      | none => h).toList

/--
Whether a trace has actions no step gave rise to: outputs of dropped inputs
before the first step, or actions held back there.
-/
def unjoined (events : List Event) : Bool :=
  (events.takeWhile fun | .dropped .. => true | .consensus .. => false).any fun
    | .dropped _ output notes => !output.isEmpty || !notes.held.isEmpty
    | .consensus .. => false

/--
Each line of a trace with the step of the history it belongs to: a step's own
index, and for a dropped input the index of the step before it, which its outputs
join. A dropped input before the first step belongs to step 0.
-/
def placed (events : List Event) : List (Nat × Event) :=
  (events.foldl (init := (0, #[])) fun ((steps, acc) : Nat × Array (Nat × Event)) e =>
    match e with
    | .consensus .. => (steps + 1, acc.push (steps, e))
    | .dropped .. => (steps, acc.push (steps - 1, e))).2.toList

/-- A held-back action, with the step of the history and the line of the trace it was held back in. -/
structure HeldAt extends Held where
  step : Nat
  line : Nat
deriving DecidableEq, Repr

/-- A dropped input, with the step of the history its outputs join and its line in the trace. -/
structure DroppedAt where
  kind : Dropped
  output : List Output
  step : Nat
  line : Nat
deriving DecidableEq, Repr

/-- The notes of a line. -/
def Event.notes : Event → Notes
  | .consensus _ _ notes | .dropped _ _ notes => notes

/-- The outputs of a line. -/
def Event.output : Event → List Output
  | .consensus _ output _ | .dropped _ output _ => output

/-- The steps a trace marks as taken with the next one. -/
def joinedSteps (events : List Event) : List Nat :=
  (placed events).filterMap fun
    | (k, .consensus _ _ notes) => if notes.joined then some k else none
    | _ => none

/-- How many dropped inputs a trace has before its first step. They follow no step. -/
def leading (events : List Event) : Nat :=
  (events.takeWhile fun | .dropped .. => true | .consensus .. => false).length

/--
The actions a trace says the node held back for storage, each with where. A
trace with one held back before its first step is not checked (`unjoined`).
-/
def heldBack (events : List Event) : List HeldAt :=
  (placed events).zipIdx.flatMap fun ((k, e), line) =>
    e.notes.held.map fun x => { x with step := k, line }

/--
The dropped inputs of a trace after its first step, each with where. One before
the first step follows no step, and so explains nothing a step gave rise to.
-/
def droppedInputs (events : List Event) : List DroppedAt :=
  ((placed events).zipIdx.drop (leading events)).filterMap fun
    | ((k, .dropped kind output _), line) => some ⟨kind, output, k, line⟩
    | _ => none

/-- The DRB results the node asked for, each with the step and line it asked in. -/
def drbRequests (events : List Event) : List (DrbRequest × Nat × Nat) :=
  (placed events).zipIdx.flatMap fun ((k, e), line) =>
    e.notes.drbRequested.map fun request => (request, k, line)

/-- The steps in which a state certificate the node did not hold arrived. -/
def stateCertSteps (events : List Event) : List Nat :=
  (placed events).filterMap fun
    | (k, .consensus _ _ notes) => if notes.stateCert then some k else none
    | _ => none

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

/-- Write a trace, one line per event. -/
def renderTrace (steps : List Event) : String :=
  String.join (steps.map fun s => (toJson s).compress ++ "\n")

end NewProtocolDiff
