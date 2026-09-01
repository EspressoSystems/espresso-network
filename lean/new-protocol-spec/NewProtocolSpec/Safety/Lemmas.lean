module

public import NewProtocolSpec.Assumptions
public import NewProtocolSpec.Safety.Defs

/-!
# Working lemmas for decide safety

Kernel-checked scaffolding for `NewProtocolSpec.Safety`; an audit can skip the
file. Two of its results carry the argument, and their docstrings are worth
reading even though their proofs need not be:

* `no_gap` — no `parentCert` link steps over a committed view. This is where
  the three vote rules are combined, and it is the heart of the safety argument.
* `cert2_ancestor` — the induction down the `parentCert` links that turns
  `no_gap` into ancestry.

The rest are facts that survive between two actions of one honest node, shaped
like `retired1_stable`: a step may add, collection may only drop what is out of
reach.
-/

@[expose] public section

namespace NewProtocol

variable (tree : BlockTable)

/-- Views are equal when their numbers are. -/
theorem view_eq {a b : ViewNumber} (h : a.toNat = b.toNat) : a = b := by
  cases a; cases b; simp_all

/-! ## What a node's own history fixes

The safety argument compares two actions of one honest node, so it needs facts
that survive between them. Shaped like `retired1_stable`: a step may add,
collection may only drop what is out of reach.
-/

/-- The lock never moves to an older certificate. -/
theorem lock_stable {cfg : Config} {node : PubKey}
    {s s' : NodeState} {e : Event} {lock : Cert1}
    (ht : Transition cfg (SafetySpec cfg node) s e s') (h : s.lockedCert = some lock) :
    ∃ lock', s'.lockedCert = some lock' ∧ lock.view ≤ lock'.view := by
  cases ht with
  | step hs => exact SafetySpec.lockMono hs lock h
  | collect hg => exact ⟨lock, by rw [GcSpec.lockSame hg]; exact h, Nat.le_refl _⟩

/-- So the lock's view is monotone along a run. -/
theorem lock_run {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) {lock : Cert1} {n : Nat}
    (h : (Run.state r n).lockedCert = some lock) :
    ∀ m, n ≤ m → ∃ lock', (Run.state r m).lockedCert = some lock' ∧ lock.view ≤ lock'.view := by
  intro m hm
  induction hm with
  | refl => exact ⟨lock, h, Nat.le_refl _⟩
  | step _ ih =>
    obtain ⟨mid, hmid, hle⟩ := ih
    obtain ⟨next, hnext, hle'⟩ := lock_stable (Run.transition r _) hmid
    exact ⟨next, hnext, Nat.le_trans hle hle'⟩

/--
Node `k` has the branch it endorsed at `w` on record, or `w` has fallen out of
reach.

The same shape as `Retired1`: what a node did stays known for exactly as long as
it can still matter. Collection may drop the record below the decide floor, and
`SafetySpec.vote2AboveFloor` says no vote2 happens down there either.
-/
def Recorded (cfg : Config) (s : NodeState) (w u : ViewNumber) : Prop :=
  s.vote1Branches w = some u ∨ ¬ s.aboveDecideFloor cfg w

theorem recorded_stable {cfg : Config} {node : PubKey}
    {s s' : NodeState} {e : Event} {w u : ViewNumber}
    (ht : Transition cfg (SafetySpec cfg node) s e s') (h : Recorded cfg s w u) :
    Recorded cfg s' w u := by
  cases ht with
  | step hs =>
    rcases h with hrec | hout
    · exact Or.inl (SafetySpec.vote1BranchesRetained hs w u hrec)
    · exact Or.inr fun hc => hout fun x hx => hc x (SafetySpec.decidedRetained hs x hx)
  | collect hg =>
    rcases h with hrec | hout
    · by_cases hab : s.aboveDecideFloor cfg w
      · exact Or.inl (GcSpec.vote1BranchesRetained hg w u hab hrec)
      · exact Or.inr fun hc => hab (GcSpec.floorStable hg _ hc)
    · exact Or.inr fun hc => hout (GcSpec.floorStable hg _ hc)

/-- So a branch record, once made, is still available whenever it could matter. -/
theorem recorded_run {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) {w u : ViewNumber} {n : Nat}
    (h : Recorded cfg (Run.state r n) w u) :
    ∀ m, n ≤ m → Recorded cfg (Run.state r m) w u := by
  intro m hm
  induction hm with
  | refl => exact h
  | step _ ih => exact recorded_stable (Run.transition r _) ih

/-- Being above the decide floor is inherited by later views. -/
theorem aboveDecideFloor_mono {cfg : Config} {s : NodeState} {v w : ViewNumber}
    (h : s.aboveDecideFloor cfg v) (hle : v ≤ w) : s.aboveDecideFloor cfg w :=
  fun x hx => Nat.lt_of_lt_of_le (h x hx) hle

/-- A `Cert1` is determined by its view and the hash it certifies. -/
theorem cert1_eq {a b : Cert1} (hv : a.view = b.view)
    (hh : a.data.blockHash = b.data.blockHash) : a = b := by
  obtain ⟨⟨ab⟩, av⟩ := a
  obtain ⟨⟨bb⟩, bv⟩ := b
  simp_all

/--
A quorum-backed `Cert1` names a block, and its view is that block's.

Read off any honest signer: `vote1Justified` ties the vote's view and data to
the proposal voted on. This is what lets two certificates be compared by data
and concluded equal in view.
-/
theorem cert1Backed_block {C : Committee} {run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k)} {c : Cert1}
    (h : Cert1Backed run c) :
    ∃ b : Proposal, c.view = b.viewNumber ∧ c.data.blockHash = blockHash b := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect q q hq hq
  obtain ⟨n, o, hm, ho⟩ := hcast k hk hhon
  subst ho
  obtain ⟨input, output, -, hs, hin⟩ := emit_step (run k hhon) hm
  obtain ⟨b, -, hview, -, hhash⟩ :=
    SafetySpec.vote1Justified hs ⟨c.data, c.view, k⟩ hin
  exact ⟨b, hview, hhash⟩

/-! ## Views a node's own state fixes -/

/-- A held proposal sits under the view it names. -/
theorem proposals_keyed {cfg : Config} {node : PubKey}
    (hcfg : ConfigCoherent cfg) {s : NodeState}
    (hr : Reachable cfg (SafetySpec cfg node) (NodeState.initial cfg) s) :
    ∀ v p, s.proposals v = some p → p.viewNumber = v := by
  induction hr with
  | refl =>
    intro v p hp
    simp only [NodeState.initial] at hp
    split at hp
    · next hv => cases hp; rw [hcfg.anchorBlockView]; exact hv.symm
    · exact nomatch hp
  | step _ ht ih =>
    cases ht with
    | step hs =>
      intro v p hp
      rcases SafetySpec.proposalProvenance hs v p hp with hold | ⟨-, hv, -⟩
      · exact ih v p hold
      · exact hv
    | collect hg => intro v p hp; exact ih v p ((GcSpec.shrinks hg).proposals v p hp)

/-- Likewise for an admitted one. -/
theorem admitted_keyed {cfg : Config} {node : PubKey}
    {s : NodeState} (hr : Reachable cfg (SafetySpec cfg node) (NodeState.initial cfg) s) :
    ∀ v p, s.admitted v = some p → p.viewNumber = v := by
  induction hr with
  | refl => intro v p hp; simp [NodeState.initial] at hp
  | step _ ht ih =>
    cases ht with
    | step hs =>
      intro v p hp
      rcases SafetySpec.admissionJustified hs v p hp with hold | ⟨-, -, -, hv, -⟩
      · exact ih v p hold
      · exact hv
    | collect hg => intro v p hp; exact ih v p ((GcSpec.shrinks hg).admitted v p hp)

/--
A held lock above genesis names a block at the lock's own view.

Stated as a bare existential rather than over the current state, so it survives
collection: the block it names may be forgotten, the fact that the lock names
one at that view may not.
-/
def LockNamed (s : NodeState) : Prop :=
  ∀ lock, s.lockedCert = some lock → lock.view ≠ ViewNumber.genesis →
    ∃ b : Proposal, b.viewNumber = lock.view ∧ blockHash b = lock.data.blockHash

theorem lockNamed_stable {cfg : Config} {node : PubKey}
    {s s' : NodeState} {e : Event}
    (ht : Transition cfg (SafetySpec cfg node) s e s')
    (hkey : ∀ v p, s'.proposals v = some p → p.viewNumber = v)
    (h : LockNamed s) : LockNamed s' := by
  cases ht with
  | step hs =>
    intro lock hl hne
    rcases SafetySpec.lockJustified hs lock hl with hold | ⟨-, -, b, hadm, hhash, -⟩
    · exact h lock hold hne
    · exact ⟨b, hkey _ _ hadm, hhash⟩
  | collect hg =>
    intro lock hl hne
    exact h lock (by rw [← GcSpec.lockSame hg]; exact hl) hne

/-- So it holds at every point of a run that starts where the specification does. -/
theorem lockNamed_run {cfg : Config} {node : PubKey}
    (hcfg : ConfigCoherent cfg) (r : Run cfg (SafetySpec cfg node))
    (hstart : Run.state r 0 = NodeState.initial cfg) :
    ∀ n, LockNamed (Run.state r n)
  | 0 => by rw [hstart]; intro lock hl; simp [NodeState.initial] at hl
  | n + 1 =>
    lockNamed_stable (Run.transition r n)
      (proposals_keyed hcfg (hstart ▸ reachable_of_run r (n + 1)))
      (lockNamed_run hcfg r hstart n)

/-- What an admitted proposal carries: well-formedness, a held copy, a delivery. -/
theorem admitted_facts {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) (hstart : Run.state r 0 = NodeState.initial cfg) :
    ∀ n v p, (Run.state r n).admitted v = some p →
      ProposalWellFormed cfg p ∧ (∃ m, (Run.state r m).proposals v = some p) := by
  intro n
  induction n with
  | zero => intro v p hp; rw [hstart] at hp; simp [NodeState.initial] at hp
  | succ n ih =>
    intro v p hp
    have ht : Transition cfg (SafetySpec cfg node)
        (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
      Run.transition r n
    cases hev : Run.event r n with
    | consensus input output =>
      rw [hev] at ht
      cases ht with
      | step hs =>
        rcases SafetySpec.admissionJustified hs v p hp with hold | ⟨-, -, -, -, -, -, hwf, -, -, hprop⟩
        · exact ih v p hold
        · exact ⟨hwf, n + 1, hprop⟩
    | collect =>
      rw [hev] at ht
      cases ht with
      | collect hg => exact ih v p ((GcSpec.shrinks hg).admitted v p hp)

/-- An admitted proposal was delivered to this node at some step. -/
theorem admitted_consumed {cfg : Config} {node : PubKey}
    (r : Run cfg (SafetySpec cfg node)) (hstart : Run.state r 0 = NodeState.initial cfg) :
    ∀ n v p, (Run.state r n).admitted v = some p →
      ∃ m sender vid, Run.Consumes r m (Input.proposal sender p vid) := by
  intro n
  induction n with
  | zero => intro v p hp; rw [hstart] at hp; simp [NodeState.initial] at hp
  | succ n ih =>
    intro v p hp
    have ht : Transition cfg (SafetySpec cfg node)
        (Run.state r n) (Run.event r n) (Run.state r (n + 1)) :=
      Run.transition r n
    cases hev : Run.event r n with
    | consensus input output =>
      rw [hev] at ht
      cases ht with
      | step hs =>
        rcases SafetySpec.admissionJustified hs v p hp with hold | ⟨sender, vid, hinput, -⟩
        · exact ih v p hold
        · exact ⟨n, sender, vid, output, by rw [hev, hinput]⟩
    | collect =>
      rw [hev] at ht
      cases ht with
      | collect hg => exact ih v p ((GcSpec.shrinks hg).admitted v p hp)

/--
No gap over a committed view.

Given a `Cert2` at `c.view` and a `Cert1` at a later view over proposal `p`, the
parent `p` names cannot sit below `c.view`. A branch omitting `c.view` needs
exactly such a gap, so this is what makes decide safety go through.

The node the argument runs on is not just any member of both quorums. It is one
that signed `c1` *without being locked on `c1`* (`exists_signer_unlocked`),
which exists because a node locked on a certificate was delivered it after a
quorum had already signed it, and that regress cannot descend for ever. With
such a node, no order of its two votes is available:

* vote1 first — the record it kept (`vote1Records`) makes `vote2NotInSkippedView`
  refuse the vote2;
* vote2 first — its lock is at `c.view` or beyond (`vote2LockOrdered`,
  `lock_run`), and `Vote1Justification.safeToExtend` then refuses the vote1: the
  equal-view branch would make the lock *be* `c1`, which is what this node was
  chosen to exclude; the liveness branch contradicts the bound outright; and the
  safety branch would make the parent certificate and the lock name one block at
  two views.
-/
theorem no_gap {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) {c : Cert2} {c1 : Cert1} {p : Proposal}
    (hc2 : Network.ValidCert2 cfg N c)
    (hc1 : Network.ValidCert1 cfg N c1)
    (hph : c1.data.blockHash = blockHash p)
    (hlt : c.view < c1.view) :
    c.view ≤ p.parentCert.view := by
  refine Nat.le_of_not_lt fun hgap => ?_
  obtain ⟨q1, hq1, hcast1⟩ := hc1
  obtain ⟨q2, hq2, hcast2⟩ := hc2
  have hgen : c1.view ≠ ViewNumber.genesis := fun hz =>
    absurd (hz ▸ hlt) (Nat.not_lt_zero _)
  -- The signer is not just any member of both quorums: it is one that signed
  -- `c1` without being locked on `c1`, which is what rules out the branch of
  -- `SafeToExtend` that asks nothing about the parent.
  obtain ⟨k, hhon, n1, hk2, hm1, hunlocked⟩ :=
    exists_signer_unlocked cfg N hcfg hgen hq2
      (by
        obtain ⟨j, hj1, hj2, hjh⟩ := C.intersect q1 q2 hq1 hq2
        obtain ⟨m, o, hmem, ho⟩ := hcast1 j hj1 hjh
        exact ⟨j, hjh, m, hj2, ho ▸ hmem⟩)
  obtain ⟨n2, o2, hm2, ho2⟩ := hcast2 k hk2 hhon
  subst ho2
  obtain ⟨i1, out1, -, hs1, hin1⟩ := emit_step (N.run k hhon) hm1
  obtain ⟨i2, out2, -, hs2, hin2⟩ := emit_step (N.run k hhon) hm2
  obtain ⟨p', hjust1, hview1, -, hhash1⟩ :=
    SafetySpec.vote1Justified hs1 ⟨c1.data, c1.view, k⟩ hin1
  -- The proposal the vote1 signed is `p`.
  have hpp : p' = p := hcf p' p (by rw [← hhash1]; exact hph)
  subst hpp
  have hadm : (Run.state (N.run k hhon) (n1 + 1)).admitted c1.view = some p' := by
    have h := hjust1.proposalAdmitted
    rw [← hview1] at h
    exact h
  rcases Nat.lt_or_ge n2 n1 with hgt | hle
  · -- Vote2 first: the lock it took forbids the vote1.
    obtain ⟨lock, hlock, hlockv⟩ := SafetySpec.vote2LockOrdered hs2 ⟨c.data, c.view, k⟩ hin2
    obtain ⟨lk, hlk, hle2⟩ :=
      lock_run (N.run k hhon) hlock (n1 + 1) (Nat.succ_le_succ (Nat.le_of_lt hgt))
    have hcv : c.view ≤ lk.view := Nat.le_trans hlockv hle2
    have hsafe := hjust1.safeToExtend
    rw [hlk] at hsafe
    simp only [SafeToExtend] at hsafe
    -- Were the lock in the proposal's own view it would *be* `c1`, over the same
    -- block in the same view — and this signer was chosen for not being locked
    -- on `c1`.
    have hne : ¬ (lk.view = p'.viewNumber) := by
      intro hh
      rw [if_pos hh] at hsafe
      have heq : lk = c1 := cert1_eq (hh.trans hview1.symm) (hsafe.trans hhash1.symm)
      exact hunlocked (heq ▸ hlk)
    rw [if_neg hne] at hsafe
    rcases hsafe with heq | hlow
    · -- The parent certificate and the lock would name one block at two views.
      obtain ⟨m, sender, vid, hcons⟩ := admitted_consumed (N.run k hhon) (N.start k hhon)
        (n1 + 1) c1.view p' hadm
      obtain ⟨pb, hpbv, hpbh⟩ :=
        cert1Backed_block (N.parentCertValid k hhon m sender p' vid hcons)
      obtain ⟨b, hbv, hbh⟩ :=
        lockNamed_run hcfg (N.run k hhon) (N.start k hhon) (n1 + 1) lk hlk
          (fun hz => absurd (hz ▸ Nat.lt_of_lt_of_le hgap hcv) (Nat.not_lt_zero _))
      have hpb : pb = b := hcf pb b (by rw [← hpbh, heq, hbh])
      rw [hpb, hbv] at hpbv
      exact absurd (hpbv ▸ Nat.lt_of_lt_of_le hgap hcv) (Nat.lt_irrefl _)
    · exact absurd (Nat.lt_trans hlow hgap) (Nat.not_lt.mpr hcv)
  · -- Vote1 first: the record it kept forbids the vote2.
    obtain ⟨p2, hadm2, hbr⟩ := SafetySpec.vote1Records hs1 ⟨c1.data, c1.view, k⟩ hin1
    have hpe : p2 = p' := by
      rw [hadm] at hadm2; exact (Option.some_inj.mp hadm2).symm
    rw [hpe] at hbr
    have hrec := recorded_run (N.run k hhon)
      (Or.inl hbr : Recorded cfg _ c1.view p'.parentCert.view) (n2 + 1) (Nat.succ_le_succ hle)
    have hfloor : (Run.state (N.run k hhon) (n2 + 1)).aboveDecideFloor cfg c1.view :=
      aboveDecideFloor_mono
        (SafetySpec.vote2AboveFloor hs2 ⟨c.data, c.view, k⟩ hin2) (Nat.le_of_lt hlt)
    rcases hrec with hbr2 | hno
    · exact SafetySpec.vote2NotInSkippedView hs2 ⟨c.data, c.view, k⟩ hin2
        ⟨c1.view, p'.parentCert.view, hlt, hbr2, hgap⟩
    · exact hno hfloor

/-- Everything a valid `Cert1` gives about the block it certifies. -/
theorem cert1_proposal {C : Committee} (N : Network cfg C)
    (hres : Resolves tree N) {c1 : Cert1} (h : Network.ValidCert1 cfg N c1) :
    ∃ p : Proposal, c1.view = p.viewNumber ∧ c1.data.blockHash = blockHash p
      ∧ ProposalWellFormed cfg p ∧ tree (blockHash p) = some p
      ∧ Network.ValidCert1 cfg N p.parentCert := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect q q hq hq
  obtain ⟨n, o, hm, ho⟩ := hcast k hk hhon
  subst ho
  obtain ⟨input, output, -, hs, hin⟩ := emit_step (N.run k hhon) hm
  obtain ⟨p, hjust, hview, -, hhash⟩ :=
    SafetySpec.vote1Justified hs ⟨c1.data, c1.view, k⟩ hin
  obtain ⟨hwf, m, hprop⟩ :=
    admitted_facts (N.run k hhon) (N.start k hhon) (n + 1) p.viewNumber p hjust.proposalAdmitted
  obtain ⟨m', sender, vid, hcons⟩ :=
    admitted_consumed (N.run k hhon) (N.start k hhon) (n + 1) p.viewNumber p
      hjust.proposalAdmitted
  exact ⟨p, hview, hhash, hwf, hres k hhon m p.viewNumber p hprop,
    N.parentCertValid k hhon m' sender p vid hcons⟩

/--
A `Cert2` at `c.view` is an ancestor of every certified block at or above it.

Strong induction down the `parentCert` links: `no_gap` says no link steps over
`c.view`, so the walk from the later block reaches `c.view` itself, where
`cert2_implies_cert1` and `cert1_unique` identify the two blocks.
-/
theorem cert2_ancestor {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hcf : CollisionFree) (hres : Resolves tree N)
    {c : Cert2} (hc2 : Network.ValidCert2 cfg N c) :
    ∀ n : Nat, ∀ c1 : Cert1, c1.view.toNat ≤ n → Network.ValidCert1 cfg N c1 →
      c.view ≤ c1.view → Ancestor tree c.data.blockHash c1.data.blockHash := by
  have hbase : ∀ c1 : Cert1, Network.ValidCert1 cfg N c1 → c1.view = c.view →
      Ancestor tree c.data.blockHash c1.data.blockHash := by
    intro c1 hc1 hveq
    obtain ⟨c1', hb', hv', hh'⟩ := cert2_implies_cert1 cfg N hcfg hc2
    have := cert1_unique cfg N hc1 hb' (by rw [hveq, hv'])
    rw [this, hh']
    exact Ancestor.refl _
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1 hge
    exact hbase c1 hc1 (view_eq (Nat.le_antisymm (Nat.le_trans hle (Nat.zero_le _)) hge))
  | succ n ih =>
    intro c1 hle hc1 hge
    rcases Nat.eq_or_lt_of_le hge with heq | hlt
    · exact hbase c1 hc1 (view_eq heq.symm)
    · obtain ⟨p, hview, hhash, hwf, htree, hpar⟩ := cert1_proposal tree N hres hc1
      have hgap : c.view ≤ p.parentCert.view := no_gap N hcfg hcf hc2 hc1 hhash hlt
      have hlt' : p.parentCert.view.toNat < c1.view.toNat := by
        rw [hview]; exact hwf.1
      refine Ancestor.step (hhash ▸ htree) ?_
      exact ih p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hlt' hle)) hpar hgap

end NewProtocol
