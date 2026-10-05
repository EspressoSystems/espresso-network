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
owed from `n` on is taken *at or after* `n` (`Run.EmitsFrom`).

Read what that amounts to. Taking an action sets its mark, which is one of the
things its enabledness denies, so the antecedent and the conclusion cannot both
hold: each field says that no
action stays enabled for ever, and nothing more. The bound on the conclusion adds
no strength for the same reason — an action taken before `n` would have left a
mark the node may drop only where the action is barred anyway — and is there so
that a consumer need not re-derive that. So each field is in force precisely as a
prohibition on indefinite deferral: a node that holds what justifies an action,
and that nothing else overtakes, may not put it off for ever. The four results of
`NewProtocolSpec.Progress` consume it in that form, by contradiction.

What it does *not* say is that the action is ever justified, nor that anything
arrives to justify it; `NewProtocolSpec.Progress` is how far these reach, and
what they need beside them.

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
