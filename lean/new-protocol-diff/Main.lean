module

import NewProtocolDiff

/-!
Replay recorded traces against the reference machine.

    replay <trace-or-directory>... [--quiet]

Each argument is a trace or a directory of `*.jsonl` traces.

Every trace states, on its first line, which node it is, where its chain is
anchored, and how far behind the decided view it kept decide inputs:

    # trace {"node": "…", "anchor": "…", "decideBuffer": 20}

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
counted, but does not fail the run; see `NewProtocolDiff.Corpus`.
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
  pure ⟨← ident "node", ← ident "anchor", buffer⟩

/-- Replay one trace and say what came of it. -/
def replayOne (path : System.FilePath) : IO (Verdict × String) := do
  let text ← IO.FS.readFile path
  let steps := text.splitOn "\n" |>.filter fun l => !l.trimAscii.isEmpty && !l.startsWith "#"
  if steps.isEmpty then
    return (.empty, "no step recorded")
  let .ok said := readPreamble text |
    let .error e := readPreamble text | unreachable!
    return (.unreplayable, e)
  match parseTrace text with
  | .error e => return (.malformed, Divergence.describe (.malformed e))
  | .ok events =>
    -- The anchor sits at genesis, where no rule reads its payload commitment.
    let anchor : Block := ⟨⟨⟨0⟩⟩, ViewNumber.genesis, ⟨⟨⟨said.anchor⟩⟩, ViewNumber.genesis⟩,
      none, ⟨said.anchor⟩⟩
    let cfg : Config :=
      ⟨anchor, ⟨⟨⟨said.anchor⟩⟩, ViewNumber.genesis⟩, said.decideBuffer⟩
    let me : PubKey := ⟨said.node⟩
    return verdictOf text events (replay cfg (fun _ => some me) me events)

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
    for path in traces do
      let (verdict, said) ← replayOne path
      let label := verdict.label
      tally := (label, (tally.lookup label |>.getD 0) + 1) :: tally.filter (·.1 != label)
      if verdict.failed then failed := true
      -- An exact agreement is one line and says nothing a tally cannot; an
      -- agreement with the machine still ahead is several, and says where.
      let terse := verdict == .empty || (verdict == .agree && (said.splitOn "\n").length == 1)
      unless opts.quiet || terse do
        IO.println ""
        IO.println s!"{label}: {path}"
        IO.println (said.splitOn "\n" |>.map ("  " ++ ·) |> String.intercalate "\n")

    IO.println ""
    IO.println s!"replayed {traces.size} traces"
    for label in ["agree", "out-of-scope", "diverge", "malformed", "unreplayable", "empty"] do
      if let some n := tally.lookup label then
        IO.println s!"  {label}: {n}"
    return if failed then 1 else 0
