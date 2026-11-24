module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Lists

/-!
# What the active set is

Among validators with distinct accounts, ranking is a strict total order, so the
members of a top are fixed (`StakeTable.Top.mem_iff`), and listed by account the
committee is unique (`StakeTable.isActiveSet_unique`). One exists: rank each validator
by how many rank before it, keep those ranked below the bound, and sort them by account
(`StakeTable.top_spec`, `StakeTable.isActiveSet_exists`).

The committee reads its validators only by membership, never by their order, so lists
with the same members have the same committee (`StakeTable.isActiveSet_congr`).

A committee is never empty (`StakeTable.activeSet_ne_nil`); its members are eligible
candidates (`StakeTable.activeSet_eligible`); no eligible validator left out has more
stake than a member (`StakeTable.activeSet_stake`); its stakes are within a factor of
`vidTargetTotalStake` of each other (`StakeTable.activeSet_stake_ratio`) and add up to
no more than those of its validators (`StakeTable.activeSet_stake_le`); and drawn from
members with distinct keys, it has distinct accounts and keys
(`StakeTable.activeSet_distinct`).
-/

@[expose] public section

namespace StakeTable

/-! ## Ranking -/

theorem ranksBefore_irrefl (p : Member) : RanksBefore p p = false := by
  simp [RanksBefore]

theorem ranksBefore_trans {p q r : Member} (h : RanksBefore p q = true)
    (h' : RanksBefore q r = true) : RanksBefore p r = true := by
  simp only [RanksBefore, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq] at *
  omega

theorem ranksBefore_total {p q : Member} (h : p.account ≠ q.account) :
    RanksBefore p q = true ∨ RanksBefore q p = true := by
  have : p.account.toNat ≠ q.account.toNat := fun he => h (by
    cases hp : p.account; cases hq : q.account; rw [hp, hq] at he; simp_all)
  simp only [RanksBefore, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq]
  omega

/-- Validators with distinct accounts are distinct. -/
theorem nodup_of_accounts {ts : List Member} (hidx : (ts.map (·.account)).Nodup) : ts.Nodup :=
  nodup_of_nodup_map hidx

/-- A sublist of validators with distinct accounts has distinct accounts. -/
theorem accounts_nodup_filter {ts : List Member} (hidx : (ts.map (·.account)).Nodup)
    (f : Member → Bool) : ((ts.filter f).map (·.account)).Nodup :=
  (List.filter_sublist.map _).nodup hidx

/-- The eligible candidates of validators with distinct accounts have distinct accounts. -/
theorem accounts_nodup_eligible {usable : NetAddr → Bool} {vs : List Member}
    (hidx : (vs.map (·.account)).Nodup) :
    ((eligible (candidates usable vs)).map (·.account)).Nodup :=
  accounts_nodup_filter (accounts_nodup_filter hidx _) _

/-! ## The members of a top are fixed -/

/-- A member of one top is a member of any other. -/
theorem Top.subset_of {n : Nat} {ts S S' : List Member} (hidx : (ts.map (·.account)).Nodup)
    (h : Top n ts S) (h' : Top n ts S') : ∀ q ∈ S, q ∈ S' := by
  intro q hq
  refine Classical.byContradiction fun hq' => ?_
  have hnd : (q :: S').Nodup := List.nodup_cons.2 ⟨hq', h'.nodup⟩
  have hsub : q :: S' ⊆ S := by
    intro p hp
    rcases List.mem_cons.1 hp with rfl | hp
    · exact hq
    · have hpt := h'.subset p hp
      have hne : p.account ≠ q.account := fun he =>
        hq' (eq_of_key_eq hidx hpt (h.subset q hq) he ▸ hp)
      rcases ranksBefore_total hne with hpq | hqp
      · exact h.closed q hq p hpt hpq
      · exact absurd (h'.closed p hp q (h.subset q hq) hqp) hq'
  have := List.Nodup.length_le_of_subset hnd hsub
  rw [List.length_cons, h.size, h'.size] at this
  omega

/-- Two tops of validators with distinct accounts have the same members. -/
theorem Top.mem_iff {n : Nat} {ts S S' : List Member} (hidx : (ts.map (·.account)).Nodup)
    (h : Top n ts S) (h' : Top n ts S') (x : Member) : x ∈ S ↔ x ∈ S' :=
  ⟨h.subset_of hidx h' x, h'.subset_of hidx h x⟩

/-- A list by account has distinct accounts. -/
theorem ByAccount.accounts_nodup {as : List Member} (h : ByAccount as) :
    (as.map (·.account)).Nodup :=
  List.pairwise_map.2 (h.imp fun hlt he => Nat.ne_of_lt hlt (congrArg Address.toNat he))

/-- Lists by account with the same members are equal. -/
theorem ByAccount.eq {as as' : List Member} (h : ByAccount as) (h' : ByAccount as')
    (hm : ∀ x, x ∈ as ↔ x ∈ as') : as = as' :=
  eq_of_perm_of_pairwise (fun _ _ hab hba => Nat.lt_asymm hab hba)
    ((List.perm_ext_iff_of_nodup (nodup_of_accounts h.accounts_nodup)
      (nodup_of_accounts h'.accounts_nodup)).2 hm) h h'

/-- A stake table with distinct accounts has at most one committee. -/
theorem isActiveSet_unique {usable : NetAddr → Bool} {vs : List Member}
    (hidx : (vs.map (·.account)).Nodup) {r r' : Option (List Member)}
    (h : IsActiveSet usable vs r) (h' : IsActiveSet usable vs r') : r = r' := by
  cases r with
  | none =>
    cases r' with
    | none => rfl
    | some _ => exact absurd h h'.1
  | some as =>
    cases r' with
    | none => exact absurd h' h.1
    | some as' =>
      obtain ⟨_, hT, hB⟩ := h
      obtain ⟨_, hT', hB'⟩ := h'
      rw [hB.eq hB' (hT.mem_iff (accounts_nodup_eligible hidx) hT')]

/-! ## A top exists -/

/-- How many of `ts` rank before `q`. -/
def rank (ts : List Member) (q : Member) : Nat :=
  (ts.filter (RanksBefore · q)).length

/-- The members of `ts` that fewer than `n` others rank before, in the order of `ts`. -/
def top (n : Nat) (ts : List Member) : List Member :=
  ts.filter fun q => decide (rank ts q < n)

/-- Whoever ranks before `p` ranks before `q` too, and so does `p`: `p` has the smaller rank. -/
theorem rank_lt {ts : List Member} (hidx : (ts.map (·.account)).Nodup)
    {p q : Member} (hp : p ∈ ts) (h : RanksBefore p q = true) : rank ts p < rank ts q := by
  have hnd : (p :: ts.filter (RanksBefore · p)).Nodup := by
    refine List.nodup_cons.2 ⟨?_, (List.filter_sublist).nodup (nodup_of_accounts hidx)⟩
    simp [ranksBefore_irrefl]
  have hsub : p :: ts.filter (RanksBefore · p) ⊆ ts.filter (RanksBefore · q) := by
    intro x hx
    simp only [List.mem_cons, List.mem_filter] at hx ⊢
    rcases hx with rfl | ⟨hx, hxp⟩
    · exact ⟨hp, h⟩
    · exact ⟨hx, ranksBefore_trans hxp h⟩
  have := List.Nodup.length_le_of_subset hnd hsub
  simp only [List.length_cons] at this
  exact this

/-- No member of `ts` ranks before itself, so fewer than all of `ts` rank before it. -/
theorem rank_lt_length {ts : List Member} (hidx : (ts.map (·.account)).Nodup)
    {q : Member} (hq : q ∈ ts) : rank ts q < ts.length := by
  have hnd : (q :: ts.filter (RanksBefore · q)).Nodup := by
    refine List.nodup_cons.2 ⟨?_, (List.filter_sublist).nodup (nodup_of_accounts hidx)⟩
    simp [ranksBefore_irrefl]
  have hsub : q :: ts.filter (RanksBefore · q) ⊆ ts := by
    intro x hx
    simp only [List.mem_cons, List.mem_filter] at hx
    rcases hx with rfl | ⟨hx, _⟩
    · exact hq
    · exact hx
  have := List.Nodup.length_le_of_subset hnd hsub
  simp only [List.length_cons] at this
  exact this

theorem rank_injOn {ts : List Member} (hidx : (ts.map (·.account)).Nodup) :
    ∀ p ∈ ts, ∀ q ∈ ts, rank ts p = rank ts q → p = q := by
  intro p hp q hq h
  by_cases hs : p.account = q.account
  · exact eq_of_key_eq hidx hp hq hs
  · rcases ranksBefore_total hs with hpq | hqp
    · exact absurd h (Nat.ne_of_lt (rank_lt hidx hp hpq))
    · exact absurd h (Nat.ne_of_gt (rank_lt hidx hq hqp))

/-- The ranks of `ts` are `0, 1, …` in some order. -/
theorem map_rank_perm {ts : List Member} (hidx : (ts.map (·.account)).Nodup) :
    (ts.map (rank ts)).Perm (List.range ts.length) := by
  have h := perm_range_of_nodup
    (nodup_map_of_injOn (rank_injOn hidx) (nodup_of_accounts hidx))
    (by
      intro x hx
      obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hx
      simpa using rank_lt_length hidx hq)
  simpa using h

/-- `top` keeps `n` of `ts`, or all of them if there are fewer. -/
theorem length_top (n : Nat) {ts : List Member} (hidx : (ts.map (·.account)).Nodup) :
    (top n ts).length = min n ts.length := by
  have h := (map_rank_perm hidx).filter (fun x => decide (x < n))
  rw [List.filter_map] at h
  have := h.length_eq
  rw [List.length_map, length_filter_range_lt] at this
  exact this

/-- `top n ts` is the top `n` of `ts`. -/
theorem top_spec (n : Nat) {ts : List Member} (hidx : (ts.map (·.account)).Nodup) :
    Top n ts (top n ts) where
  nodup := List.filter_sublist.nodup (nodup_of_accounts hidx)
  subset _ hm := (List.mem_filter.1 hm).1
  size := length_top n hidx
  closed q hq p hp h := by
    simp only [top, List.mem_filter, decide_eq_true_eq] at hq ⊢
    exact ⟨hp, Nat.lt_trans (rank_lt hidx hp h) hq.2⟩

/-- A top stays a top in any order. -/
theorem Top.perm {n : Nat} {ts S S' : List Member} (h : Top n ts S) (hp : S'.Perm S) :
    Top n ts S' where
  nodup := hp.nodup_iff.2 h.nodup
  subset m hm := h.subset m (hp.subset hm)
  size := hp.length_eq ▸ h.size
  closed q hq p hpt hr := hp.symm.subset (h.closed q (hp.subset hq) p hpt hr)

/-- `ms` sorted by account. -/
def byAccount (ms : List Member) : List Member :=
  ms.mergeSort fun a b => decide (a.account.toNat ≤ b.account.toNat)

theorem byAccount_perm (ms : List Member) : (byAccount ms).Perm ms :=
  List.mergeSort_perm ..

/-- Validators with distinct accounts, sorted by account, are listed by account. -/
theorem byAccount_spec {ms : List Member} (hidx : (ms.map (·.account)).Nodup) :
    ByAccount (byAccount ms) := by
  have hle : (byAccount ms).Pairwise
      fun a b => decide (a.account.toNat ≤ b.account.toNat) = true :=
    List.pairwise_mergeSort
      (fun a b c hab hbc => by
        simp only [decide_eq_true_eq] at hab hbc ⊢
        exact Nat.le_trans hab hbc)
      (fun a b => by
        rcases Nat.le_total a.account.toNat b.account.toNat with h | h <;> simp [h])
      ms
  have hne : (byAccount ms).Pairwise fun a b => a.account ≠ b.account :=
    List.pairwise_map.1 (((byAccount_perm ms).map Member.account).nodup_iff.2 hidx)
  refine (hle.and hne).imp fun {a b} ⟨h1, h2⟩ => ?_
  simp only [decide_eq_true_eq] at h1
  rcases Nat.lt_or_eq_of_le h1 with h | h
  · exact h
  · exact absurd (by cases ha : a.account; cases hb : b.account; simp_all) h2

/-- A stake table with distinct accounts has a committee, or none. -/
theorem isActiveSet_exists (usable : NetAddr → Bool) {vs : List Member}
    (hidx : (vs.map (·.account)).Nodup) : ∃ r, IsActiveSet usable vs r := by
  by_cases h : candidates usable vs = []
  · exact ⟨none, h⟩
  · have hidx' := accounts_nodup_eligible (usable := usable) hidx
    have hT := top_spec maxValidators hidx'
    refine ⟨some (byAccount (top maxValidators _)), h, hT.perm (byAccount_perm _), ?_⟩
    exact byAccount_spec (accounts_nodup_filter hidx' _)

/-! ## The largest stake -/

theorem le_maxStake {vs : List Member} {v : Member} (h : v ∈ vs) : v.stake ≤ maxStake vs := by
  induction vs with
  | nil => simp at h
  | cons w vs ih =>
    simp only [maxStake, List.foldr_cons] at ih ⊢
    rcases List.mem_cons.1 h with rfl | h
    · exact Nat.le_max_left _ _
    · exact Nat.le_trans (ih h) (Nat.le_max_right _ _)

theorem maxStake_mem {vs : List Member} (h : vs ≠ []) : ∃ v ∈ vs, v.stake = maxStake vs := by
  induction vs with
  | nil => exact absurd rfl h
  | cons w vs ih =>
    simp only [maxStake, List.foldr_cons] at ih ⊢
    by_cases hv : vs = []
    · subst hv; exact ⟨w, .head _, by simp⟩
    · obtain ⟨v, hv, he⟩ := ih hv
      by_cases hle : w.stake ≤ List.foldr (fun v m => max v.stake m) 0 vs
      · exact ⟨v, .tail _ hv, by rw [Nat.max_eq_right hle, he]⟩
      · exact ⟨w, .head _, by rw [Nat.max_eq_left (Nat.le_of_not_le hle)]⟩

/-- The largest stake of a list is at most that of a list with all its members. -/
theorem maxStake_le_of_subset {vs vs' : List Member} (h : ∀ v ∈ vs, v ∈ vs') :
    maxStake vs ≤ maxStake vs' := by
  by_cases hn : vs = []
  · subst hn; exact Nat.zero_le _
  · obtain ⟨v, hv, he⟩ := maxStake_mem hn
    exact he ▸ le_maxStake (h v hv)

/-- The candidate with the largest stake is eligible. -/
theorem eligible_ne_nil {cs : List Member} (h : cs ≠ []) : eligible cs ≠ [] := by
  obtain ⟨v, hv, he⟩ := maxStake_mem h
  intro hn
  have := List.filter_eq_nil_iff.1 hn v hv
  simp only [decide_eq_true_eq] at this
  exact this (he ▸ Nat.div_le_self _ _)

/-! ## Only the members count -/

section

variable {usable : NetAddr → Bool} {vs vs' : List Member}

theorem mem_candidates_congr (hm : ∀ x, x ∈ vs ↔ x ∈ vs') (x : Member) :
    x ∈ candidates usable vs ↔ x ∈ candidates usable vs' := by
  simp only [candidates, List.mem_filter, hm]

theorem mem_eligible_congr (hm : ∀ x, x ∈ vs ↔ x ∈ vs') (x : Member) :
    x ∈ eligible (candidates usable vs) ↔ x ∈ eligible (candidates usable vs') := by
  have hc := mem_candidates_congr (usable := usable) hm
  have hmax : maxStake (candidates usable vs) = maxStake (candidates usable vs') :=
    Nat.le_antisymm (maxStake_le_of_subset fun v hv => (hc v).1 hv)
      (maxStake_le_of_subset fun v hv => (hc v).2 hv)
  simp only [eligible, List.mem_filter, hc, hmax]

/-- Lists with the same members, each with distinct accounts, have the same committee. -/
theorem isActiveSet_congr (hidx : (vs.map (·.account)).Nodup)
    (hidx' : (vs'.map (·.account)).Nodup) (hm : ∀ x, x ∈ vs ↔ x ∈ vs')
    {r : Option (List Member)} (h : IsActiveSet usable vs r) : IsActiveSet usable vs' r := by
  have he := mem_eligible_congr (usable := usable) hm
  cases r with
  | none =>
    exact List.eq_nil_iff_forall_not_mem.2 fun x hx =>
      List.eq_nil_iff_forall_not_mem.1 h x ((mem_candidates_congr hm x).2 hx)
  | some as =>
    obtain ⟨hne, hT, hB⟩ := h
    refine ⟨fun hn => hne (List.eq_nil_iff_forall_not_mem.2 fun x hx =>
      List.eq_nil_iff_forall_not_mem.1 hn x ((mem_candidates_congr hm x).1 hx)), ?_, hB⟩
    have hlen := ((List.perm_ext_iff_of_nodup
      (nodup_of_accounts (accounts_nodup_eligible (usable := usable) hidx))
      (nodup_of_accounts (accounts_nodup_eligible (usable := usable) hidx'))).2 he).length_eq
    exact ⟨hT.nodup, fun m hm' => (he m).1 (hT.subset m hm'), hlen ▸ hT.size,
      fun q hq p hp hr => hT.closed q hq p ((he p).2 hp) hr⟩

end

/-! ## What a committee is like -/

section

variable {usable : NetAddr → Bool} {vs as : List Member}

/-- Every member is an eligible candidate. -/
theorem activeSet_mem_eligible (h : IsActiveSet usable vs (some as)) {v : Member} (hv : v ∈ as) :
    v ∈ eligible (candidates usable vs) :=
  h.2.1.subset v hv

/-- Every member is one of the validators. -/
theorem activeSet_mem (h : IsActiveSet usable vs (some as)) {v : Member} (hv : v ∈ as) :
    v ∈ vs :=
  (List.mem_filter.1 (List.mem_filter.1 (activeSet_mem_eligible h hv)).1).1

/-- Every member is a candidate with at least the minimum stake. -/
theorem activeSet_eligible (h : IsActiveSet usable vs (some as)) {v : Member} (hv : v ∈ as) :
    v.candidate usable = true
      ∧ maxStake (candidates usable vs) / vidTargetTotalStake ≤ v.stake := by
  have hm := activeSet_mem_eligible h hv
  simp only [eligible, candidates, List.mem_filter] at hm
  exact ⟨hm.1.2, of_decide_eq_true hm.2⟩

/-- A committee has `maxValidators` members, or every eligible validator if there are fewer. -/
theorem activeSet_length (h : IsActiveSet usable vs (some as)) :
    as.length = min maxValidators (eligible (candidates usable vs)).length :=
  h.2.1.size

/-- A committee is never empty. -/
theorem activeSet_ne_nil (h : IsActiveSet usable vs (some as)) : as ≠ [] := by
  have hl := activeSet_length h
  have := List.length_pos_iff.2 (eligible_ne_nil h.1)
  intro hn
  rw [hn, List.length_nil] at hl
  simp only [maxValidators] at hl
  omega

/-- No eligible validator left out of a committee has more stake than a member. -/
theorem activeSet_stake (h : IsActiveSet usable vs (some as)) {v w : Member}
    (hv : v ∈ as) (hw : w ∈ eligible (candidates usable vs)) (hout : w ∉ as) :
    w.stake ≤ v.stake :=
  Nat.le_of_not_lt fun hlt => hout (h.2.1.closed v hv w hw (by simp [RanksBefore, hlt]))

/-- Each member has at least any other member's stake divided by `vidTargetTotalStake`. -/
theorem activeSet_stake_ratio (h : IsActiveSet usable vs (some as)) {v w : Member}
    (hv : v ∈ as) (hw : w ∈ as) : w.stake / vidTargetTotalStake ≤ v.stake :=
  Nat.le_trans
    (Nat.div_le_div_right (le_maxStake (List.mem_filter.1 (activeSet_mem_eligible h hw)).1))
    (activeSet_eligible h hv).2

/-- A committee holds no more stake than the validators it is selected from. -/
theorem activeSet_stake_le (h : IsActiveSet usable vs (some as)) :
    (as.map (·.stake)).sum ≤ (vs.map (·.stake)).sum :=
  sum_map_le_of_subset _ h.2.1.nodup fun _ hv _ => activeSet_mem h hv

end

/-- A committee drawn from members with distinct keys has distinct accounts and keys. -/
theorem activeSet_distinct {usable : NetAddr → Bool} {ms as : List Member}
    (hd : Distinct ms) (h : IsActiveSet usable ms (some as)) : Distinct as :=
  ⟨h.2.2.accounts_nodup,
    fun m hm m' hm' => hd.bls m (activeSet_mem h hm) m' (activeSet_mem h hm'),
    fun m hm m' hm' => hd.schnorr m (activeSet_mem h hm) m' (activeSet_mem h hm'),
    fun m hm m' hm' => hd.x25519 m (activeSet_mem h hm) m' (activeSet_mem h hm')⟩

end StakeTable
