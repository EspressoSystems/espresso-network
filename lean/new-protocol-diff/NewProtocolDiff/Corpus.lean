module

public import NewProtocolDiff.Json

/-!
# What a trace runs past

A trace that cannot be parsed for a reason the parser marks as outside the model
(`parseOutOfScope`) is a recording of something the specification does not cover,
not a broken one.
-/

@[expose] public section

namespace NewProtocolDiff

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
