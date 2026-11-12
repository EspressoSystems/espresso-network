module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Table

/-!
# What a payout is like

Rounding down never pays the delegators more than their exact shares, and pays each
less than one unit short (`StakeTable.sum_div_mul_le`, `StakeTable.sum_mul_le_div`).
A validator's delegations add up to its stake (`StakeTable.sum_delegated`), so the
delegators together are paid at most what the commission leaves
(`StakeTable.delegatorsPaid_le`): the payout adds up to the reward
(`StakeTable.payout_adds_up`), and the validator keeps at least its commission and
less than one unit more per delegator (`StakeTable.commission_kept`).

Only a delegator of the validator is paid (`StakeTable.mem_delegators_of_paid`).
-/

@[expose] public section

namespace StakeTable

/-- Rounding down pays no more than the exact shares. -/
theorem sum_div_mul_le {α : Type} (L : List α) (g : α → Nat) (q D : Nat) :
    (L.map fun x => g x * q / D).sum * D ≤ (L.map g).sum * q := by
  induction L with
  | nil => simp
  | cons x L ih =>
    simp only [List.map_cons, List.sum_cons, Nat.add_mul]
    exact Nat.add_le_add (Nat.div_mul_le_self _ _) ih

/-- Rounding down pays each less than one unit short. -/
theorem sum_mul_le_div {α : Type} (L : List α) (g : α → Nat) (q : Nat) {D : Nat} (hD : 0 < D) :
    (L.map g).sum * q + L.length ≤ ((L.map fun x => g x * q / D).sum + L.length) * D := by
  induction L with
  | nil => simp
  | cons x L ih =>
    have h1 := Nat.lt_div_mul_add (a := g x * q) hD
    simp only [List.map_cons, List.sum_cons, List.length_cons, Nat.add_mul, Nat.one_mul] at ih h1 ⊢
    generalize g x * q = gq at *
    generalize gq / D * D = pD at *
    generalize (L.map g).sum * q = Sq at *
    generalize (L.map fun x => g x * q / D).sum * D = SpD at *
    generalize L.length * D = kD at *
    omega

theorem sum_toNat {L : List Address} {f : Address → Int} (hf : ∀ d, 0 ≤ f d) :
    (L.map fun d => (f d).toNat).sum = ((L.map f).sum).toNat := by
  induction L with
  | nil => simp
  | cons d L ih =>
    have := hf d
    have := sum_nonneg (ds := L) hf
    simp only [List.map_cons, List.sum_cons, ih]
    omega

/-- A validator's delegations add up to its stake. -/
theorem sum_delegated {h : History} (hv : Valid h) (a : Address) :
    ((delegators h a).map fun d => (delegated h d a).toNat).sum = (member h a).stake := by
  rw [sum_toNat (delegated_nonneg hv · a)]
  unfold delegators
  rw [← stake_eq_sum h a (nodup_dedup _) fun d hd => mem_dedup.2 hd]
  rfl

/-- A delegator that never delegated to `a` nor withdrew from it has nothing delegated. -/
theorem delegated_eq_zero_of_not_mem {h : History} {d a : Address}
    (hd : d ∉ delegatorsOf h a) : delegated h d a = 0 := by
  induction h with
  | nil => rfl
  | cons c h ih =>
    have hd' : d ∉ delegatorsOf h a := fun hm => hd (by
      unfold delegatorsOf at *; rw [List.filterMap_cons]; split <;> simp_all)
    have hc : c.flow d a = 0 := by
      cases c with
      | delegate d' a' n =>
        by_cases h1 : d' = d ∧ a' = a
        · obtain ⟨rfl, rfl⟩ := h1; simp [delegatorsOf] at hd
        · simp [Change.flow, h1]
      | undelegate d' a' n =>
        by_cases h1 : d' = d ∧ a' = a
        · obtain ⟨rfl, rfl⟩ := h1; simp [delegatorsOf] at hd
        · simp [Change.flow, h1]
      | _ => rfl
    have := ih hd'
    simp only [delegated, List.map_cons, List.sum_cons] at this ⊢
    omega

/-- A delegator that is paid has delegated to `a` or withdrawn from it. -/
theorem mem_delegators_of_paid {h : History} {a d : Address} {r : Nat}
    (hp : delegatorPaid h a r d ≠ 0) : d ∈ delegators h a :=
  mem_dedup.2 <| Classical.byContradiction fun hn =>
    hp (by simp [delegatorPaid, delegated_eq_zero_of_not_mem hn])

theorem delegatorPaid_eq (h : History) (a : Address) (r : Nat) :
    delegatorPaid h a r = fun d => (delegated h d a).toNat
      * ((maxCommission - (member h a).commission) * r) / ((member h a).stake * maxCommission) := by
  funext d
  simp only [delegatorPaid, Nat.mul_assoc]

/-- The delegators together are paid at most what the commission leaves. -/
theorem delegatorsPaid_le {h : History} (hv : Valid h) (a : Address) (r : Nat) :
    ((delegators h a).map (delegatorPaid h a r)).sum * maxCommission
      ≤ (maxCommission - (member h a).commission) * r := by
  rw [delegatorPaid_eq]
  generalize hS : (member h a).stake = S
  generalize (maxCommission - (member h a).commission) * r = q
  rcases Nat.eq_zero_or_pos S with rfl | hpos
  · have : ∀ L : List Address, (L.map fun _ => (0 : Nat)).sum = 0 := by
      intro L; induction L <;> simp_all
    simp [this]
  · have h1 := sum_div_mul_le (delegators h a) (fun d => (delegated h d a).toNat) q
      (S * maxCommission)
    rw [sum_delegated hv a, hS, Nat.mul_left_comm] at h1
    exact Nat.le_of_mul_le_mul_left h1 hpos

/-- A payout adds up to the reward. -/
theorem payout_adds_up {h : History} (hv : Valid h) (a : Address) (r : Nat) :
    validatorPaid h a r + ((delegators h a).map (delegatorPaid h a r)).sum = r := by
  have h1 := delegatorsPaid_le hv a r
  have h2 : (maxCommission - (member h a).commission) * r ≤ maxCommission * r :=
    Nat.mul_le_mul_right _ (Nat.sub_le _ _)
  unfold validatorPaid
  simp only [maxCommission] at h1 h2
  omega

/--
A validator with stake keeps at least its commission of a reward, and less than one
unit more per delegator.
-/
theorem commission_kept {h : History} (hv : Valid h) {a : Address} (r : Nat)
    (hS : 0 < (member h a).stake) (hc : (member h a).commission ≤ maxCommission) :
    (member h a).commission * r ≤ maxCommission * validatorPaid h a r
      ∧ maxCommission * validatorPaid h a r
          < (member h a).commission * r + maxCommission * (delegators h a).length := by
  have hle := delegatorsPaid_le hv a r
  have hlt := sum_mul_le_div (delegators h a) (fun d => (delegated h d a).toNat)
    ((maxCommission - (member h a).commission) * r) (D := (member h a).stake * maxCommission)
    (Nat.mul_pos hS (by decide))
  rw [sum_delegated hv a, ← delegatorPaid_eq,
    Nat.mul_left_comm _ (member h a).stake maxCommission] at hlt
  have hk : 0 < (delegators h a).length := by
    rcases Nat.eq_zero_or_pos (delegators h a).length with h0 | h0
    · have := sum_delegated hv a
      rw [List.length_eq_zero_iff.1 h0] at this
      simp at this; omega
    · exact h0
  have hq : (member h a).stake * ((maxCommission - (member h a).commission) * r)
      < (member h a).stake
        * ((((delegators h a).map (delegatorPaid h a r)).sum + (delegators h a).length)
          * maxCommission) := by omega
  have hq := Nat.lt_of_mul_lt_mul_left hq
  have hsub : (maxCommission - (member h a).commission) * r
      = maxCommission * r - (member h a).commission * r := Nat.sub_mul _ _ _
  have hcr : (member h a).commission * r ≤ maxCommission * r := Nat.mul_le_mul_right _ hc
  have hadd := payout_adds_up hv a r
  rw [hsub] at hle hq
  simp only [Nat.add_mul] at hq
  generalize (member h a).commission * r = cr at *
  generalize validatorPaid h a r = vp at *
  generalize ((delegators h a).map (delegatorPaid h a r)).sum = p at *
  simp only [maxCommission] at *
  omega

end StakeTable
