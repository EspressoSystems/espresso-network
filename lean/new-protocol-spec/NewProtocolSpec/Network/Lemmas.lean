module

public import NewProtocolSpec.Network.Defs

/-!
# Working lemmas for the network results

Kernel-checked scaffolding for `NewProtocolSpec.Network`; nothing here is part of
the contract, and an audit can skip the file. Its two substantial results are
`vote1_agree` and `vote2_agree` — that a node votes at most once per view over
the whole of its run, which is the fact quorum intersection is combined with.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config)

/-! ## A run's states are reachable states

The bridge between a `Run` and everything proved over `Reachable`: a run is
a sequence of transitions, so each of its states is reachable from the first.
Every per-node invariant — `DecideInv` among them — therefore holds at every
point of every honest node's run.
-/

theorem reachable_of_run {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) : ∀ n, Reachable cfg (SafetySpec cfg node)
        (Run.state r 0) (Run.state r n)
  | 0 => .refl
  | n + 1 => .step (reachable_of_run r n) (Run.transition r n)

/-- Nothing is ever admitted at genesis: admission requires the view to be above the bar. -/
theorem admitted_above_genesis {s : NodeState}
    (hr : Reachable cfg (SafetySpec cfg node) (NodeState.initial cfg) s) :
    ∀ v p, s.admitted v = some p → ViewNumber.genesis < v := by
  induction hr with
  | refl => intro v p hp; simp [NodeState.initial] at hp
  | step _ ht ih =>
    cases ht with
    | step hs =>
      intro v p hp
      rcases SafetySpec.admissionJustified hs v p hp with hold | ⟨-, -, -, -, hbar, -⟩
      · exact ih v p hold
      · exact Nat.lt_of_le_of_lt (Nat.zero_le _) hbar
    | collect hg => intro v p hp; exact ih v p ((GcSpec.shrinks hg).admitted v p hp)

/-- A held `Cert1` is filed under the view it names. -/
theorem cert1_keyed (hcfg : ConfigCoherent cfg) {s : NodeState}
    (hr : Reachable cfg (SafetySpec cfg node) (NodeState.initial cfg) s) :
    ∀ v c, s.cert1s v = some c → c.view = v := by
  induction hr with
  | refl =>
    intro v c hc
    simp only [NodeState.initial] at hc
    split at hc
    · next hv => cases hc; rw [hcfg.anchorCertView]; exact hv.symm
    · exact nomatch hc
  | step _ ht ih =>
    cases ht with
    | step hs =>
      intro v c hc
      rcases SafetySpec.cert1Provenance hs v c hc with hold | ⟨-, hv⟩
      · exact ih v c hold
      · exact hv
    | collect hg => intro v c hc; exact ih v c ((GcSpec.shrinks hg).cert1s v c hc)

/-! ## A node votes at most once per view, for the whole of its run

`SafetySpec.vote1Once` says only that a step does not repeat a vote it has
already recorded, and collection is allowed to forget the record. What closes
the gap is the bar: a mark may only be dropped at or below it
(`GcSpec.voted1Retained`), and nothing may be voted there
(`SafetySpec.vote1NotBarred`). So the two together are what make voting once
per view a property of the whole run, not just of one step — and this is the
fact quorum intersection is combined with.
-/

/--
View `w` is spent for node `k`: either the vote is on record, or the view has
been abandoned and can never be voted in again.
-/
def Retired1 (s : NodeState) (w : ViewNumber) : Prop :=
  s.voted1Views w ∨ ¬ s.barredView < w

/--
As `Retired1`, for vote2s, with a third way for a view to be spent: the floor has
passed it.

A collection may drop a vote2 mark once the floor is past its view
(`GcSpec.voted2Retained`), and nothing needs the mark there —
`SafetySpec.vote2AboveFloor` refuses a vote2 below the floor on its own account,
and neither kind of step lets the floor descend (`SafetySpec.floorMono`,
`GcSpec.floorStable`).
-/
def Retired2 (cfg : Config) (s : NodeState) (w : ViewNumber) : Prop :=
  s.voted2Views w ∨ ¬ s.barredView < w ∨ ¬ s.aboveDecideFloor cfg w

theorem retired1_stable {cfg : Config} {node : PubKey}
    {s s' : NodeState} {e : Event} {w : ViewNumber}
    (ht : Transition cfg (SafetySpec cfg node) s e s') (h : Retired1 s w) : Retired1 s' w := by
  cases ht with
  | step hs =>
    rcases h with hv | hb
    · exact Or.inl (SafetySpec.voted1Retained hs w hv)
    · exact Or.inr (by rw [SafetySpec.barredViewUnchanged hs]; exact hb)
  | collect hg =>
    rcases h with hv | hb
    · by_cases hlt : s'.barredView < w
      · exact Or.inl (GcSpec.voted1Retained hg w hlt hv)
      · exact Or.inr hlt
    · exact Or.inr fun hc => hb (Nat.lt_of_le_of_lt (GcSpec.barredViewMono hg) hc)

theorem retired2_stable {cfg : Config} {node : PubKey}
    {s s' : NodeState} {e : Event} {w : ViewNumber}
    (ht : Transition cfg (SafetySpec cfg node) s e s') (h : Retired2 cfg s w) :
    Retired2 cfg s' w := by
  cases ht with
  | step hs =>
    rcases h with hv | hb | hf
    · exact Or.inl (SafetySpec.voted2Retained hs w hv)
    · exact Or.inr (Or.inl (by rw [SafetySpec.barredViewUnchanged hs]; exact hb))
    · exact Or.inr (Or.inr fun h' => hf (SafetySpec.floorMono hs h'))
  | collect hg =>
    rcases h with hv | hb | hf
    · by_cases hfl : s.aboveDecideFloor cfg w
      · exact Or.inl (GcSpec.voted2Retained hg w hfl hv)
      · exact Or.inr (Or.inr fun h' => hfl (GcSpec.floorStable hg _ h'))
    · exact Or.inr (Or.inl fun hc => hb (Nat.lt_of_le_of_lt (GcSpec.barredViewMono hg) hc))
    · exact Or.inr (Or.inr fun h' => hf (GcSpec.floorStable hg _ h'))

/-- Retiredness, once true, stays true. -/
theorem retired1_run {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) {w : ViewNumber} {n : Nat}
    (h : Retired1 (Run.state r n) w) : ∀ m, n ≤ m → Retired1 (Run.state r m) w := by
  intro m hm
  induction hm with
  | refl => exact h
  | step _ ih => exact retired1_stable (Run.transition r _) ih

/-- Casting a vote1 retires its view. -/
theorem retired1_of_emit {cfg : Config} {node : PubKey}
    {r : Run cfg (SafetySpec cfg node)} {n : Nat} {v : Vote1}
    (he : Output.send (.vote1 v) ∈ (Run.event r n).outputs) :
    Retired1 (Run.state r (n + 1)) v.view := by
  have ht : Transition cfg (SafetySpec cfg node)
      (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
    Run.transition r n
  cases hev : Run.event r n with
  | consensus input output =>
    rw [hev] at ht he
    cases ht with
    | step hs => exact Or.inl (SafetySpec.vote1Once hs v he).2
  | collect => rw [hev] at he; exact absurd he (by simp [Event.outputs])

/--
A node casts at most one vote1 per view in the whole of its run.

Within a step, `vote1Justified` reads the block off `admitted`, which holds
one proposal per view. Across steps, the mark rules it out unless the mark was
collected — and collection only reaches views at or below the bar, where
`vote1NotBarred` forbids voting.
-/
theorem vote1_agree {cfg : Config} {node : PubKey}
    {r : Run cfg (SafetySpec cfg node)} {n m : Nat} {v1 v2 : Vote1}
    (h1 : Output.send (.vote1 v1) ∈ (Run.event r n).outputs)
    (h2 : Output.send (.vote1 v2) ∈ (Run.event r m).outputs)
    (hv : v1.view = v2.view) : v1.data.blockHash = v2.data.blockHash := by
  -- Two casts in the same step agree because `admitted` holds one block per view.
  have same : ∀ {k : Nat} {a b : Vote1}, Output.send (.vote1 a) ∈ (Run.event r k).outputs →
      Output.send (.vote1 b) ∈ (Run.event r k).outputs → a.view = b.view →
      a.data.blockHash = b.data.blockHash := by
    intro k a b ha hb hab
    have ht : Transition cfg (SafetySpec cfg node)
        (Run.state r k) (Run.event r k) (Run.state r (k + 1)) :=
      Run.transition r k
    cases hev : Run.event r k with
    | collect => rw [hev] at ha; exact absurd ha (by simp [Event.outputs])
    | consensus input output =>
    rw [hev] at ht ha hb
    cases ht with
    | step hs =>
      obtain ⟨pa, hja, hva, -, hda, -⟩ := SafetySpec.vote1Justified hs a ha
      obtain ⟨pb, hjb, hvb, -, hdb, -⟩ := SafetySpec.vote1Justified hs b hb
      have : pa = pb := by
        have h' := Vote1Justification.proposalAdmitted hjb
        rw [← hvb, ← hab, hva] at h'
        exact Option.some_inj.mp ((Vote1Justification.proposalAdmitted hja).symm.trans h')
      rw [hda, hdb, this]
  -- Two casts in different steps cannot both happen: the first retires the view.
  have across : ∀ {i j : Nat} {a b : Vote1}, i < j →
      Output.send (.vote1 a) ∈ (Run.event r i).outputs →
      Output.send (.vote1 b) ∈ (Run.event r j).outputs → a.view = b.view → False := by
    intro i j a b hij ha hb hab
    have hret := retired1_run r (retired1_of_emit ha) j hij
    have ht : Transition cfg (SafetySpec cfg node)
        (Run.state r j) (Run.event r j) (Run.state r (j + 1)) :=
      Run.transition r j
    cases hev : Run.event r j with
    | collect => rw [hev] at hb; exact absurd hb (by simp [Event.outputs])
    | consensus input output =>
    rw [hev] at ht hb
    cases ht with
    | step hs =>
      rcases hret with hv' | hb'
      · exact (SafetySpec.vote1Once hs b hb).1 (hab ▸ hv')
      · refine absurd ?_ hb'
        have hnb := SafetySpec.vote1NotBarred hs b hb
        rw [SafetySpec.barredViewUnchanged hs] at hnb
        exact hab.symm ▸ hnb
  rcases Nat.lt_trichotomy n m with hlt | rfl | hgt
  · exact absurd (across hlt h1 h2 hv) (by simp)
  · exact same h1 h2 hv
  · exact absurd (across hgt h2 h1 hv.symm) (by simp)

/-! ## The same, for vote2s -/

/-- Retiredness, once true, stays true. -/
theorem retired2_run {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) {w : ViewNumber} {n : Nat}
    (h : Retired2 cfg (Run.state r n) w) : ∀ m, n ≤ m → Retired2 cfg (Run.state r m) w := by
  intro m hm
  induction hm with
  | refl => exact h
  | step _ ih => exact retired2_stable (Run.transition r _) ih

/-- Casting a vote2 retires its view. -/
theorem retired2_of_emit {cfg : Config} {node : PubKey}
    {r : Run cfg (SafetySpec cfg node)} {n : Nat} {v : Vote2}
    (he : Output.send (.vote2 v) ∈ (Run.event r n).outputs) :
    Retired2 cfg (Run.state r (n + 1)) v.view := by
  have ht : Transition cfg (SafetySpec cfg node)
      (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
    Run.transition r n
  cases hev : Run.event r n with
  | consensus input output =>
    rw [hev] at ht he
    cases ht with
    | step hs => exact Or.inl (SafetySpec.vote2Once hs v he).2
  | collect => rw [hev] at he; exact absurd he (by simp [Event.outputs])

/--
A node casts at most one vote2 per view in the whole of its run.

Within a step, `vote2Justified` reads the block off `admitted`, which holds
one proposal per view. Across steps, the mark rules it out unless the mark was
collected — and collection only reaches views at or below the bar, where
`vote2NotBarred` forbids voting.
-/
theorem vote2_agree {cfg : Config} {node : PubKey}
    {r : Run cfg (SafetySpec cfg node)} {n m : Nat} {v1 v2 : Vote2}
    (h1 : Output.send (.vote2 v1) ∈ (Run.event r n).outputs)
    (h2 : Output.send (.vote2 v2) ∈ (Run.event r m).outputs)
    (hv : v1.view = v2.view) : v1.data.blockHash = v2.data.blockHash := by
  -- Two casts in the same step agree because `admitted` holds one block per view.
  have same : ∀ {k : Nat} {a b : Vote2}, Output.send (.vote2 a) ∈ (Run.event r k).outputs →
      Output.send (.vote2 b) ∈ (Run.event r k).outputs → a.view = b.view →
      a.data.blockHash = b.data.blockHash := by
    intro k a b ha hb hab
    have ht : Transition cfg (SafetySpec cfg node)
        (Run.state r k) (Run.event r k) (Run.state r (k + 1)) :=
      Run.transition r k
    cases hev : Run.event r k with
    | collect => rw [hev] at ha; exact absurd ha (by simp [Event.outputs])
    | consensus input output =>
    rw [hev] at ht ha hb
    cases ht with
    | step hs =>
      obtain ⟨pa, hja, hva, -, hda, -⟩ := SafetySpec.vote2Justified hs a ha
      obtain ⟨pb, hjb, hvb, -, hdb, -⟩ := SafetySpec.vote2Justified hs b hb
      have : pa = pb := by
        have h' := Vote2Justification.proposalAdmitted hjb
        rw [← hvb, ← hab, hva] at h'
        exact Option.some_inj.mp ((Vote2Justification.proposalAdmitted hja).symm.trans h')
      rw [hda, hdb, this]
  -- Two casts in different steps cannot both happen: the first retires the view.
  have across : ∀ {i j : Nat} {a b : Vote2}, i < j →
      Output.send (.vote2 a) ∈ (Run.event r i).outputs →
      Output.send (.vote2 b) ∈ (Run.event r j).outputs → a.view = b.view → False := by
    intro i j a b hij ha hb hab
    have hret := retired2_run r (retired2_of_emit ha) j hij
    have ht : Transition cfg (SafetySpec cfg node)
        (Run.state r j) (Run.event r j) (Run.state r (j + 1)) :=
      Run.transition r j
    cases hev : Run.event r j with
    | collect => rw [hev] at hb; exact absurd hb (by simp [Event.outputs])
    | consensus input output =>
    rw [hev] at ht hb
    cases ht with
    | step hs =>
      rcases hret with hv' | hb' | hf'
      · exact (SafetySpec.vote2Once hs b hb).1 (hab ▸ hv')
      · refine absurd ?_ hb'
        have hnb := SafetySpec.vote2NotBarred hs b hb
        rw [SafetySpec.barredViewUnchanged hs] at hnb
        exact hab.symm ▸ hnb
      · exact hf' (hab ▸ SafetySpec.floorMono hs (SafetySpec.vote2AboveFloor hs b hb))
  rcases Nat.lt_trichotomy n m with hlt | rfl | hgt
  · exact absurd (across hlt h1 h2 hv) (by simp)
  · exact same h1 h2 hv
  · exact absurd (across hgt h2 h1 hv.symm) (by simp)

/-! ## Reading a vote off a run

Three facts the results of `NewProtocolSpec.Network` are assembled from: what an
honest node's timeout vote says about the view it was in, what its vote2
says about the certificate it held, and that two valid certificates share an
honest signer.
-/

/--
An honest node's timeout vote names the view it was in, or follows the
one-honest threshold.

`timeoutVoteSound` allows exactly these two, and collection emits nothing, so an
emitting step is a consensus step.
-/
theorem timeoutVote_view {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) {n : Nat} {d : TimeoutData} {v : ViewNumber}
    {e : Option CatchupEvidence}
    (he : Output.send (.timeoutVote ⟨d, v, node⟩ e) ∈ (Run.event r n).outputs) :
    (Run.state r n).currentView = v ∨ Run.Consumes r n (Input.timeoutOneHonest v) := by
  have ht : Transition cfg (SafetySpec cfg node)
      (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
    Run.transition r n
  cases hev : Run.event r n with
  | collect => rw [hev] at he; exact absurd he (by simp [Event.outputs])
  | consensus input output =>
    rw [hev] at ht he
    cases ht with
    | step hs =>
      rcases (SafetySpec.timeoutVoteSound hs _ e he).2.2.2 with ⟨-, hcur⟩ | ⟨hin, -⟩
      · exact Or.inl hcur.symm
      · exact Or.inr ⟨output, by rw [hev, hin]⟩

/--
A vote2 is cast holding a `Cert1` over the same block, at the
vote's own view, which is above genesis.

`Vote2Justification.certMatches` supplies the certificate; the view is fixed by
the keying of `cert1s`, and the genesis exclusion is discharged rather than
assumed — the vote required an admitted proposal, and nothing is ever admitted at
genesis.
-/
theorem vote2_holds_cert1 {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) (hstart : Run.state r 0 = NodeState.initial cfg)
    (hcfg : ConfigCoherent cfg) {n : Nat} {v : Vote2}
    (he : Output.send (.vote2 v) ∈ (Run.event r n).outputs) :
    ∃ c1 : Cert1, (Run.state r (n + 1)).cert1s v.view = some c1 ∧ c1.view = v.view
      ∧ c1.data.blockHash = v.data.blockHash ∧ c1.data.epoch = v.data.epoch
      ∧ v.view ≠ ViewNumber.genesis := by
  have hreach : Reachable cfg (SafetySpec cfg node)
      (NodeState.initial cfg) (Run.state r (n + 1)) := by
    have := reachable_of_run r (n + 1)
    rwa [hstart] at this
  have ht : Transition cfg (SafetySpec cfg node)
      (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
    Run.transition r n
  cases hev : Run.event r n with
  | collect => rw [hev] at he; exact absurd he (by simp [Event.outputs])
  | consensus input output =>
    rw [hev] at ht he
    cases ht with
    | step hs =>
      obtain ⟨p, hj, hview, -, hhash, hep⟩ := SafetySpec.vote2Justified hs _ he
      obtain ⟨c1, hc1, hc1hash, hc1ep⟩ := Vote2Justification.certMatches hj
      have hgen : ViewNumber.genesis < p.viewNumber :=
        admitted_above_genesis cfg hreach _ p (Vote2Justification.proposalAdmitted hj)
      refine ⟨c1, by rw [hview]; exact hc1, ?_, by rw [hc1hash, hhash], by rw [hc1ep, hep], ?_⟩
      · rw [cert1_keyed cfg hcfg hreach _ c1 hc1]; exact hview.symm
      · rw [hview]
        exact fun hc => absurd (hc ▸ hgen) (Nat.lt_irrefl _)

/-- Two valid `Cert1`s share an honest signer, who voted for both. -/
theorem valid1_shared {C : Committee} {N : Network cfg C} {c c' : Cert1}
    (h : Network.ValidCert1 cfg N c) (h' : Network.ValidCert1 cfg N c')
    (he : c.data.epoch = c'.data.epoch) :
    ∃ k, ∃ hh : C.honest k, ∃ n m,
      Output.send (.vote1 ⟨c.data, c.view, k⟩) ∈ (Run.event (N.run k hh) n).outputs
        ∧ Output.send (.vote1 ⟨c'.data, c'.view, k⟩) ∈ (Run.event (N.run k hh) m).outputs := by
  obtain ⟨q, hq, hc⟩ := h
  obtain ⟨q', hq', hc'⟩ := h'
  obtain ⟨k, hk, hk', hh⟩ := C.intersect _ q q' hq (he ▸ hq')
  obtain ⟨n, o, hmem, rfl⟩ := hc k hk hh
  obtain ⟨m, o', hmem', rfl⟩ := hc' k hk' hh
  exact ⟨k, hh, n, m, hmem, hmem'⟩

/-- As `valid1_shared`, for `Cert2`s. -/
theorem valid2_shared {C : Committee} {N : Network cfg C} {c c' : Cert2}
    (h : Network.ValidCert2 cfg N c) (h' : Network.ValidCert2 cfg N c')
    (he : c.data.epoch = c'.data.epoch) :
    ∃ k, ∃ hh : C.honest k, ∃ n m,
      Output.send (.vote2 ⟨c.data, c.view, k⟩) ∈ (Run.event (N.run k hh) n).outputs
        ∧ Output.send (.vote2 ⟨c'.data, c'.view, k⟩) ∈ (Run.event (N.run k hh) m).outputs := by
  obtain ⟨q, hq, hc⟩ := h
  obtain ⟨q', hq', hc'⟩ := h'
  obtain ⟨k, hk, hk', hh⟩ := C.intersect _ q q' hq (he ▸ hq')
  obtain ⟨n, o, hmem, rfl⟩ := hc k hk hh
  obtain ⟨m, o', hmem', rfl⟩ := hc' k hk' hh
  exact ⟨k, hh, n, m, hmem, hmem'⟩

/-! ## Where a certificate came from

A `Cert1` above genesis is not in the state of a fresh node, and no step invents
one, so a node holding it took it from an input at some earlier step. That step
is what `Network.cert1Delivered` locates in time.
-/

/-- A held `Cert1` above genesis was delivered at a strictly earlier step. -/
theorem cert1_from_input {cfg : Config} {node : PubKey} (hcfg : ConfigCoherent cfg)
    (r : Run cfg (SafetySpec cfg node)) (hstart : Run.state r 0 = NodeState.initial cfg) :
    ∀ n v c, c.view ≠ ViewNumber.genesis → (Run.state r n).cert1s v = some c →
      ∃ m, m < n ∧ (Run.Consumes r m (Input.certificate1 c)
        ∨ Run.Consumes r m (Input.advanceView c)) := by
  intro n
  induction n with
  | zero =>
    intro v c hne hc
    rw [hstart] at hc
    simp only [NodeState.initial] at hc
    split at hc
    · exact absurd (hcfg.anchorCertView ▸ (Option.some.inj hc) ▸ rfl) hne
    · exact absurd hc (by simp)
  | succ n ih =>
    intro v c hne hc
    have ht : Transition cfg (SafetySpec cfg node) (Run.state r n) (Run.event r n)
        (Run.state r (n + 1)) := Run.transition r n
    cases hev : Run.event r n with
    | consensus input output =>
      rw [hev] at ht
      cases ht with
      | step hs =>
        rcases SafetySpec.cert1Provenance hs v c hc with hold | ⟨hinput, -⟩
        · obtain ⟨m, hlt, hm⟩ := ih v c hne hold
          exact ⟨m, Nat.lt_succ_of_lt hlt, hm⟩
        · refine ⟨n, Nat.lt_succ_self n, ?_⟩
          rcases hinput with h | h
          · exact Or.inl ⟨output, by rw [hev, h]⟩
          · exact Or.inr ⟨output, by rw [hev, h]⟩
    | collect =>
      rw [hev] at ht
      cases ht with
      | collect hg =>
        obtain ⟨m, hlt, hm⟩ := ih v c hne ((GcSpec.shrinks hg).cert1s v c hc)
        exact ⟨m, Nat.lt_succ_of_lt hlt, hm⟩

/-- Being locked on a `Cert1` above genesis means it was delivered earlier. -/
theorem lock_from_input {cfg : Config} {node : PubKey} (hcfg : ConfigCoherent cfg)
    (r : Run cfg (SafetySpec cfg node)) (hstart : Run.state r 0 = NodeState.initial cfg) :
    ∀ n c, c.view ≠ ViewNumber.genesis → (Run.state r n).lockedCert = some c →
      ∃ m, m < n ∧ (Run.Consumes r m (Input.certificate1 c)
        ∨ Run.Consumes r m (Input.advanceView c)) := by
  intro n
  induction n with
  | zero =>
    intro c hne hl
    rw [hstart] at hl
    simp only [NodeState.initial] at hl
    exact absurd hl (by simp)
  | succ n ih =>
    intro c hne hl
    have ht : Transition cfg (SafetySpec cfg node) (Run.state r n) (Run.event r n)
        (Run.state r (n + 1)) := Run.transition r n
    cases hev : Run.event r n with
    | consensus input output =>
      rw [hev] at ht
      cases ht with
      | step hs =>
        rcases SafetySpec.lockJustified hs c hl with hold | ⟨-, hcert, -⟩
        · obtain ⟨m, hlt, hm⟩ := ih c hne hold
          exact ⟨m, Nat.lt_succ_of_lt hlt, hm⟩
        · exact cert1_from_input hcfg r hstart (n + 1) c.view c hne hcert
    | collect =>
      rw [hev] at ht
      cases ht with
      | collect hg =>
        obtain ⟨m, hlt, hm⟩ := ih c hne ((GcSpec.lockSame hg).symm ▸ hl)
        exact ⟨m, Nat.lt_succ_of_lt hlt, hm⟩

/--
A `Cert1` an honest node holds is one a quorum signed.

Not assumed: a held certificate was delivered (`cert1_from_input`) and what
was delivered was signed (`Network.cert1Delivered`). The ordering is what
collapses the two into one — without it, delivery could only be assumed at the
state the certificate is held in, which is the same statement over again.
-/
theorem cert1_backed {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (k : PubKey) (h : C.honest k) (n : Nat) (v : ViewNumber) (c : Cert1)
    (hne : c.view ≠ ViewNumber.genesis)
    (hheld : (Run.state (N.run k h) n).cert1s v = some c) : Cert1Backed N.run c := by
  obtain ⟨m, -, hdel⟩ :=
    cert1_from_input hcfg (N.run k h) (N.start k h) n v c hne hheld
  exact (N.cert1Delivered k h m c hne hdel).backed

/--
A lock a node holds is a certificate the network really formed.

The same route as `cert1_backed`, from the other slot: a lock is only ever
taken on a certificate that arrived, and what arrived was signed. Used where
the argument needs the *epoch* a lock names, which is its block's only because
a quorum signed it so.
-/
theorem lock_backed {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (k : PubKey) (h : C.honest k) (n : Nat) (c : Cert1)
    (hne : c.view ≠ ViewNumber.genesis)
    (hheld : (Run.state (N.run k h) n).lockedCert = some c) : Cert1Backed N.run c := by
  obtain ⟨m, -, hdel⟩ := lock_from_input hcfg (N.run k h) (N.start k h) n c hne hheld
  exact (N.cert1Delivered k h m c hne hdel).backed

/--
An honest node was really in view `v` when one-honest evidence for `v` exists.

Descent on `Network.Before`: the evidence follows an honest node's timeout vote
for `v` (`Network.timeoutOneHonestBacked`), and by `timeoutVoteSound` that node
either had its own timer fire while in `v` — which is the conclusion — or was
itself answering one-honest evidence, which follows a strictly earlier honest
vote. Well-foundedness ends the chain.
-/
theorem oneHonest_reached {C : Committee} (N : Network cfg C)
    (k : PubKey) (h : C.honest k) (n : Nat) (v : ViewNumber)
    (hcons : Run.Consumes (N.run k h) n (Input.timeoutOneHonest v)) :
    ∃ j, ∃ hj : C.honest j, ∃ m, (Run.state (N.run j hj) m).currentView = v := by
  suffices H : ∀ s : NodeStep C,
      (∃ e d, Output.send (.timeoutVote ⟨d, v, s.node⟩ e)
          ∈ (Run.event (N.run s.node s.honest) s.index).outputs) →
      ∃ j, ∃ hj : C.honest j, ∃ m, (Run.state (N.run j hj) m).currentView = v by
    obtain ⟨j, hj, m, e, d, hmem, -⟩ := N.timeoutOneHonestBacked k h n v hcons
    exact H ⟨j, hj, m⟩ ⟨e, d, hmem⟩
  intro s
  induction s using N.beforeWF.induction with
  | _ s ih =>
    intro ⟨e, d, hmem⟩
    obtain ⟨input, output, hev, hs, hin⟩ := emit_step (N.run s.node s.honest) hmem
    obtain ⟨-, -, -, hfired | ⟨hinput, -⟩⟩ :=
      SafetySpec.timeoutVoteSound hs ⟨d, v, s.node⟩ e hin
    · exact ⟨s.node, s.honest, s.index, hfired.2.symm⟩
    · obtain ⟨j, hj, m, e', d', hmem', hbefore⟩ :=
        N.timeoutOneHonestBacked s.node s.honest s.index v ⟨output, by rw [hev, hinput]⟩
      exact ih ⟨j, hj, m⟩ hbefore ⟨e', d', hmem'⟩

/-! ## The first signer

A quorum's votes for a `Cert1` precede its delivery, so a node acting on that
certificate acted after votes it was not one of. Following that back cannot go
on for ever (`Network.beforeWF`), which is what says some signer voted without
the certificate in hand.
-/

/--
Among a quorum's honest members that signed `c1`, one did so without being
locked on `c1` at the end of the step it voted in.

Descent on `Network.Before`: a member that *was* locked on it was delivered it
earlier (`lock_from_input`), and what was delivered was signed earlier still
(`Network.cert1Delivered`) — by a quorum that meets `q` in an honest node, whose
own vote is therefore strictly earlier. Well-foundedness ends the chain.
-/
theorem exists_signer_unlocked {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) {c1 : Cert1} (hne : c1.view ≠ ViewNumber.genesis)
    {q : PubKey → Prop} (hq : C.Quorum c1.data.epoch q)
    (hsome : ∃ k, ∃ h : C.honest k, ∃ n, q k ∧
      Output.send (.vote1 ⟨c1.data, c1.view, k⟩) ∈ (Run.event (N.run k h) n).outputs) :
    ∃ k, ∃ h : C.honest k, ∃ n, q k ∧
      Output.send (.vote1 ⟨c1.data, c1.view, k⟩) ∈ (Run.event (N.run k h) n).outputs ∧
      (Run.state (N.run k h) (n + 1)).lockedCert ≠ some c1 := by
  obtain ⟨k₀, h₀, n₀, hq₀, hmem₀⟩ := hsome
  suffices H : ∀ s : NodeStep C, q s.node →
      Output.send (.vote1 ⟨c1.data, c1.view, s.node⟩)
          ∈ (Run.event (N.run s.node s.honest) s.index).outputs →
      ∃ k, ∃ h : C.honest k, ∃ n, q k ∧
        Output.send (.vote1 ⟨c1.data, c1.view, k⟩) ∈ (Run.event (N.run k h) n).outputs ∧
        (Run.state (N.run k h) (n + 1)).lockedCert ≠ some c1 from
    H ⟨k₀, h₀, n₀⟩ hq₀ hmem₀
  intro s
  induction s using N.beforeWF.induction with
  | _ s ih =>
    intro hqs hmem
    by_cases hheld : (Run.state (N.run s.node s.honest) (s.index + 1)).lockedCert = some c1
    · obtain ⟨m, hlt, hdel⟩ := lock_from_input hcfg (N.run s.node s.honest)
        (N.start s.node s.honest) (s.index + 1) c1 hne hheld
      obtain ⟨q', hq', hvotes⟩ := N.cert1Delivered s.node s.honest m c1 hne hdel
      obtain ⟨k', hk'q, hk'B, hh'⟩ := C.intersect _ q' q hq' hq
      obtain ⟨n', hmem', hbefore⟩ := hvotes k' hk'q hh'
      refine ih ⟨k', hh', n'⟩ ?_ hk'B hmem'
      rcases Nat.lt_or_ge m s.index with hm | hm
      · exact N.beforeTrans hbefore (Network.before_of_lt cfg N s.node s.honest hm)
      · have hme : m = s.index := Nat.le_antisymm (Nat.lt_succ_iff.mp hlt) hm
        subst hme
        exact hbefore
    · exact ⟨s.node, s.honest, s.index, hqs, hmem, hheld⟩

end NewProtocol
