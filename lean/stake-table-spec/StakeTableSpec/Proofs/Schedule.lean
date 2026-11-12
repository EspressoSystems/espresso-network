module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Lists

/-!
# What the leader schedule gives

The schedule order is unique (`StakeTable.inScheduleOrder_unique`), and entries with
distinct keys have one, the entries sorted by `xor`ed key
(`StakeTable.inScheduleOrder_mergeSort`). The shares split `[0, total stake)`: every
breakpoint below the total stake lies in exactly one share
(`StakeTable.shareContains_exists`, `StakeTable.shareContains_unique`), and each share
holds as many breakpoints as its validator has stake
(`StakeTable.shareContains_count`).

So of a committee with distinct consensus keys and stake, each view has exactly one
leader (`StakeTable.leads_exists`, `StakeTable.leads_unique`), and each member leads
the breakpoints of an interval as long as its stake (`StakeTable.leads_iff`). The
leader is that of any other listing of the committee (`StakeTable.leads_perm`).
-/

@[expose] public section

namespace StakeTable

/-- The schedule order is unique. -/
theorem inScheduleOrder_unique {drb : List Nat} {es o o' : List Entry}
    (h : InScheduleOrder drb es o) (h' : InScheduleOrder drb es o') : o = o' :=
  eq_of_perm_of_pairwise (fun _ _ hab hba => List.lt_asymm hab hba) (h.1.trans h'.1.symm) h.2 h'.2

theorem xor_cancel_right {a b p : Nat} (h : a ^^^ p = b ^^^ p) : a = b := by
  have := congrArg (· ^^^ p) h
  simpa only [Nat.xor_assoc, Nat.xor_self, Nat.xor_zero] using this

/-- Distinct keys have distinct `xor`ed keys. -/
theorem xorKey_injective {drb a b : List Nat} (h : xorKey drb a = xorKey drb b) : a = b := by
  have hl : a.length = b.length := by
    have := congrArg List.length h
    simpa [xorKey] using this
  apply List.ext_getElem hl
  intro i ha hb
  have := congrArg (·[i]?) h
  simp only [xorKey, List.getElem?_mapIdx, List.getElem?_eq_getElem ha,
    List.getElem?_eq_getElem hb, Option.map_some, Option.some.injEq] at this
  exact xor_cancel_right this

/-- Entries with distinct keys, sorted by `xor`ed key, are in schedule order. -/
theorem inScheduleOrder_mergeSort (drb : List Nat) {es : List Entry}
    (hk : (es.map (·.key)).Nodup) :
    InScheduleOrder drb es (es.mergeSort fun a b => decide (xorKey drb a.key ≤ xorKey drb b.key)) := by
  have hperm : (es.mergeSort fun a b => decide (xorKey drb a.key ≤ xorKey drb b.key)).Perm es :=
    List.mergeSort_perm ..
  have hle : (es.mergeSort fun a b => decide (xorKey drb a.key ≤ xorKey drb b.key)).Pairwise
      fun a b => decide (xorKey drb a.key ≤ xorKey drb b.key) = true :=
    List.pairwise_mergeSort
      (fun a b c hab hbc => by
        simp only [decide_eq_true_eq] at hab hbc ⊢
        exact List.le_trans hab hbc)
      (fun a b => by
        rcases List.le_total (xorKey drb a.key) (xorKey drb b.key) with h | h <;> simp [h])
      es
  have hne : (es.mergeSort fun a b => decide (xorKey drb a.key ≤ xorKey drb b.key)).Pairwise
      fun a b => a.key ≠ b.key :=
    List.pairwise_map.1 ((hperm.map Entry.key).nodup_iff.2 hk)
  refine ⟨hperm, (hle.and hne).imp fun {a b} ⟨h1, h2⟩ => ?_⟩
  simp only [decide_eq_true_eq] at h1
  rcases List.le_iff_lt_or_eq.1 h1 with h | h
  · exact h
  · exact absurd (xorKey_injective h) h2

/-- Entries in schedule order are distinct. -/
theorem InScheduleOrder.nodup {drb : List Nat} {es o : List Entry}
    (h : InScheduleOrder drb es o) : o.Nodup :=
  h.2.imp fun {a b} hab he => by subst he; exact List.lt_irrefl _ hab

theorem totalStake_cons (e : Entry) (es : List Entry) :
    totalStake (e :: es) = e.stake + totalStake es := by
  simp [totalStake]

theorem stakeBefore_zero (es : List Entry) : stakeBefore es 0 = 0 := by
  simp [stakeBefore, totalStake]

theorem stakeBefore_cons_succ (e : Entry) (es : List Entry) (i : Nat) :
    stakeBefore (e :: es) (i + 1) = e.stake + stakeBefore es i := by
  simp only [stakeBefore, List.take_succ_cons, totalStake_cons]

theorem stakeBefore_succ {es : List Entry} {i : Nat} (h : i < es.length) :
    stakeBefore es (i + 1) = stakeBefore es i + es[i].stake := by
  simp only [stakeBefore, totalStake, List.take_add_one, List.getElem?_eq_getElem h,
    Option.toList_some, List.map_append, List.sum_append, List.map_cons, List.map_nil,
    List.sum_cons, List.sum_nil, Nat.add_zero]

theorem stakeBefore_mono {es : List Entry} {i j : Nat} (h : i ≤ j) :
    stakeBefore es i ≤ stakeBefore es j := by
  induction es generalizing i j with
  | nil => simp [stakeBefore, totalStake]
  | cons e es ih =>
    cases i with
    | zero => rw [stakeBefore_zero]; exact Nat.zero_le _
    | succ i =>
      cases j with
      | zero => omega
      | succ j =>
        rw [stakeBefore_cons_succ, stakeBefore_cons_succ]
        exact Nat.add_le_add_left (ih (by omega)) _

theorem stakeBefore_length (es : List Entry) : stakeBefore es es.length = totalStake es := by
  simp [stakeBefore]

/-- A breakpoint lies in at most one share. -/
theorem shareContains_unique {es : List Entry} {b i j : Nat} (h : ShareContains es b i)
    (h' : ShareContains es b j) : i = j := by
  have key : ∀ {i j}, ShareContains es b i → ShareContains es b j → i < j → False :=
    fun hi hj hlt => by
      have := stakeBefore_mono (es := es) (show _ + 1 ≤ _ from hlt)
      have := hi.2.2; have := hj.2.1; omega
  rcases Nat.lt_trichotomy i j with hlt | heq | hgt
  · exact (key h h' hlt).elim
  · exact heq
  · exact (key h' h hgt).elim

/-- A breakpoint below the total stake lies in some share. -/
theorem shareContains_exists {es : List Entry} {b : Nat} (hb : b < totalStake es) :
    ∃ i, ShareContains es b i := by
  induction es generalizing b with
  | nil => simp [totalStake] at hb
  | cons e es ih =>
    rw [totalStake_cons] at hb
    by_cases h : b < e.stake
    · refine ⟨0, by simp, by rw [stakeBefore_zero]; omega, ?_⟩
      rw [stakeBefore_cons_succ, stakeBefore_zero]; omega
    · obtain ⟨i, hl, h1, h2⟩ := ih (b := b - e.stake) (by omega)
      refine ⟨i + 1, by simp; omega, ?_, ?_⟩
      · rw [stakeBefore_cons_succ]; omega
      · rw [stakeBefore_cons_succ]; omega

theorem stakeBefore_succ_le_total {es : List Entry} {i : Nat} (h : i < es.length) :
    stakeBefore es (i + 1) ≤ totalStake es := by
  rw [← stakeBefore_length]; exact stakeBefore_mono h

/--
Of the breakpoints below the total stake, a validator's share holds as many as it
has stake.
-/
theorem shareContains_count {es : List Entry} {i : Nat} (h : i < es.length) :
    ((List.range (totalStake es)).filter fun b => decide (ShareContains es b i)).length
      = es[i].stake := by
  have hc : ((List.range (totalStake es)).filter fun b => decide (ShareContains es b i))
      = (List.range (totalStake es)).filter fun b =>
          decide (stakeBefore es i ≤ b ∧ b < stakeBefore es i + es[i].stake) := by
    apply List.filter_congr
    intro b _
    simp only [ShareContains, stakeBefore_succ h, h, true_and]
  rw [hc, length_filter_range_interval]
  have := stakeBefore_succ_le_total h
  rw [stakeBefore_succ h] at this
  omega

theorem totalStake_perm {es es' : List Entry} (h : es.Perm es') :
    totalStake es = totalStake es' :=
  (h.map Entry.stake).sum_nat

theorem le_totalStake {es : List Entry} {e : Entry} (h : e ∈ es) : e.stake ≤ totalStake es := by
  induction es with
  | nil => simp at h
  | cons e' es ih =>
    rw [totalStake_cons]
    rcases List.mem_cons.1 h with rfl | h
    · omega
    · have := ih h; omega

/-- With any stake at all, every breakpoint is drawn by some entry. -/
theorem draws_exists {es : List Entry} (h : 0 < totalStake es) (b : Nat) :
    ∃ e, Draws es b e := by
  obtain ⟨i, hi⟩ := shareContains_exists (es := es) (Nat.mod_lt b h)
  exact ⟨es[i]'hi.1, h, i, hi, List.getElem?_eq_getElem hi.1⟩

/-- Every breakpoint is drawn by at most one entry. -/
theorem draws_unique {es : List Entry} {b : Nat} {e e' : Entry}
    (h : Draws es b e) (h' : Draws es b e') : e = e' := by
  obtain ⟨_, i, hi, he⟩ := h
  obtain ⟨_, j, hj, he'⟩ := h'
  rw [shareContains_unique hi hj] at he
  rw [he] at he'
  exact Option.some.inj he'

/-! ## The leader of a committee -/

section

variable {b : Beacon} {as : List Member}

/-- The committee's entries have the committee's stake. -/
theorem totalStake_entries (b : Beacon) (as : List Member) :
    totalStake (as.map b.entry) = (as.map (·.stake)).sum := by
  simp only [totalStake, List.map_map]
  rfl

/-- Members with distinct consensus keys, each holding one, have distinct key bytes. -/
theorem entry_key_inj (hd : Distinct as) (hk : ∀ m ∈ as, m.blsKey.isSome) {m m' : Member}
    (hm : m ∈ as) (hm' : m' ∈ as) (h : (b.entry m).key = (b.entry m').key) : m = m' := by
  obtain ⟨k, hk1⟩ := Option.isSome_iff_exists.1 (hk m hm)
  obtain ⟨k', hk2⟩ := Option.isSome_iff_exists.1 (hk m' hm')
  simp only [Beacon.entry, hk1, hk2, Option.map_some, Option.getD_some] at h
  exact hd.bls m hm m' hm' k hk1 (b.keyBytes_injective h ▸ hk2)

theorem entries_keys_nodup (hd : Distinct as) (hk : ∀ m ∈ as, m.blsKey.isSome) :
    ((as.map b.entry).map (·.key)).Nodup := by
  rw [List.map_map]
  exact nodup_map_of_injOn (fun m hm m' hm' h => entry_key_inj hd hk hm hm' h)
    (nodup_of_nodup_map hd.accounts)

/-- The committee in schedule order. -/
theorem scheduled_exists (b : Beacon) (hd : Distinct as) (hk : ∀ m ∈ as, m.blsKey.isSome) :
    ∃ o, InScheduleOrder b.drb (as.map b.entry) o :=
  ⟨_, inScheduleOrder_mergeSort b.drb (entries_keys_nodup hd hk)⟩

/-- A leader of a view leads it in any other listing of the committee. -/
theorem leads_perm {as' : List Member} {v : Nat} {m : Member} (hp : as.Perm as')
    (h : Leads b as v m) : Leads b as' v m :=
  let ⟨hm, o, ho, hd⟩ := h
  ⟨hp.subset hm, o, ⟨ho.1.trans (hp.map _), ho.2⟩, hd⟩

/-- Of a committee with distinct consensus keys and some stake, each view has a leader. -/
theorem leads_exists (b : Beacon) (v : Nat) (hd : Distinct as) (hk : ∀ m ∈ as, m.blsKey.isSome)
    (hs : 0 < (as.map (·.stake)).sum) : ∃ m, Leads b as v m := by
  obtain ⟨o, ho⟩ := scheduled_exists b hd hk
  have ht : 0 < totalStake o := by
    rw [totalStake_perm ho.1, totalStake_entries]; exact hs
  obtain ⟨e, he⟩ := draws_exists ht (b.hash b.drb v (o.map (·.key)))
  obtain ⟨_, i, hi, hie⟩ := he
  have : e ∈ as.map b.entry := ho.1.subset (List.mem_of_getElem? hie)
  obtain ⟨m, hm, rfl⟩ := List.mem_map.1 this
  exact ⟨m, hm, o, ho, ht, i, hi, hie⟩

/-- Of a committee with distinct consensus keys, each view has at most one leader. -/
theorem leads_unique {v : Nat} {m m' : Member} (hd : Distinct as)
    (hk : ∀ m ∈ as, m.blsKey.isSome) (h : Leads b as v m) (h' : Leads b as v m') : m = m' := by
  obtain ⟨hm, o, ho, hl⟩ := h
  obtain ⟨hm', o', ho', hl'⟩ := h'
  rw [inScheduleOrder_unique ho' ho] at hl'
  exact entry_key_inj hd hk hm hm' (congrArg Entry.key (draws_unique hl hl'))

/--
A member of a committee with distinct consensus keys leads exactly the breakpoints
below the total stake that lie in an interval as long as its stake.
-/
theorem leads_iff (b : Beacon) (hd : Distinct as) (hk : ∀ m ∈ as, m.blsKey.isSome) {m : Member}
    (hm : m ∈ as) :
    ∃ lo, lo + m.stake ≤ (as.map (·.stake)).sum ∧
      ∀ v < (as.map (·.stake)).sum,
        Leads { b with hash := fun _ v _ => v } as v m ↔ lo ≤ v ∧ v < lo + m.stake := by
  let b' : Beacon := { b with hash := fun _ v _ => v }
  have he : b'.entry = b.entry := rfl
  obtain ⟨o, ho⟩ := scheduled_exists b' hd hk
  have htot : totalStake o = (as.map (·.stake)).sum := by
    rw [totalStake_perm ho.1, totalStake_entries]
  obtain ⟨i, hi, hoi⟩ :=
    List.getElem_of_mem (ho.1.symm.subset (List.mem_map_of_mem (f := b'.entry) hm))
  have hsucc := stakeBefore_succ hi
  rw [hoi] at hsucc
  have hle := stakeBefore_succ_le_total hi
  refine ⟨stakeBefore o i, by rw [← htot]; simp only [Beacon.entry] at hsucc; omega, ?_⟩
  intro v hv
  rw [← htot] at hv
  have hmod : v % totalStake o = v := Nat.mod_eq_of_lt hv
  simp only [Beacon.entry] at hsucc
  constructor
  · rintro ⟨_, o', ho', _, j, hj, hje⟩
    rw [inScheduleOrder_unique ho' ho] at hj hje
    simp only [hmod] at hj
    obtain ⟨hjl, hje⟩ := List.getElem?_eq_some_iff.1 hje
    have hji : j = i := (ho.nodup.getElem_inj (hi := hjl) (hj := hi)).1 (hje.trans hoi.symm)
    subst hji
    obtain ⟨_, h1, h2⟩ := hj
    exact ⟨h1, by omega⟩
  · rintro ⟨h1, h2⟩
    refine ⟨hm, o, ho, by omega, i, ?_, ?_⟩
    · show ShareContains o (v % totalStake o) i
      rw [hmod]; exact ⟨hi, h1, by omega⟩
    · rw [List.getElem?_eq_getElem hi, hoi]

end

end StakeTable
