module

public import NewProtocolImpl.Conformance
public import NewProtocolSpec.Proofs.Liveness.Epochs

/-!
# What every witness needs

A witness runs the machine on a fixed schedule of inputs per node, with a time for
each step, and shows the liveness premises hold. The parts that do not depend on
the schedule are here: a node's trace and history, what it received and sent, that
it obeys the rules and owes nothing, its view and epoch from a candidate the
witness supplies, and delivery bounds over a time function.

A witness names its schedule `input` and writes `tr` and `H` for `Kit.tr cfg
leader input` and `Kit.H cfg leader input`, as local notation, so these lemmas
apply to its own terms by rewriting.
-/

@[expose] public section

namespace NewProtocolImpl
namespace Kit

open NewProtocol History

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (input : PubKey → Nat → Input)

/-- Node `k` running the machine on its schedule. -/
def tr (k : PubKey) : Trace := traceOf cfg leader k (input k)

/-- Its history after `n` steps. -/
abbrev H (k : PubKey) (n : Nat) : History := (tr cfg leader input k).history n

/-! ## The trace -/

variable {cfg leader input} {k : PubKey}

theorem received {n : Nat} {i : Input} : (H cfg leader input k n).Received i ↔ ∃ j, j < n ∧ input k j = i :=
  Trace.received_history _

theorem received_mono {n m : Nat} {i : Input} (hnm : n ≤ m) (hr : (H cfg leader input k n).Received i) :
    (H cfg leader input k m).Received i := by
  obtain ⟨j, hj, hji⟩ := received.mp hr
  exact received.mpr ⟨j, by omega, hji⟩

theorem upTo_H {n m : Nat} : (H cfg leader input k n).upTo m = H cfg leader input k (min m n) :=
  Liveness.upTo_history _

theorem upTo_self {n : Nat} : (H cfg leader input k n).upTo n = H cfg leader input k n :=
  Trace.history_upTo _ (Nat.le_refl _)

theorem getElem_H (j : Nat) : (H cfg leader input k (j + 1))[j]? = some (tr cfg leader input k j) :=
  Trace.history_getElem? _ (Nat.lt_succ_self j)

theorem hpre (j : Nat) : historyOf cfg leader k (input k) j = H cfg leader input k j :=
  (traceOf_history cfg leader k (input k) j).symm

theorem tr_step (j : Nat) :
    tr cfg leader input k j = step cfg leader k (H cfg leader input k j) (input k j) := by
  rw [← hpre]; rfl

theorem sent_iff {n : Nat} {m : Message} :
    (H cfg leader input k n).Sent m ↔ ∃ j, j < n ∧ Output.send m ∈ (tr cfg leader input k j).output :=
  Trace.sent_history _

variable (input) in
/-- After every step, the node owes nothing. -/
theorem settled (k : PubKey) (n : Nat) (o : Obligation) : ¬ Owed cfg leader k (H cfg leader input k (n + 1)) o := by
  rw [← hpre]; exact historyOf_settled (input k) n o

variable (input) in
/-- Every prefix obeys the rules, with validity taken as reported. -/
theorem protocol (hv : ∀ b, BlockValid b) (k : PubKey) (n : Nat) :
    ProtocolHistory cfg leader k (fun _ => True) (fun _ => True) (H cfg leader input k n) := by
  rw [← hpre]; exact historyOf_protocol (fun _ _ _ _ b _ => hv b) n

variable (input) in
/-- A timeout vote is sent in the step its view's timer or one-honest indication arrives. -/
theorem timeout_vote_input (hv : ∀ b, BlockValid b) {j : Nat} {vote : TimeoutVote}
    (hx : Output.send (.timeoutVote vote) ∈ (tr cfg leader input k j).output) :
    input k j = .timeout vote.view ∨ input k j = .timeoutOneHonest vote.view := by
  obtain ⟨-, -, -, hcase⟩ := (protocol input hv k (j + 1)).timeoutJustified j _ vote (getElem_H j) hx trivial
  have hin : (tr cfg leader input k j).input = input k j := by rw [tr_step]; rfl
  rcases hcase with ⟨h, -⟩ | ⟨h, -⟩
  · exact Or.inl (hin ▸ h)
  · exact Or.inr (hin ▸ h)

variable (input) in
/-- A view the node decided is after the anchor's, and that of a block whose proposal it holds. -/
theorem decided_proposal (hv : ∀ b, BlockValid b) {n : Nat} {w : ViewNumber}
    (hd : (H cfg leader input k n).DecidedView w) :
    ∃ b, (H cfg leader input k n).HasProposal cfg b ∧ b.viewNumber = w ∧ cfg.anchorView < w := by
  obtain ⟨st, hst, blocks, c1, c2, b, hm, hb, rfl⟩ := hd
  obtain ⟨j, hj, rfl⟩ := (Trace.mem_history _).mp hst
  obtain ⟨-, -, -, -, -, -, -, -, hheld⟩ :=
    (protocol input hv k (j + 1)).decideJustified j _ blocks c1 c2 (getElem_H j) hm trivial
  obtain ⟨hp, hgen⟩ := hheld b hb
  rw [upTo_self] at hp
  exact ⟨b, Liveness.hasProposal_grows (fun _ => received_mono hj) hp, rfl, hgen⟩

variable (input) in
/-- A view is after the floor if every proposal the node holds is less than the decide buffer past it. -/
theorem afterFloor_of (hv : ∀ b, BlockValid b) {n : Nat} {v : ViewNumber} (hgen : cfg.anchorView < v)
    (hle : ∀ x, (H cfg leader input k n).HasProposal cfg x → x.viewNumber.toNat < v.toNat + cfg.decideBuffer) :
    (H cfg leader input k n).AfterFloor cfg v := by
  refine ⟨hgen, fun w hw => ?_⟩
  obtain ⟨x, hx, rfl, -⟩ := decided_proposal input hv hw
  have := hle x hx
  have : cfg.anchorView.toNat < v.toNat := hgen
  show x.viewNumber.toNat - cfg.decideBuffer < v.toNat
  omega

/-! ## Views and epochs, from a candidate -/

theorem inView_unique {h : History} {v w : ViewNumber} (hv : h.InView cfg v) (hw : h.InView cfg w) : v = w :=
  ViewNumber.le_antisymm (hw.2 _ hv.1) (hv.2 _ hw.1)

theorem epoch_ext {e f : EpochNumber} (h : e.toNat = f.toNat) : e = f := by
  cases e; cases f; cases h; rfl

theorem view_inj {x y : Nat} (h : (⟨x⟩ : ViewNumber) = ⟨y⟩) : x = y := congrArg ViewNumber.toNat h

theorem number_inj {x y : Nat} (h : (⟨x⟩ : BlockNumber) = ⟨y⟩) : x = y := congrArg BlockNumber.toNat h

/-- Two certificates each no later than the other are of one epoch and one view. -/
theorem lockLE_antisymm {x y : Cert1} (h1 : LockLE x y) (h2 : LockLE y x) :
    x.data.epoch = y.data.epoch ∧ x.view = y.view := by
  rcases h1 with h1 | ⟨he, hv⟩ <;> rcases h2 with h2 | ⟨he', hv'⟩
  · exact absurd (Nat.lt_trans h1 h2) (Nat.lt_irrefl _)
  · rw [he'] at h1; exact absurd h1 (Nat.lt_irrefl _)
  · rw [he] at h2; exact absurd h2 (Nat.lt_irrefl _)
  · exact ⟨he, ViewNumber.le_antisymm hv hv'⟩

theorem notBehind_of {h : History} {e₀ e : EpochNumber} (h₀ : h.InEpoch cfg e₀) (hle : e₀.toNat ≤ e.toNat) :
    NotBehind cfg h e :=
  fun e' he' => by rw [Liveness.inEpoch_unique he' h₀]; exact hle

theorem le_of_notBehind {h : History} {e₀ e : EpochNumber} (h₀ : h.InEpoch cfg e₀) (hnb : NotBehind cfg h e) :
    e₀.toNat ≤ e.toNat :=
  hnb _ h₀

/-! ## Time -/

/-- A function that never decreases from one step to the next never decreases. -/
theorem mono_of_succ {f : Nat → Nat} (hs : ∀ n, f n ≤ f (n + 1)) {n m : Nat} (h : n ≤ m) : f n ≤ f m := by
  induction m with
  | zero => rw [Nat.le_zero.mp h]; exact Nat.le_refl _
  | succ m ih =>
    by_cases hnm : n = m + 1
    · rw [hnm]; exact Nat.le_refl _
    · exact Nat.le_trans (ih (by omega)) (hs m)

variable {C : Committee} {N : TimedNetwork cfg leader C} {tm : Nat → Nat}

/-- What holds within `Δ` holds within `2Δ`. -/
theorem by_slower {hk : C.Honest k} {T Δ : Nat} {P : History → Prop} (h : N.By k hk (T + Δ) P) :
    N.By k hk (T + 2 * Δ) P :=
  let ⟨n, hn, hp⟩ := h
  ⟨n, fun i hi => by have := hn i hi; omega, hp⟩

/-- What holds by a time, anything it implies holds by then too. -/
theorem by_imp {hk : C.Honest k} {T : Nat} {P Q : History → Prop} (hpq : ∀ h, P h → Q h)
    (h : N.By k hk T P) : N.By k hk T Q :=
  let ⟨n, hn, hp⟩ := h
  ⟨n, hn, hpq _ hp⟩

/-- By time `T` the node's history satisfies `P`, if it does after `m` steps, all at `T` or before. -/
theorem by_at (htr : ∀ k hk, N.trace k hk = tr cfg leader input k) (htm : ∀ k hk n, N.time k hk n = tm n)
    (hs : ∀ n, tm n ≤ tm (n + 1)) {hk : C.Honest k} {T m : Nat} {P : History → Prop}
    (hm : 0 < m → tm (m - 1) ≤ T) (hp : P (H cfg leader input k m)) : N.By k hk T P :=
  ⟨m, fun i hi => by rw [htm]; exact Nat.le_trans (mono_of_succ hs (show i ≤ m - 1 by omega)) (hm (by omega)),
    by rw [htr]; exact hp⟩

/--
A node that sends timeout votes for `v` only up to step `B` does not keep timing `v`
out, so `Synchrony.timeoutCatchUp` asks nothing of it.
-/
theorem catchUp_vacuous (htr : ∀ k hk, N.trace k hk = tr cfg leader input k)
    (htm : ∀ k hk n, N.time k hk n = tm n) (hs : ∀ n, tm n ≤ tm (n + 1)) {hk : C.Honest k}
    {v : ViewNumber} (B : Nat)
    (hb : ∀ m (vote : TimeoutVote), Output.send (.timeoutVote vote) ∈ (tr cfg leader input k m).output →
      vote.view = v → m ≤ B)
    (hrep : ∀ T, ∃ m vote, T < N.time k hk m ∧ vote.view = v
      ∧ Output.send (.timeoutVote vote) ∈ (N.trace k hk m).output) : False := by
  obtain ⟨m, vote, hT, hvv, hout⟩ := hrep (tm B)
  rw [htr] at hout
  rw [htm] at hT
  have := mono_of_succ hs (hb m vote hout hvv)
  omega

/-- What the node sent by time `t`, it sent at a step no later than `t`. -/
theorem sentBy (htr : ∀ k hk, N.trace k hk = tr cfg leader input k) (htm : ∀ k hk n, N.time k hk n = tm n)
    {hk : C.Honest k} {t : Nat} {m : Message} (hs : N.SentByTime k hk t m) :
    ∃ j, tm j ≤ t ∧ Output.send m ∈ (tr cfg leader input k j).output := by
  obtain ⟨n, hn, hsent⟩ := hs
  rw [htr] at hsent
  obtain ⟨j, hj, hjm⟩ := sent_iff.mp hsent
  exact ⟨j, by rw [← htm k hk j]; exact hn j hj, hjm⟩

/-! ## Uniform honesty -/

/-- When honesty does not change with the epoch, every honest node is steady. -/
theorem steady_of_uniform {C : Committee} (hu : ∀ e e' k, C.honest e k → C.honest e' k)
    {k : PubKey} (h : C.Honest k) : C.Steady k :=
  let ⟨e, he⟩ := h
  fun e' => hu e e' k he

end Kit
end NewProtocolImpl
