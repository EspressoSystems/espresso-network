module

public import NewProtocolDiff.Sim

/-!
# Random settings

A run's setting drawn at random within what the specification allows and the Rust
node's test membership can reproduce: four to ten nodes, committees of four or more
of them that change from the third epoch on, and up to as many faulty nodes as a
committee tolerates, each faulty in some epochs or all and doing one of the things
a faulty node can (`NewProtocolDiff.Sim.Behaviour`). GST, `Δ` and `τ` are drawn too, `τ` close to the
least the timing premise allows, and so is how the network delays messages before
and after GST: uniformly or at the extremes, with slow nodes, partitions, messages
held until GST, and certificates of an epoch's last block held from some nodes.
Timeout certificates may name the earliest lock a quorum allows, and certificates
that need faulty votes may reach some nodes only.

What the network delivers stays within the specification's assumptions
(`NewProtocol.Synchrony`), so every run must keep the rules, decide one chain, and
grow it after GST.
-/

@[expose] public section

namespace NewProtocolDiff
namespace Sim

/-- Draws from a seed, as `Sim.rand` does. -/
abbrev Gen := StateM Nat

/-- A number below `n`. -/
def draw (n : Nat) : Gen Nat := do
  let r := ((← get) * 6364136223846793005 + 1442695040888963407) % 2 ^ 64
  set r
  return (r / 2 ^ 33) % (max n 1)

/-- Whether one in `n` happens. -/
def chance (n : Nat) : Gen Bool := (· == 0) <$> draw n

/-- `k` of the nodes `1` to `n`, in random order. -/
def sample (k n : Nat) : Gen (List Nat) := do
  let mut left := (List.range n).map (· + 1)
  let mut out := []
  for _ in List.range (min k n) do
    let i ← draw left.length
    out := out ++ [left[i]!]
    left := left.eraseIdx i
  return out

/-- Every kind of faulty behaviour. -/
def behaviours : List Behaviour :=
  [.silent, .equivocate, .misbehave, .selective, .doubleVote, .lateLeader, .lowLock]

/-- The epochs a run gets faults and committee changes drawn for: more than any run reaches. -/
def epochsDrawn : Nat := 20

/-- The committees: a first one for the first two epochs, then each later epoch changes it one time in three. -/
def drawCommittees (nodes size : Nat) : Gen (List (Nat × List Nat)) := do
  let first ← sample size nodes
  let mut out := [(0, first)]
  for e in List.range' 3 epochsDrawn do
    if ← chance 3 then out := out ++ [(e, ← sample size nodes)]
  return out

/--
Up to as many faulty nodes as a committee tolerates, each with one behaviour, and
faulty in some epochs only or in all of them. The other nodes are honest throughout,
so every committee keeps an honest quorum and some nodes are checked.
-/
def drawFaults (P : Params) : Gen (List (Nat × Nat × Behaviour)) := do
  let faulty ← sample (← draw (P.faultBound + 1)) P.nodes
  let mut out := []
  for k in faulty do
    let b := behaviours[← draw behaviours.length]!
    let always ← chance 2
    for e in List.range epochsDrawn do
      if always || (← chance 2) then out := out ++ [(k, e, b)]
  return out

/-- Partitions before GST: up to two, of up to half the nodes each. -/
def drawPartitions (nodes gst : Nat) : Gen (List (Nat × Nat × List Nat)) := do
  if gst = 0 then return []
  let mut out := []
  for _ in List.range (← draw 3) do
    let start ← draw gst
    let stop := start + 1 + (← draw (gst - start))
    out := out ++ [(start, stop, ← sample (1 + (← draw (nodes / 2))) nodes)]
  return out

/-- The setting of random run `seed`. -/
def randomParams (seed : Nat) : Params :=
  (go).run' (seed * 7919 + 104729)
where
  go : Gen Params := do
    let nodes := 4 + (← draw 7)
    let size ← if ← chance 2 then pure nodes else (4 + ·) <$> draw (nodes - 3)
    let committees ← drawCommittees nodes size
    let delta := 2 + (← draw 4)
    let tau := 8 * delta + 1 + (← draw (2 * delta))
    let gst ← if ← chance 3 then pure 0 else (20 + ·) <$> draw 280
    let preGst := 1 + (← draw (2 * tau))
    let heldUntilGst ← if ← chance 2 then pure 0 else (10 + ·) <$> draw 40
    let boundaryHeld ← if ← chance 2 then pure 0 else (2 + ·) <$> draw 4
    let extremeDelays ← chance 2
    let slow ← if ← chance 3 then (fun k => [1 + k]) <$> draw nodes else pure []
    let partitions ← drawPartitions nodes gst
    let lowTimeoutLocks ← chance 2
    let partialCerts ← chance 2
    let base : Params :=
      { seed, nodes, size, committees, faulty := [], epochHeight := 10, delta, tau, gst, preGst,
        heldUntilGst, boundaryHeld, extremeDelays, slow, partitions, lowTimeoutLocks,
        partialCerts, minHeight := 12, stopHeight := 40, horizon := gst + 400 }
    return { base with byzantine := ← drawFaults base }

/-- A description of a random setting, for a report. -/
def describeRandom (P : Params) : String :=
  let faults := (P.byzantine.map fun (k, e, b) => s!"{k}@{e}:{repr b}")
  s!"seed {P.seed}: {P.nodes} nodes, committees of {P.size} ({P.committees.length - 1} changes), \
    Δ {P.delta}, τ {P.tau}, GST {P.gst}, preGst {P.preGst}, held 1/{P.heldUntilGst}, \
    boundary 1/{P.boundaryHeld}, extreme {P.extremeDelays}, slow {P.slow}, \
    partitions {P.partitions.length}, low locks {P.lowTimeoutLocks}, partial {P.partialCerts}, \
    faults {faults.length}"

end Sim
end NewProtocolDiff
