module

public import NewProtocolImpl.Machine
public import NewProtocolDiff.Check
public import NewProtocolDiff.ProtocolCheck

/-!
# Random runs of the machine

Several nodes running the reference machine (`NewProtocolImpl.step`) over a
simulated network with random delays, and every node's history checked afterwards.

The network does what `NewProtocol.Synchrony` asks of one and no more: a message
arrives by `max t GST + Δ`, a quorum's votes become a certificate at every node, a
proposal reaches the members of its epoch's committee with their shares and the
others without, the validity report and the payload follow, a leader is handed a
header for the parents it may build on, the last block of an epoch with a `Cert2`
becomes an epoch change, and each node's timer fires `τ` after it entered a view
and again every `τ` while it stays there. Faulty nodes are silent.

A run passes when

* every node's history obeys `NewProtocol.ProtocolHistory`, by the checker whose
  soundness is proved (`NewProtocolDiff.checkProtocol_sound`);
* what the nodes decide lies on one chain: no two decided blocks share a height;
* the chain grew: the blocks decided reach a minimum height.

The machine is proved to obey the rules on any inputs, so the first check tests
the compiled machine and checker rather than the proofs. The other two test what
no proof here covers: that the delivery assumptions are ones a network can meet
with the machine, across epoch changes and before and after GST.
-/

@[expose] public section

namespace NewProtocolDiff
namespace Sim

open NewProtocol NewProtocol.Lists NewProtocolImpl

/-- The parameters of one run. Node keys are `1` to `nodes`. -/
structure Params where
  /-- Seeds the delays. -/
  seed : Nat

  /-- How many nodes there are. -/
  nodes : Nat := 5

  /-- The silent nodes. -/
  faulty : List Nat := [4]

  /--
  Faulty nodes that run the machine but, as leaders, send one block to half the
  members of the committee and another to the rest.
  -/
  equivocators : List Nat := []

  /-- Nodes silent in one epoch each, and honest in the others: what they send for that epoch is lost. -/
  silentIn : List (Nat × Nat) := []

  /-- Members to an epoch's committee. Epoch `e`'s are the `size` nodes from `e + 1` on, cyclically. -/
  size : Nat := 4

  /-- Blocks to an epoch; zero for no epochs. -/
  epochHeight : Nat := 0

  /-- The longest delay after GST. -/
  delta : Nat := 3

  /-- The view timer. -/
  tau : Nat := 30

  /-- When the network stabilises. -/
  gst : Nat := 0

  /-- The longest delay before GST. -/
  preGst : Nat := 40

  /-- How long the run lasts. -/
  horizon : Nat := 400

  /-- The height the decided chain must reach. -/
  minHeight : Nat := 3
deriving Repr

namespace Params

variable (P : Params)

def members (e : EpochNumber) : List PubKey :=
  (List.range P.size).map fun i => ⟨(e.toNat + i) % P.nodes + 1⟩

def isMember (e : EpochNumber) (k : PubKey) : Bool := (P.members e).contains k

/-- The faulty members a committee tolerates. -/
def faultBound : Nat := (P.size - 1) / 3

def quorum : Nat := P.size - P.faultBound

/-- Each epoch's members lead its views in turn. -/
def leader (e : EpochNumber) (v : ViewNumber) : Option PubKey := (P.members e)[v.toNat % P.size]?

/-- The nodes running the machine. -/
def running : List PubKey :=
  (List.range P.nodes).filterMap fun i => if P.faulty.contains (i + 1) then none else some ⟨i + 1⟩

/-- The nodes whose histories are checked: those that obey the rules in some epoch. -/
def checked : List PubKey := P.running.filter fun k => !P.equivocators.contains k.toNat

def faultyIn (e : EpochNumber) (k : PubKey) : Bool :=
  P.faulty.contains k.toNat || P.equivocators.contains k.toNat || P.silentIn.contains (k.toNat, e.toNat)

/-- The epoch a message is signed for, if it is signed. -/
def signedFor : Output → Option EpochNumber
  | .send (.proposal p) => some p.epoch
  | .send (.revote r) => some r.cert.data.epoch
  | .send (.vote1 v) => some v.data.epoch
  | .send (.vote2 v) => some v.data.epoch
  | .send (.timeoutVote v) => some v.data.epoch
  | _ => none

/-- An output a node silent in its epoch sends is lost. -/
def lost (k : PubKey) (o : Output) : Bool :=
  match signedFor o with
  | some e => P.silentIn.contains (k.toNat, e.toNat)
  | none => false

def epoch0 : EpochNumber := epochOf 0 P.epochHeight

def anchor : Block :=
  ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, P.epoch0, ⟨⟨⟨0⟩, P.epoch0, 0⟩, ViewNumber.genesis⟩, none, ⟨1⟩⟩

def cfg : Config := ⟨P.anchor, ⟨⟨⟨1⟩, P.epoch0, 0⟩, ViewNumber.genesis⟩, 20, P.epochHeight⟩

/-- Every committee keeps a quorum of honest members, as `TimedNetwork.honestQuorum` asks. -/
def coherent : Bool :=
  (List.range (P.nodes + 1)).all fun e =>
    ((P.members ⟨e⟩).filter fun k => !P.faultyIn ⟨e⟩ k).length ≥ P.quorum

end Params

/-- An input due at a node, or its timer for a view. -/
structure Event where
  time : Nat
  seq : Nat
  node : PubKey
  input : Input
  timer : Bool

/-- The state of a run. -/
structure State where
  rng : Nat
  seq : Nat := 0
  queue : List Event := []
  hist : Array History
  ids : List (Proposal × Nat) := []
  nextId : Nat := 2
  nextPayload : Nat := 1
  blocks : List Block := []
  vote1s : List ((Vote1Data × ViewNumber) × List PubKey) := []
  vote2s : List ((Vote2Data × ViewNumber) × List PubKey) := []
  timeouts : List ((ViewNumber × EpochNumber) × List (PubKey × Cert1)) := []
  oneHonest : List (ViewNumber × List PubKey) := []
  cert1s : List Cert1 := []
  timeoutCerts : Nat := 0
  epochChanges : Nat := 0
  revotes : Nat := 0
  headers : List (PubKey × ViewNumber × BlockHash) := []
  decided : List (PubKey × Block) := []

abbrev SimM := StateM State

variable (P : Params)

def rand (n : Nat) : SimM Nat := do
  let s ← get
  let r := (s.rng * 6364136223846793005 + 1442695040888963407) % 2 ^ 64
  set { s with rng := r }
  return (r / 2 ^ 33) % (max n 1)

/-- When a message sent at `now` arrives: by `max now GST + Δ`, and at most `preGst` late before GST. -/
def arrival (now : Nat) : SimM Nat := do
  if now < P.gst then
    let a ← rand P.preGst
    let b ← rand P.delta
    return min (now + 1 + a) (P.gst + 1 + b)
  else
    let b ← rand P.delta
    return now + 1 + b

def insert (e : Event) : List Event → List Event
  | [] => [e]
  | x :: xs =>
    if e.time < x.time || (e.time == x.time && e.seq < x.seq) then e :: x :: xs else x :: insert e xs

def push (time : Nat) (k : PubKey) (i : Input) (timer : Bool := false) : SimM Unit :=
  modify fun s => { s with seq := s.seq + 1, queue := insert ⟨time, s.seq, k, i, timer⟩ s.queue }

/-- Deliver an input to every node `pick` holds of. -/
def sendTo (now : Nat) (pick : PubKey → Bool) (i : PubKey → Input) : SimM Unit := do
  for k in P.running do
    if pick k then push (← arrival P now) k (i k)

/-- The identity the network gives a block a leader built. -/
def identify (p : Proposal) : SimM Block := do
  let s ← get
  match s.ids.find? (·.1 == p) with
  | some (_, id) => return { p with identity := ⟨id⟩ }
  | none =>
    let b := { p with identity := ⟨s.nextId⟩ }
    set { s with ids := (p, s.nextId) :: s.ids, nextId := s.nextId + 1, blocks := b :: s.blocks }
    return b

/-- Record a vote; true when it completes a quorum of its epoch's members for the first time. -/
def tally {κ : Type} [DecidableEq κ] (key : κ) (e : EpochNumber) (k : PubKey)
    (tbl : List (κ × List PubKey)) : List (κ × List PubKey) × Bool :=
  let old := (tbl.find? (·.1 == key)).map (·.2) |>.getD []
  let new := if old.contains k then old else k :: old
  let count (l : List PubKey) := (l.filter (P.isMember e)).length
  ((key, new) :: tbl.filter (·.1 != key), count old < P.quorum && count new ≥ P.quorum)

def later (a b : Cert1) : Cert1 :=
  if a.data.epoch.toNat < b.data.epoch.toNat
    || (a.data.epoch = b.data.epoch && a.view.toNat ≤ b.view.toNat) then b else a

/-- What the network does with an output of node `k` at `now`. -/
def handle (now : Nat) (k : PubKey) : Output → SimM Unit
  | .send (.proposal p) => do
    let b ← identify p
    let other ← identify { p with blockHeader := { p.blockHeader with payloadCommit := ⟨p.payloadCommit.toNat + 2 ^ 32⟩ } }
    let members := P.members b.epoch
    for k' in P.running do
      let t ← arrival P now
      let b' := if P.equivocators.contains k.toNat && members.idxOf k' ≥ members.length / 2 then other else b
      if P.isMember b.epoch k' then
        push t k' (.proposal k b' (some ⟨b'.viewNumber, b'.payloadCommit⟩))
        push (t + 1) k' (.blockValidated b'.viewNumber (blockHash b'))
      else push t k' (.proposal k b' none)
  | .send (.revote r) => do
    modify fun s => { s with revotes := s.revotes + 1 }
    sendTo P now (P.isMember r.cert.data.epoch) fun _ => .revote k r
  | .send (.vote1 v) => do
    let s ← get
    let (tbl, done) := tally P (v.data, v.view) v.data.epoch k s.vote1s
    set { s with vote1s := tbl }
    if done then
      let c : Cert1 := ⟨v.data, v.view⟩
      modify fun s => { s with cert1s := c :: s.cert1s }
      sendTo P now (fun _ => true) fun _ => .certificate1 c
      if let some b := s.blocks.find? (blockHash · == v.data.blockHash) then
        sendTo P now (P.isMember v.data.epoch) fun _ => .blockReconstructed b.viewNumber b.payloadCommit
  | .send (.vote2 v) => do
    let s ← get
    let (tbl, done) := tally P (v.data, v.view) v.data.epoch k s.vote2s
    set { s with vote2s := tbl }
    if done then
      let c2 : Cert2 := ⟨v.data, v.view⟩
      sendTo P now (fun _ => true) fun _ => .certificate2 c2
      if let some b := s.blocks.find? (blockHash · == v.data.blockHash) then
        if decide (IsLastBlock b.blockHeader.blockNumber P.epochHeight) then
          if let some c1 := s.cert1s.find? (fun c => c.data.toVote2 == v.data && c.view == b.viewNumber) then
            modify fun s => { s with epochChanges := s.epochChanges + 1 }
            sendTo P now (fun _ => true) fun _ => .epochChange c1 c2 b
  | .send (.timeoutVote tv) => do
    let s ← get
    let key := (tv.view, tv.data.epoch)
    let old := (s.timeouts.find? (·.1 == key)).map (·.2) |>.getD []
    let new := if old.any (·.1 == k) then old else (k, tv.data.lock) :: old
    let count (l : List (PubKey × Cert1)) := (l.filter fun x => P.isMember tv.data.epoch x.1).length
    set { s with timeouts := (key, new) :: s.timeouts.filter (·.1 != key) }
    if count old < P.quorum && count new ≥ P.quorum then
      let lock := (new.map (·.2)).foldl later P.cfg.anchorCert
      modify fun s => { s with timeoutCerts := s.timeoutCerts + 1 }
      sendTo P now (fun _ => true) fun _ => .timeoutCertificate ⟨⟨tv.data.epoch, lock⟩, tv.view⟩
    let s ← get
    let oldH := (s.oneHonest.find? (·.1 == tv.view)).map (·.2) |>.getD []
    let newH := if oldH.contains k then oldH else k :: oldH
    set { s with oneHonest := (tv.view, newH) :: s.oneHonest.filter (·.1 != tv.view) }
    if oldH.length ≤ P.faultBound && newH.length > P.faultBound then
      sendTo P now (fun _ => true) fun _ => .timeoutOneHonest tv.view
  | .send _ => pure ()
  | .decided blocks _ _ => modify fun s => { s with decided := blocks.map (k, ·) ++ s.decided }

/-- A leader of its view is handed a header for its lock and for the latest `Cert1` it holds. -/
def headers (now : Nat) (k : PubKey) (h : History) : SimM Unit := do
  let v := viewOf P.cfg h
  let e := epochOfHistory P.cfg h
  if P.leader e v == some k || P.leader (e + 1) v == some k then
    let best := (cert1sHeld P.cfg h).foldl (fun a c => if a.view.toNat ≤ c.view.toNat then c else a)
      P.cfg.anchorCert
    for c in [lockOf P.cfg h, best] do
      let s ← get
      unless s.headers.contains (k, v, c.data.blockHash) do
        set { s with headers := (k, v, c.data.blockHash) :: s.headers, nextPayload := s.nextPayload + 1 }
        push (now + 1) k (.headerBuilt v c.data.blockHash ⟨⟨s.nextPayload⟩, c.data.blockNumber + 1⟩)

/-- Node `k` takes input `i` at `now`. A timer for a view the node has left is dropped. -/
def stepNode (now : Nat) (k : PubKey) (i : Input) (timer : Bool) : SimM Unit := do
  let h := (← get).hist[k.toNat - 1]!
  let v0 := viewOf P.cfg h
  if timer && i != .timeout v0 then return
  let st := step P.cfg P.leader k h i
  let h' := h ++ [st]
  modify fun s => { s with hist := s.hist.set! (k.toNat - 1) h' }
  for o in st.output do
    unless P.lost k o do handle P now k o
  let v1 := viewOf P.cfg h'
  if timer then push (now + P.tau) k i true
  if v1 != v0 then push (now + P.tau) k (.timeout v1) true
  headers P now k h'

def start : SimM Unit := do
  for k in P.running do
    push P.tau k (.timeout (viewOf P.cfg [])) true
    headers P 0 k []

def loop : Nat → SimM Unit
  | 0 => pure ()
  | fuel + 1 => do
    let s ← get
    match s.queue with
    | [] => pure ()
    | e :: rest =>
      if e.time > P.horizon then pure ()
      else
        set { s with queue := rest }
        stepNode P e.time e.node e.input e.timer
        loop fuel

/-- The state at the end of a run. -/
def run : State :=
  ((start P *> loop P 1000000).run { rng := P.seed, hist := Array.replicate P.nodes [] }).2

/-- What a run found. -/
structure Outcome where
  failures : List String
  height : Nat
  steps : Nat
  timeoutCerts : Nat
  epochChanges : Nat
  revotes : Nat

/-- Check a run's end state: the rules, agreement on one chain, and growth. -/
def checkState (s : State) : Outcome :=
  let rules := P.checked.filterMap fun k =>
    let h := s.hist[k.toNat - 1]!
    let broken := (checkRules P.cfg k h).filterMap fun (n, ok) => if ok then none else some n
    if !broken.isEmpty then some s!"node {k.toNat} breaks {broken}"
    else (protocolFault P.cfg h k P.leader).map fun (n, why) => s!"node {k.toNat} breaks {why} at step {n}"
  let mine := s.decided.filter fun (k, _) => P.checked.contains k
  let fork := mine.filterMap fun (k, b) =>
    if mine.any fun (_, b') => b'.blockHeader.blockNumber == b.blockHeader.blockNumber
      && blockHash b' != blockHash b then
      some s!"node {k.toNat} decided a block at height {b.blockHeader.blockNumber.toNat} another did not"
    else none
  let height := mine.foldl (fun m (_, b) => max m b.blockHeader.blockNumber.toNat) 0
  let growth := if height < P.minHeight then [s!"the chain reached height {height} only"] else []
  ⟨rules ++ fork.take 1 ++ growth, height, (s.hist.toList.map List.length).foldl (· + ·) 0,
    s.timeoutCerts, s.epochChanges, s.revotes⟩

def check : Outcome := checkState P (run P)

/--
The checks catch what they claim to: a run with a vote1 for a second block in a
view, and one with two blocks decided at one height, both fail.
-/
def selfTest : List String :=
  let s := run P
  let k := P.checked.headD ⟨1⟩
  let h := s.hist[k.toNat - 1]!
  let forged := (h.flatMap fun st => vote1sOf st).head?.map fun v =>
    let bad : Step := ⟨.timeout ⟨0⟩, [.send (.vote1 { v with data := { v.data with blockHash := ⟨2 ^ 40⟩ } })]⟩
    { s with hist := s.hist.set! (k.toNat - 1) (h ++ [bad]) }
  let doubled := s.decided.head?.map fun (k, b) =>
    { s with decided := (k, { b with identity := ⟨2 ^ 40⟩ }) :: s.decided }
  let caught (x : Option State) := x.any fun s' => !(checkState P s').failures.isEmpty
  (if caught forged then [] else ["a forged vote1 was not caught"])
    ++ (if caught doubled then [] else ["two blocks decided at one height were not caught"])

end Sim
end NewProtocolDiff
