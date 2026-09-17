module

public import NewProtocolImpl.Floor
public import NewProtocolSpec

/-!
# The machine as a conforming implementation

That the reference machine satisfies `Conforms`, whose definition lives
with the specification.

The machine's representation invariant is `Impl.WF`, which restores the
keying the specification has no reason to ask for.

Conformance splits the way `Conforms` does. Safety is `NextConforms` and
`GcConforms`, carried along `NextPreservesWF` and `GcPreservesWF`.
Progress is the eagerness route: `Impl.Settled` — nothing is owed — holds
initially, is re-established by every step and survives collection, so
`WeaklyFair`'s antecedent is never satisfied for any action it names.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-! ## The abstraction -/

/--
Abstraction from the protocol state to the specification state.

Trusted, and deliberately *boring*: every field is a relabelling — a lookup
or a membership test on the same-named machine field, nothing else. Any
computation here could define a `StepSpec` condition into truth and make
conformance vacuous; auditors should treat anything beyond relabelling as a
red flag.
-/
def Impl.State.abstract (s : Impl.State) : NodeState where
  proposals := fun v => s.proposals.get? v
  admitted := fun v => s.admitted.get? v
  proposedViews := fun v => v ∈ s.proposedViews
  vidShares := fun v => s.vidShares.get? v
  validated := fun v => s.validated.get? v
  blocksReconstructed := fun v pc => (v, pc) ∈ s.blocksReconstructed
  headers := fun v h => s.headers.get? (v, h)
  cert1s := fun v => s.cert1s.get? v
  cert2s := fun v => s.cert2s.get? v
  timeoutCerts := fun v => s.timeoutCerts.get? v
  lockedCert := s.lockedCert
  decidedViews := fun v => v ∈ s.decidedViews
  voted1Views := fun v => v ∈ s.voted1Views
  vote1Branches := fun v => s.vote1Branches.get? v
  voted2Views := fun v => v ∈ s.voted2Views
  timeoutView := s.timeoutView
  barredView := s.barredView
  currentView := s.currentView

/-! ## The representation invariant -/

/--
The machine's representation invariant: entries are stored under the view
their value names, held proposals are well-formed, every admitted proposal is
also held, everything the validity table records really is valid, and the
decided set is never empty.

The specification constrains behaviour, not storage, so it asks for no such
invariant. It resurfaces here because several `StepSpec` obligations look an
entry up by the view a *value* names (`lockJustified`, `decideJustified`),
which matches the machine's key-based lookups only on a well-keyed state — and
because the machine reads the decide floor off the maximum of the decided
views, which is the specification's floor only while there is a maximum
(`Impl.aboveFloor_iff`).
-/
structure Impl.WF (s : Impl.State) : Prop where
  proposals : ∀ v p, s.proposals.get? v = some p → p.viewNumber = v

  /--
  Every held proposal is well-formed — except the anchor, which has no
  parent to point back to. Nothing walks past it: its view is decided from
  the start, so the decide walk stops there rather than stepping through.
  -/
  proposalsWellFormed : ∀ v p, s.proposals.get? v = some p → v ≠ ViewNumber.genesis →
    Impl.wellFormed p
  admitted : ∀ v p, s.admitted.get? v = some p → s.proposals.get? v = some p
  cert1s : ∀ v c, s.cert1s.get? v = some c → c.view = v
  cert2s : ∀ v c, s.cert2s.get? v = some c → c.view = v
  timeoutCerts : ∀ v tc, s.timeoutCerts.get? v = some tc → tc.view + 1 = v

  /--
  Everything the validity table records is a valid block.

  Unlike the rest of this structure, this is not bookkeeping the specification
  declined to ask for: it is the machine's half of `ValidityReported`, and the
  only thing that lets it discharge `Vote1Justification.blockValid`, which is a
  statement about the block rather than about having been told. Preserving it
  is where the assumption is consumed — `Input.blockValidated` is the only arm
  that writes the table, and `ValidityReported` is exactly what that arm needs.
  -/
  validated : ∀ v p, s.validated.get? v = some (blockHash p) → BlockValid p

  /-- Something is decided, so the floor is where `Impl.State.floor` says. -/
  decided : s.decidedViews.max?.isSome

  /--
  A branch record sits where a vote1 was cast, or in abandoned territory.

  What it rules out is a record with no vote behind it *in a view still in play* —
  and that is what makes the vote1's record safe to write: the vote is
  only cast where no vote was cast and the view is not abandoned, so the slot it
  writes is free, and `SafetySpec.vote1BranchesRetained` is not at risk of an
  overwrite. The second disjunct is collection's residue: it prunes the vote
  marks at the bar and the branch records at the decide floor, so between the two
  watermarks a record outlives its mark by design (`GcSpec.vote1BranchesRetained`).
  -/
  branches : ∀ v u, s.vote1Branches.get? v = some u → v ∈ s.voted1Views ∨ v ≤ s.barredView

namespace Impl

/-- The machine's floor test is the specification's. -/
theorem aboveFloor_abstract {cfg : Config} {s : State} (hwf : WF s) (v : ViewNumber) :
    s.aboveFloor cfg v = true ↔ s.abstract.aboveDecideFloor cfg v :=
  aboveFloor_iff cfg s hwf.decided v

end Impl

/-! ## Safety -/

/-- The invariant is preserved by every transition. -/
def NextPreservesWF : Prop :=
  ∀ (s : Impl.State) (input : Input), ValidityReported input →
    Impl.WF s → Impl.WF (Impl.next cfg leader node s input).1

/--
Every transition of the machine from a state satisfying the invariant
satisfies the step specification.
-/
def NextConforms : Prop :=
  ∀ (s : Impl.State) (input : Input), ValidityReported input → Impl.WF s →
    let (s', outputs) := Impl.next cfg leader node s input
    StepSpec cfg leader node s.abstract input outputs s'.abstract

/-- Collection satisfies the specification's pruning rule. -/
def GcConforms : Prop :=
  ∀ s : Impl.State, Impl.WF s → GcSpec cfg s.abstract (s.gc cfg).abstract

/-- Collection preserves the machine's invariant. -/
def GcPreservesWF : Prop :=
  ∀ s : Impl.State, Impl.WF s → Impl.WF (s.gc cfg)

/-! ## Progress -/

namespace Impl

/--
Nothing is owed.

The machine's route to the progress half of `Conforms`: an implementation
that leaves nothing enabled makes `WeaklyFair`'s antecedent — enabled from
some point on for ever — false at every state, so every field of it holds
vacuously.

Deferring would be legal too, and would owe the genuine fairness argument
instead. Eagerness is the machine's own choice.
-/
structure Settled (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)
    (s : State) : Prop where
  vote1 : ∀ p, ¬ Vote1Enabled s.abstract p
  vote2 : ∀ p, ¬ Vote2Enabled cfg s.abstract p
  decide : ∀ v, ¬ DecideEnabled cfg s.abstract v
  propose : ∀ p, ¬ ProposeEnabled leader node s.abstract p

end Impl

/-- A fresh node owes nothing: it holds the anchor and has taken no input. -/
def InitialSettled : Prop :=
  ConfigCoherent cfg → Impl.Settled cfg leader node (Impl.initial cfg)

/-- Every step ends with nothing owed. -/
def NextSettles : Prop :=
  ∀ (s : Impl.State) (input : Input), ValidityReported input → Impl.WF s →
    Impl.Settled cfg leader node (Impl.next cfg leader node s input).1

/--
Collection creates no obligation.

Pruning only removes content and raises the bar, and every enabledness
predicate that could be turned on by a removal — a view becoming undecided, a
branch record going missing — is guarded by the decide floor, which
`GcSpec.floorStable` keeps in place.
-/
def GcSettles : Prop :=
  ∀ s : Impl.State, Impl.WF s →
    Impl.Settled cfg leader node s → Impl.Settled cfg leader node (s.gc cfg)

/-! ## Conformance -/

/--
The machine is a conforming implementation — and therefore a witness that
the specification can be met.

Safety rests on `NextPreservesWF`, `NextConforms`, `GcPreservesWF` and
`GcConforms` (the initial state's invariant is `Impl.initial_wf`); progress
on `InitialSettled`, `NextSettles` and `GcSettles`.
-/
def ProtocolConforms : Prop :=
  ConfigCoherent cfg →
    Conforms cfg leader node (Impl.initial cfg) (Impl.next cfg leader node)
      (Impl.State.gc cfg) Impl.State.abstract Impl.WF ValidityReported

end NewProtocol
