import Lean
import NewProtocolSpec

/-!
# Results that never reach the document

The reference splices a declaration by name, so a rename fails the build and the
prose cannot drift from what it describes. What no build checks is the other
direction: a result added to the specification and never spliced is simply
absent, and the document reads as complete because nothing says otherwise. That
happened — two results of `NewProtocolSpec.Progress` were stated and never named
here.

So this checks it. It reads the document's source, collects the declarations
`{docstring …}` and `{includeDocstring …}` name, and fails if a theorem of a
results module is missing from that set.

    coverage

Only the results modules are scanned. `X/Defs.lean` holds definitions, which the
document splices where it introduces them and is not required to splice
exhaustively, and `X/Lemmas.lean` is scaffolding an audit skips — that is the
claim {ref "top"}[the audit section] makes, and splicing from it would contradict
it.

The companion checks are in the other package: `../Lint.lean` fails on a
backticked name in a docstring that resolves to nothing, and
`NewProtocolSpec.Checks` on an axiom footprint beyond Lean's own.
-/

open Lean

/--
The modules whose theorems are results.

Hand-maintained, and the one list here that is: a module is a results module
because of what it is for, which nothing in the environment records. Adding a
module without adding it here leaves its results unchecked, which is the failure
this file exists to prevent — so the list is short and sits next to the check
that reads it.
-/
def resultModules : List Name :=
  [`NewProtocolSpec.Base,
   `NewProtocolSpec.Safety, `NewProtocolSpec.Network, `NewProtocolSpec.DecideStream,
   `NewProtocolSpec.Invariants, `NewProtocolSpec.Progress, `NewProtocolSpec.Deadlock,
   `NewProtocolSpec.Round]

/--
Every theorem those modules declare, ignoring the auxiliaries Lean generates.

`Name.isInternal` catches most of them; the rest are the ones elaboration
reserves a name for, such as the `congr_simp` a structure projection brings with
it, which `isReservedName` is how Lean itself recognises. Neither catches what a
structure's constructor brings — `mk.injEq`, `mk.inj`, `mk.sizeOf_spec` — so
those go by the shape of the name, which is the only place their provenance
shows.
-/
def statedResults (env : Environment) : Array Name := Id.run do
  let mut out := #[]
  for (name, idx) in env.const2ModIdx.toList do
    let some m := env.allImportedModuleNames[idx.toNat]? | continue
    unless resultModules.contains m do continue
    unless (env.find? name).any (·.isTheorem) do continue
    if name.isInternal || isReservedName env name then continue
    if name.components.contains `mk then continue
    out := out.push name
  return out

/-- The declarations the document splices, by name. -/
def splicedIn (text : String) : NameSet := Id.run do
  let mut out := {}
  for role in ["{docstring ", "{includeDocstring "] do
    for part in (text.splitOn role).drop 1 do
      let name := part.takeWhile fun (c : Char) =>
        c.isAlphanum || c == '_' || c == '.' || c == '\''
      out := out.insert name.toName
  return out

def main : IO UInt32 := do
  initSearchPath (← findSysroot)
  let env ← importModules #[Import.mk `NewProtocolSpec false true false] Options.empty
  let spliced := splicedIn (← IO.FS.readFile "Reference.lean")
  let missing := (statedResults env).filter (!spliced.contains ·)
  if missing.isEmpty then
    IO.println "every result of the specification reaches the reference"
    return 0
  IO.eprintln s!"{missing.size} result(s) are stated but never spliced into Reference.lean:"
  for n in missing do
    IO.eprintln s!"  {n}"
  return 1
