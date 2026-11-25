module

public import StakeTableSpec.Types
public import StakeTableSpec.Basic
public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Table
public import StakeTableSpec.Proofs.Select
public import StakeTableSpec.Proofs.Lists
public import StakeTableSpec.Proofs.Events
public import StakeTableSpec.Proofs.Reward
public import StakeTableSpec.Proofs.Schedule
public import StakeTableSpec.Proofs.Reads
public import StakeTableSpec.Proofs.Payout
public import StakeTableSpec.Proofs.Commitment
public import StakeTableSpec.Proofs.Properties

/-!
# The stake table specification

`StakeTableSpec.Spec` holds the whole specification, from the stake table as a
function of its history to every claim made about it; `StakeTableSpec.Types` the
data it reads, and `StakeTableSpec.Basic` the helpers it is written with.
`StakeTableSpec.Proofs` holds the proofs, with `StakeTableSpec.Proofs.Properties`
proving each claim.
-/
