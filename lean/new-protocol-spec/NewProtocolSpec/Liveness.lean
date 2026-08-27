module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run

/-!
# Fairness

`WeaklyFair`, the per-node half of liveness: an action continuously enabled from
some point on is eventually taken, at or after that point.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
Honest nodes do not starve their own obligations: an action enabled from
some point on forever is eventually taken.

Each field pairs an enabledness predicate from `NewProtocolSpec.Step` with
the output it owes, and both halves are anchored at the same step `n`: what is
owed from `n` on is taken *at or after* `n` (`Run.EmitsFrom`). Anything weaker
would let a node discharge an obligation with an action it took before the
obligation arose.

Every pairing is exact — taking the action clears its mark and so disables it,
meaning the antecedent really does say "the node sat on this forever". So each
field is in force precisely as a prohibition on indefinite deferral: a node that
holds what justifies an action, and that nothing else overtakes, may not put it
off for ever. What it does *not* say is that the action is ever justified, nor
that anything arrives to justify it; `NewProtocolSpec.Progress` is how far these
reach, and what they need beside them.

Every obligation is discharged by a `Message` or a decide, since those are
the only outputs there are: each enabledness predicate reads only what the
node holds, so nothing a node owes ever waits on state it would have to
procure from outside. A decide, in particular, is owed for a certified block
*in hand* — one that never arrives is a gap, not an obligation (see
`DecideInv`).
-/
structure WeaklyFair {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) : Prop where
  /-- A vote1 owed from step `n` on is cast at or after `n`. -/
  vote1 : ∀ p n, r.AlwaysFrom n (fun s => Vote1Enabled s p) →
    r.EmitsFrom n fun o => ∃ vote, o = Output.send (.vote1 vote) ∧ vote.view = p.viewNumber

  /-- A vote2 owed from step `n` on is cast at or after `n`. -/
  vote2 : ∀ p n, r.AlwaysFrom n (fun s => Vote2Enabled cfg s p) →
    r.EmitsFrom n fun o => ∃ vote, o = Output.send (.vote2 vote) ∧ vote.view = p.viewNumber

  /-- A decide owed from step `n` on delivers that view at or after `n`. -/
  decide : ∀ v n, r.AlwaysFrom n (fun s => DecideEnabled cfg s v) →
    r.EmitsFrom n fun o => ∃ blocks c1 c2 b, o = Output.decided blocks c1 c2 ∧ b ∈ blocks
      ∧ b.viewNumber = v

  /-- A proposal owed from step `n` on is sent at or after `n`. -/
  propose : ∀ p n, r.AlwaysFrom n (fun s => ProposeEnabled leader node s p) →
    r.EmitsFrom n fun o => ∃ q, o = Output.send (.proposal q) ∧ q.viewNumber = p.viewNumber

end NewProtocol
