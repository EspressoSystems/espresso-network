module

public import StakeTableImpl.Node

/-!
# Lookups in the list state

Finding a delegation or a validator in the list state, and how `credit`, `debit`,
`State.admit`, `State.update` and `State.remove` change what is found. These are the
facts the indexed state's hash maps are compared against.
-/

@[expose] public section

namespace StakeTable

/-- `List.lookup` on a delegation list, by its first entry. -/
theorem lookup_cons' (d : Address) (e : Address × Nat) (es : List (Address × Nat)) :
    List.lookup d (e :: es) = if e.1 = d then some e.2 else List.lookup d es := by
  obtain ⟨k, v⟩ := e
  show (match d == k with | true => some v | false => List.lookup d es) = _
  by_cases h : k = d
  · subst h; simp
  · rw [beq_false_of_ne (Ne.symm h)]; simp [h]

theorem amountOf_eq_lookup (d : Address) (ds : List (Address × Nat)) :
    amountOf d ds = (List.lookup d ds).getD 0 := by
  induction ds with
  | nil => rfl
  | cons e es ih => simp only [amountOf, lookup_cons']; split <;> simp [ih]

theorem delegates_eq_lookup (d : Address) (ds : List (Address × Nat)) :
    delegates d ds = (List.lookup d ds).isSome := by
  induction ds with
  | nil => rfl
  | cons e es ih => simp only [delegates, lookup_cons']; split <;> simp_all

theorem lookup_credit (d d' : Address) (amount : Nat) (ds : List (Address × Nat)) :
    List.lookup d' (credit d amount ds)
      = if d = d' then some (amountOf d ds + amount) else List.lookup d' ds := by
  induction ds with
  | nil => by_cases h : d = d' <;> simp [credit, lookup_cons', amountOf, h]
  | cons e es ih =>
    simp only [credit, amountOf, lookup_cons']
    by_cases he : e.1 = d
    · subst he
      by_cases h : e.1 = d' <;> simp [h, lookup_cons']
    · simp only [he, ↓reduceIte, lookup_cons', ih]
      by_cases h : d = d'
      · subst h; simp [he]
      · simp [h]

theorem lookup_none_of_not_mem {d : Address} {ds : List (Address × Nat)}
    (h : d ∉ ds.map (·.1)) : List.lookup d ds = none := by
  induction ds with
  | nil => rfl
  | cons e es ih =>
    simp only [List.map_cons, List.mem_cons, not_or] at h
    simp only [lookup_cons', Ne.symm h.1, ↓reduceIte, ih h.2]

theorem lookup_debit (d d' : Address) (amount : Nat) {ds : List (Address × Nat)}
    (hn : (ds.map (·.1)).Nodup) :
    List.lookup d' (debit d amount ds)
      = if d = d' then
          (if amountOf d ds - amount = 0 then none else some (amountOf d ds - amount))
        else List.lookup d' ds := by
  induction ds with
  | nil => by_cases h : d = d' <;> simp [debit, amountOf, h]
  | cons e es ih =>
    simp only [List.map_cons, List.nodup_cons] at hn
    simp only [debit, amountOf, lookup_cons']
    by_cases he : e.1 = d
    · subst he
      by_cases hz : e.2 - amount = 0
      · simp only [hz, ↓reduceIte]
        by_cases h : e.1 = d'
        · subst h; simp [lookup_none_of_not_mem hn.1]
        · simp [h]
      · by_cases h : e.1 = d' <;> simp [hz, h, lookup_cons']
    · simp only [he, ↓reduceIte, lookup_cons', ih hn.2]
      by_cases h : d = d'
      · subst h; simp [he]
      · simp [h]

namespace State

theorem registered_eq_find? (s : State) (a : Address) :
    s.registered a = (s.find? a).isSome := by
  simp only [State.registered, State.find?]
  induction s.validators with
  | nil => rfl
  | cons v vs ih =>
    simp only [List.any_cons, List.find?_cons]
    cases h : v.account == a <;> simp [ih]

theorem find?_admit {s : State} {v : Validator} (b : Address)
    (hreg : ¬ s.registered v.account = true) :
    (s.admit v).find? b = if v.account = b then some v else s.find? b := by
  simp only [State.find?, State.admit, List.find?_append, List.find?_cons, List.find?_nil]
  by_cases h : v.account = b
  · subst h
    have : s.find? v.account = none := by
      rw [registered_eq_find?] at hreg
      simpa using hreg
    simp only [State.find?] at this
    simp [this]
  · have hb : (v.account == b) = false := by simpa using h
    simp only [h, ↓reduceIte, hb]
    cases s.validators.find? (·.account == b) <;> rfl

theorem find?_update {s : State} {a : Address} {f : Validator → Validator} (b : Address)
    (hf : ∀ v, (f v).account = v.account) :
    (s.update a f).find? b = (s.find? b).map (fun v => if v.account = a then f v else v) := by
  simp only [State.find?, State.update, List.find?_map]
  congr 2
  funext v
  by_cases h : v.account = a <;> simp [h, hf]

theorem find?_remove (s : State) (a b : Address) :
    (s.remove a).find? b = if a = b then none else s.find? b := by
  simp only [State.find?, State.remove]
  induction s.validators with
  | nil => simp
  | cons v vs ih =>
    simp only [List.filter_cons, List.find?_cons]
    by_cases hv : v.account = a
    · subst hv
      simp only [bne_self_eq_false, Bool.false_eq_true, ↓reduceIte, ih]
      by_cases h : v.account = b
      · simp [h]
      · have hb : (v.account == b) = false := by simpa using h
        simp [h, hb]
    · have : (v.account != a) = true := by simpa using hv
      simp only [this, ↓reduceIte, List.find?_cons, ih]
      by_cases h : a = b
      · subst h
        have ha : (v.account == a) = false := by simpa using hv
        simp [ha]
      · simp [h]

end State

end StakeTable
