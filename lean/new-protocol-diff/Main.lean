module

import NewProtocolDiff

/-!
Replay recorded traces against the reference machine.

    replay <trace-or-directory>... [--quiet]

Each argument is a trace or a directory of `*.jsonl` traces.

Every trace starts with the header `NewProtocolDiff.Header` reads. A trace
restored from storage, or one in which a test set the node's view directly, is
out of scope: its steps do not show the state the node started from.

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

/-- Replay one trace and say what came of it. -/
def replayOne (path : System.FilePath) : IO (Verdict × String × Nat × Nat) := do
  let text ← IO.FS.readFile path
  let steps := text.splitOn "\n" |>.filter fun l => !l.trimAscii.isEmpty && !l.startsWith "#"
  if steps.isEmpty then
    return (.empty, "no step recorded", 0, 0)
  match readPreamble text with
  | .error e => return (.unreplayable, e, 0, steps.length)
  | .ok said =>
  if said.anchorView ≠ 0 then
    return (.outOfScope "restart", s!"restored from view {said.anchorView}, and restarts are not covered",
      0, steps.length)
  if signsNoLock text then
    return (.outOfScope "unsigned locks",
      "its timeouts sign no lock, as before the certificate rule", 0, steps.length)
  if let some line := (text.splitOn "\n").find?
      (fun l => l.startsWith "# placed " || l.startsWith "# forced ") then
    return (.outOfScope "placed", s!"the test set the node's state directly ({(line.drop 2).toString})",
      0, steps.length)
  match parseTrace text with
  | .error e =>
    match parseOutOfScope e with
    | some reason => return (.outOfScope reason, s!"cannot be read: {reason}", 0, steps.length)
    | none => return (.malformed, Divergence.describe (.malformed e), 0, steps.length)
  | .ok events =>
  -- Held outputs are actions the node took; dropping them could let a trace pass.
  match heldOutputs text with
  | .error e => return (.malformed, Divergence.describe (.malformed s!"held outputs: {e}"), 0, steps.length)
  | .ok held =>
    let events := withHeld events held
    let leader := leaderIn (readLeaders text)
    let h := historyOf events
    let cfg := said.configFor h
    let node : PubKey := ⟨said.node⟩
    let obeys := checkSafe cfg node h && (protocolFault cfg h node leader).isNone
    let outcome := replay cfg leader node events
    let (verdict, said) := verdictOf text events outcome obeys
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
    for label in ["agree", "differ", "out-of-scope", "diverge", "malformed", "unreplayable", "empty"] do
      if let some n := tally.lookup label then
        IO.println s!"  {label}: {n}"
    return if failed then 1 else 0
