module

public import Std.Data.TreeMap
public import Std.Data.TreeSet
public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface

/-!
# The protocol

The operational definition of the core consensus protocol: the state of one
participant (`Impl.State`) and the transition function
(`Impl.next`), mapping the current state and one input event to the
successor state and the outputs to emit.

This is **one implementation, not the specification**. The contract lives
entirely in `NewProtocolSpec`, which does not refer to this module; the
machine is here for three reasons:

* it *witnesses* that the specification is satisfiable — obligations written
  independently of one another can turn out to be jointly unachievable, and
  `NewProtocolImpl.Conformance` proves these are not;
* it is executable, so it can be driven against the production
  implementation in differential tests — with one caveat: `blockHash` is an
  `opaque` constant, which at runtime evaluates to a single fixed value, so
  every hash comparison holds vacuously. Real execution needs it made a
  parameter first;
* it is a worked example of one way to discharge the obligations.

Where the specification leaves freedom, the choices here are the machine's
own: the eager reaction pass (after recording an input it re-examines every
view its own tables have content at), the representation and the output
order. None of that is normative.

What the machine does *not* do is ask its own subsystems for anything — no
request for a block to propose, for a validity verdict, for a dispersal, or
for the view timer. Those are outside the specification's outputs
(`Output`), so a real node must issue them somehow; the machine simply
waits to be told, which is all conformance asks of it.

The leader's own proposal is not special-cased: it is broadcast and comes
back as an `Input.proposal` like anyone else's.

**Bootstrap.** `initial` holds the configured anchor block and certificate,
so view 1 already has a parent to extend and a certificate to name. What it
lacks is a reason to act: feed `Input.advanceView` with `cfg.anchorCert` to
move the node into view 1, then `Input.headerBuilt` to give its leader
something to propose.
-/

@[expose] public section

open Std (TreeMap TreeSet)

namespace NewProtocol

-- Lexicographic order on pairs, for tree keys.
instance [Ord α] [Ord β] : Ord (α × β) := lexOrd

namespace Impl

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
The protocol state.

One concrete representation of the specification's `NodeState`; see
`NewProtocolImpl.Conformance` for the correspondence. Every field is
finite, which is what lets the reaction pass of `next` enumerate the views
an action could be owed at instead of guessing which ones the input woke.
-/
structure State where
  /-- Proposals we hold, by view; the ancestry the decide walk runs on. -/
  proposals : TreeMap ViewNumber Proposal

  /-- Proposals that passed the admission rule; the only ones we vote on. -/
  admitted : TreeMap ViewNumber Proposal

  /-- Views we proposed in. -/
  proposedViews : TreeSet ViewNumber

  /-- Our VID share per view. -/
  vidShares : TreeMap ViewNumber VidShare

  /-- Block hashes reported valid, by view. -/
  validated : TreeMap ViewNumber BlockHash

  /-- Payloads reconstructed from VID shares (or dispersed by us). -/
  blocksReconstructed : TreeSet (ViewNumber × PayloadCommit)

  /-- Block headers available to propose, keyed by (view, parent block hash). -/
  headers : TreeMap (ViewNumber × BlockHash) BlockHeader

  /-- `Cert1`s by view. -/
  cert1s : TreeMap ViewNumber Cert1

  /-- `Cert2`s by view. -/
  cert2s : TreeMap ViewNumber Cert2

  /-- Timeout certificates, keyed by the view they advanced *into*. -/
  timeoutCerts : TreeMap ViewNumber TimeoutCert

  /-- The locked certificate. -/
  lockedCert : Option Cert1

  /-- Views decided (emitted in an `Output.decided`). -/
  decidedViews : TreeSet ViewNumber

  /-- Views we cast a vote1 in. -/
  voted1Views : TreeSet ViewNumber

  /-- The view each vote1's proposal was justified at, by view voted in. -/
  vote1Branches : TreeMap ViewNumber ViewNumber

  /-- Views we cast a vote2 in. -/
  voted2Views : TreeSet ViewNumber

  /-- Highest view that timed out; no vote1 and no proposal at or below it. -/
  timeoutView : ViewNumber

  /-- Views at or below this are abandoned; see `NodeState.barredView`. -/
  barredView : ViewNumber

  /-- The view we are currently in. -/
  currentView : ViewNumber

/--
The state of a fresh node starting from the configured anchor.

The anchor block and its certificate come with the configuration rather than
over the wire; see `NodeState.initial`, which this mirrors.
-/
def initial : State where
  proposals := TreeMap.empty.insert ViewNumber.genesis cfg.anchorBlock
  admitted := TreeMap.empty
  proposedViews := TreeSet.empty
  vidShares := TreeMap.empty
  validated := TreeMap.empty
  blocksReconstructed := TreeSet.empty
  headers := TreeMap.empty
  cert1s := TreeMap.empty.insert ViewNumber.genesis cfg.anchorCert
  cert2s := TreeMap.empty
  timeoutCerts := TreeMap.empty
  lockedCert := none
  decidedViews := TreeSet.empty.insert ViewNumber.genesis
  voted1Views := TreeSet.empty
  vote1Branches := TreeMap.empty
  voted2Views := TreeSet.empty
  timeoutView := ViewNumber.genesis
  barredView := ViewNumber.genesis
  currentView := ViewNumber.genesis

/-! ## Derived values -/

/-- The highest decided view. -/
def State.lastDecided (s : State) : ViewNumber :=
  s.decidedViews.max?.getD ViewNumber.genesis

/-- The decide floor: views at or below `lastDecided - decideBuffer` can no longer decide. -/
def State.floor (s : State) : ViewNumber :=
  s.lastDecided - cfg.decideBuffer

/-- Whether `v` is above the decide floor. -/
def State.aboveFloor (s : State) (v : ViewNumber) : Bool :=
  s.floor cfg < v

/--
A vote1 in a later view endorsed a branch holding no block at `v`.

The decidable counterpart of `Vote1SkippedView`: a vote at `w > v` whose
proposal was justified below `v` skipped over `v`, so committing `v` afterwards
would put this node on both sides.

Folded over the tree rather than over `toList`: the vote2 round asks this once
per admitted view, and building the entry list each time allocates a copy of the
whole record set per question.
-/
def State.vote1Skipped (s : State) (v : ViewNumber) : Bool :=
  s.vote1Branches.foldl (fun found w u => found || (v < w && u < v)) false

/-- Whether the held lock (if any) sits below view `v`. -/
def State.lockBelow (s : State) (v : ViewNumber) : Bool :=
  match s.lockedCert with
  | some l => decide (l.view < v)
  | none => true

/--
The highest certificate we hold: the locked certificate or the newest
timeout certificate, whichever is newer (ties go to the timeout
certificate).
-/
def State.catchupEvidence (s : State) : Option CatchupEvidence :=
  match s.timeoutCerts.maxEntry?, s.lockedCert with
  | some (_, tc), some qc =>
    if tc.view < qc.view then some (.cert1 qc) else some (.timeout tc)
  | some (_, tc), none => some (.timeout tc)
  | none, some qc => some (.cert1 qc)
  | none, none => none

/-- Whether the payload of the block held at `v` was reconstructed. -/
def State.reconstructed (s : State) (v : ViewNumber) (p : Proposal) : Bool :=
  s.blocksReconstructed.contains (v, p.payloadCommit)

/-- Whether we hold exactly the block `h` at view `v`. -/
def State.holds (s : State) (v : ViewNumber) (h : BlockHash) : Bool :=
  match s.proposals.get? v with
  | some q => blockHash q == h
  | none => false

/--
Whether `p`'s parent link is established: the parent proposal is held,
`p.parentCert` certifies exactly it, and its block is reconstructed.
Genesis parents are exempt.
-/
def State.parentLinked (s : State) (p : Proposal) : Bool :=
  p.parentCert.view == ViewNumber.genesis ||
    match s.proposals.get? p.parentCert.view with
    | some parent =>
      p.parentCert.data.blockHash == blockHash parent
        && s.reconstructed p.parentCert.view parent
    | none => false

/-! ## Admission checks

Decidable forms of the specification's admission rules, all of them gathered
in `State.admits`. The configured anchor is exempt: it is placed in
`proposals` by `initial` rather than arriving, and has no parent to point
back to.

This machine holds only what it admits, which is less than `NodeState`
permits: the specification lets a node keep an arrival that failed the guards
as ancestry, and that is where a fetched block would go in an implementation
that fetches. Keeping the two maps apart still earns its place here, because
collection cuts them at different watermarks — `admitted` at the bar,
`proposals` at the decide floor, which is lower.
-/

/-- Decidable form of `ProposalWellFormed`. -/
def wellFormed (p : Proposal) : Bool :=
  p.parentCert.view < p.viewNumber
    && (p.parentCert.view + 1 == p.viewNumber
        || match p.timeoutEvidence with
           | some tc => tc.view + 1 == p.viewNumber
           | none => false)

/-- Decidable form of `ShareMatches`. -/
def shareMatches (p : Proposal) (vid : VidShare) : Bool :=
  vid.view == p.viewNumber && p.payloadCommit == vid.payloadCommit

/-- Decidable form of the admission rule `SafeToExtend`. -/
def safeToExtend (locked : Option Cert1) (p : Proposal) : Bool :=
  match locked with
  | none => true
  | some lock =>
    if lock.view = p.viewNumber then
      lock.data.blockHash == blockHash p
    else
      p.parentCert.data == lock.data || lock.view < p.parentCert.view

/-- Whether writing `x` into the slot `o` loses nothing: the decidable `Writable`. -/
def writable {α : Type} [DecidableEq α] (o : Option α) (x : α) : Bool :=
  o == none || o == some x

/--
Whether an arriving proposal is admitted: the admission rule, on a view we
have not abandoned, into slots holding nothing else.

The slot conditions are what keeps the state append-only above the decide
floor. A proposal whose view already holds a *different* proposal, share or
admission is declined rather than overwritten — `StepSpec.contentRetained`
forbids the overwrite, and `StepSpec.proposalIngested` asks for nothing in that
case, its `Writable` guards failing.
-/
def State.admits (s : State) (p : Proposal) (vid : VidShare) : Bool :=
  s.barredView < p.viewNumber
    && writable (s.admitted.get? p.viewNumber) p
    && writable (s.proposals.get? p.viewNumber) p
    && writable (s.vidShares.get? p.viewNumber) vid
    && wellFormed p
    && shareMatches p vid
    && safeToExtend s.lockedCert p

/-!
## Slot discipline

Above the decide floor the specification's state is append-only, so each arm
of `handle` writes a slot only when it is free — or already holds the value
being written, which is the same thing. No arm ever clears one.

## Actions

Each action fires iff its gates hold, marks itself done, and emits its
outputs; re-running an action is always harmless. `next` re-runs them
eagerly after every input, over the views its tables have content at.
-/

/-- A state transformation emitting outputs. -/
abbrev StepFn := State → State × List Output

/-- Run `fs` left to right, accumulating outputs. -/
def seq : List StepFn → StepFn
  | [], s => (s, [])
  | f :: fs, s =>
    let r := f s
    let r' := seq fs r.1
    (r'.1, r.2 ++ r'.2)

/-! ### The lock

The lock moves before any vote is cast, and to the highest certificate the
state licenses rather than to the one a particular vote needs. Both are
forced by the shape of the obligations, which read the lock as it stands at
the *end* of the step: `Vote1Justification.safeToExtend` judges a vote1's
proposal against the lock, and `SafetySpec.vote2LockOrdered` requires the lock
to have reached the view of every vote2. Deciding where the lock ends
up first is what lets both rounds test the lock they will be judged against —
and taking the highest licensed certificate removes the circularity of
choosing the lock by which vote2s are due while the vote2s wait
on the lock.
-/

/-- The certificate at `v` we may lock on: over the admitted, reconstructed block there. -/
def State.lockable (s : State) (v : ViewNumber) : Option Cert1 :=
  match s.cert1s.get? v, s.admitted.get? v with
  | some c, some p =>
    if c.data.blockHash = blockHash p ∧ s.reconstructed v p then some c else none
  | _, _ => none

/-- The newer of two candidate certificates. -/
def better (a : Option Cert1) (c : Cert1) : Cert1 :=
  match a with
  | some b => if b.view < c.view then c else b
  | none => c

/-- The highest certificate the state licenses a lock on. -/
def State.bestLock (s : State) : Option Cert1 :=
  s.admitted.keys.foldl (fun best v =>
    match s.lockable v with
    | some c => some (better best c)
    | none => best) none

/--
Move the lock to `State.bestLock` if that is newer than the lock we hold,
enter the view past it, and broadcast the certificate.
-/
def advanceLock : StepFn := fun s =>
  match s.bestLock with
  | some c =>
    if s.lockBelow c.view then
      ({ s with lockedCert := some c, currentView := max s.currentView (c.view + 1) },
       [.send (.cert1 c)])
    else (s, [])
  | none => (s, [])

/-! ### Votes -/

/--
Cast the vote1 for view `v` if it is fresh and justified: above both bars, an
admitted proposal with our VID share, its state transition validated, and its
parent linked. The vote is accompanied by our VID share so peers can reconstruct
the block.

`safeToExtend` is re-checked against the lock as it stands now, not as it stood
when the proposal was admitted: the lock may have moved since, onto a view this
branch skips. Where the lock has reached `v` itself that check passes on a
commitment match, and nothing here refuses the vote on that ground —
`NewProtocolSpec.Network` derives safety from the order votes were cast in
rather than from a bar on this one, so the vote is permitted and adds nothing.
-/
def tryVote1 (v : ViewNumber) : StepFn := fun s =>
  if v ≤ s.timeoutView ∨ v ≤ s.barredView ∨ s.voted1Views.contains v
      then (s, []) else
  match s.admitted.get? v, s.vidShares.get? v with
  | some p, some share =>
    if s.validated.get? v = some (blockHash p) ∧ s.parentLinked p
        ∧ safeToExtend s.lockedCert p then
      ({ s with voted1Views := s.voted1Views.insert v,
                vote1Branches := s.vote1Branches.insert v p.parentCert.view },
       [.send (.vote1 ⟨⟨blockHash p⟩, v, node⟩), .send (.vidShare share)])
    else (s, [])
  | _, _ => (s, [])

/--
Cast the vote2 for view `v`.

Requires a `Cert1` over exactly the admitted proposal, the
reconstructed block, and the lock already at `v` or beyond — which
`advanceLock` has seen to. Freshness is the rest: not yet voted, no `Cert2`
for `v`, `v` undecided, above the floor and above the bar, and not a view an
earlier vote1 skipped.
-/
def tryVote2 (v : ViewNumber) : StepFn := fun s =>
  if v ≤ s.barredView ∨ s.voted2Views.contains v ∨ (s.cert2s.get? v).isSome
      ∨ s.decidedViews.contains v ∨ ¬ s.aboveFloor cfg v ∨ s.vote1Skipped v
      ∨ s.lockBelow v then (s, []) else
  match s.lockable v with
  | some _ =>
    match s.admitted.get? v with
    | some p =>
      ({ s with voted2Views := s.voted2Views.insert v },
       [.send (.vote2 ⟨⟨blockHash p⟩, v, node⟩)])
    | none => (s, [])
  | none => (s, [])

/-! ### Deciding -/

/--
Walk the undecided ancestors of the block with hash `h` at view `v`, oldest
last, stopping at the decide `floor`, at a view `settled` before this step, at
a missing or mismatching proposal — or when the fuel runs out.

`settled` is the decided set as it was when the step began, not as it stands
mid-pass: `StepSpec.decideJustified` asks that the chain reach ground that was
already settled *before* the step, so a view this very step decided is no
stopping place.

The fuel cannot run out first. Every proposal that arrives is well-formed, so
each step strictly decreases the view, and the walk stops at genesis, which
is decided from the start. The anchor sitting there is never stepped through,
which is why it need not be well-formed itself.
-/
def State.chainFrom (s : State) (settled : TreeSet ViewNumber) (floor : ViewNumber) :
    Nat → BlockHash → ViewNumber → List Block
  | 0, _, _ => []
  | fuel + 1, h, v =>
    if v ≤ floor ∨ settled.contains v then [] else
    match s.proposals.get? v with
    | some q =>
      if blockHash q = h then
        q :: s.chainFrom settled floor fuel q.parentCert.data.blockHash q.parentCert.view
      else []
    | none => []

/--
The decide floor implied by a set of decided views.

The decide round is judged against the views decided when the round began, and
derives the floor from them rather than reading the floor it is standing on: a
decide earlier in the same round moves both, and a chain may not claim as
settled the ground the round itself has just laid.
-/
def floorOf (cfg : Config) (settled : TreeSet ViewNumber) : ViewNumber :=
  (settled.max?.getD ViewNumber.genesis) - cfg.decideBuffer

/-- The chain a decide of the block `p` would deliver: `p`, then its undecided ancestors. -/
def State.decideChain (s : State) (settled : TreeSet ViewNumber) (floor : ViewNumber)
    (p : Proposal) : List Block :=
  p :: s.chainFrom settled floor p.parentCert.view.toNat p.parentCert.data.blockHash
    p.parentCert.view

/--
Decide view `v` if it is fresh and justified: above the floor, a `Cert2`
over exactly the proposal we hold, and a `Cert1` present. Emits the newest
block together with its undecided ancestors, and marks them all decided.

The chain is whatever the walk reaches: a missing or mismatching ancestor
truncates it and the view it names is skipped, not waited for
(`StepSpec.decideJustified` — the stream promises no completeness). A skipped
view is delivered later only if a `Cert2` of its own arrives, which is this
very function at that view.

`settled` is the round's watermark, so every decide of one round is judged
against the same ground.
-/
def tryDecide (settled : TreeSet ViewNumber) (v : ViewNumber) : StepFn := fun s =>
  if s.decidedViews.contains v ∨ ¬ floorOf cfg settled < v then (s, []) else
  match s.cert2s.get? v, s.cert1s.get? v, s.proposals.get? v with
  | some c2, some c1, some p =>
    if c2.data.blockHash = blockHash p then
      ({ s with
           decidedViews :=
             (s.decideChain settled (floorOf cfg settled) p).foldl
               (fun d b => d.insert b.viewNumber) s.decidedViews },
       [Output.decided (s.decideChain settled (floorOf cfg settled) p) c1 c2])
    else (s, [])
  | _, _, _ => (s, [])

/-! ### Proposing

`ProposalJustification` pins a proposal down once its view is fixed: the
parent certificate is the lock (after a timeout) or the certificate of the
preceding view, the certificate names the parent block, and the headers content
supplies the header. So a view admits at most the two candidates below, and
trying both is a complete search.
-/

/--
The identity a proposal we build ourselves carries.

We cannot know the real one: identities are assigned by hashing a serialised
form the machine does not construct. Nothing reads this either —
`tryPropose` emits the proposal without storing or hashing it, and every hash
the machine computes is of a block it *holds*, which arrived carrying a real
identity. A driver translating traces should keep real identities clear of this
value; reserving the odd naturals for them does it.
-/
def unassignedIdentity : BlockHash := ⟨0⟩

/-- Whether the parent block `parent` is the one `pcert` names. -/
def parentMatches (pcert : Cert1) (parent : Proposal) : Bool :=
  pcert.view == ViewNumber.genesis || blockHash parent == pcert.data.blockHash

/-- The proposal we would make at `v` on the timeout certificate for `v`. -/
def State.timeoutCandidate (s : State) (v : ViewNumber) : Option Proposal :=
  (s.timeoutCerts.get? v).bind fun tc =>
    s.lockedCert.bind fun pcert =>
      if pcert.view < v then
        (s.proposals.get? pcert.view).bind fun parent =>
          if parentMatches pcert parent then
            (s.headers.get? (v, blockHash parent)).map fun h =>
              ⟨h, v, pcert, some tc, unassignedIdentity⟩
          else none
      else none

/-- The proposal we would make at `v` extending the immediately preceding view. -/
def State.normalCandidate (s : State) (v : ViewNumber) : Option Proposal :=
  (s.cert1s.get? (v - 1)).bind fun pcert =>
    if pcert.view + 1 = v then
      (s.proposals.get? pcert.view).bind fun parent =>
        if parentMatches pcert parent then
          (s.headers.get? (v, blockHash parent)).map fun h =>
            ⟨h, v, pcert, none, unassignedIdentity⟩
        else none
    else none

/--
Propose in view `v` if we lead it, have not proposed yet, and `v` is above
both bars.
-/
def tryPropose (v : ViewNumber) : StepFn := fun s =>
  if v ≤ s.timeoutView ∨ v ≤ s.barredView ∨ s.proposedViews.contains v
      ∨ leader v ≠ some node then (s, []) else
  match (s.timeoutCandidate v).or (s.normalCandidate v) with
  | some p => ({ s with proposedViews := s.proposedViews.insert v }, [.send (.proposal p)])
  | none => (s, [])

/-! ## Collection

Pruning is a transition of its own, not part of `next`: it consumes no
input and emits nothing. See `GcSpec`.
-/

/--
Prune everything the node can no longer act on.

Two watermarks, because the vote path and the decide path go stale at
different times. The bar moves to just below the view we are in — entering a
view is evidence the earlier ones are settled — and takes the vote path and
the vote and proposal marks with it. The decide path is cut at the decide
floor instead, since a late `Cert2` can still decide an older view.

`decidedViews` is cut at the floor *inclusively*: the floor is derived from
the highest decided view, so dropping that view would move the floor down and
bring abandoned views back into scope, which `GcSpec.floorStable` forbids.
Keeping the boundary keeps the highest decided view, whatever the buffer.

`max` with the old bar keeps it monotone even if the machine is somehow
behind its own bar; the guard on `currentView = genesis` is what stops a
fresh node barring genesis itself.
-/
def State.gc (s : State) : State :=
  let bar := max s.barredView (s.currentView - 1)
  let floor := s.floor cfg
  { s with
      barredView := bar

      -- The vote path, and the marks for actions that can no longer be taken.
      admitted := s.admitted.filter fun v _ => bar < v
      vidShares := s.vidShares.filter fun v _ => bar < v
      validated := s.validated.filter fun v _ => bar < v
      headers := s.headers.filter fun k _ => bar < k.1
      timeoutCerts := s.timeoutCerts.filter fun v _ => bar < v
      voted1Views := s.voted1Views.filter fun v => bar < v
      voted2Views := s.voted2Views.filter fun v => floor < v
      proposedViews := s.proposedViews.filter fun v => bar < v

      -- The decide path, which outlives the views it belongs to. `proposals` is
      -- cut at the lower of the two watermarks: the vote path keeps `admitted`
      -- above the bar, and every admitted proposal must remain held.
      proposals := s.proposals.filter fun v _ => min floor bar < v
      cert1s := s.cert1s.filter fun v _ => floor < v
      cert2s := s.cert2s.filter fun v _ => floor < v
      blocksReconstructed := s.blocksReconstructed.filter fun k => floor < k.1
      decidedViews := s.decidedViews.filter fun v => floor ≤ v
      vote1Branches := s.vote1Branches.filter fun v _ => floor < v }

/-! ## The transition function -/

/--
Record one input and emit its immediate outputs.

Everything an input *enables* — votes, decides, proposals — is left to the
reaction pass in `next`.
-/
def handle (input : Input) : StepFn := fun s =>
  match input with
  | .blockReconstructed v pc =>
    ({ s with blocksReconstructed := s.blocksReconstructed.insert (v, pc) }, [])

  | .certificate1 c => if s.aboveFloor cfg c.view ∧ ¬ s.cert1s.contains c.view
      then ({ s with cert1s := s.cert1s.insert c.view c }, [])
      else (s, [])

  | .certificate2 c => if s.aboveFloor cfg c.view ∧ ¬ s.cert2s.contains c.view
      then (
        { s with cert2s := s.cert2s.insert c.view c },
        if s.decidedViews.contains c.view then [] else [.send (.cert2 c)]
      )
      else (s, [])

  | .advanceView c =>
    -- The certificate is recorded like any other; the view advance is what
    -- distinguishes this input, and it happens whether or not it is new. Both
    -- the guard and the parent lookup read the state as it arrived: recording a
    -- certificate touches neither the view we are in nor the proposals we hold.
    let recorded := if s.aboveFloor cfg c.view ∧ ¬ s.cert1s.contains c.view
      then { s with cert1s := s.cert1s.insert c.view c }
      else s
    ({ recorded with currentView := max s.currentView (c.view + 1) }, [])

  | .headerBuilt v parent h =>
    if s.headers.contains (v, parent) then (s, [])
    else ({ s with headers := s.headers.insert (v, parent) h }, [])

  | .proposal _sender p vid =>
    let v := p.viewNumber
    if s.admits p vid then
      ({ s with
          proposals := s.proposals.insert v p
          admitted := s.admitted.insert v p
          vidShares := s.vidShares.insert v vid }, [])
    else (s, [])

  | .blockValidated v h =>
    if s.validated.contains v then (s, [])
    else ({ s with validated := s.validated.insert v h }, [])

  -- A local timer fires for the view we are in; a stale one is ignored, and
  -- one naming a view we have not reached is not ours to answer.
  | .timeout v =>
    if v ≠ s.currentView then (s, []) else
    ({ s with timeoutView := max s.timeoutView v },
     [.send (.timeoutVote ⟨(), v, node⟩ s.catchupEvidence)])

  -- Joining on the one-honest threshold: others have timed out already, so
  -- the view may be ahead of us.
  | .timeoutOneHonest v =>
    if v < s.currentView then (s, []) else
    ({ s with timeoutView := max s.timeoutView v },
     [.send (.timeoutVote ⟨(), v, node⟩ s.catchupEvidence)])

  | .timeoutCertificate tc =>
    let v := tc.view + 1
    -- The view advance is unconditional, so that a duplicate certificate
    -- can never leave us behind the view it justifies; the certificate itself
    -- is only recorded if it is new and we have not already left the view.
    if v < s.currentView ∨ s.timeoutCerts.contains v then
      ({ s with currentView := max s.currentView v }, [])
    else
    ({ s with timeoutCerts := s.timeoutCerts.insert v tc,
              currentView := max s.currentView v },
     [.send (.timeoutCert tc v)])

/--
The reaction pass: every action the state can owe, in the order the
obligations force.

Decides come first, because deciding raises the decide floor and a vote2
vote may not be cast below it (`SafetySpec.vote2AboveFloor`, which reads the
floor as it stands at the end of the step). The lock moves next, so that both
vote rounds see the lock they will be judged against. Vote1s precede
vote2s, because a vote1 that skips a view withdraws the vote2
vote there (`SafetySpec.vote2NotInSkippedView`) while the converse is not true — and
proposals come last, over the settled lock.

Each round ranges over the views its own table has entries at, which is every
view the action could be owed in: a vote needs an admitted proposal, a decide
a `Cert2`, a proposal a headers header. Scanning those keys is a complete
search, and one pass over them suffices: no attempt can make a view that an
earlier attempt passed over become owed, since deciding only raises the floor
and only marks views decided, and both of those retire opportunities rather
than creating them.

The decide round's watermark is `s.decidedViews`, read once here: every
attempt of the round is judged against the ground the round started on.
-/
def rounds (s : State) : List StepFn :=
  s.cert2s.keys.map (tryDecide cfg s.decidedViews)
    ++ [advanceLock]
    ++ s.admitted.keys.map (tryVote1 node)
    ++ s.admitted.keys.map (tryVote2 cfg node)
    ++ s.headers.keys.map fun k => tryPropose leader node k.1

/--
The transition function: record the input, then take every action the new
state owes.

Nothing is left owed at the end of a step, which is how the machine
discharges the progress half of conformance; see
`NewProtocolImpl.Conformance`.
-/
def next (s : State) (input : Input) : State × List Output :=
  let taken := handle cfg node input s
  let reacted := seq (rounds cfg leader node taken.1) taken.1
  (reacted.1, taken.2 ++ reacted.2)

end Impl

end NewProtocol
