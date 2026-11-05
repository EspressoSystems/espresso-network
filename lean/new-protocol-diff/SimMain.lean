module

import NewProtocolDiff.Sim
import NewProtocolDiff.Scenario
import NewProtocolDiff.Random

/-!
Random runs of the machine over a simulated network.

    sim [seeds]
    sim --scenarios <dir> [first] [count]
    sim --random-check [first] [count]

Runs every setting below for seeds `1` to `seeds` (default 4) and checks each run
(`NewProtocolDiff.Sim.check`), after checking that the checks catch a forged vote
and a fork (`NewProtocolDiff.Sim.selfTest`). The exit status is 1 if anything fails.

With `--scenarios`, it runs `count` random settings from seed `first` on, drawn
within what the Rust node's tests can reproduce (`NewProtocolDiff.Sim.randomParams`),
and writes what each node checked was given to `<dir>`, one scenario per node and
run (`NewProtocolDiff.Scenario`). With `--random-check`, it only runs and checks them.
-/

open NewProtocolDiff NewProtocolDiff.Sim

/-- Who is faulty: one silent node, one equivocating leader, or one member of each epoch's committee, a different node each epoch. -/
def faults : List (String × (Params → Params)) :=
  [ ("one silent node", id),
    ("one equivocating leader", fun p => { p with faulty := [], equivocators := [4] }),
    ("a different member faulty in each epoch", fun p =>
      { p with faulty := [], silentIn := (List.range 100).map fun e => (e % p.nodes + 1, e) }) ]

/-- No epochs and epochs of one, two and three blocks, with GST at zero and later, for each kind of fault. -/
def settings (seed : Nat) : List (String × Params) :=
  faults.flatMap fun (name, f) => [0, 1, 2, 3].flatMap fun h => [0, 60].map fun g =>
    (name, f { seed, epochHeight := h, gst := g, horizon := 300 + g })

def describe (name : String) (p : Params) : String :=
  s!"{name}, epoch height {p.epochHeight}, GST {p.gst}, seed {p.seed}"

/--
Run `runs`, each named and with its seed, and write each checked node's scenario to
`dir`; without `dir`, only check them.
-/
def runAll (dir : Option System.FilePath) (runs : List (String × Nat × Params)) : IO UInt32 := do
  if let some d := dir then IO.FS.createDirAll d
  let mut failed := 0
  let mut written := 0
  -- Each run in a task of its own, so the runs use every core.
  -- An incoherent setting is not run at all.
  let tasks := runs.map fun (name, seed, p) =>
    (name, seed, p, if p.coherent then some (Task.spawn fun _ => let s := run p; (s, checkState p s))
      else none)
  for (name, seed, p, task) in tasks do
    let some task := task | do
      failed := failed + 1
      IO.println s!"incoherent setting: {describe name p}"
      continue
    let (s, o) := task.get
    unless o.failures.isEmpty do
      failed := failed + 1
      IO.println s!"FAILED: {describe name p}: {String.intercalate "; " o.failures}"
      if name == "random" then IO.println s!"  {describeRandom p}"
      continue
    if let some d := dir then
      for k in p.checked do
        let file := d / s!"{name}-seed{seed}-gst{p.gst}-node{k.toNat}.jsonl"
        IO.FS.writeFile file (Scenario.render p s k)
        written := written + 1
    IO.println s!"ok: {describe name p}: height {o.height}, {o.steps} steps"
  if dir.isSome then IO.println s!"{written} scenarios written, {failed} runs failed"
  else IO.println s!"{runs.length} runs, {failed} failed"
  return if failed = 0 then 0 else 1

/-- Random runs `first` to `first + count - 1` (`NewProtocolDiff.Sim.randomParams`). -/
def randomRuns (first count : Nat) : List (String × Nat × Params) :=
  (List.range' first count).map fun seed => ("random", seed, randomParams seed)

public def main (args : List String) : IO UInt32 := do
  let num (i d : Nat) := ((args.drop i).head?.bind String.toNat?).getD d
  match args with
  | "--scenarios" :: dir :: _ => return ← runAll (some dir) (randomRuns (num 2 1) (num 3 48))
  | "--random-check" :: _ => return ← runAll none (randomRuns (num 1 1) (num 2 48))
  | "--random-describe" :: _ =>
    for (_, _, p) in randomRuns (num 1 1) (num 2 48) do IO.println (describeRandom p)
    return 0
  | _ => pure ()
  let seeds := (args.head?.bind String.toNat?).getD 4
  let self := selfTest { seed := 1 }
  unless self.isEmpty do
    IO.println s!"self-test FAILED: {String.intercalate "; " self}"
    return 1
  let mut failed := 0
  let mut runs := 0
  -- Each run in a task of its own, so the runs use every core.
  let all := (List.range' 1 seeds).flatMap fun seed => (settings seed).map fun (name, p) =>
    (name, p, if p.coherent then some (Task.spawn fun _ => check p) else none)
  for (name, p, task) in all do
    runs := runs + 1
    let some task := task | do
      IO.println s!"incoherent setting: {describe name p}"
      failed := failed + 1
    let o := task.get
    if o.failures.isEmpty then
      IO.println s!"ok: {describe name p}: height {o.height}, {o.steps} steps, \
        {o.timeoutCerts} timeout certificates, {o.epochChanges} epoch changes, {o.revotes} re-vote requests"
    else
      failed := failed + 1
      IO.println s!"FAILED: {describe name p}: {String.intercalate "; " o.failures}"
  IO.println s!"{runs} runs, {failed} failed"
  return if failed = 0 then 0 else 1
