module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run

/-!
# What the decide stream guarantees, as an invariant

`DecideInv` is the statement; `NewProtocolSpec.DecideStream` proves it holds
at every reachable state, and `NewProtocolSpec.DecideStream.Lemmas` carries the
induction.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/--
What the decide stream guarantees, as an invariant of the state.

A decide is never taken back: a decided view above the floor still holds the
block it decided, so nothing may take it away or swap it out from under the
record. This is retention on one node, not agreement between nodes — that is
`decideSafety`. Below the floor the node has given up; state may be pruned and
nothing is claimed.

There is deliberately no closure clause — the stream promises nothing about
a decided block's ancestors. What each *event* guarantees is
`StepSpec.decideJustified`, a self-certifying chain.

What the stream as a whole does not guarantee is **completeness**. Consensus
provides agreement and progress; it cannot promise that one node's stream
contains every committed block — a node restarting from an anchor never sees
what preceded it, a late joiner sees no history at all, and under partial
synchrony any node falls arbitrarily far behind. So a missing ancestor truncates
a decide rather than blocking it, and the skipped view is delivered only if a
`Cert2` of its own turns up while the view is still above the floor. A node must
not close the gap by re-deriving finality from ancestry — an event so justified
would need the stream's history to check it — and recovering history in general
is the sync and archival layer's job, not this stream's.

`Config.decideBuffer` bounds the courtesy: it retains decide inputs behind the
watermark so a late `Cert2` can still land, and past it a gap is permanent on
this node.
-/
structure DecideInv (s : NodeState) : Prop where
  /-- A decided view still holds the block it decided. -/
  held : ∀ v, s.aboveDecideFloor cfg v → s.decidedViews v → (s.proposals v).isSome

end NewProtocol
