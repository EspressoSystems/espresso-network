module

public import NewProtocolDiff.Trace
public import NewProtocolDiff.Corpus
public import NewProtocolDiff.PromptCheck
public import NewProtocolDiff.Check
public import NewProtocolDiff.ProtocolCheck
public import NewProtocolDiff.Coverage
public import NewProtocolDiff.Header
public meta import NewProtocolSpec.Interface
public meta import NewProtocolDiff.Json
public meta import NewProtocolDiff.Trace
public meta import NewProtocolDiff.Corpus
public meta import NewProtocolDiff.PromptCheck
public meta import NewProtocolDiff.Check
public meta import NewProtocolDiff.ProtocolCheck
public meta import NewProtocolDiff.Coverage
public meta import NewProtocolDiff.Header

/-!
# Checks on the harness itself

A comparison harness that silently mis-reads its input is worse than none, so
the format is checked at build time in both directions:

* the identity injection does not collide and preserves the recorder's text, and
  every constructor round-trips through JSON — which catches an instance the
  deriving got wrong as well as one a future hand-written override breaks;
* the step check passes a node that votes, and answers its timers, in the step it
  owes them in, or holds them back for storage there, and fails one that does not;
* the signing rules refuse a vote whose identity does not match the block, since
  that is how a hash-namespace mistake would show up;
* a timeout vote needs a timer for its view at or before it, and names the lock
  the node held at that timer.
-/

@[expose] public section

namespace NewProtocolDiff
namespace Tests

open Lean NewProtocol

/-! ## Identities

The injection a recorder's strings go through. Checked on the shapes a recorder
actually emits — base64 and hex of a 32-byte value — and on the property that
matters, which is that different text never collides, including across lengths
and outside ASCII.
-/

private def identityTexts : List String :=
  [ "", "A", "AA", "AAA", "aA", "Aa", "0", "/", "+", "café",
    "3q2+7wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    "deadbeef00000000000000000000000000000000000000000000000000000000" ]

/-- info: true -/
#guard_msgs in
#eval
  let nats := identityTexts.map identToNat
  (nats.eraseDups).length == identityTexts.length

-- Text in, the same text out: a trace this package re-renders is the one it read.
/-- info: true -/
#guard_msgs in
#eval identityTexts.all fun t => natToIdent (identToNat t) == some t

-- A number naming no string still survives, going out as a number instead.
/-- info: true -/
#guard_msgs in
#eval
  let n := 2 ^ 200
  match natToIdent n with
  | some s => identToNat s == n
  | none => match cryptoFromJson (cryptoToJson n) with
    | .ok m => m == n
    | .error _ => false

/-! ## Round trip -/

/-- A lock for the timeout votes and certificates below. -/
private def lk : Cert1 := ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩

private def prop1 : Proposal :=
  ⟨⟨⟨42⟩, 1⟩, ⟨1⟩, ⟨0⟩, ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨3⟩⟩

private def prop2 : Proposal :=
  ⟨⟨⟨8⟩, 2⟩, ⟨5⟩, ⟨0⟩, ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩, some ⟨⟨⟨0⟩, lk⟩, ⟨4⟩⟩, ⟨9⟩⟩

/-- Every input constructor, with `timeoutEvidence` present and absent. -/
private def inputs : List Input :=
  [ .blockReconstructed ⟨1⟩ ⟨42⟩,
    .certificate1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩,
    .certificate2 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩,
    .headerBuilt ⟨2⟩ ⟨3⟩ ⟨⟨7⟩, 2⟩,
    .proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩),
    .proposal ⟨2⟩ prop2 (some ⟨⟨5⟩, ⟨8⟩⟩),
    .blockValidated ⟨1⟩ ⟨3⟩,
    .timeout ⟨4⟩,
    .timeoutCertificate ⟨⟨⟨0⟩, lk⟩, ⟨4⟩⟩,
    .timeoutOneHonest ⟨6⟩,
    .epochChange ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ prop2,
    .revote ⟨1⟩ ⟨⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩, ⟨6⟩, none⟩,
    .revote ⟨1⟩ ⟨⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩, ⟨7⟩, some ⟨⟨⟨1⟩, lk⟩, ⟨6⟩⟩⟩ ]

/-- Every output constructor. -/
private def outputs : List Output :=
  [ .send (.proposal prop1),
    .send (.vote1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩, ⟨1⟩⟩),
    .send (.vote2 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩, ⟨1⟩⟩),
    .send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨4⟩, ⟨1⟩⟩),
    .send (.timeoutCert ⟨⟨⟨0⟩, lk⟩, ⟨4⟩⟩),
    .send (.cert1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩),
    .send (.cert2 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩),
    .send (.vidShare ⟨⟨1⟩, ⟨42⟩⟩),
    .send (.epochChange ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩ prop2),
    .send (.revote ⟨⟨⟨⟨9⟩, ⟨1⟩, 2⟩, ⟨5⟩⟩, ⟨7⟩, some ⟨⟨⟨1⟩, lk⟩, ⟨6⟩⟩⟩),
    .decided [] ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩ ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩,
    .decided [prop2, prop1] ⟨⟨⟨9⟩, ⟨0⟩, 2⟩, ⟨5⟩⟩ ⟨⟨⟨9⟩, ⟨0⟩, 2⟩, ⟨5⟩⟩ ]

/-- Through `toJson`, the text, and back — the whole path a trace file takes. -/
private def roundTrips {α : Type} [DecidableEq α] [ToJson α] [FromJson α]
    (xs : List α) : Bool :=
  xs.all fun x =>
    match Json.parse (toJson x).compress >>= (fromJson? · : Json → Except String α) with
    | .ok y => y == x
    | .error _ => false

private def inputRoundTrips : Bool := roundTrips inputs

private def outputRoundTrips : Bool := roundTrips outputs

/-- info: true -/
#guard_msgs in #eval inputRoundTrips

/-- info: true -/
#guard_msgs in #eval outputRoundTrips

/-- A whole trace survives writing and reading. -/
private def trace : List Event :=
  [ .consensus (.certificate1 ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩)) [.send (.vote1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩, ⟨1⟩⟩)],
    .consensus (.timeout ⟨4⟩)
      [.send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨4⟩, ⟨1⟩⟩), .send (.cert1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩)] ]

/-- info: true -/
#guard_msgs in
#eval match parseTrace (renderTrace trace) with
  | .ok es => es == trace
  | .error _ => false

/-- A 32-byte identity survives the JSON path, which a plain number could not. -/
private def wide : Proposal :=
  ⟨⟨⟨2 ^ 250 + 7⟩, 1⟩, ⟨1⟩, ⟨0⟩, ⟨⟨⟨2 ^ 255 - 1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨2 ^ 248 + 3⟩⟩

/-- info: true -/
#guard_msgs in #eval roundTrips [Input.proposal ⟨2 ^ 200⟩ wide (some ⟨⟨1⟩, ⟨2 ^ 250 + 7⟩⟩)]

-- A hand-written trace may use plain numbers where a recorder writes base64.
/-- info: true -/
#guard_msgs in
#eval match Json.parse "{\"data\":{\"blockHash\":3,\"epoch\":0,\"blockNumber\":1},\"view\":1}" >>=
    (fromJson? · : Json → Except String Cert1) with
  | .ok c => c == (⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩ : Cert1)
  | .error _ => false

/-! ## Acting promptly

A run that owes a vote1 once the block is validated, voting in the same step,
holding the vote back for storage, voting in a later step, or not at all. A vote with an identity that does not match the block ends the obligation
too, since a node votes once per view, so the signing rules are what refuse it: the
namespace mistake a structural hash would have caused everywhere.
-/

private def anchor : Block :=
  ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, ⟨0⟩, ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, none, ⟨1⟩⟩

private def cfg : Config := ⟨anchor, ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩, 20, 0⟩
private def me : PubKey := ⟨1⟩

private def opening : List Event :=
  [ .consensus (.certificate1 ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩)) [],
    .consensus (.blockReconstructed ViewNumber.genesis ⟨7⟩) [] ]

private def voteStep (votes : List Output) : Event :=
  .consensus (.blockValidated ⟨1⟩ ⟨3⟩) votes

private def vote (identity : BlockHash) : Output := .send (.vote1 ⟨⟨identity, ⟨0⟩, 1⟩, ⟨1⟩, me⟩)

/-- An action of `kind` for view `v` in epoch `e`, held back in step `k` for the writes `waits`. -/
private def hold (kind : HeldKind) (v e k : Nat) (waits : List Write := []) : HeldAt :=
  { kind, view := ⟨v⟩, epoch := ⟨e⟩, waits, step := k, line := k }

/--
What a run owes after a step it did not act in, with the step, given the actions
it held back.
-/
private def late (es : List Event) (held : List HeldAt := []) : List String :=
  (stepFaults cfg (fun _ _ => some me) me (historyOf es) [] held).map fun (m, o) =>
    match o with
    | .vote1 p => s!"vote1 in view {p.viewNumber.toNat} after step {m}"
    | _ => "another obligation"

/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep [vote ⟨3⟩]])

/-- info: ["vote1 in view 1 after step 3"] -/
#guard_msgs in #eval late (opening ++ [voteStep []])

-- A vote held back for storage is acted on in the step that held it back.
/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [hold .vote1 1 0 3]

-- A vote held back at the view in another epoch was another.
/-- info: ["vote1 in view 1 after step 3"] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [hold .vote1 1 1 3]

-- A vote held back before the obligation arose was another.
/-- info: ["vote1 in view 1 after step 3"] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [hold .vote1 1 0 2]

-- A vote cast on a later input is late, however soon that input came.
/-- info: ["vote1 in view 1 after step 3"] -/
#guard_msgs in #eval late (opening ++ [voteStep [], .consensus (.blockValidated ⟨9⟩ ⟨9⟩) [vote ⟨3⟩]])

/-- info: true -/
#guard_msgs in #eval checkSafe cfg me (historyOf (opening ++ [voteStep [vote ⟨3⟩]]))

/-- info: false -/
#guard_msgs in #eval checkSafe cfg me (historyOf (opening ++ [voteStep [vote ⟨999⟩]]))

/-- A child of `prop1`, on `prop1`'s certificate. -/
private def child : Proposal :=
  ⟨⟨⟨43⟩, 2⟩, ⟨2⟩, ⟨0⟩, ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩, none, ⟨5⟩⟩

/-- A vote1 on `child`, after the node holds `prop1`, and its payload if `withParentPayload`. -/
private def childVote (withParentPayload : Bool) : List Event :=
  opening
    ++ (if withParentPayload then [.consensus (.blockReconstructed ⟨1⟩ ⟨42⟩) []] else [])
    ++ [ .consensus (.certificate1 ⟨⟨⟨3⟩, ⟨0⟩, 1⟩, ⟨1⟩⟩) [],
         .consensus (.proposal ⟨1⟩ child (some ⟨⟨1⟩, ⟨43⟩⟩)) [],
         .consensus (.blockValidated ⟨2⟩ ⟨5⟩) [.send (.vote1 ⟨⟨⟨5⟩, ⟨0⟩, 2⟩, ⟨2⟩, me⟩)] ]

-- A vote1 needs the parent's payload, not only the parent (`ParentReady`).
/-- info: true -/
#guard_msgs in #eval checkSafe cfg me (historyOf (childVote true))

/-- info: false -/
#guard_msgs in #eval checkSafe cfg me (historyOf (childVote false))

-- A timeout vote held back for the view ends what the node owes in it.
/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [hold .timeout 1 0 3]

-- So does one held back for a later view.
/-- info: [] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [hold .timeout 2 0 3]

-- One held back for an earlier view does not.
/-- info: ["vote1 in view 1 after step 3"] -/
#guard_msgs in #eval late (opening ++ [voteStep []]) [hold .timeout 0 0 3]

/--
What a run still owes at its end and held back, though storage confirmed what
the action waited for, with the step that held it back.
-/
private def unreleased (es : List Event) (held : List HeldAt) (dropped : List DroppedAt := []) :
    List String :=
  let h := historyOf es
  (unreleasedIn (owedAfter cfg (fun _ _ => some me) me h) h.length held dropped).map fun (k, o) =>
    match o with
    | .vote1 p => s!"vote1 in view {p.viewNumber.toNat} held back in step {k}"
    | _ => "another obligation"

/-- A step that changes nothing the node owes. -/
private def idleStep : Event := .consensus (.blockReconstructed ⟨9⟩ ⟨8⟩) []

/-- The vote1 of view 1, held back in step 3 until storage confirms the vote action. -/
private def heldVote : HeldAt := hold .vote1 1 0 3 [.action ⟨1⟩ .vote]

/-- Storage confirming `w` on line `line`, after step 3. -/
private def stored (w : Write) (line : Nat) : DroppedAt := ⟨.stored w, [], 3, line⟩

-- A vote whose write storage has not confirmed is still under way, however long ago it was held back.
/-- info: [] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], idleStep]) [heldVote]

-- So is one waiting for another write than the one confirmed.
/-- info: [] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], idleStep]) [heldVote] [stored (.share ⟨1⟩) 4]

-- A confirmation before the vote was held back does not count.
/-- info: [] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], idleStep]) [heldVote] [stored (.action ⟨1⟩ .vote) 2]

-- One whose write was confirmed since, and which is still owed, was never released.
/-- info: ["vote1 in view 1 held back in step 3"] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], idleStep]) [heldVote] [stored (.action ⟨1⟩ .vote) 4]

-- One released since is no longer owed.
/-- info: [] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], .consensus (.blockReconstructed ⟨9⟩ ⟨8⟩) [vote ⟨3⟩]])
  [heldVote] [stored (.action ⟨1⟩ .vote) 4]

-- A lock at least as late as the one awaited confirms it. An earlier one does not.
#guard confirms (.lock ⟨5⟩ ⟨2⟩) (.lock ⟨5⟩ ⟨2⟩)
#guard confirms (.lock ⟨3⟩ ⟨3⟩) (.lock ⟨5⟩ ⟨2⟩)
#guard !confirms (.lock ⟨4⟩ ⟨2⟩) (.lock ⟨5⟩ ⟨2⟩)
#guard !confirms (.share ⟨5⟩) (.lock ⟨5⟩ ⟨2⟩)

/-- How a run stopped owing the vote1 it owed after step 3, if it did. -/
private def ending (es : List Event) (held : List HeldAt := []) :
    Option Ending :=
  let h := historyOf es
  let owed := owedAfter cfg (fun _ _ => some me) me h
  match (owed.getD 4 []).head? with
  | some o => endingOf me owed h held o 3
  | none => none

-- The vote comes a step late.
/-- info: some (NewProtocolDiff.Ending.acted 4) -/
#guard_msgs in #eval ending (opening ++ [voteStep [], .consensus (.blockReconstructed ⟨9⟩ ⟨8⟩) [vote ⟨3⟩]])

-- The node times the view out without having voted.
/-- info: some (NewProtocolDiff.Ending.lapsed 4) -/
#guard_msgs in #eval ending (opening ++ [voteStep [], .consensus (.timeout ⟨1⟩) [.send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨1⟩, me⟩)]])

-- The node holds its timeout vote back: it gave the view up, and did not act.
/-- info: some (NewProtocolDiff.Ending.lapsed 4) -/
#guard_msgs in #eval ending (opening ++ [voteStep [], .consensus (.timeout ⟨1⟩) []]) [hold .timeout 1 0 4]

-- A vote held back before the view was given up is not reported as unreleased.
/-- info: [] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], .consensus (.timeout ⟨1⟩) [], idleStep])
  [heldVote, hold .timeout 1 0 4] [stored (.action ⟨1⟩ .vote) 5]

-- It never stops owing it.
/-- info: none -/
#guard_msgs in #eval ending (opening ++ [voteStep [], idleStep])

/-! A timer the node takes in view 1, answered in the same step, held back, in a later step, or not at all. -/

private def timerStep (outs : List Output) : Event := .consensus (.timeout ⟨1⟩) outs

private def timeoutVote : Output := .send (.timeoutVote ⟨⟨⟨0⟩, lk⟩, ⟨1⟩, me⟩)

/-- The timers a run did not answer in their step, given the actions it held back. -/
private def unanswered (es : List Event) (held : List HeldAt := [])
    (member : EpochNumber → Bool := fun _ => true) : List String :=
  (timerFaults cfg me member (historyOf es) held).map fun (n, v) =>
    s!"timeout vote for view {v.toNat} in step {n}"

/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [timeoutVote]])

-- A node outside the committee of its epoch has no timeout vote to cast.
/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep []]) [] (fun _ => false)

#guard memberIn [(⟨1⟩, [⟨7⟩, ⟨8⟩])] ⟨7⟩ ⟨1⟩
#guard !memberIn [(⟨1⟩, [⟨7⟩, ⟨8⟩])] ⟨9⟩ ⟨1⟩
-- An epoch the trace names no committee for is read as one the node is in.
#guard memberIn [(⟨1⟩, [⟨7⟩, ⟨8⟩])] ⟨9⟩ ⟨2⟩
-- An epoch written with no members is one whose committee the node could not resolve.
#guard !memberIn [(⟨1⟩, [])] ⟨7⟩ ⟨1⟩
-- The last line for an epoch counts.
#guard memberIn [(⟨1⟩, []), (⟨1⟩, [⟨7⟩])] ⟨7⟩ ⟨1⟩
#guard (readCommittees "# committee 3 \n# committee 4 5 6\n").map (·.1) == [⟨3⟩, ⟨4⟩]
#guard (readCommittees "# committee 3 \n").map (·.2.length) == [0]

/-- info: ["timeout vote for view 1 in step 3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep []])

/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep []]) [hold .timeout 1 0 3]

-- A timeout vote answers the timer in whatever epoch the node is.
/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep []]) [hold .timeout 1 7 3]

-- The vote must come in the timer's step.
/-- info: ["timeout vote for view 1 in step 3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [], voteStep [timeoutVote]])

/-! The one-honest indication may be for a later view than the node's. -/

private def oneHonestStep (outs : List Output) : Event := .consensus (.timeoutOneHonest ⟨2⟩) outs

private def timeoutVoteFor (epoch view : Nat) (lock : Cert1) : Output :=
  .send (.timeoutVote ⟨⟨⟨epoch⟩, lock⟩, ⟨view⟩, me⟩)

/-- info: [] -/
#guard_msgs in #eval unanswered (opening ++ [oneHonestStep [timeoutVoteFor 0 2 lk]])

/-- info: ["timeout vote for view 2 in step 3"] -/
#guard_msgs in #eval unanswered (opening ++ [oneHonestStep []])

/-! Grounds past the timer's view in a later step do not answer it: here a timeout certificate. -/

private def genesisCert : Cert1 := ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩

private def certStep : Event := .consensus (.timeoutCertificate ⟨⟨⟨0⟩, genesisCert⟩, ⟨1⟩⟩) []

/-- info: ["timeout vote for view 1 in step 3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [], certStep])

/-! The vote must name the epoch the node was in at the timer. -/

/-- info: ["timeout vote for view 1 in step 3"] -/
#guard_msgs in #eval unanswered (opening ++ [timerStep [timeoutVoteFor 1 1 lk]])

/-!
`ProtocolHistory.timeoutJustified`: a timeout vote answers a timer taken at or before
it, and names the lock the node held then. The step that breaks it, if any.
-/

private def timeoutFault (es : List Event) : Option Nat :=
  (protocolFault cfg (historyOf es) me (fun _ _ => some me)).map (·.1)

/-- info: none -/
#guard_msgs in #eval timeoutFault (opening ++ [timerStep [], voteStep [timeoutVoteFor 0 1 genesisCert]])

-- No timer for the vote's view.
/-- info: some 3 -/
#guard_msgs in #eval timeoutFault (opening ++ [voteStep [timeoutVoteFor 0 1 genesisCert]])

-- The lock was taken after the timer.
/-- info: some 5 -/
#guard_msgs in #eval
  timeoutFault (opening ++ [timerStep [], .consensus (.certificate1 lk) [], voteStep [timeoutVoteFor 0 1 lk]])

/-- What a run exercises, as the coverage report counts it. -/
private def covered (es : List Event) : String :=
  let h := historyOf es
  let cov := Coverage.add cfg (fun _ _ => some me) me {} h (joinedSteps es) (heldBack es)
    (fun _ => true)
  s!"owed {cov.owed.getD "vote1" 0}, ended by notVoted {cov.ended.getD ("vote1", "notVoted") 0}, \
    blocked by validated {cov.soleFalse.getD ("vote1", "validated") 0}"

-- The vote1 is owed once validated, and ends by being cast. Before, after the proposal and
-- after the reconstruction of its parent, the anchor, validation alone was missing.
/-- info: "owed 1, ended by notVoted 1, blocked by validated 2" -/
#guard_msgs in #eval covered (opening ++ [voteStep [], .consensus (.blockValidated ⟨9⟩ ⟨9⟩) [vote ⟨3⟩]])

-- A step taken with the next is not counted on its own: here the proposal's.
/-- info: "owed 1, ended by notVoted 1, blocked by validated 1" -/
#guard_msgs in #eval covered
  [ .consensus (.certificate1 ⟨⟨⟨1⟩, ⟨0⟩, 0⟩, ViewNumber.genesis⟩) [],
    .consensus (.proposal ⟨1⟩ prop1 (some ⟨⟨1⟩, ⟨42⟩⟩)) [] { joined := true },
    .consensus (.blockReconstructed ViewNumber.genesis ⟨7⟩) [],
    voteStep [], .consensus (.blockValidated ⟨9⟩ ⟨9⟩) [vote ⟨3⟩] ]

-- A timer whose vote the node holds back in its step ends there, whenever the vote is released.
#guard timerEnding cfg me (fun _ => true) (historyOf (opening ++ [timerStep [], voteStep [timeoutVote]])) [] 3 ⟨1⟩
  == "answered in a later step"
#guard timerEnding cfg me (fun _ => true) (historyOf (opening ++ [timerStep [], voteStep [timeoutVote]]))
  [hold .timeout 1 0 3] 3 ⟨1⟩ == "held back in its step"
#guard timerEnding cfg me (fun _ => false) (historyOf (opening ++ [timerStep []])) [] 3 ⟨1⟩
  == "outside the committee"

/-! ## What a trace says beyond its steps

A dropped input is a line of its own. Its outputs join the step before it in the
history, and what the node held back there belongs to that step.
-/

private def annotated : List Event :=
  [ .consensus (.certificate1 lk) [] { joined := true },
    .consensus (.blockReconstructed ⟨1⟩ ⟨7⟩) []
      { held := [⟨.vote1, ⟨3⟩, ⟨2⟩, [.action ⟨3⟩ .vote]⟩], drbRequested := [⟨⟨3⟩, some ⟨3⟩, none⟩] },
    .dropped (.stored (.action ⟨3⟩ .vote)) [vote ⟨3⟩],
    .dropped (.drbResult ⟨3⟩) [] { held := [⟨.timeout, ⟨4⟩, ⟨2⟩, []⟩] },
    .consensus (.timeout ⟨4⟩) [] { stateCert := true } ]

/-- info: true -/
#guard_msgs in
#eval match parseTrace (renderTrace annotated) with
  | .ok es => es == annotated
  | .error _ => false

#guard (historyOf annotated).length == 3
#guard ((historyOf annotated)[1]?.map (·.output)) == some [vote ⟨3⟩]
#guard !unjoined annotated
#guard unjoined (.dropped (.other "Stored") [vote ⟨3⟩] :: annotated)
#guard unjoined (.dropped (.other "VidShare") [] { held := [⟨.vote1, ⟨1⟩, ⟨0⟩, []⟩] } :: annotated)
#guard !unjoined (.dropped (.drbResult ⟨2⟩) [] :: annotated)
#guard joinedSteps annotated == [0]
#guard (heldBack annotated).map (fun x => (x.kind, x.step, x.line)) == [(.vote1, 1, 1), (.timeout, 1, 3)]
#guard (droppedInputs annotated).map (fun d => (d.step, d.line)) == [(1, 2), (1, 3)]
#guard drbRequests annotated == [(⟨⟨3⟩, some ⟨3⟩, none⟩, 1, 1)]
#guard stateCertSteps annotated == [2]
#guard groupStart [4] 5 5 == 4
#guard groupStart [4] 4 4 == 4

/-! An action at a dropped input releases one held back for a write the input confirms, or waited for a DRB result. -/

/-- The actions a trace's node took at dropped inputs with no reason to take them there. -/
private def strays (es : List Event) (owed : Bool := true) : List (Nat × HeldKind) :=
  (droppedActionFaults cfg me (fun _ _ _ _ => owed) (heldBack es) (droppedInputs es)
    (drbRequests es)).map fun (k, kind, _, _) => (k, kind)

/-- A step that holds the vote1 of view 1 back for the vote action. -/
private def holding : Event :=
  .consensus (.blockValidated ⟨1⟩ ⟨3⟩) [] { held := [⟨.vote1, ⟨1⟩, ⟨0⟩, [.action ⟨1⟩ .vote]⟩] }

#guard strays [holding, .dropped (.stored (.action ⟨1⟩ .vote)) [vote ⟨3⟩]] == []
-- Storage confirmed a write the vote did not wait for.
#guard strays [holding, .dropped (.stored (.share ⟨1⟩)) [vote ⟨3⟩]] == [(0, .vote1)]
-- The vote was not held back.
#guard strays [voteStep [], .dropped (.stored (.action ⟨1⟩ .vote)) [vote ⟨3⟩]] == [(0, .vote1)]
-- The input is not one a vote waits for.
#guard strays [holding, .dropped (.blockBuilt ⟨1⟩) [vote ⟨3⟩]] == [(0, .vote1)]
-- A proposal waits for the block the node built for its view, and for no other.
#guard strays [voteStep [], .dropped (.blockBuilt ⟨1⟩) [] { held := [⟨.proposal, ⟨1⟩, ⟨0⟩, []⟩] }] == []
#guard strays [voteStep [], .dropped (.blockBuilt ⟨2⟩) [] { held := [⟨.proposal, ⟨1⟩, ⟨0⟩, []⟩] }]
  == [(0, .proposal)]
-- The vote is first held back at an input it does not wait for.
#guard strays [voteStep [], .dropped (.other "VidShare") []
    { held := [⟨.vote1, ⟨1⟩, ⟨0⟩, [.action ⟨1⟩ .vote]⟩] }] == [(0, .vote1)]
-- An action the node did not owe, it may take at any input.
#guard strays [holding, .dropped (.stored (.share ⟨1⟩)) [vote ⟨3⟩]] (owed := false) == []
#guard strays [voteStep [], .dropped (.other "VidShare") []
    { held := [⟨.vote1, ⟨1⟩, ⟨0⟩, [.action ⟨1⟩ .vote]⟩] }] (owed := false) == []
-- A timeout vote held back there answers a timer.
#guard strays [voteStep [], .dropped (.other "VidShare") [] { held := [⟨.timeout, ⟨1⟩, ⟨0⟩, []⟩] }] == []
-- What a dropped input draws that no node owes is no action.
#guard strays [holding, .dropped (.other "VidShare") [.send (.cert1 lk)]] == []

/-! The DRB result an action waits for is the next epoch's, in an epoch's transition, after the first epoch. -/

private def epochs : Config := { cfg with epochHeight := 10 }

#guard !inTransition ⟨16⟩ 10
#guard inTransition ⟨17⟩ 10
#guard inTransition ⟨20⟩ 10
#guard !inTransition ⟨21⟩ 10
#guard waitsForDrb epochs .vote1 ⟨2⟩ (some ⟨18⟩) ⟨3⟩
#guard !waitsForDrb epochs .vote1 ⟨2⟩ (some ⟨18⟩) ⟨4⟩
#guard !waitsForDrb epochs .vote1 ⟨2⟩ (some ⟨13⟩) ⟨3⟩
#guard !waitsForDrb epochs .vote1 ⟨1⟩ (some ⟨8⟩) ⟨2⟩
#guard !waitsForDrb epochs .vote2 ⟨2⟩ (some ⟨18⟩) ⟨3⟩

-- An action whose block is not known is not read as waiting.
#guard !waitsForDrb epochs .proposal ⟨2⟩ none ⟨3⟩

/-- A proposal owed in epoch 2 for view 18. -/
private def owedProposal : Obligation := .propose ⟨2⟩ ⟨18⟩

/-- A request for epoch 3's DRB result on line 4, in step 4, for the block the node built for view 18. -/
private def asked (block : Nat := 18) (view : Nat := 18) (epoch : Nat := 3) :
    DrbRequest × Nat × Nat :=
  (⟨⟨epoch⟩, some ⟨view⟩, some ⟨block⟩⟩, 4, 4)

-- Asked for by step 4, for the block of that view, and not yet answered.
#guard drbAwaited epochs [asked] [] owedProposal 6 6
-- Not asked for.
#guard !drbAwaited epochs [] [] owedProposal 6 6
-- Asked for another epoch's.
#guard !drbAwaited epochs [asked (epoch := 4)] [] owedProposal 6 6
-- Asked for a block that waits for none: one before the epoch's transition.
#guard !drbAwaited epochs [asked (block := 13)] [] owedProposal 6 6
-- Asked for the block of another view.
#guard !drbAwaited epochs [asked (view := 17)] [] owedProposal 6 6
-- Asked for with no block named.
#guard !drbAwaited epochs [(⟨⟨3⟩, none, none⟩, 4, 4)] [] owedProposal 6 6
-- Answered before the step the node gave the proposal up in.
#guard !drbAwaited epochs [asked] [⟨.drbResult ⟨3⟩, [], 5, 6⟩] owedProposal 6 6
-- Answered after the step it gave the proposal up in, which the result did not come in time for.
#guard drbAwaited epochs [asked] [⟨.drbResult ⟨3⟩, [], 6, 7⟩] owedProposal 6 6
-- Answered after the last step of a run in which the node never proposed.
#guard !drbAwaited epochs [asked] [⟨.drbResult ⟨3⟩, [], 6, 7⟩] owedProposal 6 7
-- The result arrived, and the node proposed on it.
#guard drbArrivedInto epochs [asked] [⟨.drbResult ⟨3⟩, [], 5, 6⟩] owedProposal 5
#guard !drbArrivedInto epochs [asked] [⟨.drbResult ⟨3⟩, [], 5, 6⟩] owedProposal 4
-- The result is not one the node asked for, for this proposal.
#guard !drbArrivedInto epochs [asked (block := 13)] [⟨.drbResult ⟨3⟩, [], 5, 6⟩] owedProposal 5

-- The block whose state failed is the one the vote1 is owed for.
#guard stateValidationFailedFor [⟨.stateValidationFailed ⟨1⟩ (blockHash prop1), [], 2, 5⟩] (.vote1 prop1) 3
-- Another block's.
#guard !stateValidationFailedFor [⟨.stateValidationFailed ⟨1⟩ ⟨999⟩, [], 2, 5⟩] (.vote1 prop1) 3
-- It failed after the node stopped owing the vote.
#guard !stateValidationFailedFor [⟨.stateValidationFailed ⟨1⟩ (blockHash prop1), [], 4, 5⟩] (.vote1 prop1) 3
-- A proposal is not excused by it.
#guard !stateValidationFailedFor [⟨.stateValidationFailed ⟨1⟩ (blockHash prop1), [], 2, 5⟩]
  (.propose ⟨0⟩ ⟨1⟩) 3

-- A dropped input before the first step follows none: it is not read as following step 0.
#guard (droppedInputs (.dropped (.drbResult ⟨2⟩) [] :: annotated)).map (·.line) == [3, 4]
-- A line written by hand may leave its notes out.
#guard (parseTrace r#"{"dropped":{"kind":{"drbResult":{"epoch":3}},"output":[]}}"#).toOption
  == some [.dropped (.drbResult ⟨3⟩) []]

-- A vote held back with no write named is not judged as never released.
/-- info: [] -/
#guard_msgs in #eval unreleased (opening ++ [voteStep [], idleStep]) [hold .vote1 1 0 3]

/-! ## A boundary of the model is not a failure

A header whose payload commitment is `null` is a block from before a version
boundary. The parser refuses it, and the refusal has to read as out of scope
rather than as a broken trace, or every recorded cutover run fails the build.

The two lines below are written out rather than encoded from the types, because
neither is a value the types admit — that is what makes them worth testing. Kept
on one line each: `parseTrace` splits on newlines, so a step broken across two
would be malformed for the wrong reason and the first test would pass without
touching the case it names.
-/

/-- A step delivering a proposal whose header carries no payload commitment. -/
private def preCutoverLine : String :=
  r#"{"consensus":{"input":{"proposal":{"sender":1,"p":{"blockHeader":"# ++
  r#"{"payloadCommit":null},"viewNumber":1,"parentCert":{"data":"# ++
  r#"{"blockHash":7},"view":0},"timeoutEvidence":null,"identity":9},"# ++
  r#""share":{"view":1,"payloadCommit":3}}},"output":[]}}"#

/-- A step that is simply incomplete. -/
private def brokenLine : String :=
  r#"{"consensus":{"input":{"proposal":{"sender":1}}}}"#

/-- How a parse failure classifies: the boundary of the model, or a bad recording. -/
private def classify (line : String) : String :=
  match parseTrace line with
  | .ok _ => "parsed, which it should not"
  | .error e => if (parseOutOfScope e).isSome then "out of scope" else "malformed"

/-- info: "out of scope" -/
#guard_msgs in #eval classify preCutoverLine

/-- info: "malformed" -/
#guard_msgs in #eval classify brokenLine

end Tests
end NewProtocolDiff

