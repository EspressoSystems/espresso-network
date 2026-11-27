module

public import StakeTableSpec

/-!
# Ordering the schedule

`StakeTable.scheduleOrder` sorts the committee by `xor`ed key with `mergeSort`. With
distinct keys the result is in schedule order (`StakeTable.scheduleOrder_spec`).

`StakeTable.leader` finds the entry that draws a breakpoint by walking the shares,
subtracting each stake from the breakpoint until it falls within one
(`StakeTable.pickIndex_eq_some_iff`, `StakeTable.leader_eq_some_iff`).
`StakeTable.committeeLeader` puts the two together and gives the specification's
leader of a view (`StakeTable.committeeLeader_eq_some_iff`).
-/

@[expose] public section

namespace StakeTable

/-- The committee sorted by `xor`ed key, lexicographically. -/
def scheduleOrder (drb : List Nat) (es : List Entry) : List Entry :=
  es.mergeSort fun a b => decide (xorKey drb a.key ≤ xorKey drb b.key)

theorem scheduleOrder_spec (drb : List Nat) {es : List Entry} (hk : (es.map (·.key)).Nodup) :
    InScheduleOrder drb es (scheduleOrder drb es) :=
  inScheduleOrder_mergeSort drb hk

/--
The position of the validator whose share contains breakpoint `b`, or `none` if `b` is
at or beyond the total stake.
-/
def pickIndex : List Entry → Nat → Option Nat
  | [], _ => none
  | e :: es, b => if b < e.stake then some 0 else (pickIndex es (b - e.stake)).map (· + 1)

/-- The entry that draws breakpoint `b`, or `none` if there is no stake. -/
def leader (es : List Entry) (b : Nat) : Option Entry :=
  if totalStake es = 0 then none
  else (pickIndex es (b % totalStake es)).bind (es[·]?)

/-- Breakpoint `b` picks position `i` exactly when `i`'s share contains it. -/
theorem pickIndex_eq_some_iff :
    ∀ {es : List Entry} {b i : Nat}, pickIndex es b = some i ↔ ShareContains es b i
  | [], b, i => by simp [pickIndex, ShareContains]
  | e :: es, b, 0 => by
    simp only [pickIndex, ShareContains, stakeBefore_zero, Nat.zero_add, stakeBefore_cons_succ,
      Nat.add_zero, List.length_cons]
    by_cases hb : b < e.stake <;> simp [hb]
  | e :: es, b, j + 1 => by
    simp only [pickIndex, ShareContains, stakeBefore_cons_succ, List.length_cons]
    by_cases hb : b < e.stake
    · simp only [hb, ite_true, Option.some.injEq]
      constructor
      · intro h; omega
      · rintro ⟨_, h1, _⟩; omega
    · simp only [hb, ite_false, Option.map_eq_some_iff]
      constructor
      · rintro ⟨k, hk, hkj⟩
        have hkj : k = j := by omega
        subst hkj
        obtain ⟨hl, h1, h2⟩ := pickIndex_eq_some_iff.1 hk
        exact ⟨by omega, by omega, by omega⟩
      · rintro ⟨hl, h1, h2⟩
        exact ⟨j, pickIndex_eq_some_iff.2 ⟨by omega, by omega, by omega⟩, rfl⟩

/-- `leader` gives the entry that draws the breakpoint. -/
theorem leader_eq_some_iff {es : List Entry} {b : Nat} {e : Entry} :
    leader es b = some e ↔ Draws es b e := by
  unfold leader Draws
  by_cases ht : totalStake es = 0
  · simp [ht]
  · simp only [ht, ite_false, Option.bind_eq_some_iff, pickIndex_eq_some_iff]
    constructor
    · rintro ⟨i, hi, he⟩; exact ⟨by omega, i, hi, he⟩
    · rintro ⟨_, i, hi, he⟩; exact ⟨i, hi, he⟩

/--
The leader of a view: the committee's entries sorted into schedule order, and the
breakpoint the view draws from the sorted keys.
-/
def committeeLeader (b : Beacon) (as : List Member) (v : Nat) : Option Entry :=
  let o := scheduleOrder b.drb (as.map b.entry)
  leader o (b.hash b.drb v (o.map (·.key)))

/--
For a committee whose key bytes are distinct, `committeeLeader` gives the entry of the
specification's leader, and gives one whenever the committee has stake.
-/
theorem committeeLeader_eq_some_iff {b : Beacon} {as : List Member} {v : Nat}
    (hk : ((as.map b.entry).map (·.key)).Nodup) {e : Entry} :
    committeeLeader b as v = some e ↔ ∃ m, Leads b as v m ∧ b.entry m = e := by
  have ho := scheduleOrder_spec b.drb hk
  simp only [committeeLeader, leader_eq_some_iff]
  constructor
  · intro hl
    obtain ⟨hpos, i, hi, hie⟩ := hl
    obtain ⟨m, hm, rfl⟩ := List.mem_map.1 (ho.1.subset (List.mem_of_getElem? hie))
    exact ⟨m, ⟨hm, _, ho, hpos, i, hi, hie⟩, rfl⟩
  · rintro ⟨m, ⟨_, o, ho', hl⟩, rfl⟩
    rwa [inScheduleOrder_unique ho' ho] at hl

end StakeTable
