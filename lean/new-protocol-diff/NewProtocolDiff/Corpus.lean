module

public import NewProtocolDiff.Replay

/-!
# Replaying a corpus

A divergence is not always a disagreement. The specification does not cover
epochs, restart, or block fetching, so a recording that acted on one of those
acted on something the machine was never told, and could not have followed.
Such a trace is counted apart: it is not evidence that the implementation
conforms, and not evidence that it does not.

Two signals say a trace is of that kind, and both are read off the trace itself
rather than guessed from the name of the test that produced it.

* The trace carries a dropped input of a kind the specification has no
  counterpart for. `Recorder::record` writes those steps as comments, so the
  actions they released ride on the next step that is written, and the recording
  can act on state no input explains.
* The recording acted in a view whose proposal — or whose parent's proposal —
  the trace never delivers, so its state came from somewhere the trace does not
  show. Seeding does this, and so does an arrival from before recording began.

Only a `Divergence.recordingAhead` is ever excused this way. A
`Divergence.differentDetail` says both sides acted and disagreed about what they
acted on, which no missing input explains.
-/

@[expose] public section

namespace NewProtocolDiff

open NewProtocol

/-- Dropped inputs standing for something the specification does not cover. -/
def unmodelledInputs : List (String × String) :=
  [ ("EpochChange", "epoch machinery"),
    ("EpochRootCertificates", "epoch machinery"),
    ("DrbResult", "epoch machinery"),
    ("FetchedProposal", "block fetching"),
    ("StateValidationFailed", "state validation") ]

/-- The first unmodelled feature a trace's dropped inputs name, if any. -/
def unmodelledDropped (text : String) : Option String :=
  let tag := "# dropped input: "
  text.splitOn "\n" |>.findSome? fun line =>
    if line.startsWith tag then
      let kind := (line.drop tag.length).toString
      unmodelledInputs.findSome? fun (name, feature) =>
        if kind.startsWith name then some s!"{feature} (dropped {name})" else none
    else none

/-- Each view a trace delivers a proposal for, paired with its parent's view. -/
def proposalParents (events : List Event) : List (ViewNumber × ViewNumber) :=
  events.filterMap fun
    | .consensus (.proposal _ p _) _ => some (p.viewNumber, p.parentCert.view)
    | _ => none

/--
The view an action at `view` needs and the trace does not deliver.

Acting in a view needs that view's proposal and, unless the parent is genesis,
the parent's too: `Vote1Justification.parentLinked` reads the parent block. A
recording that acted anyway held ancestry the trace does not show.
-/
def missingAncestor (parents : List (ViewNumber × ViewNumber)) (view : ViewNumber) :
    Option ViewNumber :=
  let rec go (fuel : Nat) (v : ViewNumber) : Option ViewNumber :=
    match fuel with
    | 0 => none
    | fuel + 1 =>
      if v == ViewNumber.genesis then none
      else match parents.lookup v with
        | none => some v
        | some u => go fuel u
  go (parents.length + 1) view

/-- What a replay of one trace amounted to. -/
inductive Verdict where
  /-- Every action the recording took, the machine took too. -/
  | agree
  /-- The recording ran past what the specification covers, for this reason. -/
  | outOfScope (reason : String)
  /-- The implementation and the specification disagree. -/
  | diverge
  /-- The trace could not be read. -/
  | malformed
  /-- The trace records no step. -/
  | empty
  /-- The trace names no node or anchor, so nothing can be replayed. -/
  | unreplayable
deriving DecidableEq, Repr

/-- What to call each verdict in a report, and whether it fails the run. -/
def Verdict.label : Verdict → String
  | .agree => "agree"
  | .outOfScope _ => "out-of-scope"
  | .diverge => "diverge"
  | .malformed => "malformed"
  | .empty => "empty"
  | .unreplayable => "unreplayable"

/--
Whether this verdict is one to act on.

`unreplayable` counts, though it is not a disagreement: it means the harness could
not do its job, and a recorder that stopped writing its header would otherwise
turn every trace green. `empty` does not — a run that reached the consensus core
with nothing to record has nothing to answer for.
-/
def Verdict.failed : Verdict → Bool
  | .diverge | .malformed | .unreplayable => true
  | _ => false

/--
Why this divergence is a boundary of the specification rather than a
disagreement, if it is one.
-/
def excuse (text : String) (events : List Event) : Divergence → Option String
  | .recordingAhead _ _ view =>
    match unmodelledDropped text with
    | some reason => some reason
    | none =>
      match missingAncestor (proposalParents events) view with
      | none => none
      | some missing =>
        if missing == view then
          some s!"acts in view {view.toNat}, whose proposal the trace never delivers"
        else
          some s!"acts in view {view.toNat}, whose ancestor at view {missing.toNat} \
            the trace never delivers"
  | _ => none

/-- The verdict on one replayed trace, and what to print under it. -/
def verdictOf (text : String) (events : List Event) (o : Outcome) : Verdict × String :=
  match o.divergence with
  | none => (.agree, o.report)
  | some d =>
    match excuse text events d with
    | some reason => (.outOfScope reason, o.report ++ s!"\n  reason: {reason}")
    | none => (.diverge, o.report)

end NewProtocolDiff
