module

public import StakeTableDiff.Decode

/-!
# Comparing the node with the implementation

`StakeTableDiff.checkCase` runs the implementation on one case's input and compares
with the node's result the case records. It returns the differences it found, none if
the two agree.

Events replay on the indexed state (`StakeTable.Indexed.apply`), the implementation
that is proved to refine the specification; the active validators are selected from
it (`StakeTable.Indexed.select`). The rest is compared function by function:
`StakeTable.blockReward`, `StakeTable.payout`, `StakeTable.orderLogs`,
`StakeTable.chunks`, and `StakeTable.scheduleOrder` with `StakeTable.leader`.
-/

@[expose] public section

open Lean StakeTable

namespace StakeTableDiff

/-- A fatal error as the corpus names it. -/
def fatalName : StakeTableError → String
  | .alreadyRegistered _ => "alreadyRegistered"
  | .validatorNotFound _ => "validatorNotFound"
  | .delegatorNotFound _ => "delegatorNotFound"
  | .blsKeyAlreadyUsed _ => "blsKeyAlreadyUsed"
  | .validatorAlreadyExited _ => "validatorAlreadyExited"
  | .zeroDelegatorStake _ => "zeroDelegatorStake"
  | .insufficientStake => "insufficientStake"
  | .authenticationFailed => "authenticationFailed"
  | .invalidCommission _ _ => "invalidCommission"
  | .x25519KeyAlreadyUsed _ => "x25519KeyAlreadyUsed"
  | .stakeOverflow => "stakeOverflow"

/-- An expected error as the corpus names it. -/
def skippedName : ExpectedError → String
  | .schnorrKeyAlreadyUsed _ => "schnorrKeyAlreadyUsed"
  | .invalidBlsKey => "invalidBlsKey"
  | .invalidSchnorrKey => "invalidSchnorrKey"

/--
Replay events from the empty indexed state: each outcome as the corpus names it, and
the state if no event was fatal. A fatal event stops the replay.
-/
def replay (es : List Event) : List String × Option Indexed.State := Id.run do
  let mut s := Indexed.State.empty
  let mut out := #[]
  for e in es do
    match Indexed.apply s e with
    | .ok (.applied s') =>
      s := s'
      out := out.push "applied"
    | .ok (.skipped err s') =>
      s := s'
      out := out.push s!"skipped:{skippedName err}"
    | .error err =>
      return ((out.push s!"fatal:{fatalName err}").toList, none)
  return (out.toList, some s)

/-- A difference, if `node` and `lean` differ. -/
def differ {α : Type} [BEq α] [Repr α] (what : String) (node lean : α) : List String :=
  if node == lean then [] else [s!"{what}: node {repr node}, Lean {repr lean}"]

def checkReplay (j : Json) : Except String (List String) := do
  let events ← arr j "events" event
  let outcomes ← arr j "outcomes" Json.getStr?
  let (lean, state) := replay events
  let firstDiff := (List.range (max outcomes.length lean.length)).find? fun i =>
    outcomes[i]? != lean[i]?
  if let some i := firstDiff then
    return [s!"event {i} ({repr events[i]?}): node {outcomes[i]?}, Lean {lean[i]?}"]
  match ← opt j "state" stateView, state with
  | none, none => return []
  | some _, none | none, some _ => return ["one replay finished, the other did not"]
  | some node, some s =>
    let mut diffs := differ "state" node (viewOf s)
    for sel in ← arr j "selections" pure do
      let usable ← arr sel "usable" Json.getStr?
      let nodeActive ← opt sel "active" fun a => do (← a.getArr?).toList.mapM ident
      let leanActive := (Indexed.select (fun a => usable.contains a.toString) s).map
        (·.map (·.toNat))
      diffs := diffs ++ differ "active validators" nodeActive leanActive
    return diffs

def checkReward (j : Json) : Except String (List String) := do
  let supply ← amount j "supply"
  let stake ← amount j "stake"
  let avgMs ← num j "avgMs"
  let node ← opt j "reward" decimalJ
  return differ s!"reward for supply {supply}, stake {stake}, {avgMs} ms" node
    (blockReward supply stake avgMs)

/-- What each account is credited, summed over its credits, by account. -/
def credits (cs : List (Nat × Nat)) : List (Nat × Nat) :=
  let accounts := sortNats (cs.map (·.1)).eraseDups
  accounts.map fun a => (a, ((cs.filter (·.1 == a)).map (·.2)).sum)

def checkPayout (j : Json) : Except String (List String) := do
  let account ← address j "account"
  let delegators ← arr j "delegators" (pair (fun d => Address.mk <$> ident d) decimalJ)
  let v : Validator :=
    { account, blsKey := none, schnorrKey := none, x25519Key := none, p2pAddr := none,
      stake := ← amount j "stake", commission := ← num j "commission", delegators,
      authenticated := true }
  let reward ← amount j "reward"
  let node ← opt j "credits" fun cs => do credits <$> (← cs.getArr?).toList.mapM (pair ident decimalJ)
  let lean := (payout v reward).toOption.map fun cs => credits (cs.map fun (a, n) => (a.toNat, n))
  return differ s!"payout of {reward} at commission {v.commission}" node lean

def checkOrder (j : Json) : Except String (List String) := do
  let keys ← arr j "keys" (pair Json.getNat? Json.getNat?)
  let node ← arr j "order" Json.getNat?
  return differ s!"order of {keys.length} logs" node ((orderLogs keys.zipIdx).map (·.2))

def checkChunks (j : Json) : Except String (List String) := do
  let first ← num j "first"
  let last ← num j "last"
  let size ← num j "size"
  let node ← arr j "chunks" (pair Json.getNat? Json.getNat?)
  return differ s!"chunks of [{first}, {last}] by {size}" node (chunks first last size)

def checkSchedule (j : Json) : Except String (List String) := do
  let bytes (j : Json) : Except String (List Nat) := do
    return (← hexBytes (← j.getStr?)).map (·.toNat)
  let drb ← bytes (← field j "drb")
  let entries ← arr j "entries" fun e => do
    let (key, stake) ← pair bytes decimalJ e
    return ({ key, stake } : Entry)
  let ordered := scheduleOrder drb entries
  let position (e : Entry) := entries.idxOf e
  let mut diffs := differ "schedule order" (← arr j "order" Json.getNat?) (ordered.map position)
  let hashes ← arr j "hashes" decimalJ
  let leaders ← arr j "leaders" Json.getNat?
  if hashes.length != leaders.length then
    throw s!"{hashes.length} view hashes but {leaders.length} leaders"
  for (view, h, node) in (List.range hashes.length).zip (hashes.zip leaders) do
    diffs := diffs ++ differ s!"leader of view {view}" (some node) ((leader ordered h).map position)
  return diffs

/-- The differences between the node and the implementation on one case. -/
def checkCase (j : Json) : Except String (List String) := do
  match ← str j "kind" with
  | "replay" => checkReplay j
  | "reward" => checkReward j
  | "payout" => checkPayout j
  | "order" => checkOrder j
  | "chunks" => checkChunks j
  | "schedule" => checkSchedule j
  | k => throw s!"unknown kind {k}"

end StakeTableDiff
