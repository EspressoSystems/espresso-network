module

public import StakeTableSpec.Proofs.Select
public import StakeTableSpec.Proofs.Table
public import StakeTableSpec.Proofs.Payout

/-!
# What the commitment fixes

Histories with the same commitment have tables with the same members, perhaps in a
different order (`StakeTable.mem_table_congr`). The committee depends only on the
members (`StakeTable.isActiveSet_congr`), so they select the same one
(`StakeTable.commitment_fixes_committee`).

They agree on a registered validator and on every delegation to it, so they pay each
of its delegators the same. Their lists of its delegators may differ, but only in
order and in delegators that are paid nothing (`StakeTable.mem_delegators_of_paid`),
so they pay the validator the same too (`StakeTable.commitment_fixes_payout`).
-/

@[expose] public section

namespace StakeTable

/-- Histories with the same commitment have tables with the same members. -/
theorem mem_table_congr {h h' : History} (hs : SameCommitment h h') (m : Member) :
    m ∈ table h ↔ m ∈ table h' := by
  obtain ⟨hreg, hmem, -⟩ := hs
  simp only [table, List.mem_map]
  constructor
  · rintro ⟨a, ha, rfl⟩
    exact ⟨a, (hreg a).1 ha, ((hmem a ha).1).symm⟩
  · rintro ⟨a, ha, rfl⟩
    have ha' : Registered h a := (hreg a).2 ha
    exact ⟨a, ha', (hmem a ha').1⟩

/-- Valid histories with the same commitment select the same committee. -/
theorem commitment_fixes_committee {usable : NetAddr → Bool} {h h' : History}
    {r r' : Option (List Member)} (hv : Valid h) (hv' : Valid h') (hs : SameCommitment h h')
    (hr : IsActiveSet usable (table h) r) (hr' : IsActiveSet usable (table h') r') : r = r' :=
  isActiveSet_unique (table_distinct hv').accounts
    (isActiveSet_congr (table_distinct hv).accounts (table_distinct hv').accounts
      (mem_table_congr hs) hr) hr'

/--
Histories with the same commitment credit every account the same when a registered
validator is paid.
-/
theorem commitment_fixes_payout {h h' : History} (hs : SameCommitment h h') {a : Address}
    (ha : Registered h a) (r : Nat) (x : Address) : credited h a r x = credited h' a r x := by
  obtain ⟨hm, hd⟩ := hs.2.1 a ha
  have hp : delegatorPaid h' a r = delegatorPaid h a r := by
    funext d; simp only [delegatorPaid, hm, hd]
  have hsum : ((delegators h a).map (delegatorPaid h a r)).sum
      = ((delegators h' a).map (delegatorPaid h a r)).sum :=
    Nat.le_antisymm
      (sum_map_le_of_subset _ (nodup_dedup _) fun _ _ hd => mem_delegators_of_paid (hp ▸ hd))
      (sum_map_le_of_subset _ (nodup_dedup _) fun _ _ hd => mem_delegators_of_paid hd)
  simp only [credited, validatorPaid, hp, hsum]

end StakeTable
