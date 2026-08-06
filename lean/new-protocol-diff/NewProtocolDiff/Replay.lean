module

public import NewProtocolDiff.Trace

/-!
# The comparison

What it is *not* is output equality per step. The machine is eager — after
recording an input it does everything the new state owes — while a real
implementation defers, acting on particular views on particular inputs. Both are
conforming; their per-step outputs differ anyway.

What holds instead is containment on the marks. Any action the recorded
implementation took at step `n`, the eager machine took at some step `≤ n`, so
after every step:

* every view the recording has voted, proposed or decided in, the machine has too;
* at the end, if the recording has caught up, the two agree exactly.

A violation is informative in either direction. The recording ahead of the
machine means it acted where the specification does not permit it. The machine
permanently ahead means the recording dropped an obligation — which is what
`WeaklyFair` forbids, and what no step-local check can see.

Each mark carries what the action was about as well as the view it was in, so two
runs that act in the same view for different reasons still part company. What a
mark cannot carry is anything the machine does not know: a proposal it builds has
no real identity (`Impl.unassignedIdentity`), and the parent certificate it names
is a choice the rules leave open when both a `Cert1` and a timeout certificate are
available.

Both sides' marks come from their *outputs*, never from the machine's state.
That symmetry is what makes replaying collections safe: pruning drops the marks
a state carries, so a state-derived comparison would report the machine falling
behind its own past every time a trace collected. It also disposes of the
anchor, which `Impl.initial` records as decided by configuration rather than by
any action, and which no recording would ever report.
-/

@[expose] public section

open Std (TreeMap)

namespace NewProtocolDiff

open NewProtocol

/--
What an action was about, beyond the view it acted in.

Only values both sides can be expected to hold. A vote's block hash and signer
come from a proposal that arrived and from the node key, so the machine's copies
are the recording's. A proposal's header arrived as an `Input.headerBuilt`, so the
same is true of it. A decided block is a proposal the node holds.
-/
inductive Detail where
  /-- The block a vote signed, and who signed it. -/
  | vote (block : BlockHash) (signer : PubKey)
  /-- The header a proposal carried. -/
  | proposal (header : BlockHeader)
  /-- The block a decide delivered. -/
  | decide (block : BlockHash)
deriving DecidableEq, Repr

/--
A cryptographic value as the recorder wrote it, abbreviated.

`identToNat` turns an identifier into a number in the hundreds of digits, which no
report should print. `natToIdent` recovers the text, and the ends are what tell two
values apart: a mismatch usually differs everywhere, but a corrupted trace may
differ in one character.
-/
def renderIdent (n : Nat) : String :=
  match natToIdent n with
  | none => toString n
  | some text =>
    if text.length ≤ 24 then text
    else (text.take 12).toString ++ "…" ++ (text.drop (text.length - 8)).toString

/-- A detail as it appears in a divergence report. -/
def Detail.render : Detail → String
  | .vote b k => s!"block {renderIdent b.toNat} signed by {renderIdent k.toNat}"
  | .proposal h => s!"payload {renderIdent h.payloadCommit.toNat}"
  | .decide b => s!"block {renderIdent b.toNat}"

/--
The marks a run is compared on: the views a node acted in, and what it did there.

Keyed by view, because that is what both sides can agree on even though the
machine is eager. The `Detail` is checked only where both sides acted in the same
view, so an eager machine is never faulted for a view the recording has not
reached yet.
-/
structure Marks where
  voted1 : TreeMap ViewNumber Detail := ∅
  voted2 : TreeMap ViewNumber Detail := ∅
  proposed : TreeMap ViewNumber Detail := ∅
  decided : TreeMap ViewNumber Detail := ∅

namespace Marks

/-- A view acted in twice, named by the kind of action. -/
abbrev Repeat := String × ViewNumber

/--
The marks a list of outputs witnesses, and every view already marked before it.

Acting twice in a view is non-conformance on its own: `SafetySpec.vote1Once`,
`SafetySpec.vote2Once`, `StepSpec.proposeOnce` and the freshness in
`StepSpec.decideJustified` each forbid a second action. The marks are keyed by
view, so the second would otherwise overwrite the first and leave nothing to see
— the one violation a containment check is blind to.

A repeat is judged against the marks as they stood *before* this step, not
against the accumulator. Two of a kind inside one step's output list prove
nothing: a recorder attaches the actions of a step the model has no input for to
the next step it can write, so one list can hold the work of several steps. The
rules quantify over one step's outputs and are satisfied by two copies of the
same action; what they forbid is acting again *later*, and that is what is
reported.
-/
def ofOutputs (m : Marks) (outputs : List Output) : Marks × List Repeat :=
  outputs.foldl (step m) (m, [])
where
  step (before : Marks) : Marks × List Repeat → Output → Marks × List Repeat
    | (m, rs), .send (.vote1 v) =>
      ({ m with voted1 := m.voted1.insert v.view (.vote v.data.blockHash v.signer) },
        if before.voted1.contains v.view then ("voted1", v.view) :: rs else rs)
    | (m, rs), .send (.vote2 v) =>
      ({ m with voted2 := m.voted2.insert v.view (.vote v.data.blockHash v.signer) },
        if before.voted2.contains v.view then ("voted2", v.view) :: rs else rs)
    | (m, rs), .send (.proposal p) =>
      ({ m with proposed := m.proposed.insert p.viewNumber (.proposal p.blockHeader) },
        if before.proposed.contains p.viewNumber then ("proposed", p.viewNumber) :: rs else rs)
    | (m, rs), .decided bs _ _ =>
      bs.foldl (fun (m, rs) b =>
        ({ m with decided := m.decided.insert b.viewNumber (.decide (blockHash b)) },
          if before.decided.contains b.viewNumber then ("decided", b.viewNumber) :: rs else rs))
        (m, rs)
    | acc, _ => acc

end Marks

/-- Where the two implementations parted company. -/
inductive Divergence where
  /-- The recording acted in a view the machine has not: unjustified by the specification. -/
  | recordingAhead (step : Nat) (kind : String) (view : ViewNumber)
  /-- Both acted in the view, but not on the same thing. -/
  | differentDetail (step : Nat) (kind : String) (view : ViewNumber) (recorded machine : Detail)
  /-- One side acted twice in a view, which the `once` rules forbid outright. -/
  | actedTwice (step : Nat) (side : String) (kind : String) (view : ViewNumber)
  /-- The trace could not be read. -/
  | malformed (reason : String)
deriving Repr

def Divergence.describe : Divergence → String
  | .recordingAhead n kind v =>
    s!"step {n}: the recording {kind} in view {v.toNat}, the machine did not"
  | .differentDetail n kind v r m =>
    s!"step {n}: both {kind} in view {v.toNat}, the recording on {r.render}, " ++
      s!"the machine on {m.render}"
  | .actedTwice n side kind v =>
    s!"step {n}: the {side} {kind} in view {v.toNat} twice"
  | .malformed r => s!"malformed trace: {r}"

/-- The first view the machine is behind the recording on, if any. -/
def behind (n : Nat) (recorded machine : Marks) : Option Divergence :=
  let check (kind : String) (r m : TreeMap ViewNumber Detail) : Option Divergence :=
    r.toList.findSome? fun (v, d) =>
      match m.get? v with
      | none => some (.recordingAhead n kind v)
      | some d' => if d == d' then none else some (.differentDetail n kind v d d')
  check "voted1" recorded.voted1 machine.voted1
    |>.orElse fun _ => check "voted2" recorded.voted2 machine.voted2
    |>.orElse fun _ => check "proposed" recorded.proposed machine.proposed
    |>.orElse fun _ => check "decided" recorded.decided machine.decided

/-- What a replay concluded. -/
structure Outcome where
  steps : Nat
  recorded : Marks
  machine : Marks
  divergence : Option Divergence

/--
Feed a trace to the machine, checking containment after every step.

A `collect` step in the trace prunes the machine too, which is the only way the
`GcSpec` side of the specification gets exercised: pruning is where the machine
could lose something it still owes, and the comparison would then show it
falling behind a recording that kept going.
-/
def replay (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)
    (trace : List Event) : Outcome :=
  let rec go (n : Nat) (s : Impl.State) (recorded machine : Marks) : List Event → Outcome
    | [] => ⟨n, recorded, machine, none⟩
    | .collect :: rest => go (n + 1) (s.gc cfg) recorded machine rest
    | .consensus input output :: rest =>
      let (s', emitted) := Impl.next cfg leader node s input
      let (recorded', recordedTwice) := recorded.ofOutputs output
      let (machine', machineTwice) := machine.ofOutputs emitted
      let twice (side : String) (rs : List Marks.Repeat) : Option Divergence :=
        rs.head?.map fun (kind, v) => .actedTwice n side kind v
      match twice "recording" recordedTwice
          |>.orElse (fun _ => twice "machine" machineTwice)
          |>.orElse (fun _ => behind n recorded' machine') with
      | some d => ⟨n, recorded', machine', some d⟩
      | none => go (n + 1) s' recorded' machine' rest
  go 0 (Impl.initial cfg) {} {} trace

/-- Views the machine acted in that the recording never caught up on. -/
def Outcome.machineAhead (o : Outcome) : List (String × ViewNumber) :=
  let extra (kind : String) (m r : TreeMap ViewNumber Detail) : List (String × ViewNumber) :=
    (m.toList.filterMap fun (v, _) => if r.contains v then none else some v).map (kind, ·)
  extra "voted1" o.machine.voted1 o.recorded.voted1
    ++ extra "voted2" o.machine.voted2 o.recorded.voted2
    ++ extra "proposed" o.machine.proposed o.recorded.proposed
    ++ extra "decided" o.machine.decided o.recorded.decided

/--
What a replay concluded, in words.

The machine being ahead is reported in two groups, because the two mean different
things. An action the machine took and the recording did not may be one the
recording still owes. A *proposal* is not: the replay makes this node the leader
of every view, which the network did not, so a proposal the machine made says
nothing about the recording at all.
-/
def Outcome.report (o : Outcome) : String :=
  match o.divergence with
  | some d => s!"DIVERGED after {o.steps} steps\n  {d.describe}"
  | none =>
    let ahead := o.machineAhead
    if ahead.isEmpty then s!"OK: {o.steps} steps, marks agree exactly"
    else
      let (proposals, acts) := ahead.partition (·.1 == "proposed")
      let line (kv : String × ViewNumber) : String := s!"    {kv.1} view {kv.2.toNat}"
      let owed :=
        if acts.isEmpty then ""
        else
          s!"\n  the machine is still ahead on {acts.length}, which the recording may owe:\n" ++
            String.intercalate "\n" (acts.map line)
      let led :=
        if proposals.isEmpty then ""
        else
          s!"\n  and proposed in {proposals.length} views, being leader of every view here:\n" ++
            String.intercalate "\n" (proposals.map line)
      s!"OK: {o.steps} steps, recording contained in machine" ++ owed ++ led

end NewProtocolDiff
