module

import NewProtocolImpl.Conformance.Owed
meta import Lean.Elab.Command

/-!
# Checks on the claims this package makes about itself

`NewProtocolImpl` claims in prose that the machine conforms on nothing but Lean's
own axioms. Prose cannot check that, so it is checked here at build time, the way
`NewProtocolSpec.Checks` checks the specification's own claims.

The specification's checks cannot cover this one. They run in a package that knows
nothing of this one — the dependency points the other way — so `Impl.conforms` is
invisible there, and the whole of the conformance argument with it. A `sorry` in
the proof joining the machine to the specification built green until this file
existed.
-/

open Lean

namespace NewProtocol
namespace Impl
namespace Checks

/-! ## Axioms

`propext`, `Classical.choice` and `Quot.sound` are Lean's own. `sorryAx` would
appear here if any step of the conformance argument were incomplete, including the
parts of it proved in the `Conformance` modules this depends on.
-/

/-- info: 'NewProtocol.Impl.conforms' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms conforms

end Checks
end Impl
end NewProtocol
