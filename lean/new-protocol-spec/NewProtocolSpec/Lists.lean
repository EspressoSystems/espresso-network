module

public import NewProtocolSpec.Rules

/-!
# A history as lists

What a history holds, the grounds it gives for a view, an epoch and a lock, and
what each step sends, as finite lists. Most lists are proved to hold exactly what
the specification's predicate over the history names, so a predicate that
quantifies over what was received or sent can be decided by scanning a list.
Some are proved in one direction only, which is all their users need: each
element satisfies the predicate (`cert2sHeld`, `proposalsIn`), or each element sent is in the
list (`timeoutVotesOf`, `proposalsOf`).

The machine acts on these, and the trace checker in `new-protocol-diff` checks
recorded traces with them.
-/

@[expose] public section

namespace NewProtocol.Lists

open NewProtocol History

/-! ## What a step carries -/

/-- The vote1s a step sends. -/
def vote1sOf (st : Step) : List Vote1 :=
  st.output.filterMap fun o => match o with
    | .send (.vote1 v) => some v
    | _ => none

/-- The vote2s a step sends. -/
def vote2sOf (st : Step) : List Vote2 :=
  st.output.filterMap fun o => match o with
    | .send (.vote2 v) => some v
    | _ => none

/-- The decides a step delivers. -/
def decidesOf (st : Step) : List (List Block × Cert1 × Cert2) :=
  st.output.filterMap fun o => match o with
    | .decided blocks c1 c2 => some (blocks, c1, c2)
    | _ => none

/-- The proposal a step received with this node's share, if it received one. -/
def proposalsIn (st : Step) : List (PubKey × Proposal × VidShare) :=
  match st.input with
  | .proposal sender p (some vid) => [(sender, p, vid)]
  | _ => []

theorem mem_vote1sOf {st : Step} {v : Vote1} :
    v ∈ vote1sOf st ↔ Output.send (.vote1 v) ∈ st.output := by
  unfold vote1sOf
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨o, ho, he⟩
    match o, he with
    | .send (.vote1 _), rfl => exact ho
  · intro h; exact ⟨_, h, rfl⟩

theorem mem_vote2sOf {st : Step} {v : Vote2} :
    v ∈ vote2sOf st ↔ Output.send (.vote2 v) ∈ st.output := by
  unfold vote2sOf
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨o, ho, he⟩
    match o, he with
    | .send (.vote2 _), rfl => exact ho
  · intro h; exact ⟨_, h, rfl⟩

theorem mem_decidesOf {st : Step} {blocks : List Block} {c1 : Cert1} {c2 : Cert2} :
    (blocks, c1, c2) ∈ decidesOf st ↔ Output.decided blocks c1 c2 ∈ st.output := by
  unfold decidesOf
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨o, ho, he⟩
    match o, he with
    | .decided _ _ _, rfl => exact ho
  · intro h; exact ⟨_, h, rfl⟩

theorem mem_proposalsIn {st : Step} {x : PubKey × Proposal × VidShare} (h : x ∈ proposalsIn st) :
    st.input = .proposal x.1 x.2.1 (some x.2.2) := by
  unfold proposalsIn at h
  split at h
  · next hin => simp at h; subst h; exact hin
  · simp at h

/-! ## What a history holds, as lists -/

variable (cfg : Config)

/-- The `Cert1`s a history holds. -/
def cert1sHeld (h : History) : List Cert1 :=
  cfg.anchorCert :: h.filterMap fun st => match st.input with
    | .certificate1 c => some c
    | .epochChange c _ _ => some c
    | _ => none

/-- The `Cert2`s a history holds. -/
def cert2sHeld (h : History) : List Cert2 :=
  h.filterMap fun st => match st.input with
    | .certificate2 c => some c
    | .epochChange _ c _ => some c
    | _ => none

/-- The proposals a history holds. -/
def proposalsHeld (h : History) : List Proposal :=
  cfg.anchorBlock :: h.filterMap fun st => match st.input with
    | .proposal _ p _ => some p
    | .epochChange _ _ p => some p
    | _ => none

variable {cfg}

theorem hasCert1_of_mem {h : History} {c : Cert1} (hc : c ∈ cert1sHeld cfg h) :
    h.HasCert1 cfg c := by
  rcases List.mem_cons.mp hc with rfl | hc
  · exact Or.inl rfl
  · obtain ⟨st, hst, he⟩ := List.mem_filterMap.mp hc
    revert he
    cases hin : st.input <;> intro he <;> simp at he
    · subst he; exact Or.inr (Or.inl ⟨st, hst, hin⟩)
    · subst he; exact Or.inr (Or.inr ⟨_, _, st, hst, hin⟩)

theorem hasCert2_of_mem {h : History} {c : Cert2} (hc : c ∈ cert2sHeld h) : h.HasCert2 c := by
  obtain ⟨st, hst, he⟩ := List.mem_filterMap.mp hc
  revert he
  cases hin : st.input <;> intro he <;> simp at he
  · subst he; exact Or.inl ⟨st, hst, hin⟩
  · subst he; exact Or.inr ⟨_, _, st, hst, hin⟩

theorem hasProposal_of_mem {h : History} {b : Block} (hb : b ∈ proposalsHeld cfg h) :
    h.HasProposal cfg b := by
  rcases List.mem_cons.mp hb with rfl | hb
  · exact Or.inl rfl
  · obtain ⟨st, hst, he⟩ := List.mem_filterMap.mp hb
    revert he
    cases hin : st.input <;> intro he <;> simp at he
    · subst he; exact Or.inr (Or.inr ⟨_, _, st, hst, hin⟩)
    · subst he; exact Or.inr (Or.inl ⟨_, _, st, hst, hin⟩)

/-! ## Deciding the pieces -/

instance (h : History) (i : Input) : Decidable (h.Received i) :=
  inferInstanceAs (Decidable (∃ st ∈ h, st.input = i))

instance (h : History) (v : ViewNumber) (pc : PayloadCommit) : Decidable (h.HasPayload cfg v pc) :=
  inferInstanceAs (Decidable (_ ∨ _))

instance (v : Vote1) (b : Block) : Decidable (Vote1For v b) := inferInstanceAs (Decidable (_ ∧ _))

instance (v : Vote1) (r : RevoteRequest) : Decidable (Vote1Again v r) := inferInstanceAs (Decidable (_ ∧ _))

instance (c : Cert1) (b : Block) : Decidable (Certifies c b) := inferInstanceAs (Decidable (_ ∧ _))

instance (c : Cert2) (b : Block) : Decidable (Commits c b) := inferInstanceAs (Decidable (_ ∧ _))

/-- The view clause of `ProposalWellFormed`, as a test. -/
def evidenceCovers (p : Proposal) : Bool :=
  match p.timeoutEvidence with
  | none => decide (p.parentCert.view + 1 = p.viewNumber)
  | some tc => decide (tc.view + 1 = p.viewNumber)

theorem evidenceCovers_iff (p : Proposal) :
    evidenceCovers p = true ↔ (p.timeoutEvidence = none ∧ p.parentCert.view + 1 = p.viewNumber)
      ∨ ∃ tc, p.timeoutEvidence = some tc ∧ tc.view + 1 = p.viewNumber := by
  unfold evidenceCovers
  cases p.timeoutEvidence <;> simp

instance (p : Proposal) : Decidable (ProposalWellFormed cfg p) :=
  decidable_of_iff
    (p.parentCert.view < p.viewNumber
      ∧ evidenceCovers p = true
      ∧ p.epoch = epochOf p.blockHeader.blockNumber cfg.epochHeight
      ∧ p.parentCert.data.blockNumber + 1 = p.blockHeader.blockNumber)
    ⟨fun ⟨a, b, c, d⟩ => ⟨a, (evidenceCovers_iff p).mp b, c, d⟩,
      fun ⟨a, b, c, d⟩ => ⟨a, (evidenceCovers_iff p).mpr b, c, d⟩⟩

instance decChainLinked : (l : List Block) → Decidable (ChainLinked l)
  | [] => isTrue trivial
  | [_] => isTrue trivial
  | _ :: b' :: rest =>
    have := decChainLinked (b' :: rest)
    inferInstanceAs (Decidable (_ ∧ _ ∧ ChainLinked (b' :: rest)))

instance (cfg : Config) (c1 : Cert1) (c2 : Cert2) (p : Proposal) :
    Decidable (EpochChangeWellFormed cfg c1 c2 p) :=
  decidable_of_iff
    (p.viewNumber ≤ c2.view ∧ c1.data.toVote2 = c2.data
      ∧ c1.data = ⟨blockHash p, p.epoch, p.blockHeader.blockNumber⟩ ∧ p.viewNumber = c1.view
      ∧ ProposalWellFormed cfg p ∧ IsLastBlock p.blockHeader.blockNumber cfg.epochHeight)
    ⟨fun ⟨a, b, c, d, e, f⟩ => ⟨a, b, c, d, e, f⟩, fun ⟨a, b, c, d, e, f⟩ => ⟨a, b, c, d, e, f⟩⟩

instance (cfg : Config) (p : Proposal) : Decidable (EntersEpoch cfg p) :=
  inferInstanceAs (Decidable (IsLastBlock _ _))

variable (cfg : Config) (h : History)

/-- Every block the history was told is valid is valid. -/
def ValidityTruthful : Prop :=
  ∀ v hash, h.Received (.blockValidated v hash) → ∀ b : Block, blockHash b = hash → BlockValid b

/-! ## The grounds, as lists -/

/-- The epoch changes a history took: received and well formed. -/
def epochChangesTaken : List (Cert1 × Cert2 × Proposal) :=
  h.filterMap fun st => match st.input with
    | .epochChange c1 c2 p => if EpochChangeWellFormed cfg c1 c2 p then some (c1, c2, p) else none
    | _ => none

/-- The timeout certificates received. -/
def timeoutCertsIn : List TimeoutCert :=
  h.filterMap fun st => match st.input with
    | .timeoutCertificate tc => some tc
    | _ => none

/-- The certificates the node could lock on (`History.Lockable`). -/
def lockables : List Cert1 :=
  let bs := proposalsHeld cfg h
  cfg.anchorCert
    :: ((cert1sHeld cfg h).filter fun c =>
          decide (∃ b ∈ bs, Certifies c b ∧ h.HasPayload cfg b.viewNumber b.payloadCommit))
    ++ (epochChangesTaken cfg h).map (·.1)

/-- The certificates the node could build on (`History.Buildable`). -/
def buildables : List Cert1 :=
  let bs := proposalsHeld cfg h
  cfg.anchorCert :: (cert1sHeld cfg h).filter fun c => decide (∃ b ∈ bs, Certifies c b)

/-- The latest epoch of the certificates the node could lock on. -/
def lockEpoch : EpochNumber :=
  ((lockables cfg h).map (·.data.epoch)).foldl max cfg.anchorCert.data.epoch

/-- The latest view of the certificates of that epoch the node could lock on. -/
def lockView : ViewNumber :=
  (((lockables cfg h).filter fun c => decide (c.data.epoch = lockEpoch cfg h)).map (·.view)).foldl
    max (0 : ViewNumber)

/-- The certificates the node is locked on (`History.LockedOn`): latest by epoch, then by view. -/
def lockedOn : List Cert1 :=
  (lockables cfg h).filter fun c => decide (c.data.epoch = lockEpoch cfg h ∧ c.view = lockView cfg h)

/-- The views the node has grounds to be in (`History.ViewGround`). -/
def viewGrounds : List ViewNumber :=
  ((cert1sHeld cfg h).map (·.view + 1))
    ++ ((timeoutCertsIn h).map (·.view + 1))
    ++ ((epochChangesTaken cfg h).map (·.2.1.view + 1))

/-- The view the node is in (`History.InView`). -/
def viewOf : ViewNumber := (viewGrounds cfg h).foldl max (cfg.anchorCert.view + 1)

/-- The epochs the node has grounds to be in (`History.EpochGround`). -/
def epochGrounds : List EpochNumber :=
  cfg.startEpoch
    :: ((epochChangesTaken cfg h).map (·.2.1.data.epoch + 1))
    ++ ((timeoutCertsIn h).map (·.data.epoch))
    ++ ((cert1sHeld cfg h).filterMap fun c =>
          if c.data.epoch = epochOf c.data.blockNumber cfg.epochHeight then some c.data.epoch
          else none)

/-- The epoch the node is in (`History.InEpoch`). -/
def epochOfHistory : EpochNumber := (epochGrounds cfg h).foldl max cfg.startEpoch

/-- The timeout votes a step sends. -/
def timeoutVotesOf (st : Step) : List TimeoutVote :=
  st.output.filterMap fun o => match o with
    | .send (.timeoutVote v) => some v
    | _ => none

/-- The proposals a step sends. -/
def proposalsOf (st : Step) : List Proposal :=
  st.output.filterMap fun o => match o with
    | .send (.proposal p) => some p
    | _ => none

/-! ## The lists hold exactly the grounds -/

variable {cfg h}

theorem mem_cert1sHeld {c : Cert1} : c ∈ cert1sHeld cfg h ↔ h.HasCert1 cfg c := by
  refine ⟨hasCert1_of_mem, ?_⟩
  rintro (rfl | ⟨st, hst, hin⟩ | ⟨c2, p, st, hst, hin⟩)
  · exact List.mem_cons_self
  all_goals exact List.mem_cons_of_mem _ (List.mem_filterMap.mpr ⟨st, hst, by simp [hin]⟩)

theorem mem_proposalsHeld {b : Block} : b ∈ proposalsHeld cfg h ↔ h.HasProposal cfg b := by
  refine ⟨hasProposal_of_mem, ?_⟩
  rintro (rfl | ⟨s, share, st, hst, hin⟩ | ⟨c1, c2, st, hst, hin⟩)
  · exact List.mem_cons_self
  all_goals exact List.mem_cons_of_mem _ (List.mem_filterMap.mpr ⟨st, hst, by simp [hin]⟩)

theorem mem_epochChangesTaken {x : Cert1 × Cert2 × Proposal} :
    x ∈ epochChangesTaken cfg h ↔ h.TookEpochChange cfg x.1 x.2.1 x.2.2 := by
  obtain ⟨c1, c2, p⟩ := x
  constructor
  · intro hm
    obtain ⟨st, hst, he⟩ := List.mem_filterMap.mp hm
    revert he
    cases hin : st.input <;> intro he <;> simp at he
    obtain ⟨hwf, rfl, rfl, rfl⟩ := he
    exact ⟨⟨st, hst, hin⟩, hwf⟩
  · rintro ⟨⟨st, hst, hin⟩, hwf⟩
    exact List.mem_filterMap.mpr ⟨st, hst, by simp [hin, hwf]⟩

theorem mem_timeoutCertsIn {tc : TimeoutCert} :
    tc ∈ timeoutCertsIn h ↔ h.Received (.timeoutCertificate tc) := by
  constructor
  · intro hm
    obtain ⟨st, hst, he⟩ := List.mem_filterMap.mp hm
    revert he
    cases hin : st.input <;> intro he <;> simp at he
    subst he; exact ⟨st, hst, hin⟩
  · rintro ⟨st, hst, hin⟩
    exact List.mem_filterMap.mpr ⟨st, hst, by simp [hin]⟩

theorem mem_lockables {c : Cert1} : c ∈ lockables cfg h ↔ h.Lockable cfg c := by
  simp only [lockables, List.mem_cons, List.mem_append, List.mem_filter, List.mem_map,
    decide_eq_true_eq, mem_cert1sHeld, mem_proposalsHeld, mem_epochChangesTaken, Lockable, or_assoc]
  constructor
  · rintro (rfl | ⟨hc, b, hb, hcert, hpay⟩ | ⟨⟨c1, c2, p⟩, htook, rfl⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hc, b, hb, hcert, hpay⟩)
    · exact Or.inr (Or.inr ⟨c2, p, htook⟩)
  · rintro (rfl | ⟨hc, b, hb, hcert, hpay⟩ | ⟨c2, p, htook⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨hc, b, hb, hcert, hpay⟩)
    · exact Or.inr (Or.inr ⟨(c, c2, p), htook, rfl⟩)

theorem mem_buildables {c : Cert1} : c ∈ buildables cfg h ↔ h.Buildable cfg c := by
  simp only [buildables, List.mem_cons, List.mem_filter, decide_eq_true_eq, mem_cert1sHeld,
    mem_proposalsHeld, Buildable]

theorem mem_viewGrounds {v : ViewNumber} : v ∈ viewGrounds cfg h ↔ h.ViewGround cfg v := by
  simp only [viewGrounds, List.mem_append, List.mem_map, mem_cert1sHeld,
    mem_timeoutCertsIn, mem_epochChangesTaken, ViewGround, or_assoc]
  constructor
  · rintro (⟨c, hc, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨⟨c1, c2, p⟩, htook, rfl⟩)
    · exact Or.inl ⟨c, hc, rfl⟩
    · exact Or.inr (Or.inl ⟨tc, htc, rfl⟩)
    · exact Or.inr (Or.inr ⟨c1, c2, p, htook, rfl⟩)
  · rintro (⟨c, hc, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, htook, rfl⟩)
    · exact Or.inl ⟨c, hc, rfl⟩
    · exact Or.inr (Or.inl ⟨tc, htc, rfl⟩)
    · exact Or.inr (Or.inr ⟨(c1, c2, p), htook, rfl⟩)

theorem mem_epochGrounds {e : EpochNumber} : e ∈ epochGrounds cfg h ↔ h.EpochGround cfg e := by
  simp only [epochGrounds, List.mem_cons, List.mem_append, List.mem_map, List.mem_filterMap,
    mem_cert1sHeld, mem_timeoutCertsIn, mem_epochChangesTaken, EpochGround,
    Option.ite_none_right_eq_some, Option.some.injEq, or_assoc]
  constructor
  · rintro (rfl | ⟨⟨c1, c2, p⟩, htook, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c, hc, hep, rfl⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨c1, c2, p, htook, rfl⟩)
    · exact Or.inr (Or.inr (Or.inl ⟨tc, htc, rfl⟩))
    · exact Or.inr (Or.inr (Or.inr ⟨c, hc, hep, rfl⟩))
  · rintro (rfl | ⟨c1, c2, p, htook, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c, hc, hep, rfl⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl ⟨(c1, c2, p), htook, rfl⟩)
    · exact Or.inr (Or.inr (Or.inl ⟨tc, htc, rfl⟩))
    · exact Or.inr (Or.inr (Or.inr ⟨c, hc, hep, rfl⟩))

/-! ## The largest element -/

section Fold

variable {α : Type} [Max α] (f : α → Nat) (hmax : ∀ a b : α, max a b = if f a ≤ f b then b else a)
include hmax

theorem foldl_max_bound :
    ∀ (l : List α) (d : α), f d ≤ f (l.foldl max d) ∧ ∀ x ∈ l, f x ≤ f (l.foldl max d)
  | [], _ => ⟨Nat.le_refl _, by simp⟩
  | a :: l, d => by
    obtain ⟨hd, hl⟩ := foldl_max_bound l (max d a)
    have hda : f d ≤ f (max d a) ∧ f a ≤ f (max d a) := by
      rw [hmax]; split <;> omega
    refine ⟨Nat.le_trans hda.1 hd, fun x hx => ?_⟩
    rcases List.mem_cons.mp hx with rfl | hx
    · exact Nat.le_trans hda.2 hd
    · exact hl x hx

theorem foldl_max_mem : ∀ (l : List α) (d : α), l.foldl max d = d ∨ l.foldl max d ∈ l
  | [], _ => Or.inl rfl
  | a :: l, d => by
    rcases foldl_max_mem l (max d a) with he | he
    · rw [List.foldl_cons, he, hmax]
      split
      · exact Or.inr List.mem_cons_self
      · exact Or.inl rfl
    · exact Or.inr (List.mem_cons_of_mem _ he)

end Fold

theorem viewNumber_max (a b : ViewNumber) : max a b = if a.toNat ≤ b.toNat then b else a := rfl

theorem epochNumber_max (a b : EpochNumber) : max a b = if a.toNat ≤ b.toNat then b else a := rfl

/-! ## The derived view, epoch and lock are the specification's -/

variable (cfg h)

theorem inView_viewOf : h.InView cfg (viewOf cfg h) := by
  have hmem : viewOf cfg h ∈ viewGrounds cfg h := by
    rcases foldl_max_mem _ viewNumber_max (viewGrounds cfg h) (cfg.anchorCert.view + 1) with he | he
    · rw [viewOf, he]; exact mem_viewGrounds.mpr (Or.inl ⟨cfg.anchorCert, Or.inl rfl, rfl⟩)
    · exact he
  refine ⟨mem_viewGrounds.mp hmem, fun v' hv' => ?_⟩
  exact (foldl_max_bound _ viewNumber_max _ _).2 v' (mem_viewGrounds.mpr hv')

theorem inEpoch_epochOfHistory : h.InEpoch cfg (epochOfHistory cfg h) := by
  have hmem : epochOfHistory cfg h ∈ epochGrounds cfg h := by
    rcases foldl_max_mem _ epochNumber_max (epochGrounds cfg h) cfg.startEpoch
      with he | he
    · rw [epochOfHistory, he]; exact List.mem_cons_self
    · exact he
  refine ⟨mem_epochGrounds.mp hmem, fun e' he' => ?_⟩
  exact (foldl_max_bound _ epochNumber_max _ _).2 e' (mem_epochGrounds.mpr he')

variable {cfg h}

theorem viewOf_of_inView {v : ViewNumber} (hv : h.InView cfg v) : viewOf cfg h = v :=
  ViewNumber.le_antisymm (hv.2 _ (inView_viewOf cfg h).1) ((inView_viewOf cfg h).2 _ hv.1)

theorem lockEpoch_bound {c : Cert1} (hc : c ∈ lockables cfg h) :
    c.data.epoch.toNat ≤ (lockEpoch cfg h).toNat :=
  (foldl_max_bound _ epochNumber_max _ _).2 c.data.epoch (List.mem_map.mpr ⟨c, hc, rfl⟩)

theorem lockView_bound {c : Cert1} (hc : c ∈ lockables cfg h) (he : c.data.epoch = lockEpoch cfg h) :
    c.view.toNat ≤ (lockView cfg h).toNat :=
  (foldl_max_bound _ viewNumber_max _ _).2 c.view
    (List.mem_map.mpr ⟨c, List.mem_filter.mpr ⟨hc, by simp [he]⟩, rfl⟩)

theorem lockedOn_of_mem {c : Cert1} (hc : c ∈ lockedOn cfg h) : h.LockedOn cfg c := by
  obtain ⟨hl, hv⟩ := List.mem_filter.mp hc
  simp only [decide_eq_true_eq] at hv
  obtain ⟨he, hv⟩ := hv
  refine ⟨mem_lockables.mp hl, fun c' hc' => ?_⟩
  have hc'l := mem_lockables.mpr hc'
  have hb := lockEpoch_bound hc'l
  rw [← he] at hb
  rcases Nat.lt_or_eq_of_le hb with hlt | heq
  · exact Or.inl hlt
  · have he' : c'.data.epoch = lockEpoch cfg h := by rw [← he]; exact EpochNumber.ext heq
    refine Or.inr ⟨he'.trans he.symm, ?_⟩
    show c'.view.toNat ≤ c.view.toNat
    rw [hv]; exact lockView_bound hc'l he'

theorem mem_timeoutVotesOf {st : Step} {v : TimeoutVote}
    (hm : Output.send (.timeoutVote v) ∈ st.output) : v ∈ timeoutVotesOf st :=
  List.mem_filterMap.mpr ⟨_, hm, rfl⟩

theorem mem_proposalsOf {st : Step} {p : Proposal}
    (hm : Output.send (.proposal p) ∈ st.output) : p ∈ proposalsOf st :=
  List.mem_filterMap.mpr ⟨_, hm, rfl⟩

/-! ## Locks over time -/

theorem mem_lockedOn {h : History} {c : Cert1} : c ∈ lockedOn cfg h ↔ h.LockedOn cfg c := by
  refine ⟨lockedOn_of_mem, fun ⟨hl, hmax⟩ => ?_⟩
  have hcl := mem_lockables.mpr hl
  -- The latest epoch is that of some lockable certificate, which `c` is no earlier than.
  have he : c.data.epoch = lockEpoch cfg h := by
    apply EpochNumber.ext
    apply Nat.le_antisymm (lockEpoch_bound hcl)
    rcases foldl_max_mem _ epochNumber_max ((lockables cfg h).map (·.data.epoch))
        cfg.anchorCert.data.epoch with hm | hm
    · show (lockEpoch cfg h).toNat ≤ c.data.epoch.toNat
      rw [lockEpoch, hm]
      rcases hmax cfg.anchorCert (Or.inl rfl) with h1 | ⟨h1, -⟩
      · exact Nat.le_of_lt h1
      · exact Nat.le_of_eq (congrArg EpochNumber.toNat h1)
    · obtain ⟨c', hc', he'⟩ := List.mem_map.mp hm
      show (lockEpoch cfg h).toNat ≤ c.data.epoch.toNat
      rw [lockEpoch, ← he']
      rcases hmax c' (mem_lockables.mp hc') with h1 | ⟨h1, -⟩
      · exact Nat.le_of_lt h1
      · exact Nat.le_of_eq (congrArg EpochNumber.toNat h1)
  refine List.mem_filter.mpr ⟨hcl, ?_⟩
  simp only [decide_eq_true_eq]
  refine ⟨he, ViewNumber.le_antisymm (lockView_bound hcl he) ?_⟩
  rcases foldl_max_mem _ viewNumber_max
      (((lockables cfg h).filter fun c => decide (c.data.epoch = lockEpoch cfg h)).map (·.view))
      (0 : ViewNumber) with hm | hm
  · show (lockView cfg h).toNat ≤ c.view.toNat
    rw [lockView, hm]; exact Nat.zero_le _
  · obtain ⟨c', hc', he'⟩ := List.mem_map.mp hm
    obtain ⟨hc'l, hc'e⟩ := List.mem_filter.mp hc'
    simp only [decide_eq_true_eq] at hc'e
    show (lockView cfg h).toNat ≤ c.view.toNat
    rw [lockView, ← he']
    rcases hmax c' (mem_lockables.mp hc'l) with h1 | ⟨-, h2⟩
    · exfalso
      have : c'.data.epoch.toNat < c.data.epoch.toNat := h1
      rw [hc'e, he] at this
      exact Nat.lt_irrefl _ this
    · exact h2

theorem lockable_mono {h1 h2 : History} (hr : ∀ i, h1.Received i → h2.Received i) {c : Cert1}
    (hl : h1.Lockable cfg c) : h2.Lockable cfg c := by
  have hc1 : ∀ c, h1.HasCert1 cfg c → h2.HasCert1 cfg c := by
    rintro c (rfl | h | ⟨c2, p, h⟩)
    · exact Or.inl rfl
    · exact Or.inr (Or.inl (hr _ h))
    · exact Or.inr (Or.inr ⟨c2, p, hr _ h⟩)
  rcases hl with rfl | ⟨hc, b, hb, hcert, hpay⟩ | ⟨c2, p, hr', hw⟩
  · exact Or.inl rfl
  · refine Or.inr (Or.inl ⟨hc1 _ hc, b, ?_, hcert, ?_⟩)
    · rcases hb with rfl | ⟨s, share, h⟩ | ⟨c1, c2, h⟩
      · exact Or.inl rfl
      · exact Or.inr (Or.inl ⟨s, share, hr _ h⟩)
      · exact Or.inr (Or.inr ⟨c1, c2, hr _ h⟩)
    · rcases hpay with h | h
      · exact Or.inl h
      · exact Or.inr (hr _ h)
  · exact Or.inr (Or.inr ⟨c2, p, hr _ hr', hw⟩)

/-- Every history is locked on something. -/
theorem exists_lockedOn (h : History) : ∃ l, h.LockedOn cfg l := by
  -- Some lockable certificate has the latest epoch.
  obtain ⟨c0, hc0, hc0e⟩ : ∃ c0 ∈ lockables cfg h, c0.data.epoch = lockEpoch cfg h := by
    rcases foldl_max_mem _ epochNumber_max ((lockables cfg h).map (·.data.epoch))
        cfg.anchorCert.data.epoch with hm | hm
    · exact ⟨cfg.anchorCert, List.mem_cons_self, by rw [lockEpoch, hm]⟩
    · obtain ⟨c', hc', he'⟩ := List.mem_map.mp hm
      exact ⟨c', hc', by rw [lockEpoch, ← he']⟩
  -- Among those, some has the latest view.
  have hmem0 : c0 ∈ (lockables cfg h).filter fun c => decide (c.data.epoch = lockEpoch cfg h) :=
    List.mem_filter.mpr ⟨hc0, by simp [hc0e]⟩
  rcases foldl_max_mem _ viewNumber_max
      (((lockables cfg h).filter fun c => decide (c.data.epoch = lockEpoch cfg h)).map (·.view))
      (0 : ViewNumber) with hm | hm
  · refine ⟨c0, mem_lockedOn.mp (List.mem_filter.mpr ⟨hc0, ?_⟩)⟩
    simp only [decide_eq_true_eq]
    refine ⟨hc0e, ViewNumber.le_antisymm ?_ ?_⟩
    · exact lockView_bound hc0 hc0e
    · show (lockView cfg h).toNat ≤ c0.view.toNat
      rw [lockView, hm]; exact Nat.zero_le _
  · obtain ⟨c, hc, hv⟩ := List.mem_map.mp hm
    obtain ⟨hcl, hce⟩ := List.mem_filter.mp hc
    simp only [decide_eq_true_eq] at hce
    exact ⟨c, mem_lockedOn.mp (List.mem_filter.mpr ⟨hcl, by simp only [decide_eq_true_eq]; exact ⟨hce, hv⟩⟩)⟩

/-- The certificate the node is locked on: the first of `lockedOn`. -/
def lockOf (c : Config) (h : History) : Cert1 := (lockedOn c h).headD c.anchorCert

theorem lockOf_lockedOn (h : History) : h.LockedOn cfg (lockOf cfg h) := by
  obtain ⟨l, hl⟩ := exists_lockedOn (cfg := cfg) h
  have hmem := mem_lockedOn.mpr hl
  unfold lockOf
  cases hc : lockedOn cfg h with
  | nil => rw [hc] at hmem; cases hmem
  | cons a rest =>
    exact mem_lockedOn.mp (by rw [hc]; exact List.mem_cons_self)

end NewProtocol.Lists
