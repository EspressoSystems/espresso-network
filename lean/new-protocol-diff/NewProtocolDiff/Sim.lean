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

/-- What a node does with what it sends for an epoch it is faulty in. -/
inductive Behaviour where
  /-- It sends nothing. -/
  | silent
  /-- As leader, it sends one block to half the committee and another to the rest. -/
  | equivocate
  /-- As leader, it sends proposals that break the rules (`Params.misbehave`). -/
  | misbehave
  /-- It sends each proposal and re-vote request to some nodes only, and each vote now and then not at all. -/
  | selective
  /-- It also votes for every block it is given and every `Cert1` it gets. -/
  | doubleVote
  /-- It holds its proposals and re-vote requests back for up to a view timer. -/
  | lateLeader
  /-- It times out each view as soon as it enters it, naming the anchor's certificate as its lock. -/
  | lowLock
deriving Repr, BEq, DecidableEq, Inhabited

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

  /--
  Faulty nodes that run the machine but, as leaders, send a proposal that breaks
  the rules, a different way by turns (`Params.misbehave`). The certificates it
  carries are genuine: a faulty node cannot forge a quorum's signatures.
  -/
  misbehaving : List Nat := []

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

  /-- The longest delay before GST, for a message that is not held. -/
  preGst : Nat := 40

  /--
  One in this many messages sent before GST is held until GST, however long that
  is; zero for none. A late GST then gives delays longer than any timer.
  -/
  heldUntilGst : Nat := 0

  /--
  Before GST, one in this many nodes gets the `Cert1` of an epoch's last block,
  and the announcement that its payload was rebuilt, only at GST; zero for none.
  The leader that asks for a re-vote of that block has the certificate, so such a
  node can get the request first.
  -/
  boundaryHeld : Nat := 0

  /-- Nodes faulty in one epoch each, with what they do in it. -/
  byzantine : List (Nat × Nat × Behaviour) := []

  /--
  A message takes the shortest delay or the longest allowed as often as any other;
  otherwise delays are uniform. Before GST the longest is `preGst`, unless the
  message is held.
  -/
  extremeDelays : Bool := false

  /-- Nodes every message reaches as late as allowed. -/
  slow : List Nat := []

  /--
  Before GST, nodes cut off from a time until another: what is sent to them in
  between arrives after it.
  -/
  partitions : List (Nat × Nat × List Nat) := []

  /--
  A timeout certificate may name the earliest lock a quorum of its votes allows
  rather than the latest of all of them; each node gets one or the other.
  -/
  lowTimeoutLocks : Bool := false

  /--
  A certificate that needs faulty votes reaches some nodes only, and the others
  within `Δ` of the first arrival or of GST, whichever is later; or none at all.
  -/
  partialCerts : Bool := false

  /-- How long the run lasts. -/
  horizon : Nat := 400

  /-- The height the decided chain must reach. -/
  minHeight : Nat := 3

  /-- The run ends once a checked node decides this height; zero for no such end. -/
  stopHeight : Nat := 0

  /--
  The committees, as the Rust node's test membership schedules them: each from the
  epoch it names until the next, its members in the order they lead. Each has
  `size` members, and the first is from epoch `0`. Empty, each epoch's
  committee is the `size` nodes from `e + 1` on, cyclically.
  -/
  committees : List (Nat × List Nat) := []
deriving Repr

namespace Params

variable (P : Params)

def members (e : EpochNumber) : List PubKey :=
  match (P.committees.filter (·.1 ≤ e.toNat)).getLast? with
  | some (_, ms) => ms.map (⟨·⟩)
  | none => (List.range P.size).map fun i => ⟨(e.toNat + i) % P.nodes + 1⟩

def isMember (e : EpochNumber) (k : PubKey) : Bool := (P.members e).contains k

/-- The faulty members a committee tolerates. -/
def faultBound : Nat := (P.size - 1) / 3

def quorum : Nat := P.size - P.faultBound

/-- Each epoch's members lead its views in turn. -/
def leader (e : EpochNumber) (v : ViewNumber) : Option PubKey := (P.members e)[v.toNat % P.size]?

/-- The nodes running the machine. -/
def running : List PubKey :=
  (List.range P.nodes).filterMap fun i => if P.faulty.contains (i + 1) then none else some ⟨i + 1⟩

/-- What node `k` does in epoch `e` if it is faulty in it, other than being silent throughout. -/
def behaviour (k : PubKey) (e : EpochNumber) : Option Behaviour :=
  match P.byzantine.find? fun (n, e', _) => n == k.toNat && e' == e.toNat with
  | some (_, _, b) => some b
  | none =>
    if P.equivocators.contains k.toNat then some .equivocate
    else if P.misbehaving.contains k.toNat then some .misbehave
    else if P.silentIn.contains (k.toNat, e.toNat) then some .silent
    else none

/-- The nodes that send proposals breaking the rules in some epoch. -/
def misbehavers : List Nat :=
  (P.misbehaving ++ (P.byzantine.filter (·.2.2 == .misbehave)).map (·.1)).eraseDups

/--
The nodes whose histories are checked: those that obey the rules in some epoch.
A node faulty in some epochs other than by being silent is not: what the others
get from it is not what it sent.
-/
def checked : List PubKey :=
  P.running.filter fun k => !P.equivocators.contains k.toNat && !P.misbehaving.contains k.toNat
    && !P.byzantine.any fun (n, _, b) => n == k.toNat && b != .silent

def faultyIn (e : EpochNumber) (k : PubKey) : Bool :=
  P.faulty.contains k.toNat || (P.behaviour k e).isSome

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
  | some e => P.behaviour k e == some .silent
  | none => false

/-- The latest a message sent at `now` may arrive: `Δ` after `now`, or after GST if that is later. -/
def latest (now : Nat) : Nat := max now P.gst + P.delta

def epoch0 : EpochNumber := epochOf 0 P.epochHeight

def anchor : Block :=
  ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, P.epoch0, ⟨⟨⟨0⟩, P.epoch0, 0⟩, ViewNumber.genesis⟩, none, ⟨1⟩⟩

def cfg : Config := ⟨P.anchor, ⟨⟨⟨1⟩, P.epoch0, 0⟩, ViewNumber.genesis⟩, 20, P.epochHeight⟩

/--
What a misbehaving leader sends instead of the proposal `p` the machine made, and
whether its block is valid, by its turn to lead: one a block too high, one of the wrong
epoch, one that skips views without a timeout certificate, one on a parent before
its timeout certificate's lock, and one whose block is not valid. Each but the
last is refused by the rules, and the last by the validity check. Where the
proposal carries nothing to break, it is made too high instead.

The unsafe one builds on a certificate the node holds from earlier in the lock's
epoch, `cert1s` or the anchor's.
-/
def misbehave (cert1s : List Cert1) (p : Proposal) : Proposal × Bool :=
  let taller := { p with blockHeader := { p.blockHeader with blockNumber := p.blockHeader.blockNumber + 1 } }
  match p.viewNumber.toNat / P.size % 5 with
  | 0 => (taller, true)
  | 1 => ({ p with epoch := p.epoch + 1 }, true)
  | 2 => (if p.timeoutEvidence.isSome then { p with timeoutEvidence := none } else taller, true)
  | 3 =>
    let earlier (tc : TimeoutCert) (c : Cert1) :=
      c.data.epoch = tc.data.lock.data.epoch && c.view.toNat < tc.data.lock.view.toNat
    match p.timeoutEvidence with
    | some tc =>
      match (P.cfg.anchorCert :: cert1s).find? (earlier tc) with
      | some c =>
        let height := c.data.blockNumber + 1
        let header := { p.blockHeader with blockNumber := height }
        let behind := { p with parentCert := c, epoch := epochOf height P.epochHeight, blockHeader := header }
        (behind, true)
      | none => (taller, true)
    | none => (taller, true)
  | _ => (p, false)

/--
Every committee keeps a quorum of honest members, as `TimedNetwork.honestQuorum`
asks, and the scheduled ones have `size` distinct nodes.
-/
def coherent : Bool :=
  P.committees.all (fun (_, ms) =>
    ms.length == P.size && ms.eraseDups.length == ms.length && ms.all (fun k => 1 ≤ k && k ≤ P.nodes))
  && (List.range (P.nodes + 1 + (P.committees.map (·.1)).foldl max 0)).all fun e =>
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
  /-- The inputs each node took, latest first, with the time and whether it was its timer. -/
  log : Array (List (Nat × Bool × Input))
  ids : List (Proposal × Nat) := []
  nextId : Nat := 2
  nextPayload : Nat := 1
  blocks : List Block := []
  /-- The identities of the blocks that are not valid. -/
  invalid : List Nat := []
  /-- Who sent each block, and the nodes it was sent to, by its identity. -/
  sentTo : List (Nat × PubKey × List PubKey) := []
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
  /-- The greatest height a checked node decided. -/
  height : Nat := 0

abbrev SimM := StateM State

variable (P : Params)

def rand (n : Nat) : SimM Nat := do
  let s ← get
  let r := (s.rng * 6364136223846793005 + 1442695040888963407) % 2 ^ 64
  set { s with rng := r }
  return (r / 2 ^ 33) % (max n 1)

/--
When a message sent at `now` arrives at node `k`: by `max now GST + Δ`
(`Params.latest`). Before GST at most `preGst` late unless it is held until GST
or `k` is cut off, and as late as allowed if `k` is slow.
-/
def arrival (now : Nat) (k : PubKey) : SimM Nat := do
  if let some (_, stop, _) := P.partitions.find? fun (start, stop, ks) =>
      start ≤ now && now < stop && ks.contains k.toNat then
    return min (stop + 1 + (← rand P.delta)) (P.latest now)
  if P.slow.contains k.toNat then return P.latest now
  if P.extremeDelays then
    match ← rand 3 with
    | 0 => return now + 1
    | 1 => return if now < P.gst then min (now + P.preGst) (P.latest now) else P.latest now
    | _ => pure ()
  if now < P.gst then
    let held ← if P.heldUntilGst = 0 then pure false else (· == 0) <$> rand P.heldUntilGst
    let a ← rand P.preGst
    let b ← rand P.delta
    return if held then P.gst + 1 + b else min (now + 1 + a) (P.gst + 1 + b)
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
def sendTo (now : Nat) (pick : PubKey → Bool) (i : PubKey → Input) (held : List PubKey := []) :
    SimM Unit := do
  for k in P.running do
    if pick k then
      let t ← if held.contains k then (P.gst + 1 + ·) <$> rand P.delta else arrival P now k
      push t k (i k)

/--
Deliver what node `l` sends for epoch `e` to the nodes `pick` holds of. A faulty
node may send it to some of them only, or late: the network owes nothing for it.
-/
def sendFrom (now : Nat) (l : PubKey) (e : EpochNumber) (pick : PubKey → Bool) (i : PubKey → Input) :
    SimM Unit := do
  let b := P.behaviour l e
  for k in P.running do
    if pick k then
      if b == some .selective && (← rand 2) == 0 then continue
      let t ← if b == some .lateLeader then (now + 1 + ·) <$> rand P.tau else arrival P now k
      push t k (i k)

/--
Deliver a certificate that needs faulty votes: to some of the nodes `pick` holds of,
and to the others within `Δ` of the first arrival or of GST, whichever is later, as
a certificate an honest node holds spreads (`Synchrony.certSpread`); to none if the
first part is empty.
-/
def sendPartly (now : Nat) (pick : PubKey → Bool) (i : PubKey → Input) : SimM Unit := do
  let ks := P.running.filter pick
  let first ← ks.filterM fun _ => (· == 0) <$> rand 2
  let mut earliest : Option Nat := none
  for k in first do
    let t ← arrival P now k
    push t k (i k)
    earliest := some (min t (earliest.getD t))
  if let some t0 := earliest then
    for k in ks.filter (!first.contains ·) do
      push (max t0 P.gst + 1 + (← rand P.delta)) k (i k)

/-- The nodes the `Cert1` `c`, formed at `now`, is held from until GST (`Params.boundaryHeld`). -/
def boundaryHeldFrom (now : Nat) (c : Cert1) : SimM (List PubKey) := do
  if now ≥ P.gst || P.boundaryHeld = 0 || !decide (IsLastBlock c.data.blockNumber P.epochHeight) then
    return []
  P.running.filterM fun _ => (· == 0) <$> rand P.boundaryHeld

/-- Whether the votes of `voters` for epoch `e` need faulty ones to make a quorum. -/
def needsFaulty (e : EpochNumber) (voters : List PubKey) : Bool :=
  (voters.filter fun k => P.isMember e k && !P.faultyIn e k).length < P.quorum

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

/-- Insert `x` into `xs`, kept in order of `le`. -/
def insertBy {α : Type} (le : α → α → Bool) (x : α) : List α → List α
  | [] => [x]
  | y :: ys => if le x y then x :: y :: ys else y :: insertBy le x ys

/--
The earliest lock a quorum of the timeout votes `votes` of epoch `e` can be
certified with: the latest of the quorum with the earliest locks.
-/
def lowestLock (e : EpochNumber) (votes : List (PubKey × Cert1)) : Cert1 :=
  let le (a b : PubKey × Cert1) :=
    a.2.data.epoch.toNat < b.2.data.epoch.toNat
      || (a.2.data.epoch = b.2.data.epoch && a.2.view.toNat ≤ b.2.view.toNat)
  let sorted := (votes.filter fun x => P.isMember e x.1).foldr (insertBy le) []
  ((sorted.take P.quorum).map (·.2)).foldl later P.cfg.anchorCert

/--
Deliver the proposal of the block `b`, certified by now, to the nodes it was not
sent to, within `Δ` of GST if that is later, without a share: what they would fetch
(`Synchrony.blockSpread`).
-/
def spreadBlock (now : Nat) (b : Block) : SimM Unit := do
  -- A certified block was sent by its proposer to the nodes that voted on it.
  let some (_, sender, sent) := (← get).sentTo.find? (·.1 == b.identity.toNat) | return
  for k in P.running.filter (!sent.contains ·) do
    push (max now P.gst + 1 + (← rand P.delta)) k (.proposal sender b none)
  modify fun s =>
    { s with sentTo := (b.identity.toNat, sender, P.running) :: s.sentTo.filter (·.1 != b.identity.toNat) }

/-- Note that `sender` sent the block `b` to node `k`. -/
def sentBlockTo (sender : PubKey) (b : Block) (k : PubKey) : SimM Unit :=
  modify fun s =>
    let id := b.identity.toNat
    let ks := ((s.sentTo.find? (·.1 == id)).map (·.2.2)).getD []
    { s with sentTo := (id, sender, k :: ks) :: s.sentTo.filter (·.1 != id) }

/-- What the network does with an output of node `k` at `now`. -/
def handle (now : Nat) (k : PubKey) : Output → SimM Unit
  | .send (.proposal p0) => do
    let cert1s := (← get).cert1s
    let bh := P.behaviour k p0.epoch
    let (p, valid) := if bh == some .misbehave then P.misbehave cert1s p0 else (p0, true)
    let b ← identify p
    unless valid do modify fun s => { s with invalid := b.identity.toNat :: s.invalid }
    let other ← identify { p with blockHeader := { p.blockHeader with payloadCommit := ⟨p.payloadCommit.toNat + 2 ^ 32⟩ } }
    let members := P.members b.epoch
    for k' in P.running do
      if bh == some .selective && (← rand 2) == 0 then continue
      let t ← if bh == some .lateLeader then (now + 1 + ·) <$> rand P.tau else arrival P now k'
      let b' := if bh == some .equivocate && members.idxOf k' ≥ members.length / 2 then other else b
      sentBlockTo k b' k'
      if P.isMember b.epoch k' then
        push t k' (.proposal k b' (some ⟨b'.viewNumber, b'.payloadCommit⟩))
        if valid then push (t + 1) k' (.blockValidated b'.viewNumber (blockHash b'))
      else push t k' (.proposal k b' none)
  | .send (.revote r) => do
    modify fun s => { s with revotes := s.revotes + 1 }
    sendFrom P now k r.cert.data.epoch (P.isMember r.cert.data.epoch) fun _ => .revote k r
  | .send (.vote1 v) => do
    if P.behaviour k v.data.epoch == some .selective && (← rand 2) == 0 then return
    let s ← get
    let (tbl, done) := tally P (v.data, v.view) v.data.epoch k s.vote1s
    set { s with vote1s := tbl }
    if done then
      let c : Cert1 := ⟨v.data, v.view⟩
      modify fun s => { s with cert1s := c :: s.cert1s }
      let voters := ((tbl.find? (·.1 == (v.data, v.view))).map (·.2)).getD []
      -- The payload is rebuilt from the shares the honest voters send, however the
      -- certificate travels (`Synchrony.lockSpread`).
      let held ← boundaryHeldFrom P now c
      if P.partialCerts && needsFaulty P v.data.epoch voters then
        sendPartly P now (fun _ => true) fun _ => .certificate1 c
      else
        sendTo P now (fun _ => true) (fun _ => .certificate1 c) held
      if let some b := s.blocks.find? (blockHash · == v.data.blockHash) then
        sendTo P now (P.isMember v.data.epoch) (fun _ => .blockReconstructed b.viewNumber b.payloadCommit)
          held
        spreadBlock P now b
  | .send (.vote2 v) => do
    if P.behaviour k v.data.epoch == some .selective && (← rand 2) == 0 then return
    let s ← get
    let (tbl, done) := tally P (v.data, v.view) v.data.epoch k s.vote2s
    set { s with vote2s := tbl }
    if done then
      let c2 : Cert2 := ⟨v.data, v.view⟩
      let voters := ((tbl.find? (·.1 == (v.data, v.view))).map (·.2)).getD []
      if P.partialCerts && needsFaulty P v.data.epoch voters then
        sendPartly P now (fun _ => true) fun _ => .certificate2 c2
      else
        sendTo P now (fun _ => true) fun _ => .certificate2 c2
      if let some b := s.blocks.find? (blockHash · == v.data.blockHash) then
        if decide (IsLastBlock b.blockHeader.blockNumber P.epochHeight) then
          if let some c1 := s.cert1s.find? (fun c => c.data.toVote2 == v.data && c.view == b.viewNumber) then
            modify fun s => { s with epochChanges := s.epochChanges + 1 }
            sendTo P now (fun _ => true) fun _ => .epochChange c1 c2 b
  | .send (.timeoutVote tv0) => do
    let bh := P.behaviour k tv0.data.epoch
    if bh == some .selective && (← rand 2) == 0 then return
    let tv := if bh == some .lowLock then { tv0 with data := { tv0.data with lock := P.cfg.anchorCert } } else tv0
    let s ← get
    let key := (tv.view, tv.data.epoch)
    let old := (s.timeouts.find? (·.1 == key)).map (·.2) |>.getD []
    let new := if old.any (·.1 == k) then old else (k, tv.data.lock) :: old
    let count (l : List (PubKey × Cert1)) := (l.filter fun x => P.isMember tv.data.epoch x.1).length
    set { s with timeouts := (key, new) :: s.timeouts.filter (·.1 != key) }
    if count old < P.quorum && count new ≥ P.quorum then
      let tc : TimeoutCert := ⟨⟨tv.data.epoch, (new.map (·.2)).foldl later P.cfg.anchorCert⟩, tv.view⟩
      let low : TimeoutCert := ⟨⟨tv.data.epoch, lowestLock P tv.data.epoch new⟩, tv.view⟩
      modify fun s => { s with timeoutCerts := s.timeoutCerts + 1 }
      if P.partialCerts && needsFaulty P tv.data.epoch (new.map (·.1)) then
        sendPartly P now (fun _ => true) fun _ => .timeoutCertificate tc
      else if P.lowTimeoutLocks then
        for k' in P.running do
          let t ← arrival P now k'
          let pickLow := (← rand 2) == 0
          push t k' (.timeoutCertificate (if pickLow then low else tc))
      else
        sendTo P now (fun _ => true) fun _ => .timeoutCertificate tc
    let s ← get
    let oldH := (s.oneHonest.find? (·.1 == tv.view)).map (·.2) |>.getD []
    let newH := if oldH.contains k then oldH else k :: oldH
    set { s with oneHonest := (tv.view, newH) :: s.oneHonest.filter (·.1 != tv.view) }
    if oldH.length ≤ P.faultBound && newH.length > P.faultBound then
      sendTo P now (fun _ => true) fun _ => .timeoutOneHonest tv.view
  | .send _ => pure ()
  | .decided blocks _ _ =>
    let top := blocks.foldl (fun m b => max m b.blockHeader.blockNumber.toNat) 0
    let mine := P.checked.contains k
    modify fun s =>
      { s with decided := blocks.map (k, ·) ++ s.decided, height := if mine then max s.height top else s.height }

/--
What node `k`, faulty in the epoch of input `i`, sends besides what the machine
does: a double-voter votes for every block and `Cert1` it gets, and a node that
times out early does so on entering a view.
-/
def misbehaveOn (now : Nat) (k : PubKey) (i : Input) (h' : History) (entered : Bool) : SimM Unit := do
  match i with
  | .proposal _ p _ =>
    if P.behaviour k p.epoch == some .doubleVote && P.isMember p.epoch k then
      handle P now k (.send (.vote1 ⟨⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩, p.viewNumber, k⟩))
  | .certificate1 c =>
    if P.behaviour k c.data.epoch == some .doubleVote && P.isMember c.data.epoch k then
      handle P now k (.send (.vote2 ⟨c.data.toVote2, c.view, k⟩))
  | _ => pure ()
  let e := epochOfHistory P.cfg h'
  if entered && P.behaviour k e == some .lowLock && P.isMember e k then
    handle P now k (.send (.timeoutVote ⟨⟨e, P.cfg.anchorCert⟩, viewOf P.cfg h', k⟩))

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
  modify fun s => { s with log := s.log.modify (k.toNat - 1) ((now, timer, i) :: ·) }
  let st := step P.cfg P.leader k h i
  let h' := h ++ [st]
  modify fun s => { s with hist := s.hist.set! (k.toNat - 1) h' }
  for o in st.output do
    unless P.lost k o do handle P now k o
  let v1 := viewOf P.cfg h'
  misbehaveOn P now k i h' (v1 != v0)
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
      if e.time > P.horizon || (P.stopHeight > 0 && s.height ≥ P.stopHeight) then pure ()
      else
        set { s with queue := rest }
        stepNode P e.time e.node e.input e.timer
        loop fuel

/-- The state at the end of a run. -/
def run : State :=
  ((start P *> loop P 1000000).run { rng := P.seed, hist := Array.replicate P.nodes [], log := Array.replicate P.nodes [] }).2

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
