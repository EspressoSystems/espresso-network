module

public import StakeTableImpl.Node.Types
public import StakeTableImpl.Node.Delegations
public import StakeTableImpl.Node.Step
public import StakeTableImpl.Node.Proofs.Delegations
public import StakeTableImpl.Node.Proofs.Validators
public import StakeTableImpl.Node.Proofs.Invariant
public import StakeTableImpl.Node.Proofs.Steps

/-!
# The node's stake table

The stake table as the node keeps it, and what each L1 event does to it, outcome for
outcome: applied, skipped or rejected, with the node's error
(`StakeTable.step`). The invariant of every state the events build
(`StakeTable.Inv`) is what the refinement of the specification
(`StakeTableImpl.Node.Refines`) and the node's own checks rest on.
-/
