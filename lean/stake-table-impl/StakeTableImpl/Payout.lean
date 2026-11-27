module

public import StakeTableImpl.Node.Refines

/-!
# Paying the block reward

`StakeTable.payout` is the node's `RewardDistributor::compute_rewards`, check for
check: each delegator's part is its delegation times what the commission leaves,
divided by the stake and then by `maxCommission`, and the validator's the rest, as a
list of credits. For a validator of a state that stands for a history, the credits
to each account are the specification's (`StakeTable.payout_credit`), and the node
reports an error exactly where the specification says (`StakeTable.payout_ok_iff`).
-/

@[expose] public section

namespace StakeTable

/--
The delegators' parts of `q`, what the commission leaves times the reward: each
delegation times `q`, divided by the stake and then by `maxCommission`. An error
where a product overflows or, with a delegator, there is no stake.
-/
def delegatorParts (q stake : Nat) : List (Address × Nat) → Except String (List (Address × Nat))
  | [] => .ok []
  | (d, n) :: ds =>
    if U256 ≤ n * q then .error "overflow"
    else if stake = 0 then .error "overflow"
    else (delegatorParts q stake ds).map ((d, n * q / stake / maxCommission) :: ·)

/--
The credits of paying validator `v` reward `r`: each delegator's part, then the
validator's. An error where the commission is above `maxCommission` or a product
overflows.
-/
def payout (v : Validator) (r : Nat) : Except String (List (Address × Nat)) :=
  if maxCommission < v.commission then .error "commission must not exceed"
  else
    let q := (maxCommission - v.commission) * r
    if U256 ≤ q then .error "overflow"
    else
      match delegatorParts q v.stake v.delegators with
      | .error e => .error e
      | .ok ps =>
        let paid := (ps.map (·.2)).sum
        if r < paid then .error "overflow" else .ok (ps ++ [(v.account, r - paid)])

/-- What the credits `cs` credit account `x`. -/
def creditOf (cs : List (Address × Nat)) (x : Address) : Nat :=
  ((cs.filter fun c => decide (c.1 = x)).map (·.2)).sum

theorem delegatorParts_ok {q stake : Nat} :
    ∀ {ds ps : List (Address × Nat)}, delegatorParts q stake ds = .ok ps →
      ps = ds.map fun e => (e.1, e.2 * q / stake / maxCommission)
  | [], ps, h => by simp only [delegatorParts, Except.ok.injEq] at h; cases h; rfl
  | (d, n) :: ds, ps, h => by
    simp only [delegatorParts] at h
    split at h
    · cases h
    · split at h
      · cases h
      · cases hp : delegatorParts q stake ds with
        | error => rw [hp] at h; cases h
        | ok ps' =>
          rw [hp] at h
          simp only [Except.map, Except.ok.injEq] at h
          subst h
          simp [delegatorParts_ok hp]

theorem delegates_of_mem {e : Address × Nat} :
    ∀ {ds : List (Address × Nat)}, e ∈ ds → delegates e.1 ds = true
  | [], h => by cases h
  | e' :: ds, h => by
    simp only [delegates, Bool.or_eq_true, decide_eq_true_eq]
    rcases List.mem_cons.1 h with rfl | h
    · exact .inl rfl
    · exact .inr (delegates_of_mem h)

theorem sum_map_const_zero (L : List Address) : (L.map fun _ => (0 : Nat)).sum = 0 := by
  induction L with
  | nil => rfl
  | cons _ L ih => simp only [List.map_cons, List.sum_cons, ih]

theorem amountOf_eq_zero {d : Address} :
    ∀ {ds : List (Address × Nat)}, d ∉ ds.map (·.1) → amountOf d ds = 0
  | [], _ => rfl
  | e :: ds, h => by
    simp only [List.map_cons, List.mem_cons, not_or] at h
    simp only [amountOf, Ne.symm h.1, ite_false]
    exact amountOf_eq_zero h.2

/-- What a delegation list credits `x`, with each delegator once and nothing for none. -/
theorem creditOf_map {f : Nat → Nat} (hf : f 0 = 0) (x : Address) :
    ∀ {ds : List (Address × Nat)}, (ds.map (·.1)).Nodup →
      creditOf (ds.map fun e => (e.1, f e.2)) x = f (amountOf x ds)
  | [], _ => by simp [creditOf, amountOf, hf]
  | e :: ds, hn => by
    simp only [List.map_cons, List.nodup_cons] at hn
    have ih := creditOf_map hf x hn.2
    simp only [creditOf, List.map_cons, List.filter_cons, amountOf] at ih ⊢
    by_cases he : e.1 = x
    · subst he
      rw [amountOf_eq_zero hn.1, hf] at ih
      simp [ih]
    · simp [he, ih]

/-- Summing a function of each delegation over another list that holds every delegator once. -/
theorem sum_amountOf {f : Nat → Nat} (hf : f 0 = 0) :
    ∀ {ds : List (Address × Nat)} {L : List Address}, (ds.map (·.1)).Nodup → L.Nodup →
      (∀ e ∈ ds, e.1 ∈ L) → (ds.map fun e => f e.2).sum = (L.map fun d => f (amountOf d ds)).sum
  | [], L, _, _, _ => by
    simp only [List.map_nil, List.sum_nil, amountOf, hf]
    exact (sum_map_const_zero L).symm
  | e :: ds, L, hn, hL, hsub => by
    simp only [List.map_cons, List.nodup_cons] at hn
    have ih := sum_amountOf hf hn.2 hL fun e' he' => hsub e' (List.mem_cons_of_mem _ he')
    have he : e.1 ∈ L := hsub e (List.mem_cons_self ..)
    have hpt : ∀ d, f (amountOf d (e :: ds)) = f (amountOf d ds) + if e.1 = d then f e.2 else 0 := by
      intro d
      by_cases hd : e.1 = d
      · subst hd; simp [amountOf, amountOf_eq_zero hn.1, hf]
      · simp [amountOf, hd]
    have key : ∀ L' : List Address, L'.Nodup →
        (L'.map fun d => f (amountOf d (e :: ds))).sum
          = (L'.map fun d => f (amountOf d ds)).sum + if e.1 ∈ L' then f e.2 else 0 := by
      intro L' hL'
      induction L' with
      | nil => simp
      | cons d L' ih' =>
        simp only [List.nodup_cons] at hL'
        rw [List.map_cons, List.sum_cons, hpt, ih' hL'.2, List.map_cons, List.sum_cons]
        by_cases hd : e.1 = d
        · subst hd
          simp only [ite_true, List.mem_cons_self, hL'.1, ite_false]
          omega
        · simp only [hd, ite_false, List.mem_cons, false_or]
          omega
    rw [List.map_cons, List.sum_cons, key L hL, ih]
    simp only [he, ite_true]
    omega

/--
For a validator of a state that stands for a history, the credits the node pays are
the specification's.
-/
theorem payout_credit {s : State} {h : History} (hr : Represents s h) (hi : Inv s) {v : Validator} (hmem : v ∈ s.validators) {r : Nat}
    {cs : List (Address × Nat)} (he : payout v r = .ok cs) (x : Address) :
    creditOf cs x = credited h v.account r x := by
  have hm : member h v.account = v.toMember := (hr.members v hmem).symm
  have hs := hi.sound v hmem
  unfold payout at he
  dsimp only at he
  split at he
  · cases he
  · split at he
    · cases he
    · split at he
      · cases he
      · rename_i ps hps
        split at he
        · cases he
        · simp only [Except.ok.injEq] at he
          subst he
          have hps := delegatorParts_ok hps
          subst hps
          -- Each part is the specification's.
          have hpaid : ∀ d, delegatorPaid h v.account r d
              = amountOf d v.delegators * ((maxCommission - v.commission) * r)
                / v.stake / maxCommission := by
            intro d
            rw [delegatorPaid_eq, hm, Nat.div_div_eq_div_mul]
            have := hr.delegations v hmem d
            simp only [Validator.toMember]
            congr 2
            omega
          -- The delegators the node pays are among the specification's.
          have hsub : ∀ e ∈ v.delegators, e.1 ∈ delegators h v.account := by
            intro e he
            refine mem_dedup.2 (Classical.byContradiction fun hn => ?_)
            have h0 := delegated_eq_zero_of_not_mem hn
            have h1 := hr.delegations v hmem e.1
            have h2 := (amountOf_pos hs.delegationsPositive).1 (delegates_of_mem he)
            omega
          have hsum : ((v.delegators.map fun e =>
                (e.1, e.2 * ((maxCommission - v.commission) * r) / v.stake / maxCommission)).map
                (·.2)).sum
              = ((delegators h v.account).map (delegatorPaid h v.account r)).sum := by
            rw [List.map_map]
            have hc : ((·.2) ∘ fun e : Address × Nat =>
                (e.1, e.2 * ((maxCommission - v.commission) * r) / v.stake / maxCommission))
                = fun e => e.2 * ((maxCommission - v.commission) * r) / v.stake / maxCommission :=
              rfl
            have hs2 := sum_amountOf (f := fun n =>
              n * ((maxCommission - v.commission) * r) / v.stake / maxCommission) (by simp)
              hs.delegatorsNodup (nodup_dedup _) hsub
            rw [hc, hs2]
            congr 1
            exact List.map_congr_left fun d _ => (hpaid d).symm
          have hcred := creditOf_map (f := fun n =>
              n * ((maxCommission - v.commission) * r) / v.stake / maxCommission) (by simp) x
              hs.delegatorsNodup
          simp only [creditOf, List.filter_append, List.map_append, List.sum_append] at hcred hsum ⊢
          rw [hcred, ← hpaid, hsum]
          unfold credited validatorPaid
          by_cases hx : v.account = x
          · subst hx; simp
          · simp [hx, Ne.symm hx]

theorem delegatorParts_ok_iff {q stake : Nat} {ds : List (Address × Nat)} :
    (∃ ps, delegatorParts q stake ds = .ok ps) ↔ ∀ e ∈ ds, e.2 * q < U256 ∧ 0 < stake := by
  induction ds with
  | nil => exact ⟨fun _ e he => (List.not_mem_nil he).elim, fun _ => ⟨[], rfl⟩⟩
  | cons e ds ih =>
    obtain ⟨d, n⟩ := e
    simp only [delegatorParts]
    constructor
    · rintro ⟨ps, hps⟩
      split at hps
      · cases hps
      split at hps
      · cases hps
      rename_i h1 h2
      cases hd : delegatorParts q stake ds with
      | error => rw [hd] at hps; cases hps
      | ok ps' =>
        intro e he
        rcases List.mem_cons.1 he with rfl | he
        · show n * q < U256 ∧ 0 < stake
          exact ⟨by omega, by omega⟩
        · exact ih.1 ⟨ps', hd⟩ e he
    · intro hall
      have h0 : n * q < U256 ∧ 0 < stake := hall (d, n) (List.mem_cons_self ..)
      obtain ⟨ps', hps'⟩ := ih.2 fun e he => hall e (List.mem_cons_of_mem _ he)
      refine ⟨(d, n * q / stake / maxCommission) :: ps', ?_⟩
      simp only [show ¬ U256 ≤ n * q by omega, show ¬ stake = 0 by omega, ite_false, hps']
      rfl

theorem amountOf_of_mem {e : Address × Nat} :
    ∀ {ds : List (Address × Nat)}, (ds.map (·.1)).Nodup → e ∈ ds → amountOf e.1 ds = e.2
  | [], _, he => by cases he
  | e' :: ds, hn, he => by
    simp only [List.map_cons, List.nodup_cons] at hn
    simp only [amountOf]
    rcases List.mem_cons.1 he with rfl | he
    · simp
    · have hne : e'.1 ≠ e.1 := fun h' => hn.1 (h' ▸ List.mem_map_of_mem he)
      simp only [hne, ite_false]
      exact amountOf_of_mem hn.2 he

theorem exists_mem_of_delegates {d : Address} :
    ∀ {ds : List (Address × Nat)}, delegates d ds = true → ∃ e ∈ ds, e.1 = d
  | [], h => by simp [delegates] at h
  | e :: ds, h => by
    simp only [delegates, Bool.or_eq_true, decide_eq_true_eq] at h
    rcases h with h | h
    · exact ⟨e, List.mem_cons_self .., h⟩
    · obtain ⟨e', he', h'⟩ := exists_mem_of_delegates h
      exact ⟨e', List.mem_cons_of_mem _ he', h'⟩

/-- What the delegators are paid adds up to no more than the reward. -/
theorem paid_le {v : Validator} (hs : v.Sound) (r : Nat) {ps : List (Address × Nat)}
    (hps : ps = v.delegators.map fun e =>
      (e.1, e.2 * ((maxCommission - v.commission) * r) / v.stake / maxCommission)) :
    (ps.map (·.2)).sum ≤ r := by
  subst hps
  rw [List.map_map]
  have hc : ((·.2) ∘ fun e : Address × Nat =>
      (e.1, e.2 * ((maxCommission - v.commission) * r) / v.stake / maxCommission))
      = fun e => e.2 * ((maxCommission - v.commission) * r) / (v.stake * maxCommission) := by
    funext e; simp [Nat.div_div_eq_div_mul]
  rw [hc]
  have hsum := sum_div_mul_le v.delegators (·.2) ((maxCommission - v.commission) * r)
    (v.stake * maxCommission)
  have htot : (v.delegators.map (·.2)).sum = v.stake := hs.stakeTotal.symm
  rw [htot] at hsum
  rcases Nat.eq_zero_or_pos v.stake with h0 | hpos
  · have : v.delegators = [] := by
      cases hd : v.delegators with
      | nil => rfl
      | cons e es =>
        have := hs.delegationsPositive e (by simp [hd])
        have := htot; rw [hd] at this; simp at this; omega
    simp [this]
  · rw [Nat.mul_left_comm] at hsum
    have := Nat.le_of_mul_le_mul_left hsum hpos
    have : (maxCommission - v.commission) * r ≤ maxCommission * r :=
      Nat.mul_le_mul_right _ (Nat.sub_le _ _)
    simp only [maxCommission] at *
    omega

/--
For a validator of a state that stands for a valid history, the node pays reward `r`
without error exactly where the specification says it does.
-/
theorem payout_ok_iff {s : State} {h : History} (hr : Represents s h) (hi : Inv s)
    {v : Validator} (hmem : v ∈ s.validators) (r : Nat) :
    (∃ cs, payout v r = .ok cs) ↔ PayoutDefined h v.account r := by
  have hm : member h v.account = v.toMember := (hr.members v hmem).symm
  have hs := hi.sound v hmem
  have hd : ∀ d, (delegated h d v.account).toNat = amountOf d v.delegators := by
    intro d; have := hr.delegations v hmem d; omega
  -- Every delegator has a positive amount, so with one there is stake.
  have hparts : (∃ ps, delegatorParts ((maxCommission - v.commission) * r) v.stake
        v.delegators = .ok ps)
      ↔ ∀ d ∈ delegators h v.account,
        (delegated h d v.account).toNat * ((maxCommission - v.commission) * r) < U256 := by
    rw [delegatorParts_ok_iff]
    constructor
    · intro hall d _
      rw [hd]
      by_cases hz : amountOf d v.delegators = 0
      · rw [hz, Nat.zero_mul]; decide
      · have hdel := (amountOf_pos (d := d) hs.delegationsPositive).2 (Nat.pos_of_ne_zero hz)
        obtain ⟨e, he, rfl⟩ := exists_mem_of_delegates hdel
        rw [amountOf_of_mem hs.delegatorsNodup he]
        exact (hall e he).1
    · intro hall e he
      have hamt := amountOf_of_mem hs.delegatorsNodup he
      have hsub : e.1 ∈ delegators h v.account := by
        refine mem_dedup.2 (Classical.byContradiction fun hn => ?_)
        have h0 := delegated_eq_zero_of_not_mem hn
        have h1 := hr.delegations v hmem e.1
        have h2 := (amountOf_pos hs.delegationsPositive).1 (delegates_of_mem he)
        omega
      refine ⟨by rw [← hamt, ← hd]; exact hall e.1 hsub, ?_⟩
      have := hs.delegationsPositive e he
      have : e.2 ≤ total v.delegators := hamt ▸ amountOf_le_total _ _
      have := hs.stakeTotal
      omega
  unfold payout PayoutDefined
  rw [hm]
  simp only [Validator.toMember]
  by_cases h1 : maxCommission < v.commission
  · simp only [h1, ite_true, reduceCtorEq, exists_false, false_iff, not_and]
    intro h; omega
  · simp only [h1, ite_false]
    by_cases h2 : U256 ≤ (maxCommission - v.commission) * r
    · simp only [h2, ite_true, reduceCtorEq, exists_false, false_iff, not_and]
      intro _ h; omega
    · simp only [h2, ite_false]
      rw [← hparts]
      cases hp : delegatorParts ((maxCommission - v.commission) * r) v.stake v.delegators with
      | error => simp
      | ok ps =>
        have hle := paid_le hs r (delegatorParts_ok hp)
        simp only [Nat.not_lt.2 hle, ite_false, Except.ok.injEq, exists_eq', true_iff]
        exact ⟨by omega, by omega, trivial⟩

end StakeTable
