module

import Lean
import Lean.DocString
import NewProtocolSpec

/-!
# Names in prose

The documentation is part of the specification, and it names things:
`SafetySpec.vote2NotInSkippedView`, `GcSpec.voted2Retained`,
`Vote1Justification.parentLinked`. A rename leaves those mentions behind, and a
reader who looks one up finds nothing. Nothing in Lean checks them — a docstring
is a string.

So this does. It reads every module docstring and every declaration docstring of
`NewProtocolSpec`, takes each backtick-delimited span that looks like an
identifier, and fails if it names nothing that exists.

    lint

What `new-protocol-docs` checks is the other half: a reference that splices a
declaration fails to build when that declaration is renamed. Neither covers the
other — the reference mentions only what its own prose names, and nothing there
reaches the docstrings.

`Checks.lean` cannot do this. Docstrings of imported declarations are not visible
to `run_meta` in a later module — they live in extension data that is loaded only
on request — so the check has to run as a program, importing the specification
with `loadExts := true`. That is also why it can see the root module's own
docstring, which `Checks` never could: the root imports `Checks`, not the other way
round.
-/

open Lean

/-- Every module of the specification. -/
def specModules (env : Environment) : Array Name :=
  env.allImportedModuleNames.filter fun m =>
    m == `NewProtocolSpec || (`NewProtocolSpec).isPrefixOf m

/-- The text inside each pair of backticks. -/
def backticked (text : String) : List String :=
  let parts := (text.split (· == '`')).toList.map (·.toString)
  let rec go : List String → Nat → List String
    | [], _ => []
    | p :: rest, i => if i % 2 == 1 then p :: go rest (i + 1) else go rest (i + 1)
  go parts 0

/--
Whether a backticked span is naming something rather than quoting a term.

Docstrings put plenty in backticks that is not a name — `v`, `some lock`,
`f + 1`, `s.barredView`, `X/Defs.lean`. What is checked is what cannot be a
local: a name whose first segment is capitalised, or an undotted snake_case one,
which is how every lemma here is spelled.

The gap that leaves is a bare camelCase function like `blockHash`: prose puts
variable names in backticks too, and undotted lowercase cannot tell the two
apart. Written under its namespace it is checked like anything else.
-/
def identifierShaped (s : String) : Bool :=
  let segments := s.splitOn "."
  let capitalised := (segments.head?.bind (·.toList.head?)).any Char.isUpper
  s.toList.all (fun c => c.isAlphanum || c == '_' || c == '\'' || c == '.')
    && s.length > 1 && !s.endsWith ".lean" && !segments.any (·.isEmpty)
    && (capitalised || (segments.length == 1 && s.toList.any (· == '_')))

/--
Names that exist, but not as declarations of this specification.

Two kinds: the packages the specification points at, which are not in scope here,
and Lean syntax, which is a command rather than a constant and so resolves to
nothing however it is spelled. Anything else that fails to resolve is a stale name.
-/
def elsewhere : List String :=
  ["NewProtocolImpl.Demo", "NewProtocolDiff.Corpus", "run_meta"]

/--
Whether a name resolves to a declaration, a module, or something known to be elsewhere.

A declaration may be written bare or under `NewProtocol`, and a module bare or
under `NewProtocolSpec` — the root's own module list names them the short way.
-/
def resolves (env : Environment) (s : String) : Bool :=
  let n := s.toName
  env.contains n || env.contains (`NewProtocol ++ n)
    || (env.getModuleIdx? n).isSome || (env.getModuleIdx? (`NewProtocolSpec ++ n)).isSome
    || elsewhere.contains s

/-- Where a docstring lives, for the report. -/
inductive Site where
  | «module» (name : Name)
  | declaration (name : Name) («module» : Name)

def Site.describe : Site → String
  | .module name => s!"{name} (module docstring)"
  | .declaration name m => s!"{name} (in {m})"

/-- Every backticked name in the specification's docstrings that resolves to nothing. -/
def unresolved (env : Environment) : IO (Array (Site × String)) := do
  let mods := specModules env
  let mut misses := #[]
  for m in mods do
    for d in (getModuleDoc? env m).getD #[] do
      for c in backticked d.doc do
        if identifierShaped c && !resolves env c then
          misses := misses.push (.module m, c)
  for (name, idx) in env.const2ModIdx.toList do
    let some m := env.allImportedModuleNames[idx.toNat]? | continue
    unless mods.contains m do continue
    let some doc ← Lean.findDocString? env name | continue
    for c in backticked doc do
      if identifierShaped c && !resolves env c then
        misses := misses.push (.declaration name m, c)
  return misses

public unsafe def main : IO UInt32 := do
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let env ← importModules #[Import.mk `NewProtocolSpec false true false] Options.empty
    (loadExts := true)
  let misses ← unresolved env
  if misses.isEmpty then
    IO.println "every name in the specification's prose resolves"
    return 0
  IO.eprintln s!"{misses.size} name(s) in prose resolve to nothing:"
  for (site, name) in misses do
    IO.eprintln s!"  `{name}` — {site.describe}"
  return 1
