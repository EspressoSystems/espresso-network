module

import StakeTableDiff

/-!
Check the node's stake table cases against the implementation.

    check <file-or-directory>...

Each argument is a `*.jsonl` file the node's corpus test wrote, or a directory of
them. Every line is a case: an input and the node's result. The implementation runs
on each input (`StakeTableDiff.checkCase`), and any difference is printed with the
file and line. The exit status is 1 if a case differs or cannot be read, or if there
is no case file or a file without cases, and 2 on bad usage.
-/

open Lean StakeTableDiff

/-- The case files an argument names: the file itself, or the `*.jsonl` files of a directory. -/
def casesUnder (arg : String) : IO (Array System.FilePath) := do
  let path : System.FilePath := arg
  if ← path.isDir then
    let entries ← path.readDir
    return (entries.map (·.path) |>.filter (·.extension == some "jsonl")).qsort
      (·.toString < ·.toString)
  else
    return #[path]

/-- How many cases a file held, and the problems found in it. -/
def checkFile (path : System.FilePath) : IO (Nat × Array String) := do
  let mut cases := 0
  let mut problems := #[]
  for (line, n) in ((← IO.FS.readFile path).splitOn "\n").zipIdx do
    if line.trimAscii.isEmpty then continue
    cases := cases + 1
    let result := do checkCase (← Json.parse line)
    match result with
    | .ok [] => pure ()
    | .ok diffs =>
      for d in diffs do problems := problems.push s!"{path}:{n + 1}: {d}"
    | .error e => problems := problems.push s!"{path}:{n + 1}: unreadable: {e}"
  return (cases, problems)

public def main (args : List String) : IO UInt32 := do
  if args.isEmpty || args.any (·.startsWith "--") then
    IO.eprintln "usage: check <file-or-directory>..."
    return 2
  let mut files := #[]
  for arg in args do
    files := files ++ (← casesUnder arg)
  if files.isEmpty then
    IO.eprintln "no case files"
    return 1
  let mut failed := false
  for path in files do
    let (cases, problems) ← checkFile path
    IO.println s!"{path}: {cases} cases, {problems.size} differences"
    for p in problems.toList.take 20 do
      IO.println s!"  {p}"
    if problems.size > 20 then
      IO.println s!"  and {problems.size - 20} more"
    failed := failed || !problems.isEmpty || cases == 0
  return if failed then 1 else 0
