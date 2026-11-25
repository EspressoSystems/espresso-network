module

public import StakeTableSpec.Spec
public import StakeTableSpec.Proofs.Lists

/-!
# What the stake table is like

How the table changes with one more change (`StakeTable.latest_append` and the
lemmas after it), and from that: no delegation goes negative
(`StakeTable.delegated_nonneg`), stake is the sum of the delegations
(`StakeTable.stake_eq_sum`) and stays within 256 bits (`StakeTable.stake_bounded`),
members have distinct accounts and keys (`StakeTable.table_distinct`), no key is
admitted twice (`StakeTable.keys_admitted_once`), and nothing about a validator that
exited is taken again (`StakeTable.not_accepts_of_exited`).
-/

@[expose] public section

namespace StakeTable

/-! ## One more change -/

theorem latest_append {α : Type} (f : Change → Option α) (h : History) (c : Change) :
    latest f (h ++ [c]) = (f c).or (latest f h) := by
  unfold latest
  rw [List.filterMap_append]
  cases hc : f c <;> simp [hc, List.getLast?_append]

theorem delegated_append (h : History) (c : Change) (d a : Address) :
    delegated (h ++ [c]) d a = delegated h d a + c.flow d a := by
  simp [delegated]

theorem stake_append (h : History) (c : Change) (a : Address) :
    stake (h ++ [c]) a = stake h a + c.inflow a := by
  simp [stake]

theorem registrations_append (h : History) (c : Change) :
    registrations (h ++ [c]) = registrations h ++ registrations [c] := by
  simp [registrations, List.filterMap_append]

theorem mem_registrations_append {h : History} {c : Change} {a : Address}
    (ha : a ∈ registrations h) : a ∈ registrations (h ++ [c]) := by
  unfold registrations at *
  rw [List.filterMap_append]
  exact List.mem_append_left _ ha

theorem exited_append {h : History} {c : Change} {a : Address} :
    Exited (h ++ [c]) a ↔ Exited h a ∨ c = .exit a := by
  simp [Exited, eq_comm]

/-! ## Delegations and stake -/

/-- No delegation goes negative. -/
theorem delegated_nonneg {h : History} (hv : Valid h) (d a : Address) : 0 ≤ delegated h d a := by
  induction hv with
  | nil => simp [delegated]
  | @snoc h c _ hc ih =>
    rw [delegated_append]
    cases c with
    | delegate d' a' n => simp only [Change.flow]; split <;> omega
    | undelegate d' a' n =>
      simp only [Change.flow]
      split
      · rename_i heq
        obtain ⟨rfl, rfl⟩ := heq
        have := hc.2.2; omega
      · omega
    | _ => simp [Change.flow]; exact ih

theorem sum_map_add (ds : List Address) (f g : Address → Int) :
    (ds.map fun d => f d + g d).sum = (ds.map f).sum + (ds.map g).sum := by
  induction ds with
  | nil => simp
  | cons d ds ih => simp only [List.map_cons, List.sum_cons, ih]; omega

theorem sum_map_ite_eq {ds : List Address} (hn : ds.Nodup) (d₀ : Address) (x : Int) :
    (ds.map fun d => if d₀ = d then x else 0).sum = if d₀ ∈ ds then x else 0 := by
  induction ds with
  | nil => simp
  | cons d ds ih =>
    rw [List.nodup_cons] at hn
    simp only [List.map_cons, List.sum_cons, ih hn.2, List.mem_cons]
    by_cases h₁ : d₀ = d
    · subst h₁; simp [hn.1]
    · simp [h₁]

theorem sum_map_zero (ds : List Address) : (ds.map fun _ => (0 : Int)).sum = 0 := by
  induction ds with
  | nil => simp
  | cons d ds ih => simp [ih]

/-- A validator's stake is the sum of its delegators' delegations. -/
theorem stake_eq_sum (h : History) (a : Address) {ds : List Address} (hn : ds.Nodup)
    (hds : ∀ d ∈ delegatorsOf h a, d ∈ ds) :
    stake h a = (ds.map fun d => delegated h d a).sum := by
  induction h with
  | nil => simp [stake, delegated, sum_map_zero]
  | cons c h ih =>
    have hds' : ∀ d ∈ delegatorsOf h a, d ∈ ds := fun d hd => hds d (by
      unfold delegatorsOf at *; rw [List.filterMap_cons]; split <;> simp_all)
    have hsplit : (ds.map fun d => delegated (c :: h) d a).sum
        = (ds.map fun d => c.flow d a).sum + (ds.map fun d => delegated h d a).sum := by
      rw [← sum_map_add]
      simp [delegated]
    rw [hsplit, ← ih hds']
    simp only [stake, List.map_cons, List.sum_cons]
    congr 1
    cases c with
    | delegate d' a' n =>
      simp only [Change.inflow, Change.flow]
      by_cases ha : a' = a
      · subst ha
        have hd : d' ∈ ds := hds d' (by simp [delegatorsOf])
        simp only [and_true, sum_map_ite_eq hn, hd, ite_true]
      · simp [ha, sum_map_zero]
    | undelegate d' a' n =>
      simp only [Change.inflow, Change.flow]
      by_cases ha : a' = a
      · subst ha
        have hd : d' ∈ ds := hds d' (by simp [delegatorsOf])
        simp only [and_true, sum_map_ite_eq hn, hd, ite_true]
      · simp [ha, sum_map_zero]
    | _ => simp [Change.inflow, Change.flow, sum_map_zero]

theorem mem_dedup {x : Address} : ∀ {l : List Address}, x ∈ dedup l ↔ x ∈ l
  | [] => by simp [dedup]
  | y :: ys => by
    unfold dedup
    split
    · rename_i hy
      rw [mem_dedup]
      constructor
      · exact List.mem_cons_of_mem _
      · intro hx; rcases List.mem_cons.1 hx with rfl | hx
        · exact hy
        · exact hx
    · simp [mem_dedup]

theorem nodup_dedup : ∀ l : List Address, (dedup l).Nodup
  | [] => by simp [dedup]
  | y :: ys => by
    unfold dedup
    split
    · exact nodup_dedup ys
    · rename_i hy
      exact List.nodup_cons.2 ⟨by rw [mem_dedup]; exact hy, nodup_dedup ys⟩

theorem sum_nonneg {ds : List Address} {f : Address → Int} (hf : ∀ d, 0 ≤ f d) :
    0 ≤ (ds.map f).sum := by
  induction ds with
  | nil => simp
  | cons d ds ih => simp only [List.map_cons, List.sum_cons]; have := hf d; omega

/-- Stake is never negative, and fits in 256 bits. -/
theorem stake_bounded {h : History} (hv : Valid h) (a : Address) :
    0 ≤ stake h a ∧ stake h a < U256 := by
  refine ⟨?_, ?_⟩
  · rw [stake_eq_sum h a (nodup_dedup _) fun d hd => mem_dedup.2 hd]
    exact sum_nonneg fun d => delegated_nonneg hv d a
  · induction hv with
    | nil => simp [stake, U256]
    | @snoc h c _ hc ih =>
      rw [stake_append]
      cases c with
      | delegate d' a' n =>
        simp only [Change.inflow]
        split
        · rename_i heq; subst heq; exact hc.2.2
        · omega
      | undelegate d' a' n => simp only [Change.inflow]; split <;> omega
      | _ => simp [Change.inflow]; exact ih

/-! ## Accounts and keys -/

/-- A validator that exited registered. -/
theorem mem_registrations_of_exited {h : History} (hv : Valid h) {a : Address} (he : Exited h a) :
    a ∈ registrations h := by
  induction hv with
  | nil => simp [Exited] at he
  | @snoc h c _ hc ih =>
    rcases exited_append.1 he with he | rfl
    · exact mem_registrations_append (ih he)
    · exact mem_registrations_append (List.mem_filter.1 hc).1

theorem registrations_nodup {h : History} (hv : Valid h) : (registrations h).Nodup := by
  induction hv with
  | nil => simp [registrations]
  | @snoc h c _ hc ih =>
    rw [registrations_append]
    cases c with
    | register a b k x p m auth =>
      rw [show registrations [.register a b k x p m auth] = [a] from rfl, List.nodup_append]
      refine ⟨ih, by simp, ?_⟩
      intro x hx y hy hxy
      simp only [List.mem_singleton] at hy
      subst hy hxy
      exact hc.1 hx
    | _ => simpa [registrations] using ih

/--
A kind of key: what a change sets for a validator (`keyOf`), and the keys it admits
(`keys`). Each kind of key meets these conditions.
-/
structure KeyKind (κ : Type) where
  /-- The key a change gives `a`, if it sets one. -/
  keyOf : Address → Change → Option (Option κ)
  /-- The keys a change admits. -/
  keys : Change → List κ
  /-- A key a change gives is one it admits. -/
  admitted : ∀ c a k, keyOf a c = some (some k) → k ∈ keys c
  /-- A change sets keys for one validator only. -/
  single : ∀ c a b, keyOf a c ≠ none → keyOf b c ≠ none → a = b
  /-- A change taken admits no key admitted before. -/
  fresh : ∀ h c, Accepts h c → ∀ k ∈ keys c, k ∉ h.flatMap keys

/-- The key of kind `K` that `a` holds. -/
def KeyKind.held {κ : Type} (K : KeyKind κ) (h : History) (a : Address) : Option κ :=
  (latest (K.keyOf a) h).join

theorem mem_of_latest {α : Type} {f : Change → Option α} {h : History} {x : α}
    (hx : latest f h = some x) : ∃ c ∈ h, f c = some x :=
  List.mem_filterMap.1 (List.mem_of_getLast? hx)

theorem KeyKind.held_admitted {κ : Type} (K : KeyKind κ) {h : History} {a : Address} {k : κ}
    (hk : K.held h a = some k) : k ∈ h.flatMap K.keys := by
  unfold KeyKind.held at hk
  cases hl : latest (K.keyOf a) h with
  | none => rw [hl] at hk; simp at hk
  | some x =>
    rw [hl] at hk
    simp only [Option.join_some] at hk
    subst hk
    obtain ⟨c, hc, hck⟩ := mem_of_latest hl
    exact List.mem_flatMap.2 ⟨c, hc, K.admitted c a k hck⟩

/-- No two validators hold the same key of a kind. -/
theorem KeyKind.held_unique {κ : Type} (K : KeyKind κ) {h : History} (hv : Valid h)
    {a b : Address} {k : κ} (ha : K.held h a = some k) (hb : K.held h b = some k) : a = b := by
  induction hv generalizing a b with
  | nil => simp [KeyKind.held, latest] at ha
  | @snoc h c _ hc ih =>
    unfold KeyKind.held at ha hb ih
    rw [latest_append] at ha hb
    cases hca : K.keyOf a c with
    | none =>
      cases hcb : K.keyOf b c with
      | none =>
        rw [hca] at ha; rw [hcb] at hb
        simp only [Option.none_or] at ha hb
        exact ih ha hb
      | some y =>
        rw [hca] at ha; rw [hcb] at hb
        simp only [Option.none_or] at ha
        simp only [Option.some_or, Option.join_some] at hb
        subst hb
        exact absurd (K.held_admitted (h := h) ha) (K.fresh h c hc k (K.admitted c b k hcb))
    | some x =>
      cases hcb : K.keyOf b c with
      | none =>
        rw [hca] at ha; rw [hcb] at hb
        simp only [Option.none_or] at hb
        simp only [Option.some_or, Option.join_some] at ha
        subst ha
        exact absurd (K.held_admitted (h := h) hb) (K.fresh h c hc k (K.admitted c a k hca))
      | some y => exact K.single c a b (by simp [hca]) (by simp [hcb])

/-- No key of a kind is admitted twice, where no change admits a key twice by itself. -/
theorem KeyKind.admitted_once {κ : Type} (K : KeyKind κ) (hone : ∀ c, (K.keys c).Nodup)
    {h : History} (hv : Valid h) : (h.flatMap K.keys).Nodup := by
  induction hv with
  | nil => exact List.nodup_nil
  | @snoc h c _ hc ih =>
    rw [List.flatMap_append, List.flatMap_singleton, List.nodup_append]
    exact ⟨ih, hone c, fun k hk k' hk' he => K.fresh h c hc k' hk' (he ▸ hk)⟩

/-- Consensus keys, as a kind of key. -/
def blsKind : KeyKind BlsKey where
  keyOf a c := c.blsKeyOf a
  keys := Change.blsKeys
  admitted c a k h := by
    cases c <;> simp only [Change.blsKeyOf, Change.blsKeys] at h ⊢ <;> (try split at h) <;> simp_all
  single c a b ha hb := by
    cases c <;> simp only [Change.blsKeyOf] at ha hb <;> (try split at ha) <;> (try split at hb) <;>
      simp_all
  fresh h c hc k hk := by
    cases c with
    | register => exact hc.2.1 k (by simpa [Change.blsKeys] using hk)
    | rekey => simp only [Change.blsKeys, List.mem_singleton] at hk; subst hk; exact hc.2.1
    | _ => simp [Change.blsKeys] at hk

/-- Light client keys, as a kind of key. -/
def schnorrKind : KeyKind SchnorrKey where
  keyOf a c := c.schnorrKeyOf a
  keys := Change.schnorrKeys
  admitted c a k h := by
    cases c <;> simp only [Change.schnorrKeyOf, Change.schnorrKeys] at h ⊢ <;> (try split at h) <;> simp_all
  single c a b ha hb := by
    cases c <;> simp only [Change.schnorrKeyOf] at ha hb <;> (try split at ha) <;> (try split at hb) <;>
      simp_all
  fresh h c hc k hk := by
    cases c with
    | register => exact hc.2.2.1 k (by simpa [Change.schnorrKeys] using hk)
    | rekey => simp only [Change.schnorrKeys, List.mem_singleton] at hk; subst hk; exact hc.2.2
    | _ => simp [Change.schnorrKeys] at hk

/-- Network keys, as a kind of key. -/
def x25519Kind : KeyKind X25519Key where
  keyOf a c := c.x25519KeyOf a
  keys := Change.x25519Keys
  admitted c a k h := by
    cases c <;> simp only [Change.x25519KeyOf, Change.x25519Keys] at h ⊢ <;> (try split at h) <;> simp_all
  single c a b ha hb := by
    cases c <;> simp only [Change.x25519KeyOf] at ha hb <;> (try split at ha) <;> (try split at hb) <;>
      simp_all
  fresh h c hc k hk := by
    cases c with
    | register => exact hc.2.2.2.1 k (by simpa [Change.x25519Keys] using hk)
    | setX25519 => exact hc.2 k (by simpa [Change.x25519Keys] using hk)
    | _ => simp [Change.x25519Keys] at hk

theorem table_accounts (h : History) : (table h).map (·.account) = registered h := by
  unfold table
  induction registered h with
  | nil => rfl
  | cons a l ih => simp only [List.map_cons, ih]; rfl

/-- Members have distinct accounts and keys. -/
theorem table_distinct {h : History} (hv : Valid h) : Distinct (table h) := by
  have hacc : ((table h).map (·.account)).Nodup := by
    rw [table_accounts]; exact List.filter_sublist.nodup (registrations_nodup hv)
  have same : ∀ m ∈ table h, ∀ m' ∈ table h, m.account = m'.account → m = m' := by
    intro m hm m' hm' he
    simp only [table, List.mem_map] at hm hm'
    obtain ⟨a, _, rfl⟩ := hm
    obtain ⟨b, _, rfl⟩ := hm'
    simp only [member] at he
    rw [he]
  refine ⟨hacc, ?_, ?_, ?_⟩ <;> intro m hm m' hm' k hk hk' <;> apply same m hm m' hm' <;>
    simp only [table, List.mem_map] at hm hm' <;> obtain ⟨a, _, rfl⟩ := hm <;>
    obtain ⟨b, _, rfl⟩ := hm' <;> simp only [member] at hk hk' ⊢
  · exact blsKind.held_unique hv hk hk'
  · exact schnorrKind.held_unique hv hk hk'
  · exact x25519Kind.held_unique hv hk hk'

/-- Each key is admitted once. -/
theorem keys_admitted_once {h : History} (hv : Valid h) :
    (usedBls h).Nodup ∧ (usedSchnorr h).Nodup ∧ (usedX25519 h).Nodup := by
  have ho : ∀ {κ : Type} (k : Option κ), k.toList.Nodup := fun k => by cases k <;> simp
  refine ⟨blsKind.admitted_once (fun c => ?_) hv, schnorrKind.admitted_once (fun c => ?_) hv,
    x25519Kind.admitted_once (fun c => ?_) hv⟩ <;>
  cases c <;> simp [blsKind, schnorrKind, x25519Kind, Change.blsKeys, Change.schnorrKeys,
    Change.x25519Keys, ho]

/-- Nothing about a validator that exited is taken again. -/
theorem not_accepts_of_exited {h : History} (hv : Valid h) {a : Address} (he : Exited h a)
    {c : Change} (hc : c.validator = a) : ¬ Accepts h c := by
  have hreg := mem_registrations_of_exited hv he
  have hnot : ¬ Registered h a := fun hr => by
    have := (List.mem_filter.1 hr).2
    simp [he] at this
  cases c <;> simp only [Change.validator] at hc <;> subst hc <;> simp only [Accepts]
  · exact fun h' => h'.1 hreg
  all_goals first | exact hnot | exact fun h' => hnot h'.1

/-! ## A change touches its own validator only -/

section

variable {c : Change} {b : Address}

theorem blsKeyOf_ne (h : c.validator ≠ b) : c.blsKeyOf b = none := by
  cases c <;> simp_all [Change.validator, Change.blsKeyOf]

theorem schnorrKeyOf_ne (h : c.validator ≠ b) : c.schnorrKeyOf b = none := by
  cases c <;> simp_all [Change.validator, Change.schnorrKeyOf]

theorem x25519KeyOf_ne (h : c.validator ≠ b) : c.x25519KeyOf b = none := by
  cases c <;> simp_all [Change.validator, Change.x25519KeyOf]

theorem p2pAddrOf_ne (h : c.validator ≠ b) : c.p2pAddrOf b = none := by
  cases c <;> simp_all [Change.validator, Change.p2pAddrOf]

theorem commissionOf_ne (h : c.validator ≠ b) : c.commissionOf b = none := by
  cases c <;> simp_all [Change.validator, Change.commissionOf]

theorem authenticatedOf_ne (h : c.validator ≠ b) : c.authenticatedOf b = none := by
  cases c <;> simp_all [Change.validator, Change.authenticatedOf]

theorem inflow_ne (h : c.validator ≠ b) : c.inflow b = 0 := by
  cases c <;> simp_all [Change.validator, Change.inflow]

theorem flow_ne (h : c.validator ≠ b) (d : Address) : c.flow d b = 0 := by
  cases c <;> simp_all [Change.validator, Change.flow]

/-- A change leaves every other validator as it was. -/
theorem member_append_ne (hist : History) (h : c.validator ≠ b) :
    member (hist ++ [c]) b = member hist b := by
  simp [member, latest_append, blsKeyOf_ne h, schnorrKeyOf_ne h, x25519KeyOf_ne h,
    p2pAddrOf_ne h, commissionOf_ne h, authenticatedOf_ne h, stake_append, inflow_ne h]

end

/-- Neither a registration nor an exit: the change leaves the members as they were. -/
def Change.Keeps : Change → Prop
  | .register .. | .exit _ => False
  | _ => True

theorem registered_append_keeps {h : History} {c : Change} (hk : c.Keeps) :
    registered (h ++ [c]) = registered h := by
  have hr : registrations (h ++ [c]) = registrations h := by
    rw [registrations_append]; cases c <;> simp_all [Change.Keeps, registrations]
  unfold registered
  rw [hr]
  apply List.filter_congr
  intro a _
  have : ¬ c = .exit a := by cases c <;> simp_all [Change.Keeps]
  simp [exited_append, this]

theorem registered_append_register {h : History} (hv : Valid h) {a : Address}
    (ha : a ∉ registrations h) (b : Option BlsKey) (k : Option SchnorrKey) (x : Option X25519Key)
    (p : Option NetAddr) (m : Nat) (auth : Bool) :
    registered (h ++ [.register a b k x p m auth]) = registered h ++ [a] := by
  have hr : registrations (h ++ [.register a b k x p m auth]) = registrations h ++ [a] := by
    rw [registrations_append]; rfl
  have hne : ¬ Exited h a := fun he => ha (mem_registrations_of_exited hv he)
  unfold registered
  rw [hr, List.filter_append]
  congr 1
  · apply List.filter_congr; intro x _; simp [exited_append]
  · simp [exited_append, hne]

theorem registered_append_exit (h : History) (a : Address) :
    registered (h ++ [.exit a]) = (registered h).filter (· != a) := by
  have hr : registrations (h ++ [.exit a]) = registrations h := by
    rw [registrations_append]; simp [registrations]
  unfold registered
  rw [hr, List.filter_filter]
  apply List.filter_congr
  intro x _
  by_cases hx : x = a
  · subst hx; simp [exited_append]
  · have : ¬ a = x := fun h => hx h.symm
    simp [exited_append, hx, this]

/-- A validator that never registered has nothing delegated to it. -/
theorem delegated_of_not_registered {h : History} (hv : Valid h) {a : Address}
    (ha : a ∉ registrations h) (d : Address) : delegated h d a = 0 ∧ stake h a = 0 := by
  induction hv with
  | nil => simp [delegated, stake]
  | @snoc h c _ hc ih =>
    have ha' : a ∉ registrations h := fun h' => ha (mem_registrations_append h')
    obtain ⟨ih1, ih2⟩ := ih ha'
    rw [delegated_append, stake_append, ih1, ih2]
    have reg : ∀ {a'}, Registered h a' → a' ≠ a := fun hr he =>
      ha' (he ▸ (List.mem_filter.1 hr).1)
    cases c with
    | delegate d' a' n =>
      have := reg hc.1
      simp [Change.flow, Change.inflow, this]
    | undelegate d' a' n =>
      have := reg hc.1
      simp [Change.flow, Change.inflow, this]
    | _ => simp [Change.flow, Change.inflow]

/-- A validator registered once is registered or exited. -/
theorem mem_registrations_iff {h : History} (hv : Valid h) {a : Address} :
    a ∈ registrations h ↔ Registered h a ∨ Exited h a := by
  constructor
  · intro ha
    by_cases he : Exited h a
    · exact .inr he
    · exact .inl (List.mem_filter.2 ⟨ha, by simp [he]⟩)
  · rintro (hr | he)
    · exact (List.mem_filter.1 hr).1
    · exact mem_registrations_of_exited hv he

end StakeTable
