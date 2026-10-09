module

public import NewProtocolSpec.Proofs.Votes
public import NewProtocolSpec.Proofs.Inputs
public import NewProtocolSpec.Lists
public import NewProtocolSpec.Timing

/-!
# Liveness bookkeeping

What only grows along a trace (what was received, sent, held, the view), where a
node's view was entered, and cutting a trace at a time. Nothing here is about
the protocol's progress; that is `NewProtocolSpec.Proofs.Liveness.View` and
`NewProtocolSpec.Proofs.Liveness.Epochs`.
-/

@[expose] public section

namespace NewProtocol
namespace Liveness

open History Lists

variable {cfg : Config}

/-! ## Least witnesses -/

/-- A property of numbers that holds somewhere holds at a least number. -/
theorem exists_least {P : Nat → Prop} : ∀ b, (∃ n, n ≤ b ∧ P n) →
    ∃ n, P n ∧ ∀ m, m < n → ¬ P m
  | 0, ⟨n, hn, hp⟩ => ⟨0, by rwa [Nat.le_zero.mp hn] at hp, fun _ hm => absurd hm (Nat.not_lt_zero _)⟩
  | b + 1, ⟨n, hn, hp⟩ => by
    by_cases hb : ∃ n, n ≤ b ∧ P n
    · exact exists_least b hb
    · have hnb : n = b + 1 := by
        by_cases h : n ≤ b
        · exact absurd ⟨n, h, hp⟩ hb
        · omega
      refine ⟨n, hp, fun m hm hpm => hb ⟨m, by omega, hpm⟩⟩

/-- The least number with a property, given that one has it. -/
noncomputable def least (P : Nat → Prop) (h : ∃ n, P n) : Nat :=
  Classical.choose (exists_least (P := P) h.choose ⟨h.choose, Nat.le_refl _, h.choose_spec⟩)

theorem least_spec {P : Nat → Prop} (h : ∃ n, P n) : P (least P h) :=
  (Classical.choose_spec (exists_least (P := P) h.choose ⟨h.choose, Nat.le_refl _, h.choose_spec⟩)).1

theorem least_min {P : Nat → Prop} (h : ∃ n, P n) {m : Nat} (hm : m < least P h) : ¬ P m :=
  (Classical.choose_spec (exists_least (P := P) h.choose ⟨h.choose, Nat.le_refl _, h.choose_spec⟩)).2 m hm

theorem least_le {P : Nat → Prop} (h : ∃ n, P n) {m : Nat} (hm : P m) : least P h ≤ m :=
  Nat.le_of_not_lt fun hlt => least_min h hlt hm

/-! ## What grows along a history -/

section Grows

variable {h1 h2 : History} (hr : ∀ i, h1.Received i → h2.Received i)
include hr

theorem hasCert1_grows {c : Cert1} (hc : h1.HasCert1 cfg c) : h2.HasCert1 cfg c := by
  rcases hc with rfl | h | ⟨c2, p, h⟩
  · exact Or.inl rfl
  · exact Or.inr (Or.inl (hr _ h))
  · exact Or.inr (Or.inr ⟨c2, p, hr _ h⟩)

theorem hasCert2_grows {c : Cert2} (hc : h1.HasCert2 c) : h2.HasCert2 c := by
  rcases hc with h | ⟨c1, p, h⟩
  · exact Or.inl (hr _ h)
  · exact Or.inr ⟨c1, p, hr _ h⟩

theorem hasProposal_grows {b : Block} (hb : h1.HasProposal cfg b) : h2.HasProposal cfg b := by
  rcases hb with rfl | ⟨s, share, h⟩ | ⟨c1, c2, h⟩
  · exact Or.inl rfl
  · exact Or.inr (Or.inl ⟨s, share, hr _ h⟩)
  · exact Or.inr (Or.inr ⟨c1, c2, hr _ h⟩)

theorem hasPayload_grows {v : ViewNumber} {pc : PayloadCommit} (hp : h1.HasPayload cfg v pc) :
    h2.HasPayload cfg v pc := by
  rcases hp with h | h
  · exact Or.inl h
  · exact Or.inr (hr _ h)

theorem lockable_grows {c : Cert1} (hl : h1.Lockable cfg c) : h2.Lockable cfg c :=
  lockable_mono hr hl

theorem buildable_grows {c : Cert1} (hb : h1.Buildable cfg c) : h2.Buildable cfg c := by
  rcases hb with rfl | ⟨hc, b, hbk, hcert⟩
  · exact Or.inl rfl
  · exact Or.inr ⟨hasCert1_grows hr hc, b, hasProposal_grows hr hbk, hcert⟩

theorem viewGround_grows {v : ViewNumber} (hv : h1.ViewGround cfg v) : h2.ViewGround cfg v := by
  rcases hv with ⟨c, hc, rfl⟩ | ⟨tc, htc, rfl⟩ | ⟨c1, c2, p, ⟨hrec, hwf⟩, rfl⟩
  · exact Or.inl ⟨c, hasCert1_grows hr hc, rfl⟩
  · exact Or.inr (Or.inl ⟨tc, hr _ htc, rfl⟩)
  · exact Or.inr (Or.inr ⟨c1, c2, p, ⟨hr _ hrec, hwf⟩, rfl⟩)

end Grows

theorem hasCert1_of_lockable {h : History} {c : Cert1} (hl : h.Lockable cfg c) : h.HasCert1 cfg c := by
  rcases hl with rfl | ⟨hc, _⟩ | ⟨c2, p, hr, _⟩
  · exact Or.inl rfl
  · exact hc
  · exact Or.inr (Or.inr ⟨c2, p, hr⟩)

theorem hasCert1_of_buildable {h : History} {c : Cert1} (hb : h.Buildable cfg c) :
    h.HasCert1 cfg c := by
  rcases hb with rfl | ⟨hc, _⟩
  · exact Or.inl rfl
  · exact hc

/-- A certificate the node could lock on it could build on. -/
theorem buildable_of_lockable {h : History} {c : Cert1} (hl : h.Lockable cfg c) :
    h.Buildable cfg c := by
  rcases hl with rfl | ⟨hc, b, hb, hcert, -⟩ | ⟨c2, p, hrec, hwf⟩
  · exact Or.inl rfl
  · exact Or.inr ⟨hc, b, hb, hcert⟩
  · exact Or.inr ⟨Or.inr (Or.inr ⟨c2, p, hrec⟩), p, Or.inr (Or.inr ⟨c, c2, hrec⟩),
      Nat.le_of_eq (congrArg ViewNumber.toNat hwf.cert1View), hwf.cert1Data⟩

/-- Before any step, the only certificate a node holds is the anchor's. -/
theorem hasCert1_nil {r : Trace} {c : Cert1} (hc : (r.history 0).HasCert1 cfg c) : c = cfg.anchorCert := by
  have hnr : ∀ i, ¬ (r.history 0).Received i := fun i hr => by
    obtain ⟨j, hj, -⟩ := (Trace.received_history r).mp hr; exact absurd hj (Nat.not_lt_zero _)
  rcases hc with rfl | h | ⟨_, _, h⟩
  · rfl
  all_goals exact absurd h (hnr _)

/-- Before any step, the only certificate a node could lock on is the anchor's. -/
theorem lockable_nil {r : Trace} {c : Cert1} (hl : (r.history 0).Lockable cfg c) : c = cfg.anchorCert := by
  have hnr : ∀ i, ¬ (r.history 0).Received i := fun i hr => by
    obtain ⟨j, hj, -⟩ := (Trace.received_history r).mp hr; exact absurd hj (Nat.not_lt_zero _)
  rcases hl with rfl | ⟨hc1, _⟩ | ⟨c2, p, hr, _⟩
  · rfl
  · rcases hc1 with rfl | h | ⟨_, _, h⟩
    · rfl
    all_goals exact absurd h (hnr _)
  · exact absurd hr (hnr _)

theorem parentReady_grows {h1 h2 : History} (hr : ∀ i, h1.Received i → h2.Received i) {p : Proposal}
    (hp : ParentReady cfg h1 p) : ParentReady cfg h2 p := by
  rcases hp with hg | he | ⟨parent, hb, hv, hh, hpay⟩
  · exact Or.inl hg
  · exact Or.inr (Or.inl he)
  · exact Or.inr (Or.inr ⟨parent, hasProposal_grows hr hb, hv, hh, hasPayload_grows hr hpay⟩)

/-! ## Along a trace -/

section Trace

variable (r : Trace)

theorem received_grows {m n : Nat} (hle : m ≤ n) :
    ∀ i, (r.history m).Received i → (r.history n).Received i :=
  fun _ h => Trace.received_mono r hle h

theorem decidedView_mono {m n : Nat} (hle : m ≤ n) {v : ViewNumber}
    (hd : (r.history m).DecidedView v) : (r.history n).DecidedView v := by
  obtain ⟨st, hst, rest⟩ := hd
  obtain ⟨j, hj, rfl⟩ := (Trace.mem_history r).mp hst
  exact ⟨r j, (Trace.mem_history r).mpr ⟨j, Nat.lt_of_lt_of_le hj hle, rfl⟩, rest⟩

open Classical in
/-- A trace's prefixes, restricted, are the prefixes of the restricted trace. -/
theorem restrict_history (P : EpochNumber → Prop) (n : Nat) :
    (r.history n).restrict P
      = Trace.history (fun i => ⟨(r i).input, (r i).output.filter fun o => decide (o.SignedIn P)⟩) n := by
  simp [Trace.history, History.restrict, List.map_map, Function.comp_def]

/-- What a prefix decided for the epochs `P` holds of, a longer prefix did. -/
theorem decidedView_restrict_mono (P : EpochNumber → Prop) {m n : Nat} (hle : m ≤ n) {v : ViewNumber}
    (hd : ((r.history m).restrict P).DecidedView v) : ((r.history n).restrict P).DecidedView v := by
  rw [restrict_history] at hd ⊢
  exact decidedView_mono _ hle hd

/--
Where a view was entered: if the node is in `x` after `m` steps and was not in it
at the start, some step `n < m` moved it into `x`.
-/
theorem entered {m : Nat} {x : ViewNumber} (hm : (r.history m).InView cfg x)
    (h0 : ¬ (r.history 0).InView cfg x) :
    ∃ n, n < m ∧ (r.history (n + 1)).InView cfg x ∧ ¬ (r.history n).InView cfg x := by
  have hex : ∃ j, (r.history j).InView cfg x := ⟨m, hm⟩
  let a := least _ hex
  have ha : (r.history a).InView cfg x := least_spec hex
  have hle : a ≤ m := least_le hex hm
  have hpos : a ≠ 0 := fun h0' => h0 (h0' ▸ ha)
  refine ⟨a - 1, by omega, ?_, ?_⟩
  · rw [Nat.sub_add_cancel (Nat.pos_of_ne_zero hpos)]; exact ha
  · exact least_min hex (by omega)

/-- A prefix of a prefix of a trace is a prefix of the trace. -/
theorem upTo_history {m j : Nat} : (r.history m).upTo j = r.history (min j m) := by
  by_cases h : j ≤ m
  · rw [Nat.min_eq_left h]; exact Trace.history_upTo r h
  · rw [Nat.min_eq_right (by omega)]
    simp only [History.upTo]
    exact List.take_of_length_le (by rw [Trace.history_length]; omega)

/-- A certificate the node may build on, or vote on again, it may still later. -/
theorem certJustified_grows {m n : Nat} (hle : m ≤ n) {c : Cert1} {ev : Option TimeoutCert}
    (hj : CertJustified cfg (r.history m) c ev) : CertJustified cfg (r.history n) c ev := by
  cases ev with
  | none => exact buildable_grows (received_grows r hle) hj
  | some tc =>
    obtain ⟨hc, m0, hr0, hl0⟩ := hj
    rw [upTo_history] at hr0 hl0
    refine ⟨hasCert1_grows (received_grows r hle) hc, min m0 m, ?_, ?_⟩
    · rw [upTo_history, Nat.min_eq_left (Nat.le_trans (Nat.min_le_right _ _) hle)]; exact hr0
    · rw [upTo_history, Nat.min_eq_left (Nat.le_trans (Nat.min_le_right _ _) hle)]; exact hl0

/-- A proposal the node may make, it may still make later, while its epoch is still the node's. -/
theorem proposalJustified_grows {leader : EpochNumber → ViewNumber → Option PubKey} {node : PubKey}
    {m n : Nat} (hle : m ≤ n) {p : Proposal}
    (hj : ProposalJustified cfg leader node (r.history m) p)
    (hcur : NotBehind cfg (r.history n) p.epoch) :
    ProposalJustified cfg leader node (r.history n) p := by
  obtain ⟨⟨hl, hw, hjust, ⟨parent, hb, hv, hh⟩, hopen, hsafe, -, ⟨u, hu, hvu⟩⟩, hrec⟩ := hj
  refine ⟨⟨hl, hw, certJustified_grows r hle hjust, ⟨parent, hasProposal_grows (received_grows r hle) hb,
    hv, hh⟩, fun hen => ?_, hsafe, hcur, ⟨u, viewGround_grows (received_grows r hle) hu, hvu⟩⟩,
    received_grows r hle _ hrec⟩
  obtain ⟨⟨q, hq, h3⟩, c2, hc2, h4⟩ := hopen hen
  exact ⟨⟨q, hasProposal_grows (received_grows r hle) hq, h3⟩,
    c2, hc2.imp_left (hasCert2_grows (received_grows r hle)), h4⟩

/-- A re-vote request the node may send, it may still send later, while its epoch is still the node's. -/
theorem revoteJustified_grows {leader : EpochNumber → ViewNumber → Option PubKey} {node : PubKey}
    {m n : Nat} (hle : m ≤ n) {rv : RevoteRequest}
    (hj : RevoteJustified cfg leader node (r.history m) rv)
    (hcur : NotBehind cfg (r.history n) rv.cert.data.epoch) :
    RevoteJustified cfg leader node (r.history n) rv :=
  ⟨hj.leads, hj.wellFormed, certJustified_grows r hle hj.justified,
    fun h => lockable_grows (received_grows r hle) (hj.lockable h), hj.safe, hcur,
    let ⟨u, hu, hvu⟩ := hj.reached; ⟨u, viewGround_grows (received_grows r hle) hu, hvu⟩⟩

end Trace

/-! ## Finitely many -/

/-- A bound that can be raised, found for each of finitely many things, is found for all at once. -/
theorem finite_bound {α : Type} (P : α → Nat → Prop) (hmono : ∀ a s t, s ≤ t → P a s → P a t) :
    ∀ l : List α, (∀ a ∈ l, ∃ t, P a t) → ∃ t, ∀ a ∈ l, P a t
  | [], _ => ⟨0, fun _ h => by cases h⟩
  | a :: l, h => by
    obtain ⟨s, hs⟩ := h a List.mem_cons_self
    obtain ⟨t, ht⟩ := finite_bound P hmono l fun b hb => h b (List.mem_cons_of_mem a hb)
    refine ⟨max s t, fun b hb => ?_⟩
    rcases List.mem_cons.mp hb with rfl | hb
    · exact hmono _ _ _ (Nat.le_max_left s t) hs
    · exact hmono _ _ _ (Nat.le_max_right s t) (ht b hb)

/-- A bound that can be raised, found for each honest node, is found for all of them at once. -/
theorem honest_bound {C : Committee} (P : PubKey → Nat → Prop)
    (hmono : ∀ a s t, s ≤ t → P a s → P a t) (h : ∀ k, C.Honest k → ∃ t, P k t) :
    ∃ t, ∀ k, C.Honest k → P k t := by
  obtain ⟨ks, hks⟩ := C.honestFinite
  have hall : ∀ k ∈ ks, ∃ t, C.Honest k → P k t := fun k _ => by
    by_cases hk : C.Honest k
    · obtain ⟨t, ht⟩ := h k hk; exact ⟨t, fun _ => ht⟩
    · exact ⟨0, fun h' => absurd h' hk⟩
  obtain ⟨t, ht⟩ := finite_bound (fun k t => C.Honest k → P k t)
    (fun a s t hle h hk => hmono a s t hle (h hk)) ks hall
  exact ⟨t, fun k hk => ht k (let ⟨e, he⟩ := hk; hks e k he) hk⟩

/-- The views a history decided are bounded. -/
theorem decided_bound (h : History) : ∃ V, ∀ v, h.DecidedView v → v.toNat < V := by
  have hout : ∀ o : Output, ∃ V, ∀ blocks c1 c2, o = .decided blocks c1 c2 →
      ∀ b ∈ blocks, b.viewNumber.toNat < V := by
    intro o
    cases o with
    | send m => exact ⟨0, fun _ _ _ he => by cases he⟩
    | decided blocks c1 c2 =>
      obtain ⟨V, hV⟩ := finite_bound (fun (b : Block) V => b.viewNumber.toNat < V)
        (fun _ _ _ hle h => Nat.lt_of_lt_of_le h hle) blocks fun b _ => ⟨_, Nat.lt_succ_self _⟩
      exact ⟨V, fun _ _ _ he b hb => by cases he; exact hV b hb⟩
  obtain ⟨V, hV⟩ := finite_bound (fun (st : Step) V => ∀ o ∈ st.output, ∀ blocks c1 c2,
      o = .decided blocks c1 c2 → ∀ b ∈ blocks, b.viewNumber.toNat < V)
    (fun _ _ _ hle h o ho blocks c1 c2 he b hb => Nat.lt_of_lt_of_le (h o ho blocks c1 c2 he b hb) hle)
    h fun st _ => finite_bound _
      (fun _ _ _ hle h blocks c1 c2 he b hb => Nat.lt_of_lt_of_le (h blocks c1 c2 he b hb) hle)
      st.output fun o _ => hout o
  refine ⟨V, fun v ⟨st, hst, blocks, c1, c2, b, hd, hb, hv⟩ => ?_⟩
  rw [← hv]
  exact hV st hst _ hd blocks c1 c2 rfl b hb

/-! ## Time -/

section Time

variable {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}
variable (N : TimedNetwork cfg leader C)

/-- A history that obeys the protocol in every epoch obeys it in any set of epochs. -/
theorem _root_.NewProtocol.ProtocolHistory.of_every {leader : EpochNumber → ViewNumber → Option PubKey}
    {node : PubKey} {P Q : EpochNumber → Prop} {h : History}
    (hp : ProtocolHistory cfg leader node (fun _ => True) (fun _ => True) h) :
    ProtocolHistory cfg leader node P Q h where
  toSafeHistory := .of_every hp.toSafeHistory
  vote1Leader := fun n vote ⟨st, hn, hm⟩ _ => hp.vote1Leader n vote ⟨st, hn, hm⟩ trivial
  timeoutJustified n st vote hn hm _ := hp.timeoutJustified n st vote hn hm trivial
  timeoutAnswered n st v hn hi _ := hp.timeoutAnswered n st v hn hi fun _ _ => ⟨trivial, trivial⟩
  proposeJustified := fun n p ⟨st, hn, hm⟩ _ => hp.proposeJustified n p ⟨st, hn, hm⟩ trivial
  revoteJustified := fun n r ⟨st, hn, hm⟩ _ => hp.revoteJustified n r ⟨st, hn, hm⟩ trivial
  proposeOnce m m' v e hs hs' hv hv' he he' _ := hp.proposeOnce m m' v e hs hs' hv hv' he he' trivial

/-- At most one proposal per epoch and view, of the epochs the node is honest in. -/
theorem _root_.NewProtocol.ProtocolHistory.proposal_once {leader : EpochNumber → ViewNumber → Option PubKey}
    {node : PubKey} {P Q : EpochNumber → Prop} {h : History} (hp : ProtocolHistory cfg leader node P Q h)
    (p p' : Proposal) (hs : h.Sent (.proposal p)) (hs' : h.Sent (.proposal p')) (he : P p.epoch)
    (heq : p.epoch = p'.epoch) (hv : p.viewNumber = p'.viewNumber) : p = p' := by
  have := hp.proposeOnce _ _ p.viewNumber p.epoch hs hs' rfl (by rw [hv]; rfl) rfl (by rw [heq]; rfl) he
  cases this; rfl

/--
At most one re-vote request per epoch and view, and none where the node proposed,
of an epoch the node is honest in.
-/
theorem _root_.NewProtocol.ProtocolHistory.revote_once {leader : EpochNumber → ViewNumber → Option PubKey}
    {node : PubKey} {P Q : EpochNumber → Prop} {h : History} (hp : ProtocolHistory cfg leader node P Q h)
    (r : RevoteRequest) (hs : h.Sent (.revote r)) (he : P r.cert.data.epoch) :
    (∀ r', h.Sent (.revote r') → r'.cert.data.epoch = r.cert.data.epoch → r'.view = r.view → r' = r)
      ∧ ∀ p, h.Sent (.proposal p) → p.epoch = r.cert.data.epoch → p.viewNumber ≠ r.view :=
  ⟨fun r' hs' he' hv => by
    have := hp.proposeOnce _ _ r.view r.cert.data.epoch hs' hs (by rw [← hv]; rfl) rfl (congrArg some he') rfl he
    cases this; rfl,
   fun p hs' he' hv => by
    have := hp.proposeOnce _ _ r.view r.cert.data.epoch hs' hs (by rw [← hv]; rfl) rfl (congrArg some he') rfl he
    cases this⟩

/-- A node honest in infinitely many epochs is honest in some. -/
theorem _root_.NewProtocol.Committee.HonestOften.honest {C : Committee} {k : PubKey} (h : C.HonestOften k) :
    C.Honest k :=
  let ⟨_, _, he⟩ := h ⟨0⟩; .of he

theorem time_mono (k : PubKey) (hk : C.Honest k) {i j : Nat} (hle : i ≤ j) :
    N.time k hk i ≤ N.time k hk j := by
  induction hle with
  | refl => exact Nat.le_refl _
  | step _ ih => exact Nat.le_trans ih (N.timeMono k hk _)

/-- The number of steps a node takes by time `T`: every step before it is by `T`, every step from it on is later. -/
noncomputable def cut (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n)
    (k : PubKey) (hk : C.Honest k) (T : Nat) : Nat :=
  least _ (hu k hk T)

variable (hu : ∀ k (hk : C.Honest k) T, ∃ n, T < N.time k hk n)

theorem cut_before {k : PubKey} {hk : C.Honest k} {T i : Nat} (hi : i < cut N hu k hk T) :
    N.time k hk i ≤ T :=
  Nat.le_of_not_lt (least_min (hu k hk T) hi)

theorem cut_after {k : PubKey} {hk : C.Honest k} {T i : Nat} (hi : cut N hu k hk T ≤ i) :
    T < N.time k hk i :=
  Nat.lt_of_lt_of_le (least_spec (hu k hk T)) (time_mono N k hk hi)

/-- A prefix whose steps are all by `T` is part of the cut at `T`. -/
theorem le_cut {k : PubKey} {hk : C.Honest k} {T n : Nat}
    (hn : ∀ i, i < n → N.time k hk i ≤ T) : n ≤ cut N hu k hk T := by
  by_cases h : n ≤ cut N hu k hk T
  · exact h
  · exact absurd (hn _ (by omega)) (Nat.not_le_of_lt (cut_after N hu (Nat.le_refl _)))

/-- The cut grows with the time. -/
theorem cut_mono {k : PubKey} {hk : C.Honest k} {T T' : Nat} (hle : T ≤ T') :
    cut N hu k hk T ≤ cut N hu k hk T' :=
  le_cut N hu fun _ hi => Nat.le_trans (cut_before N hu hi) hle

/-- What holds of some prefix by `T`, a monotone property makes true of the cut at `T`. -/
theorem by_cut {k : PubKey} {hk : C.Honest k} {T : Nat} {P : History → Prop}
    (hmono : ∀ m n, m ≤ n → P ((N.trace k hk).history m) → P ((N.trace k hk).history n))
    (hby : N.By k hk T P) : P ((N.trace k hk).history (cut N hu k hk T)) := by
  obtain ⟨n, hn, hp⟩ := hby
  exact hmono _ _ (le_cut N hu hn) hp

/-- A non-empty cut ends with a step, which is by its time. -/
theorem cut_pred {k : PubKey} {hk : C.Honest k} {T : Nat} (h : cut N hu k hk T ≠ 0) :
    ∃ n, cut N hu k hk T = n + 1 ∧ N.time k hk n ≤ T :=
  ⟨cut N hu k hk T - 1, (Nat.succ_pred_eq_of_ne_zero h).symm,
    cut_before N hu (show cut N hu k hk T - 1 < cut N hu k hk T by omega)⟩

/-- An obligation of an honest node, for an epoch it is honest in, cannot stay owed from a step until `δ` after it. -/
theorem not_owed_throughout {δ : Nat} (hp : Prompt N δ) {k : PubKey} {hk : C.Honest k}
    {n B : Nat} (hB : N.time k hk n + δ ≤ B) {o : Obligation}
    (howed : ∀ m, n ≤ m → N.time k hk m ≤ B →
      OwedIn cfg leader k (C.honest · k) ((N.trace k hk).history (m + 1)) o) :
    False := by
  obtain ⟨m, hm, htm, hnow⟩ := hp k hk n o (howed n (Nat.le_refl _) (by omega))
  exact hnow (howed m hm (by omega))

include hu in
/-- The views honest nodes have grounds for by time `T` are bounded. -/
theorem ground_bound (T : Nat) : ∃ B, ∀ k (hk : C.Honest k) n, (∀ i, i < n → N.time k hk i ≤ T) →
    ∀ v, ((N.trace k hk).history n).ViewGround cfg v → v.toNat < B := by
  obtain ⟨B, hB⟩ := honest_bound (C := C) (fun k B => ∀ hk : C.Honest k, ∀ n,
      (∀ i, i < n → N.time k hk i ≤ T) → ∀ v, ((N.trace k hk).history n).ViewGround cfg v → v.toNat < B)
    (fun _ _ _ hle h hk n hn v hv => Nat.lt_of_lt_of_le (h hk n hn v hv) hle)
    (fun k hk => ⟨(viewOf cfg ((N.trace k hk).history (cut N hu k hk T))).toNat + 1,
      fun hk' n hn v hv => Nat.lt_succ_of_le ((inView_viewOf cfg _).2 v
        (viewGround_grows (received_grows _ (le_cut N hu hn)) hv))⟩)
  exact ⟨B, fun k hk => hB k hk hk⟩

end Time

end Liveness
end NewProtocol
