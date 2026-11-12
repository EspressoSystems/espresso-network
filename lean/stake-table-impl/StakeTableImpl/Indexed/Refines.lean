module

public import StakeTableImpl.Indexed.Apply
public import StakeTableImpl.Indexed.Lists
public import StakeTableImpl.Conformance
public import Std.Data.HashMap.Lemmas
public import Std.Data.HashSet.Lemmas

/-!
# The indexed state refines the list state

`StakeTable.Indexed.Rel` relates an indexed state to a list state: the same
validators in the same order with the same fields, the same delegations, and the
same admitted keys and exits. Only the order of delegations and of admitted keys,
which nothing reads, is left out.

From related states, `StakeTable.Indexed.apply` and `StakeTable.apply` give the same
error, or related states (`StakeTable.Indexed.apply_rel`). With
`StakeTable.apply_outcome`, the indexed state therefore meets the specification
(`StakeTable.Indexed.applyAll_replay`).
-/

@[expose] public section

namespace StakeTable.Indexed

open Std

/-- An indexed validator and a list validator agree. -/
structure ValidatorRel (iv : Validator) (v : StakeTable.Validator) : Prop where
  account : iv.account = v.account
  blsKey : iv.blsKey = v.blsKey
  schnorrKey : iv.schnorrKey = v.schnorrKey
  x25519Key : iv.x25519Key = v.x25519Key
  p2pAddr : iv.p2pAddr = v.p2pAddr
  stake : iv.stake = v.stake
  commission : iv.commission = v.commission
  authenticated : iv.authenticated = v.authenticated
  delegators : ∀ d, iv.delegators[d]? = List.lookup d v.delegators

/-- Both absent, or both present and in agreement. -/
def OptRel : Option Validator → Option StakeTable.Validator → Prop
  | none, none => True
  | some iv, some v => ValidatorRel iv v
  | _, _ => False

/-- An indexed state and a list state agree. -/
structure Rel (is : State) (s : StakeTable.State) : Prop where
  order : is.order.toList = s.validators.map (·.account)
  validators : ∀ a, OptRel is.validators[a]? (s.find? a)
  exits : ∀ a, is.exits.contains a = true ↔ a ∈ s.exits
  usedBls : ∀ k, is.usedBls.contains k = true ↔ k ∈ s.usedBls
  usedSchnorr : ∀ k, is.usedSchnorr.contains k = true ↔ k ∈ s.usedSchnorr
  usedX25519 : ∀ k, is.usedX25519.contains k = true ↔ k ∈ s.usedX25519

/--
Two results of applying an event to `s` agree: the same error, or agreeing states.
A skipped event hands back a state that agrees with `s`.
-/
def ResultRel (s : StakeTable.State) : ApplyResult → StakeTable.ApplyResult → Prop
  | .error e, .error e' => e = e'
  | .ok (.skipped x is), .ok (.error y) => x = y ∧ Rel is s
  | .ok (.applied is), .ok (.ok s') => Rel is s'
  | _, _ => False

theorem Rel.empty : Rel State.empty StakeTable.State.empty where
  order := rfl
  validators a := by simp [State.empty, StakeTable.State.empty, StakeTable.State.find?, OptRel]
  exits a := by simp [State.empty, StakeTable.State.empty]
  usedBls k := by simp [State.empty, StakeTable.State.empty]
  usedSchnorr k := by simp [State.empty, StakeTable.State.empty]
  usedX25519 k := by simp [State.empty, StakeTable.State.empty]

theorem Rel.registered {is : State} {s : StakeTable.State} (hr : Rel is s) (a : Address) :
    is.registered a = s.registered a := by
  have hv := hr.validators a
  rw [State.registered, HashMap.contains_eq_isSome_getElem?, StakeTable.State.registered_eq_find?]
  cases hi : is.validators[a]? <;> cases hs : s.find? a <;> rw [hi, hs] at hv <;>
    simp_all [OptRel]

theorem Rel.bls_contains {is : State} {s : StakeTable.State} (hr : Rel is s) :
    is.usedBls.contains = fun k => decide (k ∈ s.usedBls) := by
  funext k; apply Bool.eq_iff_iff.2; simp [hr.usedBls k]

theorem Rel.schnorr_contains {is : State} {s : StakeTable.State} (hr : Rel is s) :
    is.usedSchnorr.contains = fun k => decide (k ∈ s.usedSchnorr) := by
  funext k; apply Bool.eq_iff_iff.2; simp [hr.usedSchnorr k]

theorem Rel.x25519_contains {is : State} {s : StakeTable.State} (hr : Rel is s) :
    is.usedX25519.contains = fun k => decide (k ∈ s.usedX25519) := by
  funext k; apply Bool.eq_iff_iff.2; simp [hr.usedX25519 k]

theorem ValidatorRel.new (a : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) (commission : Nat)
    (authenticated : Bool) :
    ValidatorRel (Validator.new a bls schnorr x25519 p2pAddr commission authenticated)
      (StakeTable.Validator.new a bls schnorr x25519 p2pAddr commission authenticated) where
  account := rfl
  blsKey := rfl
  schnorrKey := rfl
  x25519Key := rfl
  p2pAddr := rfl
  stake := rfl
  commission := rfl
  authenticated := rfl
  delegators d := by simp [Validator.new, StakeTable.Validator.new]

theorem contains_insertKey {κ : Type} [BEq κ] [Hashable κ] [LawfulBEq κ]
    (set : HashSet κ) (k? : Option κ) (x : κ) (l : List κ)
    (h : set.contains x = true ↔ x ∈ l) :
    (insertKey set k?).contains x = true ↔ x ∈ k?.toList ++ l := by
  cases k? with
  | none => simpa [insertKey] using h
  | some k =>
    simp only [insertKey, HashSet.contains_insert, Bool.or_eq_true, beq_iff_eq, h,
      Option.toList_some, List.cons_append, List.nil_append, List.mem_cons]
    constructor <;> rintro (h | h) <;> first | exact .inl h.symm | exact .inr h

theorem Rel.admit {is : State} {s : StakeTable.State} {iv : Validator} {v : StakeTable.Validator}
    (hr : Rel is s) (hv : ValidatorRel iv v) (hreg : ¬ s.registered v.account = true) :
    Rel (is.admit iv) (s.admit v) where
  order := by simp [State.admit, StakeTable.State.admit, hr.order, hv.account]
  validators b := by
    simp only [State.admit, HashMap.getElem?_insert, beq_iff_eq,
      StakeTable.State.find?_admit b hreg, hv.account]
    split
    · exact hv
    · exact hr.validators b
  exits a := hr.exits a
  usedBls k := by
    simp only [State.admit, StakeTable.State.admit, hv.blsKey]
    exact contains_insertKey _ _ _ _ (hr.usedBls k)
  usedSchnorr k := by
    simp only [State.admit, StakeTable.State.admit, hv.schnorrKey]
    exact contains_insertKey _ _ _ _ (hr.usedSchnorr k)
  usedX25519 k := by
    simp only [State.admit, StakeTable.State.admit, hv.x25519Key]
    exact contains_insertKey _ _ _ _ (hr.usedX25519 k)

theorem Rel.update {is : State} {s : StakeTable.State} {a : Address} {g : Validator → Validator}
    {f : StakeTable.Validator → StakeTable.Validator} (hr : Rel is s)
    (hacc : ∀ v, (f v).account = v.account)
    (hgf : ∀ iv v, is.validators[a]? = some iv → s.find? a = some v → ValidatorRel iv v →
      ValidatorRel (g iv) (f v)) :
    Rel (is.update a g) (s.update a f) where
  order := by
    show is.order.toList = (s.update a f).validators.map (·.account)
    rw [StakeTable.accounts_update hacc]; exact hr.order
  validators b := by
    simp only [State.update, HashMap.getElem?_modify, beq_iff_eq,
      StakeTable.State.find?_update b hacc]
    by_cases hab : a = b
    · subst hab
      have hv := hr.validators a
      simp only [↓reduceIte]
      cases hiv : is.validators[a]? <;> cases hsv : s.find? a <;> rw [hiv, hsv] at hv <;>
        simp only [OptRel] at hv ⊢
      · trivial
      · rename_i iv v
        simp only [Option.map_some, (StakeTable.find?_some hsv).2, ↓reduceIte]
        exact hgf iv v hiv hsv hv
    · simp only [hab, ↓reduceIte]
      have hv := hr.validators b
      cases hsv : s.find? b with
      | none => rw [hsv] at hv; simpa using hv
      | some v =>
        rw [hsv] at hv
        have hb := (StakeTable.find?_some hsv).2
        have : v.account ≠ a := fun h => hab (h ▸ hb)
        simpa [this] using hv
  exits a := hr.exits a
  usedBls k := hr.usedBls k
  usedSchnorr k := hr.usedSchnorr k
  usedX25519 k := hr.usedX25519 k

theorem Rel.remove {is : State} {s : StakeTable.State} (hr : Rel is s) (a : Address) :
    Rel (is.remove a) (s.remove a) where
  order := by
    simp only [State.remove, StakeTable.State.remove, Array.toList_filter, hr.order,
      List.filter_map]
    rfl
  validators b := by
    simp only [State.remove, HashMap.getElem?_erase, beq_iff_eq,
      StakeTable.State.find?_remove]
    split
    · trivial
    · exact hr.validators b
  exits x := by
    simp only [State.remove, StakeTable.State.remove, HashSet.contains_insert, Bool.or_eq_true,
      beq_iff_eq, hr.exits, List.mem_cons]
    constructor <;> rintro (h | h) <;> first | exact .inl h.symm | exact .inr h
  usedBls k := hr.usedBls k
  usedSchnorr k := hr.usedSchnorr k
  usedX25519 k := hr.usedX25519 k

theorem Rel.insertBls {is : State} {s : StakeTable.State} (hr : Rel is s) (k : BlsKey) :
    Rel { is with usedBls := is.usedBls.insert k } { s with usedBls := k :: s.usedBls } :=
  { hr with
    usedBls := fun x => by
      have := contains_insertKey is.usedBls (some k) x s.usedBls (hr.usedBls x)
      simpa [insertKey] using this }

theorem Rel.insertSchnorr {is : State} {s : StakeTable.State} (hr : Rel is s) (k : SchnorrKey) :
    Rel { is with usedSchnorr := is.usedSchnorr.insert k }
      { s with usedSchnorr := k :: s.usedSchnorr } :=
  { hr with
    usedSchnorr := fun x => by
      have := contains_insertKey is.usedSchnorr (some k) x s.usedSchnorr (hr.usedSchnorr x)
      simpa [insertKey] using this }

theorem Rel.insertX25519 {is : State} {s : StakeTable.State} (hr : Rel is s) (k : X25519Key) :
    Rel { is with usedX25519 := is.usedX25519.insert k }
      { s with usedX25519 := k :: s.usedX25519 } :=
  { hr with
    usedX25519 := fun x => by
      have := contains_insertKey is.usedX25519 (some k) x s.usedX25519 (hr.usedX25519 x)
      simpa [insertKey] using this }

theorem applyRekey_rel {is : State} {s : StakeTable.State} (hr : Rel is s) (a : Address)
    (bls : BlsKey) (schnorr : SchnorrKey) :
    ResultRel s (is.applyRekey a bls schnorr) (s.applyRekey a bls schnorr) := by
  simp only [State.applyRekey, StakeTable.State.applyRekey, hr.registered a]
  by_cases hreg : s.registered a = true
  · by_cases hb : bls ∈ s.usedBls
    · have : is.usedBls.contains bls = true := (hr.usedBls bls).2 hb
      simp [hreg, hb, this, ResultRel]
    · have hb' : ¬ is.usedBls.contains bls = true := mt (hr.usedBls bls).1 hb
      by_cases hs : schnorr ∈ s.usedSchnorr
      · have : is.usedSchnorr.contains schnorr = true := (hr.usedSchnorr schnorr).2 hs
        simp [hreg, hb, hb', hs, this, ResultRel, hr]
      · have hs' : ¬ is.usedSchnorr.contains schnorr = true := mt (hr.usedSchnorr schnorr).1 hs
        simp only [hreg, hb, hb', hs, hs', Bool.not_true, Bool.false_eq_true, ↓reduceIte,
          not_true_eq_false, ResultRel]
        have hu : Rel
            (is.update a fun v => { v with blsKey := some bls, schnorrKey := some schnorr,
                                           authenticated := true })
            (s.update a fun v => { v with blsKey := some bls, schnorrKey := some schnorr,
                                          authenticated := true }) :=
          hr.update (fun _ => rfl) fun _ _ _ _ hv =>
            { hv with blsKey := rfl, schnorrKey := rfl, authenticated := rfl }
        exact (hu.insertBls bls).insertSchnorr schnorr
  · simp [hreg, ResultRel]

theorem admitRegistration_rel {is : State} {s : StakeTable.State} (hr : Rel is s)
    {a : Address} (keys : RegistrationKeys) (commission : Nat) (x25519 : Option X25519Key)
    (p2pAddr : Option NetAddr) (hreg : ¬ s.registered a = true) :
    Rel (is.admitRegistration a keys commission x25519 p2pAddr)
      (s.admitRegistration a keys commission x25519 p2pAddr) := by
  simp only [State.admitRegistration, StakeTable.State.admitRegistration, hr.schnorr_contains,
    StakeTable.schnorr_filter]
  split
  · exact hr.admit (ValidatorRel.new ..) hreg
  · exact hr.admit (ValidatorRel.new ..) hreg

theorem lookup_indexed_credit {iv : Validator} {v : StakeTable.Validator} (hv : ValidatorRel iv v)
    (d d' : Address) (amount : Nat) :
    (credit d amount iv.delegators)[d']? = List.lookup d' (StakeTable.credit d amount v.delegators) := by
  rw [credit, HashMap.getElem?_insert, lookup_credit, HashMap.getD_eq_getD_getElem?,
    hv.delegators, ← amountOf_eq_lookup]
  simp only [beq_iff_eq, hv.delegators]

theorem lookup_indexed_debit {iv : Validator} {v : StakeTable.Validator} (hv : ValidatorRel iv v)
    (hn : (v.delegators.map (·.1)).Nodup) (d d' : Address) (amount : Nat) :
    (debit d amount iv.delegators)[d']? = List.lookup d' (StakeTable.debit d amount v.delegators) := by
  rw [lookup_debit d d' amount hn]
  have hget : iv.delegators.getD d 0 = amountOf d v.delegators := by
    rw [HashMap.getD_eq_getD_getElem?, hv.delegators, ← amountOf_eq_lookup]
  simp only [debit, hget]
  split
  · rw [HashMap.getElem?_erase]
    by_cases h : d = d' <;> simp [h, hv.delegators]
  · rw [HashMap.getElem?_insert]
    by_cases h : d = d' <;> simp [h, hv.delegators]

/-- From agreeing states, the indexed and the list `apply` agree. -/
theorem apply_rel {is : State} {s : StakeTable.State} (hr : Rel is s) (h : Inv s) (e : Event) :
    ResultRel s (apply is e) (StakeTable.apply s e) := by
  cases e with
  | register a bls schnorr commission =>
    simp only [apply, StakeTable.apply, hr.registered a, hr.bls_contains, hr.schnorr_contains]
    by_cases hx : a ∈ s.exits
    · have : is.exits.contains a = true := (hr.exits a).2 hx
      simp [hx, this, ResultRel]
    have hx' : ¬ is.exits.contains a = true := mt (hr.exits a).1 hx
    by_cases hreg : s.registered a = true
    · simp [hx, hx', hreg, ResultRel]
    cases hb : bls.filter (fun k => decide (k ∈ s.usedBls)) with
    | some k => simp [hx, hx', hreg, ResultRel]
    | none =>
      cases hs : schnorr.filter (fun k => decide (k ∈ s.usedSchnorr)) with
      | some j => simp [hx, hx', hreg, ResultRel, hr]
      | none =>
        simp only [hx, hx', hreg, ↓reduceIte, Bool.false_eq_true, ResultRel]
        exact hr.admit (ValidatorRel.new ..) hreg
  | registerV2 a keys commission =>
    simp only [apply, StakeTable.apply, hr.registered a, hr.bls_contains]
    by_cases hx : a ∈ s.exits
    · have : is.exits.contains a = true := (hr.exits a).2 hx
      simp [hx, this, ResultRel]
    have hx' : ¬ is.exits.contains a = true := mt (hr.exits a).1 hx
    by_cases hreg : s.registered a = true
    · simp [hx, hx', hreg, ResultRel]
    cases hb : keys.bls.filter (fun k => decide (k ∈ s.usedBls)) with
    | some k => simp [hx, hx', hreg, ResultRel]
    | none =>
      simp only [hx, hx', hreg, ↓reduceIte, Bool.false_eq_true, ResultRel]
      exact admitRegistration_rel hr keys commission none none hreg
  | registerV3 a keys commission x25519 p2pAddr =>
    simp only [apply, StakeTable.apply, hr.registered a, hr.bls_contains, hr.x25519_contains]
    by_cases hx : a ∈ s.exits
    · have : is.exits.contains a = true := (hr.exits a).2 hx
      simp [hx, this, ResultRel]
    have hx' : ¬ is.exits.contains a = true := mt (hr.exits a).1 hx
    by_cases hreg : s.registered a = true
    · simp [hx, hx', hreg, ResultRel]
    cases hb : keys.bls.filter (fun k => decide (k ∈ s.usedBls)) with
    | some k => simp [hx, hx', hreg, ResultRel]
    | none =>
      cases hk : x25519.filter (fun k => decide (k ∈ s.usedX25519)) with
      | some k => simp [hx, hx', hreg, ResultRel]
      | none =>
        simp only [hx, hx', hreg, ↓reduceIte, Bool.false_eq_true, ResultRel]
        exact admitRegistration_rel hr keys commission x25519 p2pAddr hreg
  | deregister a =>
    simp only [apply, StakeTable.apply, hr.registered a]
    by_cases hreg : s.registered a = true
    · simp only [hreg, Bool.not_true, Bool.false_eq_true, ↓reduceIte, not_true_eq_false,
        ResultRel]
      exact hr.remove a
    · simp [hreg, ResultRel]
  | delegate d a amount =>
    simp only [apply, StakeTable.apply]
    by_cases h0 : amount = 0
    · simp [h0, ResultRel]
    have hv := hr.validators a
    simp only [h0, ↓reduceIte]
    cases hiv : is.validators[a]? <;> cases hsv : s.find? a <;> rw [hiv, hsv] at hv <;>
      simp only [OptRel] at hv
    · simp [ResultRel]
    · rename_i iv v
      have hget : iv.delegators.getD d 0 = amountOf d v.delegators := by
        rw [HashMap.getD_eq_getD_getElem?, hv.delegators, ← amountOf_eq_lookup]
      simp only [hv.stake, hget]
      by_cases hb1 : U256 ≤ v.stake + amount
      · simp [hb1, ResultRel]
      by_cases hb2 : U256 ≤ amountOf d v.delegators + amount
      · simp [hb1, hb2, ResultRel]
      simp only [hb1, hb2, ↓reduceIte, ResultRel]
      refine hr.update (fun _ => rfl) ?_
      intro iv' v' _ _ hv'
      exact { hv' with
        stake := by simp [hv'.stake]
        delegators := fun d' => lookup_indexed_credit hv' d d' amount }
  | undelegate d a amount =>
    simp only [apply, StakeTable.apply]
    have hv := hr.validators a
    cases hiv : is.validators[a]? <;> cases hsv : s.find? a <;> rw [hiv, hsv] at hv <;>
      simp only [OptRel] at hv
    · simp [ResultRel]
    · rename_i iv v
      have hget : iv.delegators.getD d 0 = amountOf d v.delegators := by
        rw [HashMap.getD_eq_getD_getElem?, hv.delegators, ← amountOf_eq_lookup]
      have hcon : iv.delegators.contains d = delegates d v.delegators := by
        rw [HashMap.contains_eq_isSome_getElem?, hv.delegators, delegates_eq_lookup]
      simp only [hv.stake, hget, hcon]
      by_cases h1 : v.stake < amount
      · simp [h1, ResultRel]
      by_cases h2 : delegates d v.delegators = false
      · simp [h1, h2, ResultRel]
      by_cases h3 : amountOf d v.delegators < amount
      · simp [h1, h2, h3, ResultRel]
      simp only [h1, h2, h3, ↓reduceIte, ResultRel]
      refine hr.update (fun _ => rfl) ?_
      intro iv' v' _ hsv' hv'
      have hn := (h.sound v' (StakeTable.find?_some hsv').1).delegatorsNodup
      exact { hv' with
        stake := by simp [hv'.stake]
        delegators := fun d' => lookup_indexed_debit hv' hn d d' amount }
  | keyUpdate a bls schnorr =>
    cases bls <;> cases schnorr <;> simp only [apply, StakeTable.apply, ResultRel] <;>
      first | exact ⟨trivial, hr⟩ | exact applyRekey_rel hr ..
  | keyUpdateV2 a keys =>
    cases keys <;> simp only [apply, StakeTable.apply, ResultRel] <;>
      first | exact ⟨trivial, hr⟩ | exact applyRekey_rel hr .. | trivial
  | commissionUpdate a commission =>
    simp only [apply, StakeTable.apply, hr.registered a]
    by_cases hc : maxCommission < commission
    · simp [hc, ResultRel]
    by_cases hreg : s.registered a = true
    · simp only [hc, hreg, Bool.not_true, Bool.false_eq_true, ↓reduceIte, not_true_eq_false,
        ResultRel]
      refine hr.update (fun _ => rfl) ?_
      intro iv v _ _ hv
      exact { hv with commission := rfl }
    · simp [hc, hreg, ResultRel]
  | x25519Update a key =>
    simp only [apply, StakeTable.apply, hr.registered a, hr.x25519_contains]
    by_cases hreg : s.registered a = true
    · cases key with
      | none =>
        simp only [hreg, Bool.not_true, Bool.false_eq_true, ↓reduceIte, not_true_eq_false,
          Option.filter_none, ResultRel]
        refine hr.update (fun _ => rfl) ?_
        intro iv v _ _ hv
        exact { hv with x25519Key := rfl }
      | some k =>
        by_cases hk : k ∈ s.usedX25519
        · simp [hreg, hk, Option.filter, ResultRel]
        · simp only [hreg, hk, Bool.not_true, Bool.false_eq_true, ↓reduceIte, not_true_eq_false,
            Option.filter, decide_false, ResultRel]
          have hu : Rel (is.update a fun v => { v with x25519Key := some k })
              (s.update a fun v => { v with x25519Key := some k }) :=
            hr.update (fun _ => rfl) fun _ _ _ _ hv => { hv with x25519Key := rfl }
          exact hu.insertX25519 k
    · simp [hreg, ResultRel]
  | p2pAddrUpdate a p2pAddr =>
    simp only [apply, StakeTable.apply, hr.registered a]
    by_cases hreg : s.registered a = true
    · simp only [hreg, Bool.not_true, Bool.false_eq_true, ↓reduceIte, not_true_eq_false,
        ResultRel]
      refine hr.update (fun _ => rfl) ?_
      intro iv v _ _ hv
      exact { hv with p2pAddr := rfl }
    · simp [hreg, ResultRel]

/-- Two replays agree: the same fatal error, or agreeing states. -/
def AllRel : Except StakeTableError State → Except StakeTableError StakeTable.State → Prop
  | .error e, .error e' => e = e'
  | .ok is, .ok s => Rel is s
  | _, _ => False

theorem applyAll_rel {is : State} {s : StakeTable.State} (hr : Rel is s) (h : Inv s)
    (es : List Event) : AllRel (applyAll is es) (StakeTable.applyAll s es) := by
  induction es generalizing is s with
  | nil => exact hr
  | cons e es ih =>
    have hre := apply_rel hr h e
    have ho := StakeTable.apply_outcome h e
    simp only [applyAll, StakeTable.applyAll]
    cases ha : apply is e with
    | error err =>
      cases hb : StakeTable.apply s e with
      | error err' => rw [ha, hb] at hre; simp only [ResultRel] at hre; subst hre; rfl
      | ok r => rw [ha, hb] at hre; cases r <;> simp [ResultRel] at hre
    | ok r =>
      cases hb : StakeTable.apply s e with
      | error err' => rw [ha, hb] at hre; cases r <;> simp [ResultRel] at hre
      | ok r' =>
        rw [ha, hb] at hre
        rw [hb] at ho
        cases r <;> cases r' <;> simp only [ResultRel] at hre
        · simp only [ApplyResult.outcome] at ho
          exact ih hre (StakeTable.step_inv h ho.symm)
        · exact ih hre.2 h

/--
From the empty state, the indexed replay fails exactly when the specification's
does, and otherwise ends in a state that agrees with it.
-/
theorem applyAll_replay (es : List Event) :
    match applyAll State.empty es, StakeTable.replay StakeTable.State.empty es with
    | .ok is, some s => Rel is s
    | .error _, none => True
    | _, _ => False := by
  have hrel := applyAll_rel Rel.empty Inv.empty es
  have hrep := StakeTable.applyAll_replay Inv.empty es
  revert hrel hrep
  cases applyAll State.empty es <;> cases StakeTable.applyAll StakeTable.State.empty es <;>
    intro hrel hrep <;> simp only [AllRel] at hrel <;> simp only [Except.toOption] at hrep <;>
    rw [← hrep] <;> simp_all

end StakeTable.Indexed
