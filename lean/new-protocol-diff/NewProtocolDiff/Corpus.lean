module

public import NewProtocolDiff.Json

/-!
# What a trace runs past

The specification does not cover restart, the DRB, block fetching or state
validation, so a recording that acted on one of those acted on something the
rules were never told. Two signals say so, both read off the trace itself rather
than guessed from the name of the test that produced it:

* the trace carries a dropped input of a kind the specification has no
  counterpart for (`unmodelledDropped`), which can explain a node that owes
  longer than `Prompt` allows;
* the trace cannot be parsed for a reason the parser marks as outside the model
  (`parseOutOfScope`).
-/

@[expose] public section

namespace NewProtocolDiff

/--
Dropped inputs standing for something the specification does not cover.

Every name here must be one the recorder actually writes. The correspondence is one-way.
A recorder may drop inputs this list does not excuse: `Stored` and `BlockBuilt` are
dropped and deliberately absent, since their outputs join the last step written
rather than being lost, and a node that is late around one of them is late.
-/
def unmodelledInputs : List (String × String) :=
  [ ("DrbResult", "leader election"),
    ("StateValidationFailed", "state validation") ]

/--
An unmodelled feature a trace dropped an input for at or before `step`, if any.

The step matters: the recorder writes the index it was at, so the excuse can be
required to precede what it excuses.

That is necessary and not sufficient. A drop at step 0 still excuses a delay at
step 47, which is most of the epoch suites, and no ordering rule can narrow that.
What keeps it honest is the report: an excused delay is printed with its excuse.
-/
def unmodelledDropped (text : String) (step : Nat) : Option String :=
  let tag := "# dropped input: "
  text.splitOn "\n" |>.findSome? fun line =>
    if line.startsWith tag then
      let rest := (line.drop tag.length).toString
      match droppedStep rest with
      | some at' => if at' > step then none else named rest
      | none => named rest
    else none
  where
    /-- The feature a dropped input's name belongs to. -/
    named (rest : String) : Option String :=
      unmodelledInputs.findSome? fun (name, feature) =>
        if rest.startsWith name then some s!"{feature} (dropped {name})" else none
    /-- The step index the recorder noted, from `… (at step N)`. -/
    droppedStep (rest : String) : Option Nat :=
      match (rest.splitOn "(at step ").getLast? with
      | some tail => (tail.takeWhile Char.isDigit).toString.toNat?
      | none => none

/--
Why a trace could not be parsed at all, when the reason is a boundary of the
specification rather than a broken line.

`NewProtocolDiff.outOfScopeMark` is how the parser says so. The mark is looked for anywhere
in the message rather than at the front, because a `FromJson` error arrives
wrapped in the field path that produced it.
-/
def parseOutOfScope (e : String) : Option String :=
  if (e.splitOn NewProtocolDiff.outOfScopeMark).length > 1 then
    some (e.splitOn NewProtocolDiff.outOfScopeMark |>.getLast!)
  else
    none

end NewProtocolDiff
