module

import NewProtocolDiff.Header
import NewProtocolDiff.Trace
import NewProtocolDiff.Check
import NewProtocolDiff.ProtocolCheck
import NewProtocolImpl.Machine
import NewProtocolDiff.Corpus

/-!
Check recorded traces against the rules of the specification.

    check <trace-or-directory>...

Each argument is a trace or a directory of `*.jsonl` traces. A trace passes when
its history obeys every rule of `NewProtocol.ProtocolHistory`, for the leaders the
trace names; `NewProtocolDiff.checkProtocol_sound` is what makes a pass mean that.
A trace in which a test set the node's view directly is checked against the
signing rules of `NewProtocol.SafeHistory` only (`NewProtocolDiff.checkSafe_sound`),
since its steps do not show the view it is in. The exit status
is 1 if any trace breaks a rule or cannot be read.
-/

open Lean NewProtocol NewProtocolDiff

/-- What checking one trace found. -/
inductive Finding where
  | pass (stillOwed : List String) (excuse : Option String)
  | broken (rules : List String)
  | protocol (step : Nat) (why : String)
  | signingOnly (why : String)
  | outOfScope (why : String)
  | unreadable (why : String)

/--
A `# placed` or `# forced` line, which the recorder writes when a test sets the
node's view and epoch, or replaces a proposal it holds, directly. What the rules
derive from the steps is then not the node's, so only the signing rules are
checked: they read only what was received and sent, which the test did not
change.
-/
def placement (text : String) : Option String :=
  (text.splitOn "\n").find? (fun l => l.startsWith "# placed " || l.startsWith "# forced ")
    |>.map (·.drop 2 |>.toString)

/-- An obligation, in words. -/
def describeObligation : Obligation → String
  | .vote1 p => s!"vote1 in view {p.viewNumber.toNat}"
  | .vote1Again r => s!"vote1 again in view {r.view.toNat}"
  | .vote2 c => s!"vote2 in view {c.view.toNat}"
  | .decide c => s!"decide of view {c.view.toNat}"
  | .propose e v => s!"proposal of epoch {e.toNat} for view {v.toNat}"

/--
What the node still owes when its trace ends.

An obligation still owed at the end was owed since some step and never discharged.
`Prompt` allows a delay, and a test may stop a run before one ends, so this is
reported rather than failed: it is where to look for a node that drops work.
-/
def stillOwed (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)
    (h : History) : List String :=
  ((NewProtocolImpl.candidates cfg h).filter fun o => decide (Owed cfg leader node h o)).map
    describeObligation |>.eraseDups

/-- Check one trace. -/
def checkOne (path : System.FilePath) : IO Finding := do
  let text ← IO.FS.readFile path
  match readPreamble text with
  | .error e => return .unreadable e
  | .ok said =>
    if said.anchorView ≠ 0 then
      return .outOfScope s!"restored from view {said.anchorView}, and restarts are not covered"
    else if signsNoLock text then
      return .outOfScope "its timeouts sign no lock, as before the certificate rule"
    else
    match parseTrace text with
    | .error e => return .unreadable e
    | .ok events =>
      let held := (heldOutputs text).toOption.getD []
      let h := historyOf (withHeld events held)
      let broken := (checkRules (said.configFor h) ⟨said.node⟩ h).filterMap fun (name, ok) =>
        if ok then none else some name
      if !broken.isEmpty then return .broken broken
      if let some line := placement text then
        return .signingOnly s!"the test set the node's state directly ({line})"
      let leader := leaderIn (readLeaders text)
      match protocolFault (said.configFor h) h ⟨said.node⟩ leader with
      | some (n, why) => return .protocol n why
      | none =>
        -- A node waiting on something the model does not cover, a DRB result for
        -- one, can owe for that reason alone.
        return .pass (stillOwed (said.configFor h) leader ⟨said.node⟩ h)
          (unmodelledDropped text h.length)

public def main (args : List String) : IO UInt32 := do
  if args.isEmpty then
    IO.eprintln "usage: check <trace-or-directory>..."
    return 2
  let mut traces := #[]
  for arg in args do
    traces := traces ++ (← tracesUnder arg)
  let mut failed := false
  let mut passed := 0
  let mut skipped := 0
  let mut signingOnly := 0
  let mut owing := 0
  for path in traces do
    match ← checkOne path with
    | .unreadable e =>
      failed := true
      IO.println s!"unreadable: {path}: {e}"
    | .pass owed excuse =>
      passed := passed + 1
      unless owed.isEmpty do
        match excuse with
        | some reason =>
          IO.println s!"still owes {String.intercalate ", " owed}, excused by {reason}: {path}"
        | none =>
          owing := owing + 1
          IO.println s!"still owes {String.intercalate ", " owed}: {path}"
    | .outOfScope why =>
      skipped := skipped + 1
      IO.println s!"out of scope: {path}: {why}"
    | .broken rules =>
      failed := true
      IO.println s!"breaks {String.intercalate ", " rules}: {path}"
    | .signingOnly why =>
      signingOnly := signingOnly + 1
      IO.println s!"protocol rules not checked: {path}: {why}"
    | .protocol n why =>
      failed := true
      IO.println s!"breaks {why} (step {n}): {path}"
  IO.println s!"checked {traces.size} traces: {passed} obey every rule, \
    {signingOnly} every signing rule, {skipped} out of scope; \
    {owing} of those passing still owe something at the end, not counting what an \
    unmodelled input excuses"
  return if failed then 1 else 0
