module

public import NewProtocolSpec.DecideStream.Defs
public import NewProtocolSpec.DecideStream.Lemmas

/-!
# What the decide stream guarantees

`DecideInv` holds at every reachable state, so a decide is never taken back.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : ViewNumber → Option PubKey) (node : PubKey)

/-- The invariant holds at every reachable state. -/
theorem decideInv_reachable {s : NodeState}
    (hr : Reachable cfg (StepSpec cfg leader node)
        (NodeState.initial cfg) s) : DecideInv cfg s := by
  induction hr with
  | refl => exact decideInv_initial cfg
  | step _ ht ih => cases ht with
    | step hs => exact decideInv_step cfg leader node ih hs
    | collect hg => exact decideInv_gc cfg ih hg

end NewProtocol
