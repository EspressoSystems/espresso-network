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
<<<<<<< HEAD
trace names; `NewProtocolDiff.checkProtocol_sound` is what makes a pass mean that.
A trace in which a test set the node's view directly is checked against the
signing rules of `NewProtocol.SafeHistory` only (`NewProtocolDiff.checkSafe_sound`),
since its steps do not show the view it is in. The exit status
is 1 if any trace breaks a rule or cannot be read.
||||||| parent of 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))
trace names, and its node discharges what it owes within `δ` milliseconds
(`NewProtocolDiff.PromptUntil`), for a node honest in every epoch, for which
`NewProtocol.OwedIn` is `NewProtocol.Owed`. `NewProtocolDiff.checkProtocol_sound`
and `NewProtocolDiff.checkPrompt_sound` are what make a pass mean that. A delay
an unmodelled input excuses is reported and does not fail the trace, so a pass
with an excuse does not mean `PromptUntil`. A trace in which a test set the node's
view directly is checked against the signing rules of `NewProtocol.SafeHistory`
only (`NewProtocolDiff.checkSafe_sound`), since its steps do not show the view it
is in. The exit status is 1 if any trace breaks a rule or cannot be read.
=======
trace names, and its node discharges what it owes and answers its timers within
`δ` milliseconds (`NewProtocolDiff.PromptUntil`, `NewProtocolDiff.TimersUntil`),
for a node honest in every epoch, for which `NewProtocol.OwedIn` is
`NewProtocol.Owed`, and a member of every committee. `NewProtocolDiff.checkProtocol_sound`,
`NewProtocolDiff.checkPrompt_sound` and `NewProtocolDiff.checkTimers_sound` are
what make a pass mean that. A delay an unmodelled input excuses is reported and
does not fail the trace, so a pass with an excuse does not mean `PromptUntil` or
`TimersUntil`. A trace in which a test set the node's
view directly is checked against the signing rules of `NewProtocol.SafeHistory`
only (`NewProtocolDiff.checkSafe_sound`), since its steps do not show the view it
is in. The exit status is 1 if any trace breaks a rule or cannot be read.
>>>>>>> 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))
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
<<<<<<< HEAD
What the node still owes when its trace ends.
||||||| parent of 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))
Each obligation the node did not discharge within `δ`, once, with the step it was
first owed after and what excuses it, if anything does.
=======
Each obligation or timer the node did not discharge within `δ`, once, with the
step it was first owed after and what excuses it, if anything does.
>>>>>>> 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))

An obligation still owed at the end was owed since some step and never discharged.
`Prompt` allows a delay, and a test may stop a run before one ends, so this is
reported rather than failed: it is where to look for a node that drops work.
-/
<<<<<<< HEAD
def stillOwed (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)
||||||| parent of 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))
def lateness (text : String) (ms : List Moment) (δ : Nat) (faults : List (Nat × Obligation)) :
    List (String × Option String) :=
  let first := faults.foldl (fun acc (k, o) => if acc.any (·.2 == o) then acc else acc ++ [(k, o)]) []
  first.map fun (k, o) =>
    let t := timeAt ms k
    let n := (ms[k]?.map (·.step)).getD 0
    let last := (((ms.drop k).takeWhile (·.time ≤ t + δ)).getLast?.map (·.step)).getD n
    (s!"{describeObligation o} owed in step {n} (t={t} ms)", unmodelledDropped text last)

/--
What the node owes after its last step, in words.

The recording may stop within `δ` of the step that made it owed, and then it is
not late. Reported for a passing trace, as where to look for a node that stops.
-/
def owedAtEnd (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)
=======
def lateness {α : Type} [BEq α] (describe : α → String) (text : String) (ms : List Moment) (δ : Nat)
    (faults : List (Nat × α)) : List (String × Option String) :=
  let first := faults.foldl (fun acc (k, o) => if acc.any (·.2 == o) then acc else acc ++ [(k, o)]) []
  first.map fun (k, o) =>
    let t := timeAt ms k
    let n := (ms[k]?.map (·.step)).getD 0
    let last := (((ms.drop k).takeWhile (·.time ≤ t + δ)).getLast?.map (·.step)).getD n
    (s!"{describe o} owed in step {n} (t={t} ms)", unmodelledDropped text last)

/--
What the node owes after its last step, in words.

The recording may stop within `δ` of the step that made it owed, and then it is
not late. Reported for a passing trace, as where to look for a node that stops.
-/
def owedAtEnd (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)
>>>>>>> 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))
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
    -- Held outputs are actions the node took; dropping them could let a trace pass.
    match heldOutputs text with
    | .error e => return .unreadable s!"held outputs: {e}"
    | .ok held =>
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
<<<<<<< HEAD
        -- A node waiting on something the model does not cover, a DRB result for
        -- one, can owe for that reason alone.
        return .pass (stillOwed (said.configFor h) leader ⟨said.node⟩ h)
          (unmodelledDropped text h.length)
||||||| parent of 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))
      match readTiming text with
      | .error e => return .unreadable s!"step times: {e}"
      | .ok time =>
        if time.steps.size ≠ h.length then
          return .unreadable s!"{time.steps.size} step times for {h.length} steps"
        if let some n := (List.range h.length).find? fun n =>
            ((time.releases.getD n []).map (·.2)).sum > (h[n]?.map (·.output.length)).getD 0 then
          return .unreadable s!"step {n} released more outputs than it has"
        let ms := moments h time.steps time.releases
        let faults := promptFaults (said.configFor h) leader ⟨said.node⟩ ms time.ended δ
        let late := lateness text ms δ faults
        let unexcused := late.filterMap fun (what, excuse) => if excuse.isNone then some what else none
        if !unexcused.isEmpty then return .late unexcused
        return .pass (late.filterMap fun (what, excuse) => excuse.map (s!"{what}, excused by {·}"))
          (owedAtEnd (said.configFor h) leader ⟨said.node⟩ h)

/-- How long a node may take to discharge what it owes, in milliseconds, unless told otherwise. -/
def defaultDelta : Nat := 100

def usage : String := "usage: check [--delta-ms <δ>] <trace-or-directory>..."
=======
      match readTiming text with
      | .error e => return .unreadable s!"step times: {e}"
      | .ok time =>
        if time.steps.size ≠ h.length then
          return .unreadable s!"{time.steps.size} step times for {h.length} steps"
        if let some n := (List.range h.length).find? fun n =>
            ((time.releases.getD n []).map (·.2)).sum > (h[n]?.map (·.output.length)).getD 0 then
          return .unreadable s!"step {n} released more outputs than it has"
        let ms := moments h time.steps time.releases
        let faults := promptFaults (said.configFor h) leader ⟨said.node⟩ ms time.ended δ
        let timers := timerFaults (said.configFor h) ⟨said.node⟩ ms time.ended δ
        let late := lateness describeObligation text ms δ faults
          ++ lateness (fun v => s!"timeout vote for view {v.toNat}") text ms δ timers
        let unexcused := late.filterMap fun (what, excuse) => if excuse.isNone then some what else none
        if !unexcused.isEmpty then return .late unexcused
        return .pass (late.filterMap fun (what, excuse) => excuse.map (s!"{what}, excused by {·}"))
          (owedAtEnd (said.configFor h) leader ⟨said.node⟩ h)

/-- How long a node may take to discharge what it owes, in milliseconds, unless told otherwise. -/
def defaultDelta : Nat := 100

def usage : String := "usage: check [--delta-ms <δ>] <trace-or-directory>..."
>>>>>>> 4ad03c1cb (Lean spec: timers answered within δ, vote2 rules for epochs left (#5308))

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
