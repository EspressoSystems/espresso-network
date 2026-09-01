module

import NewProtocolDiff

/-!
Replay recorded traces against the reference machine.

    replay <trace-or-directory>... [--quiet]

Each argument is a trace or a directory of `*.jsonl` traces.

Every trace states, on its first line, which node it is, where its chain is
anchored, and how far behind the decided view it kept decide inputs:

    # trace {"node": "…", "anchor": "…", "decideBuffer": 20, "epochHeight": 100}

A recorder writes it. A trace written by hand has to carry one too, where a plain
number will do for the node and the anchor, as it does everywhere else a trace
names a cryptographic value. Nothing here is optional and nothing is assumed: a
run against the wrong node key, anchor or decide buffer reports agreement or
divergence just as confidently as a real one, and the report says nothing about
which it was. The buffer is no more optional than the rest — it places the decide
floor, which gates every decide and the retention of the vote2 marks
(`GcSpec.voted2Retained`).

The exit status is 1 if any trace diverged, could not be read, or could not be
replayed. A trace that runs past what the specification covers is reported and
counted, but does not fail the run; see `NewProtocolDiff.Corpus`. That includes a
trace the parser refuses for such a reason — a header from before a version
boundary carries no payload commitment this protocol accepts, and being unable to
read it is a boundary of the model rather than a broken recording.
-/

open Lean NewProtocol NewProtocolDiff

/-- What the command line asked for. -/
structure ReplayOptions where
  paths : Array System.FilePath := #[]
  quiet : Bool := false

def usage : String :=
  "usage: replay <trace-or-directory>... [--quiet]"

def parseOptions (args : List String) : Except String ReplayOptions :=
  let rec go (o : ReplayOptions) : List String → Except String ReplayOptions
    | [] => pure o
    | "--quiet" :: rest => go { o with quiet := true } rest
    | arg :: rest =>
      if arg.startsWith "--" then throw s!"unknown option {arg}"
      else go { o with paths := o.paths.push arg } rest
  go {} args

/-- The traces an argument names: itself, or every `*.jsonl` in it. -/
def tracesUnder (path : System.FilePath) : IO (Array System.FilePath) := do
  if ← path.isDir then
    let entries ← path.readDir
    let files := entries.filterMap fun e =>
      if e.path.extension == some "jsonl" then some e.path else none
    return files.qsort (·.toString < ·.toString)
  else
    return #[path]

/-- What a recorder said about the run it recorded. -/
structure Preamble where
  node : Nat
  anchor : Nat
  /-- How far behind the decided view the recording kept decide inputs. -/
  decideBuffer : Nat
  /--
  Blocks to an epoch on the network that was recorded.

  Absent from a trace written before the recorder learned to say it, and read as
  zero then, which is the static-committee reading: one epoch, no boundary.
  -/
  epochHeight : Nat

/--
What the trace says about the run it records.

The node and the anchor come in as the recorder wrote them, or as plain numbers if
a person wrote the trace — `cryptoFromJson` takes either, as it does for every
other cryptographic value in a trace.
-/
def readPreamble (text : String) : Except String Preamble := do
  let tag := "# trace "
  let line := (text.splitOn "\n").headD ""
  unless line.startsWith tag do
    throw "no `# trace` header line"
  let json ← (Json.parse (line.drop tag.length).toString).mapError
    fun e => s!"its `# trace` header is not JSON: {e}"
  let field (name : String) : Except String Json :=
    (json.getObjVal? name).mapError fun _ => s!"its `# trace` header has no `{name}`"
  let ident (name : String) : Except String Nat := do
    (cryptoFromJson (← field name)).mapError
      fun e => s!"its `# trace` header's `{name}`: {e}"
  let buffer ← (Json.getNat? (← field "decideBuffer")).mapError
    fun e => s!"its `# trace` header's `decideBuffer`: {e}"
  let height := (json.getObjVal? "epochHeight").toOption.bind (Json.getNat? · |>.toOption)
  pure ⟨← ident "node", ← ident "anchor", buffer, height.getD 0⟩

/--
Who leads each view, as the recorder saw it.

Read from the `# leader <view> <key>` lines. Every view has a leader and the model
takes the schedule as a parameter, so a replay that cannot read it has to assume
one; assuming this node leads everywhere is what made the leader clause of
`ProposalJustification` unfalsifiable, and a proposal in a view this node does not
lead replayed as agreement.

A view with no line has no leader here, which is the strict reading: a step
records the leader of its own view, so a proposal always has one, and anything
proposing in a view the trace never mentions is acting on a schedule the trace
does not show. A line reading `unknown` is the same answer said out loud: the
recorder's stake table could not name the leader, which leaves the recording
unable to claim leadership there either.
-/
def readLeaders (text : String) : Std.TreeMap ViewNumber PubKey :=
  (text.splitOn "\n").foldl (init := {}) fun leaders line =>
    let tag := "# leader "
    if !line.startsWith tag then leaders
    else
      match (line.drop tag.length).toString.trimAscii.toString.splitOn " " with
      | [v, key] =>
        match v.toNat? with
        | some v =>
          if key == "unknown" then leaders
          else
            match cryptoFromJson (Json.str (key.replace "\"" "")) with
            | .ok k => leaders.insert ⟨v⟩ ⟨k⟩
            | .error _ => leaders
        | none => leaders
      | _ => leaders

/-- Replay one trace and say what came of it. -/
def replayOne (path : System.FilePath) : IO (Verdict × String × Nat × Nat) := do
  let text ← IO.FS.readFile path
  let steps := text.splitOn "\n" |>.filter fun l => !l.trimAscii.isEmpty && !l.startsWith "#"
  if steps.isEmpty then
    return (.empty, "no step recorded", 0, 0)
  let .ok said := readPreamble text |
    let .error e := readPreamble text | unreachable!
    return (.unreplayable, e, 0, steps.length)
  match parseTrace text with
  | .error e =>
    match parseOutOfScope e with
    | some reason => return (.outOfScope reason, s!"cannot be read: {reason}", 0, steps.length)
    | none => return (.malformed, Divergence.describe (.malformed e), 0, steps.length)
  | .ok events =>
    -- The anchor sits at genesis, where no rule reads its payload commitment.
    let anchor : Block :=
      ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, epochOf 0 said.epochHeight,
        ⟨⟨⟨said.anchor⟩⟩, ViewNumber.genesis⟩, none, ⟨said.anchor⟩⟩
    let cfg : Config :=
      ⟨anchor, ⟨⟨⟨said.anchor⟩⟩, ViewNumber.genesis⟩, said.decideBuffer, said.epochHeight⟩
    let me : PubKey := ⟨said.node⟩
    let leaders := readLeaders text
    let outcome := replay cfg (leaders.get? ·) me events
    let (verdict, said) := verdictOf text events outcome
    return (verdict, said, outcome.steps, events.length)

public def main (args : List String) : IO UInt32 := do
  match parseOptions args with
  | .error e =>
    IO.eprintln e
    IO.eprintln usage
    return 2
  | .ok opts =>
    if opts.paths.isEmpty then
      IO.eprintln usage
      return 2
    let mut traces := #[]
    for path in opts.paths do
      traces := traces ++ (← tracesUnder path)
    if traces.isEmpty then
      IO.eprintln "no traces found"
      return 2

    let mut tally : List (String × Nat) := []
    let mut failed := false
    let mut stepsChecked := 0
    let mut stepsTotal := 0
    for path in traces do
      let (verdict, said, checked, total) ← replayOne path
      let label := verdict.label
      tally := (label, (tally.lookup label |>.getD 0) + 1) :: tally.filter (·.1 != label)
      if verdict.failed then failed := true
      stepsChecked := stepsChecked + checked
      stepsTotal := stepsTotal + total
      -- An exact agreement is one line and says nothing a tally cannot; an
      -- agreement with the machine still ahead is several, and says where.
      let terse := verdict == .empty || (verdict == .agree && (said.splitOn "\n").length == 1)
      unless opts.quiet || terse do
        IO.println ""
        IO.println s!"{label}: {path}"
        IO.println (said.splitOn "\n" |>.map ("  " ++ ·) |> String.intercalate "\n")

    IO.println ""
    IO.println s!"replayed {traces.size} traces"
    IO.println s!"  steps checked: {stepsChecked} of {stepsTotal}"
    for label in ["agree", "out-of-scope", "diverge", "malformed", "unreplayable", "empty"] do
      if let some n := tally.lookup label then
        IO.println s!"  {label}: {n}"
    return if failed then 1 else 0
