module

import NewProtocolDiff.Header
import NewProtocolDiff.Trace
import NewProtocolDiff.Check
import NewProtocolDiff.ProtocolCheck
import NewProtocolDiff.PromptCheck
import NewProtocolImpl.Machine
import NewProtocolDiff.Corpus
import NewProtocolDiff.Coverage

/-!
Check recorded traces against the rules of the specification.

    check [--coverage] <trace-or-directory>...

Each argument is a trace or a directory of `*.jsonl` traces. A trace passes when
its history obeys every rule of `NewProtocol.ProtocolHistory`, for the leaders the
trace names, and its node acts on what it owes, and answers its timers, in the
step it owes them in (`NewProtocolDiff.StepPrompt`, `NewProtocolDiff.TimersAnswered`),
for a node honest in every epoch, for which `NewProtocol.OwedIn` is
`NewProtocol.Owed`, and a member of every committee. `NewProtocolDiff.checkProtocol_sound`,
`NewProtocolDiff.checkSteps_sound` and `NewProtocolDiff.checkTimers_sound` are
what make a pass mean that. A step fault an input outside the model excuses
(`waitExcuse`) is reported and does not fail the trace, so a pass with an excuse
does not mean `StepPrompt`. A trace in which a test set the node's
view directly is checked against the signing rules of `NewProtocol.SafeHistory`
only (`NewProtocolDiff.checkSafe_sound`), since its steps do not show the view it
is in. The exit status is 1 if any trace breaks a rule or cannot be read, or if no
trace is checked against any rule: a recorder that stopped writing, or a run in
which every trace is out of scope, must not pass for one that conforms.

With `--coverage`, the traces checked against every rule are also read for what
they exercise (`NewProtocolDiff.Coverage`), and the report follows the verdicts.
-/

open Lean NewProtocol NewProtocolDiff

/-- What checking one trace found. -/
inductive Finding where
  | pass (excused : List String) (owedAtEnd : List String)
  | late (owed : List String)
  | broken (rules : List String)
  | protocol (step : Nat) (why : String)
  | signingOnly (why : String)
  | outOfScope (why : String)
  | unreadable (why : String)

/--
A `# placed` or `# forced` line, which the recorder writes when a test sets the
node's view and epoch, replaces a proposal it holds, restores its lock, or
resumes it as after a restart, directly. What the rules derive from the steps is
then not the node's, so only the signing rules are checked: they read only what
was received and sent, which the test did not change.
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

/-- What a trace says beyond its steps, each with where. -/
structure Annotations where
  joined : List Nat
  held : List HeldAt
  dropped : List DroppedAt
  drbRequests : List (DrbRequest × Nat × Nat)
  stateCerts : List Nat

/-- What `events` say beyond their steps. -/
def annotate (events : List Event) : Annotations :=
  ⟨joinedSteps events, heldBack events, droppedInputs events, drbRequests events,
    stateCertSteps events⟩

/--
What excuses an obligation `o`, given how the node stopped owing it (`ending`), if
it did, if anything.

A vote1 or a proposal can wait for an input the model does not have: the DRB that
names a leader, or the outcome of state validation. The DRB result excuses an
action the node took when it arrived (`drbArrivedInto`), or one the node gave up
or never took while it had asked for the result and not got it (`drbAwaited`): by
the step it gave the action up in, or by the end of the run. A
failed state validation of its block excuses a vote1 the node did not cast
(`stateValidationFailedFor`). A leader proposing on an epoch root waits for its
state certificate, which arrives with a proposal taken together with the step
the leader acted in. A vote2 can wait for the node's own share of its block,
which excuses a vote2 the node cast once the share arrived (`shareArrived`), and
which an equivocating leader can withhold for good (`lacksShare`).
-/
def waitExcuse (cfg : Config) (ann : Annotations) (h : History) (ending : Option Ending) :
    Obligation → Option String
  | o@(.vote1 _) | o@(.vote1Again _) => unmodelled o
  | o@(.propose ..) => (unmodelled o).orElse fun _ =>
      match ending with
      | some (.acted m) =>
        if ann.stateCerts.any fun k => groupStart ann.joined m m ≤ k && k ≤ m
        then some "light-client certification" else none
      | _ => none
  | o@(.vote2 _) =>
    let arrived := match ending with
      | some (.acted m) => shareArrived h ann.joined m o
      | _ => false
    if arrived then some "storing its own share, for restarts"
    else if lacksShare cfg h last o then some "its own share of the block, which it lacks"
    else none
  | .decide _ => none
where
  /-- The step after which the node no longer owed the obligation, or the last step. -/
  last : Nat :=
    match ending with
    | some (.acted d) | some (.lapsed d) => d
    | none => h.length - 1
  /-- The input outside the model the node waited for, if any. -/
  unmodelled (o : Obligation) : Option String :=
    match ending with
    | some (.acted m) =>
      if drbArrivedInto cfg ann.drbRequests ann.dropped o m then some "leader election (the DRB result)" else none
    | _ =>
      if stateValidationFailedFor ann.dropped o last then some "state validation, which failed"
      else if drbAwaited cfg ann.drbRequests ann.dropped o last (if ending.isSome then last else last + 1)
      then some "leader election (the DRB result it asked for)"
      else none

/--
Each obligation the node did not act on in a step it owed it after, once, in
words, with what excuses it, if anything does: when it arose, and when the node
acted on it or stopped owing it, if it did.
-/
def stepLateness (cfg : Config) (node : PubKey) (ann : Annotations) (h : History)
    (owed : Array (List Obligation)) : List (String × Option String) :=
  let faults := (stepFaultsIn owed h.length ann.joined ann.held).map fun (m, o) => (aroseIn owed o m, m, o)
  let first := faults.foldl (init := []) fun acc (n, m, o) =>
    if acc.any fun (n', _, o') => n' == n && o' == o then acc else acc ++ [(n, m, o)]
  first.map fun (n, m, o) =>
    let ending := endingOf node owed h ann.held o m
    let when := match ending with
      | some (.acted d) => s!"acted on in step {d}"
      | some (.lapsed d) => s!"no longer owed after step {d}, not acted on"
      | none => "never acted on"
    (s!"{describeObligation o} owed after step {n}, {when}", waitExcuse cfg ann h ending o)

/--
What the node owes after its last step, in words.

In a passing trace these are held back for storage, or excused. Reported as
where to look for a node that stops.
-/
def owedAtEnd (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)
    (h : History) : List String :=
  ((NewProtocolImpl.candidates cfg h).filter fun o => decide (Owed cfg leader node h o)).map
    describeObligation |>.eraseDups

/-- Check one trace. -/
def checkOne (coverage : Option (IO.Ref Coverage)) (path : System.FilePath) :
    IO Finding := do
  let text ← IO.FS.readFile path
  match readPreamble text with
  | .error e => return .unreadable e
  | .ok said =>
    if said.anchorView ≠ 0 then
      return .outOfScope s!"restored from view {said.anchorView}, and restarts are not covered"
    else if beforeCertificateRule text then
      return .outOfScope "recorded before the certificate rule"
    else if testFailed text then
      return .outOfScope "the test that recorded it failed"
    else
    match parseTrace text with
    | .error e =>
      match parseOutOfScope e with
      | some reason => return .outOfScope s!"cannot be read: {reason}"
      | none => return .unreadable e
    | .ok events =>
    -- Actions the trace has no step for would otherwise go unchecked.
    if unjoined events then
      return .broken ["actions before any step"]
    else
      let h := historyOf events
      let broken := (checkRules (said.configFor h) ⟨said.node⟩ h).filterMap fun (name, ok) =>
        if ok then none else some name
      if !broken.isEmpty then return .broken broken
      if let some line := placement text then
        return .signingOnly s!"the test set the node's state directly ({line})"
      let leader := leaderIn (readLeaders text)
      match protocolFault (said.configFor h) h ⟨said.node⟩ leader with
      | some (n, why) => return .protocol n why
      | none =>
      let cfg := said.configFor h
      let node : PubKey := ⟨said.node⟩
      if let some cov := coverage then
        -- Counted outside the reference, which the other traces' tasks wait on.
        let mine := ({} : Coverage).add cfg leader node h (joinedSteps events) (heldBack events)
        cov.modify (·.merge mine)
      let ann := annotate events
      let owed := owedAfter cfg leader node h
      let late := stepLateness cfg node ann h owed
        ++ (timerFaults cfg node h ann.held).map (fun (n, v) =>
          (s!"timeout vote for view {v.toNat} owed for the timer in step {n}", none))
        ++ (unreleasedIn owed h.length ann.held ann.dropped).map (fun (k, o) =>
          (s!"{describeObligation o} held back in step {k} and never released, though storage \
            confirmed what it waited for", none))
        ++ (droppedActionFaults cfg node ann.held ann.dropped ann.drbRequests).map fun (k, kind, v, d) =>
          (s!"{repr kind} for view {v.toNat} only on {repr d}, after step {k}", none)
      let unexcused := late.filterMap fun (what, excuse) => if excuse.isNone then some what else none
      if !unexcused.isEmpty then return .late unexcused
      return .pass (late.filterMap fun (what, excuse) => excuse.map (s!"{what}, excused by {·}"))
        (owedAtEnd cfg leader node h)

def usage : String := "usage: check [--coverage] <trace-or-directory>..."

/-- What the command line asked for. -/
structure CheckOptions where
  coverage : Bool := false
  paths : List String := []

/-- Read the command line, or say why it cannot be read. -/
def parseOptions : List String → CheckOptions → Option CheckOptions
  | [], o => some o
  | "--coverage" :: rest, o => parseOptions rest { o with coverage := true }
  | arg :: rest, o =>
    if arg.startsWith "--" then none else parseOptions rest { o with paths := o.paths ++ [arg] }

public def main (args : List String) : IO UInt32 := do
  let some opts := parseOptions args {} | do IO.eprintln usage; return 2
  let paths := opts.paths
  if paths.isEmpty then
    IO.eprintln usage
    return 2
  let coverage ← if opts.coverage then some <$> IO.mkRef ({} : Coverage) else pure none
  let mut traces := #[]
  for arg in paths do
    traces := traces ++ (← tracesUnder arg)
  let mut failed := false
  let mut passed := 0
  let mut skipped := 0
  let mut signingOnly := 0
  let mut excused := 0
  -- Each trace in a task of its own, so the checks use every core.
  let tasks ← traces.mapM fun path => IO.asTask (checkOne coverage path)
  for (path, task) in traces.zip tasks do
    -- A trace that cannot be read, gone since it was listed for one, is reported
    -- with the others rather than ending the run.
    let finding := match task.get with
      | .ok finding => finding
      | .error e => .unreadable (toString e)
    match finding with
    | .unreadable e =>
      failed := true
      IO.println s!"unreadable: {path}: {e}"
    | .pass late owed =>
      passed := passed + 1
      unless late.isEmpty do
        excused := excused + 1
        IO.println s!"late, {String.intercalate "; " late}: {path}"
      unless owed.isEmpty do
        IO.println s!"still owes at the end {String.intercalate ", " owed}: {path}"
    | .late owed =>
      failed := true
      IO.println s!"did not act in the step it owed in: {String.intercalate "; " owed}: {path}"
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
  IO.println s!"checked {traces.size} traces: {passed} obey every rule and discharge \
    what they owe in the step they owe it in, {signingOnly} every signing rule, {skipped} out of scope; \
    {excused} of those passing were late where an input outside the model excuses it"
  if passed + signingOnly == 0 then
    IO.println "no trace was checked against any rule"
    failed := true
  if let some cov := coverage then
    IO.println ""
    IO.print (← cov.get).render
  return if failed then 1 else 0
