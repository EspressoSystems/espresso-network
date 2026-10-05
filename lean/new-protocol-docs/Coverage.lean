import Lean
import NewProtocolSpec

/-!
# What never reaches the document

The reference splices a declaration by name, so a rename fails the build and the
prose cannot drift from what it describes. What no build checks is the other
direction: a definition added to the specification and never shown is simply
absent, and the document reads as complete because nothing says otherwise.

So this checks it. It reads the document's source, collects the declarations that
`{docstring …}`, `{includeDocstring …}` and `:::spec …` name, and fails if one of
these is missing from that set:

* a definition, structure or inductive type of a contract module, the modules a
  reader has to read to judge what is claimed;
* a main result.

    coverage

Theorems of the contract modules are not required: they are lemmas about the
definitions, such as the epoch arithmetic, and the document splices the ones its
prose leans on. Nor is anything under `Proofs/`, or `Lists`, which restates a
history for the machine and the trace checker.

The companion checks are in the specification package: `../Lint.lean` fails on a
backticked name in a docstring that resolves to nothing, and
`NewProtocolSpec.Checks` on an axiom footprint beyond Lean's own.
-/

open Lean

/--
The modules that make up the contract.

Hand-maintained, and the one list here that is: a module is part of the contract
because of what it is for, which nothing in the environment records. Adding a
module without adding it here leaves its definitions unchecked, which is the
failure this file exists to prevent, so the list sits next to the check that reads
it.
-/
def contractModules : List Name :=
  [`NewProtocolSpec.Base, `NewProtocolSpec.Types, `NewProtocolSpec.Interface,
   `NewProtocolSpec.Validity, `NewProtocolSpec.History, `NewProtocolSpec.Rules,
   `NewProtocolSpec.Network, `NewProtocolSpec.Timing, `NewProtocolSpec.Properties]

/-- The results the specification is for, and the witness that its premises can be met. -/
def mainResults : List Name :=
  [`NewProtocol.noFork, `NewProtocol.decideAgreement, `NewProtocol.decidesValid, `NewProtocol.Liveness.chainGrows,
   `NewProtocol.Witness.premises_met,
   `NewProtocol.Witness.per_epoch]

/--
Whether a declaration is one a reader meets as a definition.

Theorems are lemmas. Structure projections and constructors are shown by the
entry of the structure or type they belong to, and instances are notation. What
Lean generates around a definition, such as recursors, `noConfusion`, matchers and
the helpers of a derived instance, carries no docstring, which is how it is told
apart; every definition the specification states has one.
-/
def isDefinition (env : Environment) (name : Name) : IO Bool := do
  let some info := env.find? name | return false
  if info.isTheorem then return false
  if name.isInternal || isReservedName env name then return false
  if Meta.isInstanceCore env name then return false
  if (env.getProjectionFnInfo? name).isSome then return false
  if info matches .ctorInfo _ then return false
  return (← findDocString? env name).isSome

/-- The declarations a reader must find in the document. -/
def required (env : Environment) : IO (Array Name) := do
  let mut out := #[]
  for (name, idx) in env.const2ModIdx.toList do
    let some m := env.allImportedModuleNames[idx.toNat]? | continue
    unless contractModules.contains m do continue
    if ← isDefinition env name then out := out.push name
  return out ++ mainResults.toArray

/-- The declarations the document shows, by name. -/
def shown (text : String) : NameSet := Id.run do
  let mut out := {}
  for role in ["{docstring ", "{includeDocstring ", ":::spec "] do
    for part in (text.splitOn role).drop 1 do
      let name := part.takeWhile fun (c : Char) =>
        c.isAlphanum || c == '_' || c == '.' || c == '\''
      out := out.insert name.toName
  return out

/-
`unsafe` because loading the environment extensions, which hold the instance and
matcher tables `isDefinition` reads, runs their initializers.
-/
unsafe def main : IO UInt32 := do
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let env ← importModules #[Import.mk `NewProtocolSpec false true false] Options.empty
    (loadExts := true)
  let seen := shown (← IO.FS.readFile "Reference.lean")
  let missing := (← required env).filter (!seen.contains ·)
  if missing.isEmpty then
    IO.println "every definition of the contract and every main result reaches the reference"
    return 0
  IO.eprintln s!"{missing.size} declaration(s) are never shown in Reference.lean:"
  for n in missing.qsort (·.toString < ·.toString) do
    IO.eprintln s!"  {n}"
  return 1
