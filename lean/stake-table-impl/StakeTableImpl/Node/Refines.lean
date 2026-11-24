module

public import StakeTableImpl.Node
public import StakeTableImpl.Indexed.Lists

/-!
# The node's stake table is the specification's

The node's state stands for a history of changes (`StakeTable.Represents`): its
validators are the members of the table, in order, with the same keys, stake and
delegations, and it records the same admitted keys and exits. An event the node
applies comes to a change the table takes (`StakeTable.changeOf`), and the new state
stands for the history with that change; an event the node skips is one the
specification passes over (`StakeTable.Passes`), and one it rejects comes to no change
the table would take (`StakeTable.step_refines`). So every replay the node
finishes ends in a state that stands for the history the events make
(`StakeTable.replay_refines`), whose table is the node's members
(`StakeTable.Represents.table`).
-/

@[expose] public section

namespace StakeTable

/-- The node's state `s` stands for history `h`. -/
structure Represents (s : State) (h : History) : Prop where
  /-- The validators are the registered ones, in the order they registered. -/
  order : s.validators.map (·.account) = StakeTable.registered h
  /-- Each is the member of the table. -/
  members : ∀ v ∈ s.validators, v.toMember = member h v.account
  /-- Each has the delegations of the history. -/
  delegations : ∀ v ∈ s.validators, ∀ d, (amountOf d v.delegators : Int) = delegated h d v.account
  /-- The same consensus keys are admitted. -/
  usedBls : ∀ k, k ∈ s.usedBls ↔ k ∈ StakeTable.usedBls h
  /-- The same light client keys are admitted. -/
  usedSchnorr : ∀ k, k ∈ s.usedSchnorr ↔ k ∈ StakeTable.usedSchnorr h
  /-- The same network keys are admitted. -/
  usedX25519 : ∀ k, k ∈ s.usedX25519 ↔ k ∈ StakeTable.usedX25519 h
  /-- The same validators exited. -/
  exits : ∀ a, a ∈ s.exits ↔ Exited h a

/-- What the node outcome of an event says of the history: see `StakeTable.step_refines`. -/
def Refines (h : History) (e : Event) : Outcome → Prop
  | .applied s' => ∃ c, changeOf h e = some c ∧ Accepts h c ∧ Represents s' (h ++ [c])
  | .skipped => Passes h e
  | .rejected => ∀ c, changeOf h e = some c → ¬ Accepts h c

/-! ## What a representation gives -/

namespace Represents

variable {s : State} {h : History}

theorem table (hr : Represents s h) : s.validators.map Validator.toMember = StakeTable.table h := by
  unfold StakeTable.table
  rw [← hr.order, List.map_map]
  exact List.map_congr_left fun v hv => hr.members v hv

theorem registered (hr : Represents s h) (a : Address) : s.registered a = true ↔ Registered h a := by
  unfold Registered
  rw [← hr.order, registered_iff]
  simp

theorem registered_before (hr : Represents s h) (hv : Valid h) (a : Address) :
    (a ∈ s.exits ∨ s.registered a = true) ↔ a ∈ registrations h := by
  rw [hr.exits, hr.registered, mem_registrations_iff hv, or_comm]

theorem account_ne (hr : Represents s h) {a : Address}
    (ha : a ∉ registrations h) {v : Validator} (hv' : v ∈ s.validators) : a ≠ v.account := by
  intro he
  apply ha
  have : v.account ∈ StakeTable.registered h := hr.order ▸ List.mem_map_of_mem hv'
  exact he ▸ (List.mem_filter.1 this).1

theorem stake_eq (hr : Represents s h) (hv : Valid h) {v : Validator} (hv' : v ∈ s.validators) :
    (v.stake : Int) = stake h v.account := by
  have := congrArg Member.stake (hr.members v hv')
  simp only [Validator.toMember, member] at this
  have := (stake_bounded hv v.account).1
  omega

end Represents

/-! ## Delegations as lists -/

theorem amountOf_credit (d d' : Address) (n : Nat) (ds : List (Address × Nat)) :
    amountOf d' (credit d n ds) = amountOf d' ds + if d = d' then n else 0 := by
  rw [amountOf_eq_lookup, amountOf_eq_lookup, lookup_credit]
  by_cases h : d = d'
  · subst h; simp [amountOf_eq_lookup]
  · simp [h]

theorem amountOf_debit (d d' : Address) (n : Nat) {ds : List (Address × Nat)}
    (hn : (ds.map (·.1)).Nodup) :
    amountOf d' (debit d n ds) = if d = d' then amountOf d ds - n else amountOf d' ds := by
  rw [amountOf_eq_lookup d' (debit d n ds), lookup_debit d d' n hn]
  by_cases h : d = d'
  · subst h; simp only [ite_true]; split <;> simp_all
  · simp only [h, ite_false, amountOf_eq_lookup]

theorem amountOf_pos {d : Address} {ds : List (Address × Nat)} (hp : ∀ e ∈ ds, 0 < e.2) :
    delegates d ds = true ↔ 0 < amountOf d ds := by
  induction ds with
  | nil => simp [delegates, amountOf]
  | cons e es ih =>
    simp only [delegates, amountOf, Bool.or_eq_true, decide_eq_true_eq]
    by_cases he : e.1 = d
    · simp [he, hp e (List.mem_cons_self ..)]
    · simp [he, ih fun x hx => hp x (List.mem_cons_of_mem _ hx)]

/-! ## Changing one validator, adding one, removing one -/

/-- The node changes the validator `a` that change `c`, which neither registers nor exits, is about. -/
theorem Represents.of_update {s : State} {h : History} {c : Change} {a : Address}
    {f : Validator → Validator} {s' : State} (hr : Represents s h) (hk : c.Keeps)
    (hc : c.validator = a) (hacc : ∀ v, (f v).account = v.account)
    (hm : ∀ v ∈ s.validators, v.account = a → (f v).toMember = member (h ++ [c]) a)
    (hd : ∀ v ∈ s.validators, v.account = a →
      ∀ d, (amountOf d (f v).delegators : Int) = delegated (h ++ [c]) d a)
    (hval : s'.validators = (s.update a f).validators) (hex : s'.exits = s.exits)
    (hb : ∀ k, k ∈ s'.usedBls ↔ k ∈ StakeTable.usedBls (h ++ [c]))
    (hs : ∀ k, k ∈ s'.usedSchnorr ↔ k ∈ StakeTable.usedSchnorr (h ++ [c]))
    (hx : ∀ k, k ∈ s'.usedX25519 ↔ k ∈ StakeTable.usedX25519 (h ++ [c])) :
    Represents s' (h ++ [c]) where
  order := by rw [hval, accounts_update hacc, hr.order, registered_append_keeps hk]
  members w hw := by
    rw [hval] at hw
    obtain ⟨v, hv, ⟨ha, hw'⟩ | ⟨ha, hw'⟩⟩ := mem_update hw <;> rw [hw']
    · rw [hacc, ha]; exact hm v hv ha
    · rw [hr.members v hv, member_append_ne h (hc ▸ Ne.symm ha)]
  delegations w hw d := by
    rw [hval] at hw
    obtain ⟨v, hv, ⟨ha, hw'⟩ | ⟨ha, hw'⟩⟩ := mem_update hw <;> rw [hw']
    · rw [hacc, ha]; exact hd v hv ha d
    · rw [hr.delegations v hv d, delegated_append, flow_ne (hc ▸ Ne.symm ha), Int.add_zero]
  usedBls := hb
  usedSchnorr := hs
  usedX25519 := hx
  exits b := by
    rw [hex, hr.exits, exited_append]
    have : ¬ c = .exit b := by cases c <;> simp_all [Change.Keeps]
    simp [this]

/-- The node adds the validator change `c` registers. -/
theorem Represents.of_admit {s : State} {h : History} (hr : Represents s h) (hv : Valid h)
    {a : Address} (ha : a ∉ registrations h) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr) (commission : Nat) (auth : Bool) :
    Represents (s.admit (.new a bls schnorr x25519 p2pAddr commission auth))
      (h ++ [.register a bls schnorr x25519 p2pAddr commission auth]) where
  order := by
    rw [registered_append_register hv ha]
    simp [State.admit, hr.order, Validator.new]
  members w hw := by
    simp only [State.admit, List.mem_append, List.mem_singleton] at hw
    rcases hw with hw | rfl
    · rw [hr.members w hw, member_append_ne h (by simpa [Change.validator] using hr.account_ne ha hw)]
    · have hst := (delegated_of_not_registered hv ha a).2
      simp [Validator.toMember, Validator.new, member, latest_append, Change.blsKeyOf,
        Change.schnorrKeyOf, Change.x25519KeyOf, Change.p2pAddrOf, Change.commissionOf,
        Change.authenticatedOf, stake_append, Change.inflow, hst]
  delegations w hw d := by
    simp only [State.admit, List.mem_append, List.mem_singleton] at hw
    rcases hw with hw | rfl
    · rw [hr.delegations w hw d, delegated_append,
        flow_ne (by simpa [Change.validator] using hr.account_ne ha hw), Int.add_zero]
    · simp only [Validator.new]
      rw [delegated_append, (delegated_of_not_registered hv ha d).1]
      simp [amountOf, Change.flow]
  usedBls k := by
    simp [State.admit, Validator.new, StakeTable.usedBls, Change.blsKeys, hr.usedBls k, or_comm]
  usedSchnorr k := by
    simp [State.admit, Validator.new, StakeTable.usedSchnorr, Change.schnorrKeys, hr.usedSchnorr k,
      or_comm]
  usedX25519 k := by
    simp [State.admit, Validator.new, StakeTable.usedX25519, Change.x25519Keys, hr.usedX25519 k,
      or_comm]
  exits b := by simp [State.admit, hr.exits, exited_append]

/-- The node removes the validator that exits. -/
theorem Represents.of_remove {s : State} {h : History} (hr : Represents s h) (a : Address) :
    Represents (s.remove a) (h ++ [.exit a]) where
  order := by
    rw [registered_append_exit, ← hr.order]
    simp only [State.remove, List.filter_map]
    rfl
  members w hw := by
    simp only [State.remove, List.mem_filter, bne_iff_ne, ne_eq] at hw
    rw [hr.members w hw.1, member_append_ne h (by simpa [Change.validator] using Ne.symm hw.2)]
  delegations w hw d := by
    simp only [State.remove, List.mem_filter, bne_iff_ne, ne_eq] at hw
    rw [hr.delegations w hw.1 d, delegated_append,
      flow_ne (by simpa [Change.validator] using Ne.symm hw.2), Int.add_zero]
  usedBls k := by simp [State.remove, StakeTable.usedBls, Change.blsKeys, hr.usedBls k]
  usedSchnorr k := by simp [State.remove, StakeTable.usedSchnorr, Change.schnorrKeys, hr.usedSchnorr k]
  usedX25519 k := by simp [State.remove, StakeTable.usedX25519, Change.x25519Keys, hr.usedX25519 k]
  exits b := by
    simp only [State.remove, List.mem_cons, hr.exits, exited_append]
    simp [eq_comm, or_comm]

/-! ## Each event -/

/-- What a change, neither registering nor exiting, does to its validator as a member. -/
def Member.after (m : Member) : Change → Member
  | .delegate _ _ n => { m with stake := m.stake + n }
  | .undelegate _ _ n => { m with stake := m.stake - n }
  | .rekey _ b k => { m with blsKey := some b, schnorrKey := some k, authenticated := true }
  | .setCommission _ c => { m with commission := c }
  | .setX25519 _ k => { m with x25519Key := k }
  | .setP2pAddr _ p => { m with p2pAddr := p }
  | _ => m

theorem member_append_self {h : History} (hv : Valid h) {c : Change} (hk : c.Keeps) :
    member (h ++ [c]) c.validator = (member h c.validator).after c := by
  have hs := (stake_bounded hv c.validator).1
  cases c <;> simp only [Change.Keeps] at hk <;>
    simp only [Change.validator, member, Member.after, latest_append, Change.blsKeyOf,
      Change.schnorrKeyOf, Change.x25519KeyOf, Change.p2pAddrOf, Change.commissionOf,
      Change.authenticatedOf, stake_append, Change.inflow, ite_true, Option.none_or,
      Option.some_or, Option.join_some, Option.getD_some, Member.mk.injEq, and_true, true_and] at hs ⊢
  all_goals first | omega | (constructor <;> omega) | simp

theorem State.blsTaken_iff {s : State} {h : History} (hr : Represents s h) (k? : Option BlsKey) :
    s.blsTaken k? = true ↔ ∃ k ∈ k?, k ∈ StakeTable.usedBls h := by
  cases k? <;> simp [State.blsTaken, hr.usedBls]

theorem State.schnorrTaken_iff {s : State} {h : History} (hr : Represents s h)
    (k? : Option SchnorrKey) : s.schnorrTaken k? = true ↔ ∃ k ∈ k?, k ∈ StakeTable.usedSchnorr h := by
  cases k? <;> simp [State.schnorrTaken, hr.usedSchnorr]

theorem State.x25519Taken_iff {s : State} {h : History} (hr : Represents s h)
    (k? : Option X25519Key) : s.x25519Taken k? = true ↔ ∃ k ∈ k?, k ∈ StakeTable.usedX25519 h := by
  cases k? <;> simp [State.x25519Taken, hr.usedX25519]

/--
A change about a registered validator that keeps the members: the node's update of
the validator is the change, when it changes the member as the change does and the
delegations as the change does.
-/
theorem Represents.of_update_change {s : State} {h : History} (hr : Represents s h) (hv : Valid h)
    {c : Change} (hk : c.Keeps) {f : Validator → Validator} (hacc : ∀ v, (f v).account = v.account)
    (hm : ∀ v, (f v).toMember = v.toMember.after c)
    (hd : ∀ v ∈ s.validators, v.account = c.validator →
      ∀ d, (amountOf d (f v).delegators : Int) = delegated h d c.validator + c.flow d c.validator)
    {s' : State} (hval : s'.validators = (s.update c.validator f).validators) (hex : s'.exits = s.exits)
    (hb : ∀ k, k ∈ s'.usedBls ↔ k ∈ StakeTable.usedBls (h ++ [c]))
    (hs : ∀ k, k ∈ s'.usedSchnorr ↔ k ∈ StakeTable.usedSchnorr (h ++ [c]))
    (hx : ∀ k, k ∈ s'.usedX25519 ↔ k ∈ StakeTable.usedX25519 (h ++ [c])) :
    Represents s' (h ++ [c]) :=
  hr.of_update hk rfl hacc
    (fun v hv' ha => by rw [hm, hr.members v hv', ha, member_append_self hv hk])
    (fun v hv' ha d => by rw [hd v hv' ha d, delegated_append]) hval hex hb hs hx

theorem keys_unchanged {s : State} {h : History} (hr : Represents s h) {c : Change}
    (hb : c.blsKeys = []) (hs : c.schnorrKeys = []) (hx : c.x25519Keys = []) :
    (∀ k, k ∈ s.usedBls ↔ k ∈ StakeTable.usedBls (h ++ [c]))
      ∧ (∀ k, k ∈ s.usedSchnorr ↔ k ∈ StakeTable.usedSchnorr (h ++ [c]))
      ∧ (∀ k, k ∈ s.usedX25519 ↔ k ∈ StakeTable.usedX25519 (h ++ [c])) := by
  refine ⟨fun k => ?_, fun k => ?_, fun k => ?_⟩
  · simp [StakeTable.usedBls, hb, hr.usedBls k]
  · simp [StakeTable.usedSchnorr, hs, hr.usedSchnorr k]
  · simp [StakeTable.usedX25519, hx, hr.usedX25519 k]

theorem RegistrationKeys.verified_keys {keys : RegistrationKeys} (h : keys.isVerified = true) :
    keys.bls.isSome ∧ keys.schnorr.isSome := by
  cases keys <;> simp_all [RegistrationKeys.isVerified, RegistrationKeys.bls, RegistrationKeys.schnorr]

theorem any_mem_iff {κ : Type} [DecidableEq κ] (o : Option κ) (l : List κ) :
    (o.any fun k => decide (k ∈ l)) = true ↔ ∃ k ∈ o, k ∈ l := by
  cases o <;> simp

section

variable {s : State} {h : History} (hr : Represents s h) (hv : Valid h)
include hr hv

theorem register_refines (a : Address) (bls : Option BlsKey) (schnorr : Option SchnorrKey)
    (commission : Nat) :
    Refines h (.register a bls schnorr commission) (step s (.register a bls schnorr commission)) := by
  have hb := hr.registered_before hv a
  simp only [step]
  split
  · rename_i hcond
    intro c hc hA
    cases Option.some.inj hc
    obtain ⟨hreg, hbls, -, -, -⟩ := hA
    rcases hcond with h1 | h1 | h1
    · exact hreg (hb.1 (.inl h1))
    · exact hreg (hb.1 (.inr h1))
    · obtain ⟨k, hk, hku⟩ := (State.blsTaken_iff hr bls).1 h1; exact hbls k hk hku
  · rename_i hcond
    split
    · rename_i hsch
      obtain ⟨k, hk, hku⟩ := (State.schnorrTaken_iff hr schnorr).1 hsch
      cases hk
      refine ⟨hku, fun ha => hcond ?_, fun k hk hku => hcond ?_, by simp, by simp, by simp⟩
      · rcases hb.2 ha with h1 | h1
        · exact .inl h1
        · exact .inr (.inl h1)
      · exact .inr (.inr ((State.blsTaken_iff hr bls).2 ⟨k, hk, hku⟩))
    · rename_i hsch
      refine ⟨_, rfl, ⟨fun ha => hcond ?_, fun k hk hku => hcond ?_, fun k hk hku => hsch ?_,
        by simp, by simp⟩, hr.of_admit hv ?_ _ _ _ _ _ _⟩
      · rcases hb.2 ha with h1 | h1
        · exact .inl h1
        · exact .inr (.inl h1)
      · exact .inr (.inr ((State.blsTaken_iff hr bls).2 ⟨k, hk, hku⟩))
      · exact (State.schnorrTaken_iff hr schnorr).2 ⟨k, hk, hku⟩
      · intro ha
        rcases hb.2 ha with h1 | h1
        · exact hcond (.inl h1)
        · exact hcond (.inr (.inl h1))

theorem registerNew_refines (e : Event) (a : Address) (keys : RegistrationKeys) (commission : Nat)
    (x25519 : Option X25519Key) (p2pAddr : Option NetAddr)
    (hn : changeOf h e = some (changeOf.registration h a keys commission x25519 p2pAddr)) :
    Refines h e (s.registerNew a keys commission x25519 p2pAddr) := by
  have hb := hr.registered_before hv a
  have hst : s.schnorrTaken keys.schnorr = keys.schnorr.any (· ∈ StakeTable.usedSchnorr h) := by
    apply Bool.eq_iff_iff.2
    rw [State.schnorrTaken_iff hr, any_mem_iff]
  unfold State.registerNew
  split
  · rename_i hcond
    intro c hc hA
    rw [hn] at hc
    cases Option.some.inj hc
    have key : ∀ sch auth, ¬ Accepts h (.register a keys.bls sch x25519 p2pAddr commission auth) := by
      rintro sch auth ⟨hreg, hbls, -, hx, -⟩
      rcases hcond with h1 | h1 | h1 | h1
      · exact hreg (hb.1 (.inl h1))
      · exact hreg (hb.1 (.inr h1))
      · obtain ⟨k, hk, hku⟩ := (State.blsTaken_iff hr _).1 h1; exact hbls k hk hku
      · obtain ⟨k, hk, hku⟩ := (State.x25519Taken_iff hr _).1 h1; exact hx k hk hku
    unfold changeOf.registration at hA
    split at hA
    · exact key _ _ hA
    · exact key _ _ hA
  · rename_i hcond
    have hreg : a ∉ registrations h := fun ha => by
      rcases hb.2 ha with h1 | h1
      · exact hcond (.inl h1)
      · exact hcond (.inr (.inl h1))
    have hbls : ∀ k ∈ keys.bls, k ∉ StakeTable.usedBls h := fun k hk hku =>
      hcond (.inr (.inr (.inl ((State.blsTaken_iff hr _).2 ⟨k, hk, hku⟩))))
    have hx : ∀ k ∈ x25519, k ∉ StakeTable.usedX25519 h := fun k hk hku =>
      hcond (.inr (.inr (.inr ((State.x25519Taken_iff hr _).2 ⟨k, hk, hku⟩))))
    split
    · rename_i hsch
      have hr' : changeOf.registration h a keys commission x25519 p2pAddr
          = .register a keys.bls none x25519 p2pAddr commission false := by
        unfold changeOf.registration; rw [← hst, hsch]; rfl
      exact ⟨_, hn.trans (congrArg some hr'), ⟨hreg, hbls, by simp, hx, by simp⟩,
        hr.of_admit hv hreg _ _ _ _ _ _⟩
    · rename_i hsch
      have hr' : changeOf.registration h a keys commission x25519 p2pAddr
          = .register a keys.bls keys.schnorr x25519 p2pAddr commission keys.isVerified := by
        unfold changeOf.registration; rw [← hst]; simp [hsch]
      have hs : ∀ k ∈ keys.schnorr, k ∉ StakeTable.usedSchnorr h := fun k hk hku =>
        hsch ((State.schnorrTaken_iff hr _).2 ⟨k, hk, hku⟩)
      exact ⟨_, hn.trans (congrArg some hr'),
        ⟨hreg, hbls, hs, hx, RegistrationKeys.verified_keys⟩, hr.of_admit hv hreg _ _ _ _ _ _⟩

theorem rekey_refines (e : Event) (a : Address) (b : BlsKey) (k : SchnorrKey)
    (hn : changeOf h e = some (.rekey a b k))
    (hp : k ∈ StakeTable.usedSchnorr h ∧ Registered h a ∧ b ∉ StakeTable.usedBls h → Passes h e) :
    Refines h e (s.rekey a b k) := by
  unfold State.rekey
  split
  · rename_i hcond
    intro c hc hA
    rw [hn] at hc
    cases Option.some.inj hc
    obtain ⟨hreg, hb, -⟩ := hA
    rcases hcond with h1 | h1
    · exact h1 ((hr.registered a).2 hreg)
    · exact hb ((hr.usedBls b).1 h1)
  · rename_i hcond
    split
    · rename_i hk
      exact hp ⟨(hr.usedSchnorr k).1 hk,
        (hr.registered a).1 (by simpa using fun h' => hcond (.inl h')),
        fun h' => hcond (.inr ((hr.usedBls b).2 h'))⟩
    · rename_i hk
      refine ⟨_, hn, ⟨(hr.registered a).1 (by simpa using fun h' => hcond (.inl h')),
        fun h' => hcond (.inr ((hr.usedBls b).2 h')), fun h' => hk ((hr.usedSchnorr k).2 h')⟩,
        hr.of_update_change hv (c := .rekey a b k)
        (f := fun v => { v with blsKey := some b, schnorrKey := some k, authenticated := true })
        trivial (fun _ => rfl) (fun _ => rfl) ?_ rfl rfl ?_ ?_ ?_⟩
      · intro v _ _ d; simp [Change.flow, Change.validator, hr.delegations v ‹_› d, *]
      · intro k'; simp [StakeTable.usedBls, Change.blsKeys, hr.usedBls k', or_comm]
      · intro k'; simp [StakeTable.usedSchnorr, Change.schnorrKeys, hr.usedSchnorr k', or_comm]
      · intro k'; simp [State.update, StakeTable.usedX25519, Change.x25519Keys, hr.usedX25519 k']

theorem update_refines (e : Event) (c : Change) (hk : c.Keeps) (hn : changeOf h e = some c)
    (hcb : c.blsKeys = []) (hcs : c.schnorrKeys = []) (hcx : c.x25519Keys = [])
    (hacc : Accepts h c) (f : Validator → Validator) (hf : ∀ v, (f v).account = v.account)
    (hm : ∀ v, (f v).toMember = v.toMember.after c)
    (hd : ∀ v ∈ s.validators, v.account = c.validator →
      ∀ d, (amountOf d (f v).delegators : Int) = delegated h d c.validator + c.flow d c.validator) :
    Refines h e (.applied (s.update c.validator f)) := by
  obtain ⟨hb, hs, hx⟩ := keys_unchanged hr hcb hcs hcx
  exact ⟨c, hn, hacc, hr.of_update_change hv hk hf hm hd rfl rfl
    (fun k => by simpa [State.update] using hb k) (fun k => by simpa [State.update] using hs k)
    (fun k => by simpa [State.update] using hx k)⟩

end

/-- The outcome of every event the node reads agrees with the specification. -/
theorem step_refines {s : State} {h : History} (hr : Represents s h) (hv : Valid h) (hi : Inv s)
    (e : Event) : Refines h e (step s e) := by
  cases e with
  | register a bls schnorr c => exact register_refines hr hv a bls schnorr c
  | registerV2 a keys c => exact registerNew_refines hr hv _ a keys c none none rfl
  | registerV3 a keys c x p => exact registerNew_refines hr hv _ a keys c x p rfl
  | deregister a =>
    simp only [step]
    split
    · rename_i hreg; exact ⟨_, rfl, (hr.registered a).1 hreg, hr.of_remove a⟩
    · rename_i hreg
      intro c hc hA
      cases Option.some.inj hc
      exact hreg ((hr.registered a).2 hA)
  | delegate d a n =>
    simp only [step]
    split
    · rename_i hf
      intro c hc hA
      cases Option.some.inj hc
      have := find?_none hf
      rw [(hr.registered a).2 hA.1] at this
      exact absurd this (by decide)
    · rename_i v hf
      obtain ⟨hv', hva⟩ := find?_some hf
      have hst := hr.stake_eq hv hv'
      rw [hva] at hst
      split
      · rename_i hcond
        intro c hc hA
        cases Option.some.inj hc
        obtain ⟨-, hpos, hlt⟩ := hA
        simp only [U256] at hlt hcond
        omega
      · rename_i hcond
        simp only [U256] at hcond
        refine update_refines hr hv _ (.delegate d a n) trivial rfl rfl rfl rfl
          ⟨(hr.registered a).1 (registered_of_find? hf), by omega, by simp only [U256]; omega⟩
          _ (fun _ => rfl) (fun _ => rfl) ?_
        intro w hw hwa d'
        simp only [Change.validator] at hwa ⊢
        rw [amountOf_credit, Int.natCast_add, hr.delegations w hw d', hwa]
        simp only [Change.flow]
        by_cases hd : d = d' <;> simp [hd]
  | undelegate d a n =>
    simp only [step]
    split
    · rename_i hf
      intro c hc hA
      cases Option.some.inj hc
      have := find?_none hf
      rw [(hr.registered a).2 hA.1] at this
      exact absurd this (by decide)
    · rename_i v hf
      obtain ⟨hv', hva⟩ := find?_some hf
      have hsound := hi.sound v hv'
      have hdel := hr.delegations v hv' d
      rw [hva] at hdel
      have hle := amountOf_le_total d v.delegators
      rw [← hsound.stakeTotal] at hle
      have hpos := amountOf_pos (d := d) hsound.delegationsPositive
      split
      · rename_i hcond
        intro c hc hA
        cases Option.some.inj hc
        obtain ⟨-, hp, hn⟩ := hA
        rcases hcond with h1 | h1 | h1
        · omega
        · have : amountOf d v.delegators = 0 := by
            rcases Nat.eq_zero_or_pos (amountOf d v.delegators) with h0 | h0
            · exact h0
            · rw [hpos.2 h0] at h1; exact absurd h1 (by decide)
          omega
        · omega
      · rename_i hcond
        have hpos' : 0 < amountOf d v.delegators := hpos.1 (by
          cases hdl : delegates d v.delegators
          · exact absurd (.inr (.inl hdl)) hcond
          · rfl)
        have hn' : n ≤ amountOf d v.delegators := by
          by_cases h' : amountOf d v.delegators < n
          · exact absurd (.inr (.inr h')) hcond
          · omega
        refine update_refines hr hv _ (.undelegate d a n) trivial rfl rfl rfl rfl
          ⟨(hr.registered a).1 (registered_of_find? hf), by omega, by omega⟩ _ (fun _ => rfl)
          (fun _ => rfl) ?_
        intro w hw hwa d'
        simp only [Change.validator] at hwa ⊢
        have hwv : w = v := eq_of_find? hi.accountsNodup hf hw hwa
        subst hwv
        rw [amountOf_debit _ _ _ hsound.delegatorsNodup]
        by_cases hd : d = d'
        · subst hd
          simp only [ite_true, Change.flow, and_self]
          omega
        · have := hr.delegations w hw d'
          rw [hwa] at this
          simp [hd, Change.flow, this]
  | keyUpdate a bls schnorr =>
    cases bls with
    | none => trivial
    | some b =>
      cases schnorr with
      | none => trivial
      | some k => exact rekey_refines hr hv _ a b k rfl id
  | keyUpdateV2 a keys =>
    cases keys with
    | verified b k => exact rekey_refines hr hv _ a b k rfl id
    | failed => intro c hc; simp [changeOf] at hc
    | _ => trivial
  | commissionUpdate a c =>
    simp only [step]
    split
    · rename_i hcond
      intro c' hc hA
      cases Option.some.inj hc
      obtain ⟨hreg, hle⟩ := hA
      rcases hcond with h1 | h1
      · omega
      · exact h1 ((hr.registered a).2 hreg)
    · rename_i hcond
      exact update_refines hr hv _ (.setCommission a c) trivial rfl rfl rfl rfl
        ⟨(hr.registered a).1 (by simpa using fun h' => hcond (.inr h')), by omega⟩ _ (fun _ => rfl)
        (fun _ => rfl) (fun w hw hwa d => by simp [Change.flow, hr.delegations w hw d, ← hwa])
  | x25519Update a k =>
    cases k with
    | none =>
      simp only [step]
      split
      · rename_i hreg
        exact update_refines hr hv _ (.setX25519 a none) trivial rfl rfl rfl rfl
          ⟨(hr.registered a).1 hreg, by simp⟩ _ (fun _ => rfl) (fun _ => rfl)
          (fun w hw hwa d => by simp [Change.flow, hr.delegations w hw d, ← hwa])
      · rename_i hreg
        intro c hc hA
        cases Option.some.inj hc
        exact hreg ((hr.registered a).2 hA.1)
    | some k =>
      simp only [step]
      split
      · rename_i hcond
        intro c hc hA
        cases Option.some.inj hc
        obtain ⟨hreg, hk⟩ := hA
        rcases hcond with h1 | h1
        · exact h1 ((hr.registered a).2 hreg)
        · exact hk k rfl ((hr.usedX25519 k).1 h1)
      · rename_i hcond
        refine ⟨_, rfl, ⟨(hr.registered a).1 (by simpa using fun h' => hcond (.inl h')),
          fun k' hk' hu => by cases hk'; exact hcond (.inr ((hr.usedX25519 _).2 hu))⟩,
          hr.of_update_change hv (c := .setX25519 a (some k))
            (f := fun v => { v with x25519Key := some k }) trivial (fun _ => rfl) (fun _ => rfl)
            (fun w hw hwa d => by simp [Change.flow, hr.delegations w hw d, ← hwa]) rfl rfl
            ?_ ?_ ?_⟩
        · intro k'; simp [State.update, StakeTable.usedBls, Change.blsKeys, hr.usedBls k']
        · intro k'; simp [State.update, StakeTable.usedSchnorr, Change.schnorrKeys, hr.usedSchnorr k']
        · intro k'; simp [StakeTable.usedX25519, Change.x25519Keys, hr.usedX25519 k', or_comm]
  | p2pAddrUpdate a p =>
    simp only [step]
    split
    · rename_i hreg
      exact update_refines hr hv _ (.setP2pAddr a p) trivial rfl rfl rfl rfl
        ((hr.registered a).1 hreg) _ (fun _ => rfl) (fun _ => rfl)
        (fun w hw hwa d => by simp [Change.flow, hr.delegations w hw d, ← hwa])
    · rename_i hreg
      intro c hc hA
      cases Option.some.inj hc
      exact hreg ((hr.registered a).2 hA)

/-- The empty state stands for the empty history. -/
theorem Represents.empty : Represents State.empty [] where
  order := by simp [State.empty, StakeTable.registered, registrations]
  members := by simp [State.empty]
  delegations := by simp [State.empty]
  usedBls := by simp [State.empty, StakeTable.usedBls]
  usedSchnorr := by simp [State.empty, StakeTable.usedSchnorr]
  usedX25519 := by simp [State.empty, StakeTable.usedX25519]
  exits := by simp [State.empty, Exited]

/--
Every replay the node finishes from a state that stands for the history events `es₀`
make ends in a state that stands for the history they make followed by the replayed
events.
-/
theorem replay_reads_from {s s' : State} {h : History} {es₀ es : List Event}
    (hr : Represents s h) (hd : Reads es₀ h) (hi : Inv s) (he : replay s es = some s') :
    ∃ h', Reads (es₀ ++ es) h' ∧ Represents s' h' := by
  induction es generalizing s h es₀ with
  | nil =>
    simp only [replay, Option.some.injEq] at he
    exact ⟨h, by simpa using hd, he ▸ hr⟩
  | cons e es ih =>
    have href := step_refines hr (reads_valid hd) hi e
    have happ : es₀ ++ e :: es = (es₀ ++ [e]) ++ es := by simp
    rw [happ]
    simp only [replay] at he
    split at he
    · rename_i s'' hs
      rw [hs] at href
      obtain ⟨c, hc, ha, hr'⟩ := href
      exact ih hr' (.take hd hc ha) (step_inv hi hs) he
    · rename_i hs
      rw [hs] at href
      exact ih hr (.drop hd href) hi he
    · cases he

/-- Every replay the node finishes ends in a state that stands for the history the events make. -/
theorem replay_refines {s : State} {es : List Event} (he : replay State.empty es = some s) :
    ∃ h, Reads es h ∧ Represents s h := by
  simpa using replay_reads_from Represents.empty Reads.nil Inv.empty he

end StakeTable
