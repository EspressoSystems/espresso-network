module

public import NewProtocolDiff.PromptCheck
public import NewProtocolDiff.Json

/-!
# What a corpus of traces exercises

Which parts of `NewProtocol.Owed` a set of traces reaches. Each obligation is
owed when every one of its conjuncts holds, the conjuncts named as the fields of
`NewProtocol.OwedVote1` and its siblings. A conjunct is exercised when it is the
one that decides: the only conjunct false for a candidate obligation (`soleFalse`),
or the one whose turning false ended an obligation that was owed (`ended`). A
conjunct that never decides is a gap in what the traces test.

The conjuncts mirror `NewProtocolImpl.OwedB`. Their conjunction is checked against
`NewProtocol.Owed` at every point, and a disagreement is reported (`mismatches`),
since a split that does not add up would make the counts meaningless.

It also counts how each timer the node took ended (`History.TimerAt`): answered
in the moment it was taken, on a later release in the same step, or in a later
step, overtaken by grounds for a later view, or still open when the recording
stopped. An ending no trace reaches is a gap too.

The report also counts the kinds of input and output each trace has, which shows
an input or action that no trace contains at all.

Nothing here is proved: it is a report, and it fails nothing.
-/

@[expose] public section

namespace NewProtocolDiff

open Lean NewProtocol NewProtocol.Lists NewProtocolImpl History

/-- The kind of an obligation. -/
def _root_.NewProtocol.Obligation.kind : Obligation → String
  | .vote1 _ => "vote1"
  | .vote1Again _ => "vote1Again"
  | .vote2 _ => "vote2"
  | .decide _ => "decide"
  | .propose _ _ => "propose"

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

/-- No block `c` commits is held. -/
abbrev noBlock (c : Cert2) (h : History) : Prop := ∀ b ∈ proposalsHeld cfg h, ¬ Commits c b

/--
The conjuncts of `Owed` for `o` at `h`, each named and decided.

A conjunct about the block holds when the block is not held, so that a missing
block is reported once, by the conjunct that asks for it, rather than by every
conjunct about it. The conjunction is unchanged. For a decide, the block a `Cert2`
commits is chosen anew in each conjunct, which can disagree with `Owed` when
several held blocks match; the `mismatches` count says if it ever does.
-/
def conjuncts (h : History) : Obligation → List (String × Bool)
  | .vote1 p =>
    [ ("received", decide (∃ x ∈ receivedProposals h, x.2.1 = p
        ∧ leader p.epoch p.viewNumber = some x.1 ∧ ShareMatches p x.2.2)),
      ("wellFormed", decide (ProposalWellFormed cfg p)),
      ("validated", decide (h.Received (.blockValidated p.viewNumber (blockHash p)))),
      ("parent", decide (p.parentCert.view = cfg.anchorView ∨ EntersEpoch cfg p
        ∨ ∃ parent ∈ proposalsHeld cfg h, parent.viewNumber ≤ p.parentCert.view
          ∧ p.parentCert.data.blockHash = blockHash parent
          ∧ h.HasPayload cfg parent.viewNumber parent.payloadCommit)),
      ("safe", decide (SafeParentB p)),
      ("opens", decide (OpensB cfg h p)),
      ("current", decide (NotBehindB cfg h p.epoch)),
      ("notTimedOut", !decide (TimedOutB h p.viewNumber)),
      ("notVoted", decide (∀ vote ∈ vote1sSent h, vote.data.epoch = p.epoch → vote.view ≠ p.viewNumber)),
      ("inView", decide (viewOf cfg h = p.viewNumber)) ]
  | .vote1Again r =>
    [ ("received", decide (∃ x ∈ receivedRevotes h, x.2 = r ∧ leader r.cert.data.epoch r.view = some x.1)),
      ("wellFormed", decide (RevoteWellFormed cfg r)),
      ("safe", decide (SafeRevoteB r)),
      ("payload", decide (∃ b ∈ proposalsHeld cfg h, Certifies r.cert b
        ∧ h.HasPayload cfg b.viewNumber b.payloadCommit)),
      ("current", decide (NotBehindB cfg h r.cert.data.epoch)),
      ("notTimedOut", !decide (TimedOutB h r.view)),
      ("notVoted", decide (∀ vote ∈ vote1sSent h, vote.data.epoch = r.cert.data.epoch → vote.view ≠ r.view)),
      ("inView", decide (viewOf cfg h = r.view)) ]
  | .vote2 c =>
    [ ("holds.cert1", decide (c ∈ cert1sHeld cfg h)),
      ("holds.block", decide (∃ b ∈ proposalsHeld cfg h, Certifies c b)),
      ("holds.payload", decide ((∀ b ∈ proposalsHeld cfg h, ¬ Certifies c b)
        ∨ ∃ b ∈ proposalsHeld cfg h, Certifies c b ∧ h.HasPayload cfg b.viewNumber b.payloadCommit)),
      ("current", decide (NotBehindB cfg h c.data.epoch)),
      ("notVoted", decide (∀ vote ∈ vote2sSent h, vote.data.epoch = c.data.epoch → vote.view ≠ c.view)),
      ("notCommitted", decide (∀ c2 ∈ cert2sHeld h, c2.data.epoch = c.data.epoch → c2.view ≠ c.view)),
      ("notPast", !decide (PastViewB cfg h c.view)),
      ("afterAnchor", decide (cfg.anchorView < c.view)),
      ("notDecidedPast", decide (∀ u ∈ decidedViews h, u < c.view)) ]
  | .decide c =>
    [ ("holds.cert2", decide (c ∈ cert2sHeld h)),
      ("holds.block", decide (∃ b ∈ proposalsHeld cfg h, Commits c b)),
      ("holds.cert1", decide (noBlock cfg c h
        ∨ ∃ b ∈ proposalsHeld cfg h, Commits c b ∧ ∃ c1 ∈ cert1sHeld cfg h, Certifies c1 b)),
      ("notDecided", decide (noBlock cfg c h
        ∨ ∃ b ∈ proposalsHeld cfg h, Commits c b ∧ b.viewNumber ∉ decidedViews h)),
      ("afterFloor", decide (noBlock cfg c h
        ∨ ∃ b ∈ proposalsHeld cfg h, Commits c b ∧ AboveFloorB cfg h b.viewNumber)) ]
  | .propose e v =>
    [ ("justified", decide ((∃ p ∈ proposalCandidates cfg h v, p.epoch = e ∧ ProposalJustifiedB cfg leader node h p)
        ∨ ∃ r ∈ revoteCandidates cfg h v, r.cert.data.epoch = e ∧ RevoteJustifiedB cfg leader node h r)),
      ("notProposed", decide ((∀ p ∈ proposalsSent h, p.viewNumber = v → p.epoch ≠ e)
        ∧ ∀ r ∈ revotesSent h, r.view = v → r.cert.data.epoch ≠ e)),
      ("notTimedOut", !decide (TimedOutB h v)),
      ("inView", decide (viewOf cfg h = v)) ]

/-- What a set of traces exercised. Every map is keyed by obligation kind and conjunct, or by kind. -/
structure Coverage where
  /-- Points where the conjunct was false for a candidate. -/
  falseAt : Std.HashMap (String × String) Nat := {}
  /-- Points where the conjunct was the only one false for a candidate. -/
  soleFalse : Std.HashMap (String × String) Nat := {}
  /-- Obligations ended by the conjunct turning false. -/
  ended : Std.HashMap (String × String) Nat := {}
  /-- Obligations that became owed. -/
  owed : Std.HashMap String Nat := {}
  /-- Points where the conjuncts' conjunction disagreed with `Owed`. -/
  mismatches : Std.HashMap String Nat := {}
  /-- Timers by how they ended. -/
  timers : Std.HashMap String Nat := {}
  /-- Steps by the kind of their input. -/
  inputs : Std.HashMap String Nat := {}
  /-- Outputs by kind. -/
  outputs : Std.HashMap String Nat := {}

/-- Add one to a count. -/
def bump {κ : Type} [BEq κ] [Hashable κ] (m : Std.HashMap κ Nat) (k : κ) : Std.HashMap κ Nat :=
  m.insert k (m.getD k 0 + 1)

/-- The name a value's JSON gives its constructor: its object's first key. -/
def jsonKind (j : Json) : String :=
  match j with
  | .obj kvs => (kvs.toList.head?.map (·.1)).getD "?"
  | _ => "?"

/-- An output's kind: the message for a send, `decided` otherwise. -/
def outputKind (o : Output) : String :=
  match o with
  | .send m => jsonKind (toJson m)
  | .decided .. => "decided"

/-- The candidates at a point, each with its conjuncts. -/
def candidateConjuncts (h : History) : List (Obligation × List (String × Bool)) :=
  (candidates cfg h).eraseDups.map fun o => (o, conjuncts cfg leader node h o)

/--
Whether `h` holds the answer to the timer for `v` taken in step `n`: a timeout
vote for `v`, in that step or a later one, naming the epoch the node was in at
the timer.
-/
def timerAnswered (h : History) (n : Nat) (v : ViewNumber) : Bool :=
  let e := epochOfHistory cfg (h.take n)
  (h.drop n).any fun st => (timeoutVotesOf st).any fun tv =>
    decide (tv.view = v ∧ tv.signer = node ∧ tv.data.epoch = e)

/-- The ways a timer ends, in the order the report lists them. -/
def timerEndings : List String :=
  ["answered at once", "answered on a release", "answered in a later step", "overtaken",
   "open at the end"]

/--
How the timer for `v`, taken in step `n` at moment `k`, ended: at the first
moment from `k` on that answers it or has grounds past `v`.
-/
def timerEnding (ms : List Moment) (k n : Nat) (v : ViewNumber) : String :=
  let settles (m : Moment) := timerAnswered cfg node m.history n v || decide (v < viewOf cfg m.history)
  match (ms.drop k).zipIdx.find? (settles ·.1) with
  | none => "open at the end"
  | some (m, i) =>
    if !timerAnswered cfg node m.history n v then "overtaken"
    else if i == 0 then "answered at once"
    else if m.step == n then "answered on a release"
    else "answered in a later step"

/-- Add what one trace, read as moments, exercised. -/
def Coverage.add (cov : Coverage) (h : History) (ms : List Moment) : Coverage := Id.run do
  let mut cov := cov
  for st in h do
    cov := { cov with inputs := bump cov.inputs (jsonKind (toJson st.input)) }
    for o in st.output do
      cov := { cov with outputs := bump cov.outputs (outputKind o) }
  let mut previous : Option Nat := none
  for (m, k) in ms.zipIdx do
    if previous != some m.step then
      if let some v := timerView cfg m.history m.step then
        cov := { cov with timers := bump cov.timers (timerEnding cfg node ms k m.step v) }
    previous := some m.step
  let mut before : List (Obligation × List (String × Bool)) := []
  for m in ms do
    let now := candidateConjuncts cfg leader node m.history
    for (o, cs) in now do
      let all := cs.all (·.2)
      if all != decide (Owed cfg leader node m.history o) then
        cov := { cov with mismatches := bump cov.mismatches o.kind }
      let failing := cs.filter (!·.2)
      for (name, _) in failing do
        cov := { cov with falseAt := bump cov.falseAt (o.kind, name) }
      if let [(name, _)] := failing then
        cov := { cov with soleFalse := bump cov.soleFalse (o.kind, name) }
      let wasOwed := before.find? (·.1 == o) |>.map (·.2.all (·.2))
      if all && wasOwed != some true then
        cov := { cov with owed := bump cov.owed o.kind }
      if let some prev := before.find? (·.1 == o) then
        if prev.2.all (·.2) && !all then
          for ((name, was), (_, is)) in prev.2.zip cs do
            if was && !is then cov := { cov with ended := bump cov.ended (o.kind, name) }
    -- An obligation no longer a candidate is no longer owed, with no conjunct to name.
    for (o, cs) in before do
      if cs.all (·.2) && !now.any (·.1 == o) then
        cov := { cov with ended := bump cov.ended (o.kind, "noLongerCandidate") }
    before := now
  return cov

/-- Add one count map to another. -/
def addCounts {κ : Type} [BEq κ] [Hashable κ] (m n : Std.HashMap κ Nat) : Std.HashMap κ Nat :=
  n.fold (fun acc k c => acc.insert k (acc.getD k 0 + c)) m

/-- The counts of two coverages together. -/
def Coverage.merge (a b : Coverage) : Coverage :=
  { falseAt := addCounts a.falseAt b.falseAt, soleFalse := addCounts a.soleFalse b.soleFalse,
    ended := addCounts a.ended b.ended, owed := addCounts a.owed b.owed,
    mismatches := addCounts a.mismatches b.mismatches, timers := addCounts a.timers b.timers,
    inputs := addCounts a.inputs b.inputs, outputs := addCounts a.outputs b.outputs }

/-- Every conjunct of every kind, in the order the report lists them. -/
def allConjuncts : List (String × List String) :=
  [ ("vote1", ["received", "wellFormed", "validated", "parent", "safe", "opens", "current",
      "notTimedOut", "notVoted", "inView"]),
    ("vote1Again", ["received", "wellFormed", "safe", "payload", "current", "notTimedOut",
      "notVoted", "inView"]),
    ("vote2", ["holds.cert1", "holds.block", "holds.payload", "current", "notVoted", "notCommitted",
      "notPast", "afterAnchor", "notDecidedPast"]),
    ("decide", ["holds.cert2", "holds.block", "holds.cert1", "notDecided", "afterFloor"]),
    ("propose", ["justified", "notProposed", "notTimedOut", "inView"]) ]

/-- Every input and output kind, so that one no trace has is listed with zero. -/
def allInputs : List String :=
  ["blockReconstructed", "certificate1", "certificate2", "headerBuilt", "proposal",
   "blockValidated", "timeout", "timeoutCertificate", "timeoutOneHonest", "epochChange", "revote"]

def allOutputs : List String :=
  ["proposal", "vote1", "vote2", "timeoutVote", "timeoutCert", "cert1", "cert2", "vidShare",
   "epochChange", "revote", "decided"]

/-- The report, one line per conjunct and kind, gaps marked. -/
def Coverage.render (cov : Coverage) : String := Id.run do
  let mut out := "coverage of Owed: owed = obligations that became owed; per conjunct, \
    false = points where it was false, sole = points where it alone was, \
    ended = obligations it ended\n"
  for (kind, names) in allConjuncts do
    out := out ++ s!"  {kind}: owed {cov.owed.getD kind 0}"
    if let some n := cov.mismatches.get? kind then
      out := out ++ s!", {n} points where the conjuncts disagree with Owed"
    out := out ++ "\n"
    for name in names ++ ["noLongerCandidate"] do
      let false_ := cov.falseAt.getD (kind, name) 0
      let sole := cov.soleFalse.getD (kind, name) 0
      let ended := cov.ended.getD (kind, name) 0
      if name == "noLongerCandidate" then
        if ended > 0 then out := out ++ s!"    no longer a candidate: ended {ended}\n"
        continue
      let gap :=
        if false_ == 0 then "  <- never false"
        else if sole == 0 && ended == 0 then "  <- never decides"
        else ""
      out := out ++ s!"    {name}: false {false_}, sole {sole}, ended {ended}{gap}\n"
  out := out ++ "timers: how each timer the node took ended\n"
  for ending in timerEndings do
    let n := cov.timers.getD ending 0
    out := out ++ s!"  {ending}: {n}{if n == 0 then "  <- none" else ""}\n"
  out := out ++ "inputs:\n"
  for k in allInputs do
    let n := cov.inputs.getD k 0
    out := out ++ s!"  {k}: {n}{if n == 0 then "  <- none" else ""}\n"
  out := out ++ "outputs:\n"
  for k in allOutputs do
    let n := cov.outputs.getD k 0
    out := out ++ s!"  {k}: {n}{if n == 0 then "  <- none" else ""}\n"
  return out

end NewProtocolDiff
