module

public import StakeTableImpl.Node.Types

/-!
# Delegations

A validator's delegations, as a list of delegators with their amounts.

This is a representation, not part of what is specified: the list stands for a map
from delegator to amount, which is how the node keeps it. What matters is the amount
each delegator has (`StakeTable.amountOf`) and their sum (`StakeTable.total`); the
order of the list is incidental, and nothing reads it.
-/

@[expose] public section

namespace StakeTable

/-- The amount `d` has delegated, zero if it has delegated nothing. -/
def amountOf (d : Address) : List (Address × Nat) → Nat
  | [] => 0
  | e :: es => if e.1 = d then e.2 else amountOf d es

/-- Whether `d` has a delegation. -/
def delegates (d : Address) : List (Address × Nat) → Bool
  | [] => false
  | e :: es => decide (e.1 = d) || delegates d es

/-- The sum of all delegations. -/
def total (ds : List (Address × Nat)) : Nat :=
  (ds.map (·.2)).sum

/-- Add `amount` to `d`'s delegation, creating it if `d` has none. -/
def credit (d : Address) (amount : Nat) : List (Address × Nat) → List (Address × Nat)
  | [] => [(d, amount)]
  | e :: es => if e.1 = d then (d, e.2 + amount) :: es else e :: credit d amount es

/--
Take `amount` from `d`'s delegation, removing it if nothing is left.

The caller checks that `d` has at least `amount` delegated.
-/
def debit (d : Address) (amount : Nat) : List (Address × Nat) → List (Address × Nat)
  | [] => []
  | e :: es =>
    if e.1 = d then
      if e.2 - amount = 0 then es else (d, e.2 - amount) :: es
    else e :: debit d amount es

end StakeTable
