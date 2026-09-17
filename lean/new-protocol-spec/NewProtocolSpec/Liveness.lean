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
some point on is eventually taken.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
Honest nodes do not starve their own obligations: an action enabled from
some point on forever is eventually taken.

Each field pairs an enabledness predicate from `NewProtocolSpec.Step` with
the output it owes. Every pairing is exact — taking the action clears its
mark and so disables it, meaning the antecedent really does say "the node
sat on this forever".

Every obligation is discharged by a `Message` or a decide, since those are
the only outputs there are: each enabledness predicate reads only what the
node holds, so nothing a node owes ever waits on state it would have to
procure from outside. A decide, in particular, is owed for a certified block
*in hand* — one that never arrives is a gap, not an obligation (see
`DecideInv`).
-/
structure WeaklyFair {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    (r : Run cfg (StepSpec cfg leader node)) : Prop where
  /-- A vote1 owed from some point on is eventually cast. -/
  vote1 : ∀ p, r.EventuallyAlways (fun s => Vote1Enabled s p) →
    r.Emits fun o => ∃ vote, o = Output.send (.vote1 vote) ∧ vote.view = p.viewNumber

  /-- A vote2 owed from some point on is eventually cast. -/
  vote2 : ∀ p, r.EventuallyAlways (fun s => Vote2Enabled cfg s p) →
    r.Emits fun o => ∃ vote, o = Output.send (.vote2 vote) ∧ vote.view = p.viewNumber

  /-- A decide owed from some point on eventually delivers that view. -/
  decide : ∀ v, r.EventuallyAlways (fun s => DecideEnabled cfg s v) →
    r.Emits fun o => ∃ blocks c1 c2 b, o = Output.decided blocks c1 c2 ∧ b ∈ blocks
      ∧ b.viewNumber = v

  /-- A proposal owed from some point on is eventually sent. -/
  propose : ∀ p, r.EventuallyAlways (fun s => ProposeEnabled leader node s p) →
    r.Emits fun o => ∃ q, o = Output.send (.proposal q) ∧ q.viewNumber = p.viewNumber

end NewProtocol
