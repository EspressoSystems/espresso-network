module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run
public import NewProtocolSpec.Liveness

/-!
# The network, as an object

The committee, one run per honest node, and a certificate defined as the votes
behind it — with the premises that come as fields of them. `NewProtocolSpec.Network`
proves what the safety argument needs from these.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config)

/--
The committee, as a quorum system.

`Committee.intersect` is the whole of what the safety argument takes from
stake: any two quorums share an honest member. Under a two-thirds threshold
with less than a third faulty, that is a counting argument; here it is the
premise.
-/
structure Committee where
  /-- Nodes that follow the specification. -/
  honest : PubKey → Prop

  /-- The sets of signers that suffice to form a certificate. -/
  Quorum : (PubKey → Prop) → Prop

  /-- Any two quorums share an honest member. -/
  intersect : ∀ q q', Quorum q → Quorum q' → ∃ k, q k ∧ q' k ∧ honest k

/-- Honest node `k` emitted a timeout vote for view `v` at some point in its run. -/
def CastTimeout {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) (v : ViewNumber) : Prop :=
  Run.Emits r fun o => ∃ e, o = Output.send (.timeoutVote ⟨(), v, node⟩ e)

/-- Honest node `k` emitted this vote1 at some point in its run. -/
def CastVote1 {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) (v : Vote1) : Prop :=
  Run.Emits r fun o => o = Output.send (.vote1 v)

/--
A quorum signed `c.data` in `c.view`.

Phrased over the runs directly, like `TimeoutCertBacked`, so that the
`Network` fields can mention it before the structure is complete.
-/
def Cert1Backed {cfg : Config} {C : Committee}
    (run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k)) (c : Cert1) : Prop :=
  ∃ q, C.Quorum q ∧ ∀ k, q k → ∀ h : C.honest k, CastVote1 (run k h) ⟨c.data, c.view, k⟩

/--
One step of one honest node: what `Network.Before` orders.
-/
structure NodeStep (C : Committee) where
  /-- Whose step. -/
  node : PubKey
  /-- And that it is a node the model constrains. -/
  honest : C.honest node
  /-- Which of its steps. -/
  index : Nat

/--
A quorum signed `c`, every one of them before the step `s`.

The same statement as `Cert1Backed` with the votes placed before something,
which is what lets a certificate be *older* than an action taken because of it.
That distinction is the whole of what the ordering buys: a node cannot hold a
certificate its own vote is still needed to form.
-/
def Cert1BackedBefore {cfg : Config} {C : Committee}
    (run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k))
    (Before : NodeStep C → NodeStep C → Prop) (c : Cert1) (s : NodeStep C) : Prop :=
  ∃ q, C.Quorum q ∧ ∀ k, q k → ∀ h : C.honest k,
    ∃ n, Output.send (.vote1 ⟨c.data, c.view, k⟩) ∈ (Run.event (run k h) n).outputs
      ∧ Before ⟨k, h, n⟩ s

/-- Forgetting when the votes were cast. -/
theorem Cert1BackedBefore.backed {cfg : Config} {C : Committee}
    {run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k)}
    {Before : NodeStep C → NodeStep C → Prop} {c : Cert1} {s : NodeStep C}
    (h : Cert1BackedBefore run Before c s) : Cert1Backed run c := by
  obtain ⟨q, hq, hvotes⟩ := h
  exact ⟨q, hq, fun k hk hh => by
    obtain ⟨n, hmem, -⟩ := hvotes k hk hh
    exact ⟨n, _, hmem, rfl⟩⟩

/--
A quorum timed out on view `tc.view`.

Phrased over the runs directly rather than over a `Network`, so that
`Network.evidenceValid` can mention it before the structure is complete.
-/
def TimeoutCertBacked {cfg : Config} {C : Committee}
    (run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k)) (tc : TimeoutCert) : Prop :=
  ∃ q, C.Quorum q ∧ ∀ k, q k → ∀ h : C.honest k, CastTimeout (run k h) tc.view

/--
One run per honest node, started from the initial state.

Four of the fields are the verification layer's contract, and all four say the
same kind of thing: what an honest node was handed is what the network really
produced. Without them a node could act on a forged certificate and no argument
about quorums would reach its behaviour. Two of the four also place the
production *before* the delivery, which is what `Network.Before` is for.
-/
structure Network (C : Committee) where
  /--
  One run per honest node, obeying the safety clauses.

  Faulty nodes have no run, which is how they are modelled: nothing constrains
  what they emit.
  -/
  run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k)

  /-- Every honest node starts where the specification starts. -/
  start : ∀ k h, Run.state (run k h) 0 = NodeState.initial cfg

  /--
  Happens-before, across nodes.

  The model is otherwise a set of runs indexed independently, with nothing
  relating one node's step to another's. That is enough for every rule a node
  must obey, since those read only its own state, but not for the one fact the
  safety argument needs about certificates: that a quorum's votes precede
  anything done because of the certificate they form.

  A partial order, deliberately, not a clock. Concurrent steps at different
  nodes are incomparable, as asynchrony leaves them, and the argument never asks
  which of two came first — only that a chain of causes cannot descend for ever
  (`Network.beforeWF`). Nothing here is a requirement on an implementation: no
  node reads this, and none could. It is the vocabulary in which
  `Network.cert1Delivered` and `Network.timeoutOneHonestBacked` can say
  "before".
  -/
  Before : NodeStep C → NodeStep C → Prop

  /-- A node's own steps happen in order. -/
  beforeNext : ∀ k h n, Before ⟨k, h, n⟩ ⟨k, h, n + 1⟩

  /-- Causes of causes are causes. -/
  beforeTrans : ∀ {a b c}, Before a b → Before b c → Before a c

  /--
  No infinite chain of causes.

  What makes the two regresses terminate. A certificate is justified by votes
  cast earlier, which may themselves have been cast by nodes acting on
  certificates; a one-honest timeout vote is justified by earlier timeout votes,
  which may themselves be one-honest. Both chains reach a first vote because of
  this. Totality would give it too, but is more than asynchrony provides.
  -/
  beforeWF : WellFounded Before

  /--
  A proposal delivered to an honest node carries genuine timeout evidence.

  The verification layer's contract, and the counterpart of assuming
  certificates arrive verified. `ProposalWellFormed` only checks that the
  carried certificate names the right view — a syntactic check on a value the
  proposer chose. Safety turns on a skipped view having *really* timed out, so
  a forged certificate here would collapse the argument entirely.

  A verification layer outside consensus is expected to establish this before
  the proposal is delivered.
  -/
  evidenceValid : ∀ k (h : C.honest k) n sender p vid tc,
    Run.Consumes (run k h) n (Input.proposal sender p vid) →
    p.timeoutEvidence = some tc → TimeoutCertBacked run tc

  /--
  A one-honest timeout indication follows an honest node's timeout vote for that
  view, cast earlier.

  The threshold is f+1 timeout votes, so one of them is honest; what is assumed
  here is that its vote came first. The same shape as
  `Network.cert1Delivered` — what you were handed was produced before you were
  handed it — and it is what `oneHonest_reached` descends on to conclude that
  some honest node really was in the view. Before the ordering existed that
  conclusion had to be assumed outright, because the regress through other
  one-honest votes had no "first" to bottom out at.
  -/
  timeoutOneHonestBacked : ∀ k (h : C.honest k) n v,
    Run.Consumes (run k h) n (Input.timeoutOneHonest v) →
      ∃ j, ∃ hj : C.honest j, ∃ m e,
        Output.send (.timeoutVote ⟨(), v, j⟩ e) ∈ (Run.event (run j hj) m).outputs
          ∧ Before ⟨j, hj, m⟩ ⟨k, h, n⟩

  /--
  A `Cert1` handed to an honest node was signed before it was handed over.

  The causal contract for certificates, and one of the two premises phrased over
  `Network.Before` — `Network.timeoutOneHonestBacked` is the other. It says
  no more than that a certificate cannot be delivered before it exists, but that
  is what rules out a node's own vote being among the votes that justified
  casting it: the backing votes precede the step that delivers the certificate,
  and this node's vote in that step does not.

  Both doors are covered, because `SafetySpec.cert1Provenance` admits a
  certificate through either of them. Keyed on the delivery rather than on
  holding the certificate, which is what makes "before" strict. A node that
  receives a certificate and votes in the same step holds it only in that step's
  *post* state, so a contract phrased over the state it is held in would permit
  exactly the circle this excludes.
  -/
  cert1Delivered : ∀ k (h : C.honest k) n c, c.view ≠ ViewNumber.genesis →
    (Run.Consumes (run k h) n (Input.certificate1 c)
      ∨ Run.Consumes (run k h) n (Input.advanceView c)) →
      Cert1BackedBefore run Before c ⟨k, h, n⟩

  /--
  A proposal delivered to an honest node names a real parent.

  The companion of `Network.evidenceValid`, and owed for the same reason:
  `ProposalWellFormed` constrains only where the parent certificate *points*,
  never that it exists. The safety induction applies its hypothesis to the
  parent certificate, which is worth nothing unless a quorum stands behind it.

  Established by the same verification layer, before delivery.
  -/
  parentCertValid : ∀ k (h : C.honest k) n sender p vid,
    Run.Consumes (run k h) n (Input.proposal sender p vid) → Cert1Backed run p.parentCert

/-- Honest node `k` emitted this vote1 at some point in its run. -/
def Network.Cast1 {C : Committee} (N : Network cfg C)
    (k : PubKey) (h : C.honest k) (v : Vote1) : Prop :=
  CastVote1 (N.run k h) v

/-- Honest node `k` emitted this vote2 at some point in its run. -/
def Network.Cast2 {C : Committee} (N : Network cfg C)
    (k : PubKey) (h : C.honest k) (v : Vote2) : Prop :=
  Run.Emits (N.run k h) fun o => o = Output.send (.vote2 v)

/--
A `Cert1` is valid when a quorum signed its data in its view.

Only the honest members of the quorum are held to having voted; the rest are
faulty and may sign anything. That is exactly the strength quorum
intersection needs.
-/
def Network.ValidCert1 {C : Committee} (N : Network cfg C) (c : Cert1) : Prop :=
  ∃ q, C.Quorum q ∧ ∀ k, q k → ∀ h : C.honest k, Network.Cast1 cfg N k h ⟨c.data, c.view, k⟩

/-- A `Cert2` is valid when a quorum signed its data in its view. -/
def Network.ValidCert2 {C : Committee} (N : Network cfg C) (c : Cert2) : Prop :=
  ∃ q, C.Quorum q ∧ ∀ k, q k → ∀ h : C.honest k, Network.Cast2 cfg N k h ⟨c.data, c.view, k⟩

/-- Index order is causal order, within one node's run. -/
theorem Network.before_of_lt {C : Committee} (N : Network cfg C) (k : PubKey) (h : C.honest k)
    {m n : Nat} (hlt : m < n) : N.Before ⟨k, h, m⟩ ⟨k, h, n⟩ := by
  induction hlt with
  | refl => exact N.beforeNext k h m
  | step hlt ih => exact N.beforeTrans ih (N.beforeNext k h _)

end NewProtocol
