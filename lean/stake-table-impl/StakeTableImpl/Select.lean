module

public import StakeTableImpl.Indexed.Refines
public import StakeTableImpl.Node.Refines

/-!
# Selecting the active validators

`StakeTable.select` computes the committee: it ranks each eligible validator by how
many rank before it, keeps those ranked below `maxValidators` (`StakeTable.top`, which
`StakeTable.top_spec` shows is the top of the ranking) and sorts them by account, so
for validators with distinct accounts `select` gives the committee the specification
describes (`StakeTable.select_spec`). Its cost, quadratic in the number of eligible
validators, is small for committees of a few hundred.

The node selects from its state and also leaves out a validator without a delegator
(`StakeTable.State.select`). In a state built from L1 events that changes nothing
(`StakeTable.State.select_eq`).

`StakeTable.Indexed.select` runs `select` on the members of the indexed state
(`StakeTable.Indexed.State.summaries`). After a replay from the empty state these are
the table of the history the events make (`StakeTable.Indexed.summaries_table`), so the indexed
selection is the committee of the specification.
-/

@[expose] public section

namespace StakeTable

/-- The committee, or `none` if there is no candidate. -/
def select (usable : NetAddr → Bool) (vs : List Member) : Option (List Member) :=
  match candidates usable vs with
  | [] => none
  | _ => some (byAccount (top maxValidators (eligible (candidates usable vs))))

/-- For validators with distinct accounts, `select` computes the committee. -/
theorem select_spec (usable : NetAddr → Bool) {vs : List Member}
    (hidx : (vs.map (·.account)).Nodup) : IsActiveSet usable vs (select usable vs) := by
  unfold select
  split
  · rename_i h; exact h
  · rename_i h
    have hidx' := accounts_nodup_eligible (usable := usable) hidx
    exact ⟨h, (top_spec _ hidx').perm (byAccount_perm _),
      byAccount_spec (accounts_nodup_filter hidx' _)⟩

/-- For validators with distinct accounts, the committee is what `select` computes. -/
theorem isActiveSet_iff_select {usable : NetAddr → Bool} {vs : List Member}
    (hidx : (vs.map (·.account)).Nodup) {r : Option (List Member)} :
    IsActiveSet usable vs r ↔ select usable vs = r :=
  ⟨fun h => isActiveSet_unique hidx (select_spec usable hidx) h,
    fun h => h ▸ select_spec usable hidx⟩

/--
The committee as the node selects it from its state.

The node's `select_active_validator_set` also leaves out a validator without a
delegator. In a state built from L1 events such a validator has no stake
(`StakeTable.Inv`), so it is no candidate, and the node selects what `select` does
(`StakeTable.State.select_eq`).
-/
def State.select (usable : NetAddr → Bool) (s : State) : Option (List Member) :=
  StakeTable.select usable
    ((s.validators.filter fun v => !v.delegators.isEmpty).map Validator.toMember)

/-- Leaving out the validators without a delegator leaves out no candidate. -/
theorem candidates_delegated (usable : NetAddr → Bool) {vs : List Validator}
    (h : ∀ v ∈ vs, v.Sound) :
    candidates usable ((vs.filter fun v => !v.delegators.isEmpty).map Validator.toMember)
      = candidates usable (vs.map Validator.toMember) := by
  induction vs with
  | nil => rfl
  | cons v vs ih =>
    have ih := ih fun w hw => h w (List.mem_cons_of_mem _ hw)
    by_cases hd : v.delegators.isEmpty = true
    · have hs : v.stake = 0 := by
        rw [(h v (List.mem_cons_self ..)).stakeTotal, List.isEmpty_iff.1 hd]; rfl
      have hc : v.toMember.candidate usable = false := by
        simp [Member.candidate, Validator.toMember, hs]
      simp only [candidates, List.filter_cons, List.map_cons, hd, hc] at ih ⊢
      simpa using ih
    · simp only [candidates, List.filter_cons, List.map_cons, hd] at ih ⊢
      simp only [Bool.not_false, ite_true, List.map_cons, List.filter_cons, ih]

/-- In a state built from L1 events the node selects the committee `select` computes. -/
theorem State.select_eq (usable : NetAddr → Bool) {s : State} (h : Inv s) :
    s.select usable = StakeTable.select usable (s.validators.map Validator.toMember) := by
  unfold State.select StakeTable.select
  rw [candidates_delegated usable h.sound]

namespace Indexed

/-- An indexed validator as a member of the table. -/
def Validator.summary (iv : Validator) : Member :=
  { account := iv.account, blsKey := iv.blsKey, schnorrKey := iv.schnorrKey,
    x25519Key := iv.x25519Key, p2pAddr := iv.p2pAddr, stake := iv.stake,
    commission := iv.commission, authenticated := iv.authenticated }

/-- The members of the table, in registration order. -/
def State.summaries (is : State) : List Member :=
  is.order.toList.filterMap fun a => is.validators[a]?.map Validator.summary

/-- The accounts of the active validators, in order, or `none` if there is no candidate. -/
def select (usable : NetAddr → Bool) (is : State) : Option (List Address) :=
  (StakeTable.select usable is.summaries).map (·.map (·.account))

theorem summary_eq {iv : Validator} {v : StakeTable.Validator} (h : ValidatorRel iv v) :
    iv.summary = v.toMember := by
  simp only [Validator.summary, StakeTable.Validator.toMember, h.account, h.blsKey, h.schnorrKey,
    h.x25519Key, h.p2pAddr, h.stake, h.commission, h.authenticated]

/-- The members of the indexed state are those of the list state it refines. -/
theorem summaries_eq {is : State} {s : StakeTable.State} (hr : Rel is s) (h : Inv s) :
    is.summaries = s.validators.map StakeTable.Validator.toMember := by
  unfold State.summaries
  rw [hr.order]
  have : ∀ l : List StakeTable.Validator, (∀ v ∈ l, v ∈ s.validators) →
      (l.map (·.account)).filterMap (fun a => is.validators[a]?.map Validator.summary)
        = l.map StakeTable.Validator.toMember := by
    intro l hl
    induction l with
    | nil => rfl
    | cons v l ih =>
      have hv := hl v (List.mem_cons_self ..)
      have hfind : s.find? v.account = some v := by
        cases hf : s.find? v.account with
        | none =>
          have h1 := find?_none hf
          have h2 : s.registered v.account = true := registered_iff.2 ⟨v, hv, rfl⟩
          rw [h1] at h2
          exact absurd h2 (by decide)
        | some w => rw [eq_of_find? h.accountsNodup hf hv rfl]
      have hrel := hr.validators v.account
      rw [hfind] at hrel
      simp only [List.map_cons, List.filterMap_cons]
      cases hi : is.validators[v.account]? with
      | none => rw [hi] at hrel; exact hrel.elim
      | some iv =>
        rw [hi] at hrel
        simp only [Option.map_some, summary_eq hrel]
        rw [ih fun w hw => hl w (List.mem_cons_of_mem _ hw)]
  exact this _ fun _ hv => hv

/--
After a replay from the empty state, the members of the indexed state are the table
of the history the events make.
-/
theorem summaries_table {es : List Event} {is : State}
    (he : applyAll State.empty es = .ok is) :
    ∃ h, Reads es h ∧ is.summaries = StakeTable.table h := by
  have hrep := applyAll_replay es
  rw [he] at hrep
  split at hrep
  · rename_i is' s hi hs
    cases hi
    obtain ⟨h, hv, hr⟩ := replay_refines hs
    exact ⟨h, hv, by rw [summaries_eq hrep (replay_inv Inv.empty hs), hr.table]⟩
  · rename_i hi _; cases hi
  · rename_i hi _ _; cases hi <;> simp_all

end Indexed

end StakeTable
