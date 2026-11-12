import StakeTableImpl

/-!
# Replay timing

Replays a synthetic event sequence, `n` validators each with a few delegations,
through the list and the indexed state, and prints the time each takes.
-/

open StakeTable

def events (n : Nat) : List Event := Id.run do
  let mut es := #[]
  for i in [0:n] do
    es := es.push (.registerV2 ⟨i⟩ (.verified ⟨i⟩ ⟨i⟩) 500)
  for i in [0:n] do
    for j in [0:4] do
      es := es.push (.delegate ⟨n + 4 * i + j⟩ ⟨i⟩ (1000 + j))
  for i in [0:n / 10] do
    es := es.push (.undelegate ⟨n + 4 * i⟩ ⟨i⟩ 1000)
  for i in [0:n / 100] do
    es := es.push (.deregister ⟨i⟩)
  return es.toList

def time (label : String) (act : Unit → IO String) : IO Unit := do
  let start ← IO.monoMsNow
  let out ← act ()
  let stop ← IO.monoMsNow
  IO.println s!"{label}: {stop - start} ms ({out})"

def main (args : List String) : IO Unit := do
  let n := (args.head? >>= String.toNat?).getD 2000
  let es := events n
  IO.println s!"{es.length} events, {n} validators"
  time "indexed" fun _ => pure <| match Indexed.applyAll Indexed.State.empty es with
    | .ok s => s!"{s.order.size} validators"
    | .error _ => "rejected"
  time "list" fun _ => pure <| match applyAll State.empty es with
    | .ok s => s!"{s.validators.length} validators"
    | .error _ => "rejected"
