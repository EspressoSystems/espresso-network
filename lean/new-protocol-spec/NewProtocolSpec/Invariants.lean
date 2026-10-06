module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface
public import NewProtocolSpec.State
public import NewProtocolSpec.Step
public import NewProtocolSpec.Gc
public import NewProtocolSpec.Run

/-!
# What a node's own state satisfies

Facts about the state a node keeps, true at every state reachable under the
rules and proved by induction over the run rather than read off a clause.

They are not obligations. Nothing here asks an implementation for anything: each
is a consequence of the clauses already collected, so an implementation that
satisfies `StepSpec` and `GcSpec` satisfies these whether it meant to or not.
What they are for is the places where two rules read different fields for the
same purpose, and something has to say the two agree.

`NewProtocolSpec.DecideStream` holds the other invariant of this kind, kept apart
because what it is about — the stream handed to the application — is a subject in
its own right.
-/

@[expose] public section

namespace NewProtocol

/--
What a node has admitted above the decide floor, it holds.

`Vote2Justification` reads `NodeState.admitted`, while casting the vote2 it
justifies moves the lock, and `SafetySpec.lockJustified` reads
`NodeState.proposals`. Nothing in either says the two agree, so it is proved
here: admission writes the proposal into both (`SafetySpec.admissionJustified`),
and above the floor neither kind of step may drop it
(`StepSpec.contentRetained`, `GcSpec.keepsDecideAboveFloor`). Below the floor the
two may part, and nothing needs them not to.

Stated over `StepSpec` rather than `SafetySpec`: retention is not a safety
clause, and a node held to the safety clauses alone may drop what it admitted.

Stated of a node started from `NodeState.initial`, which is what `Network.start`
gives; a bare `Run` does not say where it began.
-/
theorem admitted_held {cfg : Config} {leader : ViewNumber → Option PubKey} {node : PubKey}
    {s : NodeState}
    (hr : Reachable cfg (StepSpec cfg leader node) (NodeState.initial cfg) s) :
    ∀ v p, s.admitted v = some p → s.aboveDecideFloor cfg v → s.proposals v = some p := by
  induction hr with
  | refl => intro v p hp; simp [NodeState.initial] at hp
  | step _ ht ih =>
    intro v p hp hfl
    cases ht with
    | step hs =>
      rcases SafetySpec.admissionJustified hs.toSafetySpec v p hp with
        hold | ⟨-, -, -, -, -, -, -, -, -, hheld⟩
      · have hfl' := SafetySpec.floorMono hs.toSafetySpec hfl
        exact ((StepSpec.contentRetained hs v hfl').decide).proposals p (ih v p hold hfl')
      · exact hheld
    | collect hg =>
      have hold := (GcSpec.shrinks hg).admitted v p hp
      have hfl' := GcSpec.floorStable hg v hfl
      exact (GcSpec.keepsDecideAboveFloor hg v hfl').proposals p (ih v p hold hfl')

end NewProtocol
