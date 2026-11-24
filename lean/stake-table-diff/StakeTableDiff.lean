module

public import StakeTableDiff.Decode
public import StakeTableDiff.Check

/-!
# Checking the node against the implementation

The node's test `stake_table_corpus` (in `crates/espresso/types`) generates
inputs, runs them through the node, and writes each input with the node's result.
`StakeTableDiff.Decode` reads them, and `StakeTableDiff.Check` runs the
implementation on each input and compares. The node depends on nothing here.
-/
