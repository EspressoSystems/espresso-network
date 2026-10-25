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

A definition must be reproduced with `:::spec`, not only described by its
docstring: the rules, premises and results read it, so a reader has to see what it
says. Abbreviations are exempt, since `{expansion …}` prints them, and so are the
accessors (`accessors`), whose docstring is the whole definition.

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

/--
Definitions that only name a field or pick a constructor's argument, shown by
their docstring alone.
-/
def accessors : List Name :=
  [`NewProtocol.Input.sentBy, `NewProtocol.Message.leaderView,
   `NewProtocol.Message.leaderEpoch, `NewProtocol.Proposal.payloadCommit,
   `NewProtocol.Vote1Data.toVote2, `NewProtocol.ViewNumber.genesis,
   `NewProtocol.Config.anchorCert2, `NewProtocol.Config.startEpoch,
   `NewProtocol.Obligation.epoch]

/-- Whether the document must reproduce `name` with `:::spec`. -/
def needsSpec (env : Environment) (name : Name) : Bool :=
  match env.find? name with
  | some (.defnInfo _) =>
    !(getReducibilityStatusCore env name matches .reducible) && !accessors.contains name
  | _ => false

/-- The declarations a reader must find in the document. -/
def required (env : Environment) : IO (Array Name) := do
  let mut out := #[]
  for (name, idx) in env.const2ModIdx.toList do
    let some m := env.allImportedModuleNames[idx.toNat]? | continue
    unless contractModules.contains m do continue
    if ← isDefinition env name then out := out.push name
  return out ++ mainResults.toArray

/-- The declarations the document shows, by name, through any of `roles`. -/
def shown (text : String)
    (roles : List String := ["{docstring ", "{includeDocstring ", ":::spec "]) :
    NameSet := Id.run do
  let mut out := {}
  for role in roles do
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
  let text ← IO.FS.readFile "Reference.lean"
  let seen := shown text
  let reproduced := shown text [":::spec "]
  let required ← required env
  let missing := required.filter (!seen.contains ·)
  let unreproduced := required.filter fun n => needsSpec env n && !reproduced.contains n
  if missing.isEmpty && unreproduced.isEmpty then
    IO.println "every definition of the contract and every main result reaches the reference"
    return 0
  unless missing.isEmpty do
    IO.eprintln s!"{missing.size} declaration(s) are never shown in Reference.lean:"
    for n in missing.qsort (·.toString < ·.toString) do
      IO.eprintln s!"  {n}"
  unless unreproduced.isEmpty do
    IO.eprintln s!"{unreproduced.size} definition(s) are shown but not reproduced with `:::spec`:"
    for n in unreproduced.qsort (·.toString < ·.toString) do
      IO.eprintln s!"  {n}"
  return 1
