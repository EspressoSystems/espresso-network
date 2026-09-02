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

/-- A `Cert1` is determined by its view, the hash it certifies, and its epoch. -/
theorem cert1_eq {a b : Cert1} (hv : a.view = b.view)
    (hh : a.data.blockHash = b.data.blockHash) (he : a.data.epoch = b.data.epoch) : a = b := by
  obtain ⟨⟨ab, ae⟩, av⟩ := a
  obtain ⟨⟨bb, be⟩, bv⟩ := b
  simp_all

/--
A quorum-backed `Cert1` names a block, and its view is that block's.

Read off any honest signer: `vote1Justified` ties the vote's view and data to
the proposal voted on. This is what lets two certificates be compared by data
and concluded equal in view.
-/
theorem cert1Backed_block {C : Committee}
    {run : ∀ k, C.honest k → Run cfg (SafetySpec cfg k)} {c : Cert1}
    (h : Cert1Backed run c) :
    ∃ b : Proposal, c.view = b.viewNumber ∧ c.data.blockHash = blockHash b
      ∧ c.data.epoch = b.epoch := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect c.data.epoch q q hq hq
  obtain ⟨n, o, hm, ho⟩ := hcast k hk hhon
  subst ho
  obtain ⟨input, output, -, hs, hin⟩ := emit_step (run k hhon) hm
  obtain ⟨b, -, hview, -, hhash, hep⟩ :=
    SafetySpec.vote1Justified hs ⟨c.data, c.view, k⟩ hin
  exact ⟨b, hview, hhash, hep⟩

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
      rcases SafetySpec.proposalProvenance hs v p hp with hold | ⟨-, hv, -⟩ | ⟨-, -, -, hv, -⟩
      · exact ih v p hold
      · exact hv
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
        rcases SafetySpec.admissionJustified hs v p hp with
          hold | ⟨-, -, -, -, -, -, hwf, -, -, hprop⟩
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
    (he : c.data.epoch = c1.data.epoch)
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
    exists_signer_unlocked cfg N hcfg hgen (he ▸ hq2)
      (by
        obtain ⟨j, hj1, hj2, hjh⟩ := C.intersect c1.data.epoch q1 q2 hq1 (he ▸ hq2)
        obtain ⟨m, o, hmem, ho⟩ := hcast1 j hj1 hjh
        exact ⟨j, hjh, m, hj2, ho ▸ hmem⟩)
  obtain ⟨n2, o2, hm2, ho2⟩ := hcast2 k hk2 hhon
  subst ho2
  obtain ⟨i1, out1, -, hs1, hin1⟩ := emit_step (N.run k hhon) hm1
  obtain ⟨i2, out2, -, hs2, hin2⟩ := emit_step (N.run k hhon) hm2
  obtain ⟨p', hjust1, hview1, -, hhash1, hepoch1⟩ :=
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
      -- The lock is a certificate the network formed, so the epoch it names is
      -- its block's — and its block is `p'`, which is `c1`'s too.
      have hlkgen : lk.view ≠ ViewNumber.genesis := by
        rw [hh, ← hview1]; exact hgen
      obtain ⟨lb, -, hlbh, hlbe⟩ :=
        cert1Backed_block (lock_backed cfg N hcfg k hhon (n1 + 1) lk hlkgen hlk)
      have hlbp : lb = p' := hcf lb p' (by rw [← hlbh, hsafe])
      have hlkep : lk.data.epoch = c1.data.epoch := by
        rw [hlbe, hlbp, hepoch1]
      have heq : lk = c1 :=
        cert1_eq (hh.trans hview1.symm) (hsafe.trans hhash1.symm) hlkep
      exact hunlocked (heq ▸ hlk)
    rw [if_neg hne] at hsafe
    rcases hsafe with heq | hlow
    · -- The parent certificate and the lock would name one block at two views.
      obtain ⟨m, sender, vid, hcons⟩ := admitted_consumed (N.run k hhon) (N.start k hhon)
        (n1 + 1) c1.view p' hadm
      obtain ⟨b, hbv, hbh⟩ :=
        lockNamed_run hcfg (N.run k hhon) (N.start k hhon) (n1 + 1) lk hlk
          (fun hz => absurd (hz ▸ Nat.lt_of_lt_of_le hgap hcv) (Nat.not_lt_zero _))
      rcases N.parentCertValid k hhon m sender p' vid hcons with hbk | hanchor
      · obtain ⟨pb, hpbv, hpbh, -⟩ := cert1Backed_block hbk
        have hpb : pb = b := hcf pb b (by rw [← hpbh, heq, hbh])
        rw [hpb, hbv] at hpbv
        exact absurd (hpbv ▸ Nat.lt_of_lt_of_le hgap hcv) (Nat.lt_irrefl _)
      · -- The parent is the anchor, so the lock names the anchor block, at genesis.
        have hba : b = cfg.anchorBlock :=
          hcf b cfg.anchorBlock (by rw [hbh, ← heq, hanchor, hcfg.anchorCertBlock])
        have hlkv : lk.view = ViewNumber.genesis := by
          rw [← hbv, hba, hcfg.anchorBlockView]
        exact absurd (hlkv ▸ Nat.lt_of_lt_of_le hgap hcv) (Nat.not_lt_zero _)
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

/--
Everything a valid `Cert1` gives about the block it certifies.

The last conjunct is the epoch boundary, and it is here rather than in a lemma
of its own because it comes from the same delivery as
`Network.parentCertValid`: the proposal an honest voter admitted was one it was
handed, and both premises read that delivery.
-/
theorem cert1_proposal {C : Committee} (N : Network cfg C)
    (hres : Resolves tree N) {c1 : Cert1} (h : Network.ValidCert1 cfg N c1) :
    ∃ p : Proposal, c1.view = p.viewNumber ∧ c1.data.blockHash = blockHash p
      ∧ c1.data.epoch = p.epoch
      ∧ ProposalWellFormed cfg p ∧ tree (blockHash p) = some p
      ∧ (Network.ValidCert1 cfg N p.parentCert ∨ p.parentCert = cfg.anchorCert)
      ∧ (EntersEpoch cfg p → ∃ bc : Cert2, Cert2Backed N.run bc
          ∧ bc.view = p.parentCert.view
          ∧ bc.data.blockHash = p.parentCert.data.blockHash
          ∧ bc.data.epoch = p.parentCert.data.epoch) := by
  obtain ⟨q, hq, hcast⟩ := h
  obtain ⟨k, hk, -, hhon⟩ := C.intersect c1.data.epoch q q hq hq
  obtain ⟨n, o, hm, ho⟩ := hcast k hk hhon
  subst ho
  obtain ⟨input, output, -, hs, hin⟩ := emit_step (N.run k hhon) hm
  obtain ⟨p, hjust, hview, -, hhash, hepoch⟩ :=
    SafetySpec.vote1Justified hs ⟨c1.data, c1.view, k⟩ hin
  obtain ⟨hwf, m, hprop⟩ :=
    admitted_facts (N.run k hhon) (N.start k hhon) (n + 1) p.viewNumber p hjust.proposalAdmitted
  obtain ⟨m', sender, vid, hcons⟩ :=
    admitted_consumed (N.run k hhon) (N.start k hhon) (n + 1) p.viewNumber p
      hjust.proposalAdmitted
  exact ⟨p, hview, hhash, hepoch, hwf, hres k hhon m p.viewNumber p hprop,
    N.parentCertValid k hhon m' sender p vid hcons,
    N.boundaryDecided k hhon m' sender p vid hcons⟩

/-- A valid `Cert1` cannot sit at genesis: nothing is admitted there to vote for. -/
theorem cert1_above_genesis {C : Committee} (N : Network cfg C)
    (hres : Resolves tree N) {c1 : Cert1} (h : Network.ValidCert1 cfg N c1) :
    c1.view ≠ ViewNumber.genesis := by
  obtain ⟨p, hview, -, -, hwf, -, -, -⟩ := cert1_proposal tree N hres h
  intro hz
  rw [hz] at hview
  exact absurd (hview ▸ hwf.1 : p.parentCert.view < ViewNumber.genesis) (Nat.not_lt_zero _)

/--
The anchor is in the tree.

Every honest node holds it from its first step, and `Resolves` puts what an
honest node holds in the tree. A valid certificate is what supplies the honest
node.
-/
theorem anchor_in_tree {C : Committee} (N : Network cfg C) (hres : Resolves tree N)
    {c1 : Cert1} (hc1 : Network.ValidCert1 cfg N c1) :
    tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock := by
  obtain ⟨q, hq, -⟩ := hc1
  obtain ⟨k, -, -, hhon⟩ := C.intersect c1.data.epoch q q hq hq
  refine hres k hhon 0 ViewNumber.genesis cfg.anchorBlock ?_
  rw [N.start k hhon]
  simp [NodeState.initial]

/--
Nothing the tree resolves is before the anchor.

`AnchorRooted` leaves the anchor's parent link unresolved, so a walk that
reaches the anchor either stops there or asks the tree for a hash it does not
answer. Two case splits, no induction: a derivation cannot get past a step the
tree refuses.
-/
theorem ancestor_anchor {tree : BlockTable} (hroot : AnchorRooted tree cfg)
    (hcoh : TreeCoherent tree) (hcf : CollisionFree) {a : BlockHash} {ba : Block}
    (h : Ancestor tree a (blockHash cfg.anchorBlock)) (hba : tree a = some ba) :
    a = blockHash cfg.anchorBlock := by
  cases h with
  | refl => rfl
  | step ht hrest =>
    rename_i b
    have hb : b = cfg.anchorBlock := hcf b cfg.anchorBlock (hcoh _ b ht)
    subst hb
    cases hrest with
    | refl => exact absurd (hroot ▸ hba) (by simp)
    | step ht' _ => exact absurd (hroot ▸ ht') (by simp)

/--
The parent of a certified block is itself certified, or is the anchor.

The two cases a walk back along a chain can end in, with the view that separates
them: a certified parent is at a later view than genesis
(`cert1_above_genesis`), and the anchor is at genesis. Every induction that
steps from a block to the one before it splits here.
-/
theorem parent_cases {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hres : Resolves tree N) {pc : Cert1}
    (hpar : Network.ValidCert1 cfg N pc ∨ pc = cfg.anchorCert) :
    (Network.ValidCert1 cfg N pc ∧ pc.view ≠ ViewNumber.genesis)
      ∨ (pc = cfg.anchorCert ∧ pc.view = ViewNumber.genesis) := by
  rcases hpar with hb | ha
  · exact Or.inl ⟨hb, cert1_above_genesis tree N hres hb⟩
  · exact Or.inr ⟨ha, by rw [ha]; exact hcfg.anchorCertView⟩

/--
A certified block in a later epoch has a block an earlier epoch decided before
it.

The epoch-crossing half of no-fork. Walking
back from `c1`, each step either stays in the epoch or crosses a boundary, and
nothing else is possible (`epochOf_succ`, `epochOf_one`). A crossing is exactly
where the block before it is its epoch's last, which is exactly where a proposal
must carry a `Cert2` over that block (`Network.boundaryDecided`). So the walk
reaches a decided block of a strictly earlier epoch, and the argument never
intersects two committees' quorums.

The walk terminates on the view, which strictly decreases at every step; at
genesis there is nothing certified to step to (`cert1_above_genesis`).
-/
theorem cert1_crosses_boundary {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) (hh : cfg.epochHeight ≠ 0) :
    ∀ n, ∀ c1 : Cert1, c1.view.toNat ≤ n → Network.ValidCert1 cfg N c1 →
      ∀ e : EpochNumber, 1 ≤ e.toNat → e < c1.data.epoch →
        ∃ bc : Cert2, Cert2Backed N.run bc ∧ bc.data.epoch + 1 = c1.data.epoch
          ∧ bc.view < c1.view
          ∧ Ancestor tree bc.data.blockHash c1.data.blockHash
          ∧ ∃ bb, tree bc.data.blockHash = some bb
              ∧ bb.blockHeader.blockNumber = bc.data.epoch.toNat * cfg.epochHeight := by
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1 _ _ _
    exact absurd (view_eq (Nat.le_antisymm hle (Nat.zero_le _)))
      (cert1_above_genesis tree N hres hc1)
  | succ n ih =>
    intro c1 hle hc1 e hepos hlt
    obtain ⟨p, hview, hhash, hepc, hwf, htree, hpar, hbd⟩ := cert1_proposal tree N hres hc1
    have hnot : p ≠ cfg.anchorBlock := by
      intro hz
      exact absurd (by rw [hview, hz, hcfg.anchorBlockView] : c1.view = ViewNumber.genesis)
        (cert1_above_genesis tree N hres hc1)
    rcases parent_cases tree N hcfg hres hpar with ⟨hpar', hgen⟩ | ⟨hanchor, -⟩
    case inr =>
      -- The block before it is the anchor, which is block zero, so this block
      -- is in the first epoch and there is no earlier one to reach.
      exfalso
      have hanct : tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock :=
        anchor_in_tree tree N hres hc1
      have hsucc : p.blockHeader.blockNumber = cfg.anchorBlock.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 p cfg.anchorBlock (hhash ▸ htree) hnot
          (by rw [hanchor, hcfg.anchorCertBlock]; exact hanct)
      have hone : c1.data.epoch = epochOf 0 cfg.epochHeight := by
        rw [hepc, hwf.2.2, hsucc, hcfg.anchorBlockNumber]
        exact epochOf_one _ hh
      have h1 : c1.data.epoch.toNat = 1 := by
        rw [hone]
        simp [epochOf, hh]
      have h2 : e.toNat < c1.data.epoch.toNat := hlt
      omega
    obtain ⟨pp, hppv, hpph, hppe, hppwf, hpptree, -, -⟩ := cert1_proposal tree N hres hpar'
    have hsucc : p.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
      hheights c1 hc1 p pp (hhash ▸ htree) hnot (hpph ▸ hpptree)
    have hviewlt : p.parentCert.view.toNat < c1.view.toNat := by rw [hview]; exact hwf.1
    by_cases hlast : IsLastBlock pp.blockHeader.blockNumber cfg.epochHeight
    · -- The block before it is its epoch's last, so this proposal had to carry
      -- the certificate that decided it.
      obtain ⟨bc, hbcb, hbcv, hbch, hbce⟩ := hbd (by
        show IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
        rw [hsucc]; simpa using hlast)
      refine ⟨bc, hbcb, ?_, ?_, ?_, pp, ?_, ?_⟩
      · rw [hbce, hppe, hepc, hwf.2.2, hppwf.2.2, hsucc,
          epochOf_succ _ _ hh hlast.1, if_pos hlast]
      · rw [hbcv]; exact hviewlt
      · refine Ancestor.step (hhash ▸ htree) ?_
        rw [hbch]
        exact Ancestor.refl _
      · rw [hbch, hpph]; exact hpptree
      · rw [hbce, hppe]
        exact lastBlock_height hlast (by rw [← hppwf.2.2])
    · -- The epoch is unchanged one block back, so keep walking.
      have hsame : p.epoch = pp.epoch := by
        rw [hwf.2.2, hppwf.2.2, hsucc]
        by_cases hz : pp.blockHeader.blockNumber = 0
        · rw [hz]; exact epochOf_one _ hh
        · rw [epochOf_succ _ _ hh hz, if_neg hlast]
      obtain ⟨bc, hbcb, hbce, hbcv, hbca, bb, hbbt, hbbl⟩ :=
        ih p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar' e hepos
          (by rw [hppe, ← hsame, ← hepc]; exact hlt)
      refine ⟨bc, hbcb, ?_, ?_, ?_, bb, hbbt, hbbl⟩
      · rw [hepc, hsame, ← hppe]; exact hbce
      · exact Nat.lt_trans hbcv hviewlt
      · exact Ancestor.step (hhash ▸ htree) hbca

/--
A block's height is at most that of any certified block after it.

Stated for a chain that ends at a certified block, because that is what makes
well-formedness available at every step: a node may hold a proposal it never
admitted (`NodeState.proposals` is wider than `NodeState.admitted`), `Resolves`
puts it in the tree, and nothing makes it well-formed. A condition on the tree
saying otherwise could not be met alongside `Resolves`, which would leave
no-fork vacuously true.
-/
theorem ancestor_height_le {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) :
    ∀ n (c1 : Cert1), c1.view.toNat ≤ n → Network.ValidCert1 cfg N c1 →
      ∀ (a : BlockHash) (ba bc : Block), Ancestor tree a c1.data.blockHash →
        tree a = some ba → tree c1.data.blockHash = some bc →
        ba.blockHeader.blockNumber ≤ bc.blockHeader.blockNumber := by
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1
    exact absurd (view_eq (Nat.le_antisymm hle (Nat.zero_le _)))
      (cert1_above_genesis tree N hres hc1)
  | succ n ih =>
    intro c1 hle hc1 a ba bc hanc hba hbc
    obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -⟩ := cert1_proposal tree N hres hc1
    have hpc : bc = p := by
      rw [hhash] at hbc; exact Option.some_inj.mp (hbc.symm.trans htree)
    cases hanc with
    | refl =>
      rw [show ba = bc from Option.some_inj.mp (hba.symm.trans hbc)]
      exact Nat.le_refl _
    | step hstep hrest =>
      -- The step's block is the certified one, since the tree answers once.
      have hb : ∀ b, tree c1.data.blockHash = some b → b = p := fun b hb =>
        Option.some_inj.mp ((hhash ▸ hb).symm.trans htree)
      rename_i b
      have hbp : b = p := hb b hstep
      subst hbp
      rcases parent_cases tree N hcfg hres hpar with ⟨hpar', hgen⟩ | ⟨hanchor, -⟩
      · obtain ⟨pp, hppv, hpph, -, -, hpptree, -, -⟩ := cert1_proposal tree N hres hpar'
        have hnot : b ≠ cfg.anchorBlock := fun hz =>
          hgen (by rw [hz, hcfg.anchorParentView])
        have hsucc : b.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
          hheights c1 hc1 b pp hstep hnot (hpph ▸ hpptree)
        have hviewlt : b.parentCert.view.toNat < c1.view.toNat := by rw [hview]; exact hwf.1
        have := ih b.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar'
          a ba pp hrest hba (hpph ▸ hpptree)
        rw [hpc, hsucc]
        omega
      · -- The walk reached the anchor, which is block zero.
        have haa : a = blockHash cfg.anchorBlock :=
          ancestor_anchor hroot hcoh hcf
          (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest) hba
        have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
        rw [hba', hcfg.anchorBlockNumber]
        exact Nat.zero_le _

/--
A different block before a certified block has a strictly smaller height.

The form the argument uses: two certified blocks of one height, one before the
other, are the same block. That is what rules out a second last block of an
epoch, which is what an epoch boundary would otherwise let a fork hide behind.
-/
theorem ancestor_height_lt {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) {c1 : Cert1}
    (hc1 : Network.ValidCert1 cfg N c1) {a : BlockHash} {ba bc : Block}
    (hanc : Ancestor tree a c1.data.blockHash) (hne : a ≠ c1.data.blockHash)
    (hba : tree a = some ba) (hbc : tree c1.data.blockHash = some bc) :
    ba.blockHeader.blockNumber < bc.blockHeader.blockNumber := by
  obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -⟩ := cert1_proposal tree N hres hc1
  have hpc : bc = p := by
    rw [hhash] at hbc; exact Option.some_inj.mp (hbc.symm.trans htree)
  cases hanc with
  | refl => exact absurd rfl hne
  | step hstep hrest =>
    rename_i b
    have hbp : b = p := Option.some_inj.mp ((hhash ▸ hstep).symm.trans htree)
    subst hbp
    rcases parent_cases tree N hcfg hres hpar with ⟨hpar', hgen⟩ | ⟨hanchor, -⟩
    · obtain ⟨pp, hppv, hpph, -, -, hpptree, -, -⟩ := cert1_proposal tree N hres hpar'
      have hnot : b ≠ cfg.anchorBlock := fun hz => hgen (by rw [hz, hcfg.anchorParentView])
      have hsucc : b.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 b pp hstep hnot (hpph ▸ hpptree)
      have hle := ancestor_height_le tree N hcfg hroot hcoh hcf hres hheights
        b.parentCert.view.toNat
        b.parentCert (Nat.le_refl _) hpar' a ba pp hrest hba (hpph ▸ hpptree)
      rw [hpc, hsucc]
      omega
    · -- The walk reached the anchor: `b` is the block after it, at height one.
      have haa : a = blockHash cfg.anchorBlock :=
        ancestor_anchor hroot hcoh hcf
          (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest) hba
      have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
      have hanct : tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock := by
        rw [← haa, ← hba']; exact hba
      have hnot : b ≠ cfg.anchorBlock := by
        intro hz
        exact absurd (by rw [hview, hz, hcfg.anchorBlockView] :
          c1.view = ViewNumber.genesis) (cert1_above_genesis tree N hres hc1)
      have hsucc : b.blockHeader.blockNumber = cfg.anchorBlock.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 b cfg.anchorBlock hstep hnot
          (by rw [hanchor, hcfg.anchorCertBlock]; exact hanct)
      rw [hpc, hsucc, hba', hcfg.anchorBlockNumber]
      omega

/--
A block of the same height as a certified block after it is that block.
-/
theorem ancestor_height_eq {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) {c1 : Cert1}
    (hc1 : Network.ValidCert1 cfg N c1) {a : BlockHash} {ba bc : Block}
    (hanc : Ancestor tree a c1.data.blockHash)
    (hba : tree a = some ba) (hbc : tree c1.data.blockHash = some bc)
    (heq : ba.blockHeader.blockNumber = bc.blockHeader.blockNumber) :
    a = c1.data.blockHash := by
  by_cases hne : a = c1.data.blockHash
  · exact hne
  · exact absurd heq (Nat.ne_of_lt
      (ancestor_height_lt tree N hcfg hroot hcoh hcf hres hheights hc1 hanc hne hba hbc))

/--
A block's view is at most that of any certified block after it.

The same induction as `ancestor_height_le`, reading `ProposalWellFormed`'s first
clause instead of the height premise, and restricted to a chain ending at a
certified block for the same reason.
-/
theorem ancestor_view_le {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves tree N) :
    ∀ n (c1 : Cert1), c1.view.toNat ≤ n → Network.ValidCert1 cfg N c1 →
      ∀ (a : BlockHash) (ba bc : Block), Ancestor tree a c1.data.blockHash →
        tree a = some ba → tree c1.data.blockHash = some bc →
        ba.viewNumber ≤ bc.viewNumber := by
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1
    exact absurd (view_eq (Nat.le_antisymm hle (Nat.zero_le _)))
      (cert1_above_genesis tree N hres hc1)
  | succ n ih =>
    intro c1 hle hc1 a ba bc hanc hba hbc
    obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -⟩ := cert1_proposal tree N hres hc1
    have hpc : bc = p := by
      rw [hhash] at hbc; exact Option.some_inj.mp (hbc.symm.trans htree)
    cases hanc with
    | refl =>
      rw [show ba = bc from Option.some_inj.mp (hba.symm.trans hbc)]
      exact Nat.le_refl _
    | step hstep hrest =>
      rename_i b
      have hbp : b = p := Option.some_inj.mp ((hhash ▸ hstep).symm.trans htree)
      subst hbp
      rcases parent_cases tree N hcfg hres hpar with ⟨hpar', -⟩ | ⟨hanchor, -⟩
      · obtain ⟨pp, hppv, hpph, -, -, hpptree, -, -⟩ := cert1_proposal tree N hres hpar'
        have hviewlt : b.parentCert.view.toNat < c1.view.toNat := by rw [hview]; exact hwf.1
        have := ih b.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar'
          a ba pp hrest hba (hpph ▸ hpptree)
        rw [hpc]
        have hppview : pp.viewNumber = b.parentCert.view := hppv.symm
        have h1 : ba.viewNumber.toNat ≤ b.parentCert.view.toNat := hppview ▸ this
        exact Nat.le_of_lt (Nat.lt_of_le_of_lt h1 hwf.1)
      · -- The walk reached the anchor, which sits at genesis.
        have haa : a = blockHash cfg.anchorBlock :=
          ancestor_anchor hroot hcoh hcf
          (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest) hba
        have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
        rw [hba', hcfg.anchorBlockView]
        exact Nat.zero_le _

/-- A different block before a certified block is at a strictly earlier view. -/
theorem ancestor_view_lt {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves tree N)
    {c1 : Cert1} (hc1 : Network.ValidCert1 cfg N c1) {a : BlockHash} {ba bc : Block}
    (hanc : Ancestor tree a c1.data.blockHash) (hne : a ≠ c1.data.blockHash)
    (hba : tree a = some ba) (hbc : tree c1.data.blockHash = some bc) :
    ba.viewNumber < bc.viewNumber := by
  obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -⟩ := cert1_proposal tree N hres hc1
  have hpc : bc = p := by
    rw [hhash] at hbc; exact Option.some_inj.mp (hbc.symm.trans htree)
  cases hanc with
  | refl => exact absurd rfl hne
  | step hstep hrest =>
    rename_i b
    have hbp : b = p := Option.some_inj.mp ((hhash ▸ hstep).symm.trans htree)
    subst hbp
    rcases parent_cases tree N hcfg hres hpar with ⟨hpar', -⟩ | ⟨hanchor, hgenv⟩
    · obtain ⟨pp, hppv, hpph, -, -, hpptree, -, -⟩ := cert1_proposal tree N hres hpar'
      have hle := ancestor_view_le tree N hcfg hroot hcoh hcf hres b.parentCert.view.toNat
        b.parentCert (Nat.le_refl _) hpar' a ba pp hrest hba (hpph ▸ hpptree)
      have hppview : pp.viewNumber = b.parentCert.view := hppv.symm
      have h1 : ba.viewNumber.toNat ≤ b.parentCert.view.toNat := hppview ▸ hle
      rw [hpc]
      exact Nat.lt_of_le_of_lt h1 hwf.1
    · -- The walk reached the anchor, at genesis; `b` is certified, so later.
      have haa : a = blockHash cfg.anchorBlock :=
        ancestor_anchor hroot hcoh hcf
          (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest) hba
      have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
      have hgen := cert1_above_genesis tree N hres hc1
      rw [hpc, hba', hcfg.anchorBlockView, ← hview]
      exact Nat.pos_of_ne_zero fun hz => hgen (view_eq hz)

/--
A `Cert2` is before every certified block of its own epoch at a later view.

The intra-epoch half of no-fork. Induction on the epoch, and inside it on the
view: `no_gap` says no link
steps over `c.view`, so the walk back from the later block reaches `c.view`
itself, where `cert2_implies_cert1` and `cert1_unique` identify the two blocks.

What the induction on the epoch pays for is that the epoch cannot change on the
way. A step that crossed a boundary would put a decided last block of the
earlier epoch at a view at or after `c.view`, while `cert1_crosses_boundary`
puts one strictly before it. Both are last blocks of one epoch, so
`lastBlock_height` gives them one height, the earlier epoch's own case of this
result puts one before the other, and `ancestor_height_eq` then makes them one
block — which would be one block at two views.
-/
theorem cert2_ancestor_epoch {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree)
    (hcf : CollisionFree)
    (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) :
    ∀ E, ∀ c : Cert2, Network.ValidCert2 cfg N c → c.data.epoch.toNat = E →
      ∀ n, ∀ c1 : Cert1, c1.view.toNat ≤ n → Network.ValidCert1 cfg N c1 →
        c.data.epoch = c1.data.epoch → c.view ≤ c1.view →
          Ancestor tree c.data.blockHash c1.data.blockHash := by
  intro E
  induction E using Nat.strongRecOn with
  | _ E ih =>
    intro c hc2 hE n
    induction n with
    | zero =>
      intro c1 hle hc1 _ _
      exact absurd (view_eq (Nat.le_antisymm hle (Nat.zero_le _)))
        (cert1_above_genesis tree N hres hc1)
    | succ n ihn =>
      intro c1 hle hc1 hep hview
      rcases Nat.eq_or_lt_of_le hview with heqv | hltv
      · -- One view, one epoch: the two certificates name one block.
        obtain ⟨cc, hccb, hccv, hcch, hcce⟩ := cert2_implies_cert1 cfg N hcfg hc2
        have heq := cert1_unique cfg N hc1 hccb
          (by rw [← hep, ← hcce]) (by rw [hccv]; exact (view_eq heqv).symm)
        rw [← hcch, ← heq]
        exact Ancestor.refl _
      · -- A later view: step back one block, which stays in the epoch.
        obtain ⟨p, hview1, hhash, hepc, hwf, htree, hpar, hbd⟩ :=
          cert1_proposal tree N hres hc1
        have hgap : c.view ≤ p.parentCert.view := no_gap N hcfg hcf hc2 hc1 hep hhash hltv
        have hviewlt : p.parentCert.view.toNat < c1.view.toNat := by
          rw [hview1]; exact hwf.1
        have hnot : p ≠ cfg.anchorBlock := by
          intro hz
          exact absurd (by rw [hview1, hz, hcfg.anchorBlockView] :
            c1.view = ViewNumber.genesis) (cert1_above_genesis tree N hres hc1)
        rcases parent_cases tree N hcfg hres hpar with ⟨hpar', hgen⟩ | ⟨-, hgenv⟩
        case inr =>
          -- The walk reached the anchor, so `c` would have to sit at genesis.
          exfalso
          obtain ⟨cc, hccb, hccv, -, -⟩ := cert2_implies_cert1 cfg N hcfg hc2
          refine absurd (show c.view = ViewNumber.genesis from ?_)
            (hccv ▸ cert1_above_genesis tree N hres hccb)
          exact view_eq (Nat.le_antisymm (hgenv ▸ hgap) (Nat.zero_le _))
        obtain ⟨pp, hppv, hpph, hppe, hppwf, hpptree, -, -⟩ := cert1_proposal tree N hres hpar'
        have hsucc : p.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
          hheights c1 hc1 p pp (hhash ▸ htree) hnot (hpph ▸ hpptree)
        have hsame : c.data.epoch = p.parentCert.data.epoch := by
          by_cases hlast : IsLastBlock pp.blockHeader.blockNumber cfg.epochHeight
          · -- A boundary here would put two last blocks in one epoch.
            exfalso
            obtain ⟨bc, hbcb, hbcv, hbch, hbce⟩ := hbd (by
              show IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
              rw [hsucc]; simpa using hlast)
            obtain ⟨cc, hccb, hccv, hcch, hcce⟩ := cert2_implies_cert1 cfg N hcfg hc2
            have hstep : pp.epoch + 1 = c.data.epoch := by
              rw [hep, hepc, hwf.2.2, hppwf.2.2, hsucc,
                epochOf_succ _ _ hlast.2.1 hlast.1, if_pos hlast]
            -- `c` itself is in the later epoch, so it too has a last block before it.
            obtain ⟨bd, hbdb, hbde, hbdv, -, bb, hbbt, hbbh⟩ :=
              cert1_crosses_boundary tree N hcfg hres hheights hlast.2.1 cc.view.toNat cc
                (Nat.le_refl _) hccb pp.epoch
                (by rw [hppwf.2.2]; exact Nat.pos_of_ne_zero (epochOf_pos hlast.2.1))
                (by rw [hcce, ← hstep]; exact Nat.lt_succ_self _)
            have hbdep : bd.data.epoch = pp.epoch :=
              EpochNumber.ext (by
                have hs : bd.data.epoch + 1 = pp.epoch + 1 := by rw [hbde, hcce, hstep]
                have h2 : bd.data.epoch.toNat + 1 = pp.epoch.toNat + 1 :=
                  congrArg EpochNumber.toNat hs
                omega)
            -- Both are last blocks of that epoch, so both are at one height.
            have hppheight :
                pp.blockHeader.blockNumber = bd.data.epoch.toNat * cfg.epochHeight :=
              lastBlock_height hlast (by rw [hbdep, ← hppwf.2.2])
            obtain ⟨bc1, hbc1b, hbc1v, hbc1h, hbc1e⟩ := cert2_implies_cert1 cfg N hcfg hbcb
            have hbc1t : tree bc1.data.blockHash = some pp := by
              rw [hbc1h, hbch, hpph]; exact hpptree
            have hbdlt : bd.view < bc1.view := by
              rw [hbc1v, hbcv]
              exact Nat.lt_of_lt_of_le (hccv ▸ hbdv) hgap
            -- The earlier epoch's own case of this result puts one before the other.
            have hstepN : pp.epoch.toNat + 1 = E := by
              have h2 : pp.epoch.toNat + 1 = c.data.epoch.toNat :=
                congrArg EpochNumber.toNat hstep
              omega
            have hord := ih pp.epoch.toNat (by omega) bd hbdb (by rw [hbdep])
              bc1.view.toNat bc1 (Nat.le_refl _) hbc1b
              (by rw [hbdep, hbc1e, hbce, hppe]) (Nat.le_of_lt hbdlt)
            have hsameblock : bd.data.blockHash = bc1.data.blockHash :=
              ancestor_height_eq tree N hcfg hroot hcoh hcf hres hheights hbc1b hord hbbt hbc1t
                (by rw [hbbh, hppheight])
            -- One block, so one view, which the strict order forbids.
            obtain ⟨bd1, hbd1b, hbd1v, hbd1h, -⟩ := cert2_implies_cert1 cfg N hcfg hbdb
            obtain ⟨bdp, hbdpv, hbdph, -, -, -, -, -⟩ := cert1_proposal tree N hres hbd1b
            obtain ⟨bcp, hbcpv, hbcph, -, -, -, -, -⟩ := cert1_proposal tree N hres hbc1b
            have hblocks : bdp = bcp :=
              hcf bdp bcp (by rw [← hbdph, hbd1h, hsameblock, hbcph])
            have hveq : bd.view = bc1.view := by
              rw [← hbd1v, hbdpv, hblocks, ← hbcpv]
            exact absurd (congrArg ViewNumber.toNat hveq) (Nat.ne_of_lt hbdlt)
          · rw [hep, hepc, hppe, hwf.2.2, hppwf.2.2, hsucc]
            by_cases hh : cfg.epochHeight = 0
            · simp [epochOf, hh]
            · by_cases hz : pp.blockHeader.blockNumber = 0
              · rw [hz]; exact epochOf_one _ hh
              · rw [epochOf_succ _ _ hh hz, if_neg hlast]
        refine Ancestor.step (hhash ▸ htree) ?_
        exact ihn p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar'
          hsame hgap

/--
An epoch's decided last block is the only certified block of its epoch at or
after it.

Two last blocks of one epoch are at one height (`lastBlock_height`), no block of
the epoch is past that height (`epochOf_height_le`), and a block before another
at the same height is that block (`ancestor_height_eq`).
-/
theorem lastBlock_unique {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree)
    (hcf : CollisionFree)
    (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) (hh : cfg.epochHeight ≠ 0)
    {bd : Cert2} (hbdb : Network.ValidCert2 cfg N bd) {bb : Block}
    (hbbt : tree bd.data.blockHash = some bb)
    (hbbh : bb.blockHeader.blockNumber = bd.data.epoch.toNat * cfg.epochHeight)
    {c1 : Cert1} (hc1 : Network.ValidCert1 cfg N c1)
    (hepq : bd.data.epoch = c1.data.epoch) (hview : bd.view ≤ c1.view) :
    bd.data.blockHash = c1.data.blockHash := by
  obtain ⟨p, hview1, hhash, hepc, hwf, htree, -, -⟩ := cert1_proposal tree N hres hc1
  have hord := cert2_ancestor_epoch tree N hcfg hroot hcoh hcf hres hheights
    bd.data.epoch.toNat bd hbdb rfl c1.view.toNat c1 (Nat.le_refl _) hc1 hepq hview
  have hle := ancestor_height_le tree N hcfg hroot hcoh hcf hres hheights c1.view.toNat c1
    (Nat.le_refl _) hc1
    bd.data.blockHash bb p hord hbbt (hhash ▸ htree)
  have hpbound : p.blockHeader.blockNumber ≤ bd.data.epoch.toNat * cfg.epochHeight := by
    rw [hepq, hepc, hwf.2.2]
    exact epochOf_height_le hh
  exact ancestor_height_eq tree N hcfg hroot hcoh hcf hres hheights hc1 hord hbbt
    (hhash ▸ htree) (by omega)

/--
A certified block has, before it, the decided last block of every earlier epoch.

`cert1_crosses_boundary` steps back one epoch; this iterates it until the epoch
asked for is the one reached.
-/
theorem cert1_reaches_epoch {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N) (hh : cfg.epochHeight ≠ 0) :
    ∀ D, ∀ c1 : Cert1, Network.ValidCert1 cfg N c1 → ∀ E : EpochNumber, 1 ≤ E.toNat →
      E.toNat < c1.data.epoch.toNat → c1.data.epoch.toNat - E.toNat ≤ D →
        ∃ bd : Cert2, Network.ValidCert2 cfg N bd ∧ bd.data.epoch = E
          ∧ Ancestor tree bd.data.blockHash c1.data.blockHash
          ∧ ∃ bb, tree bd.data.blockHash = some bb
              ∧ bb.blockHeader.blockNumber = E.toNat * cfg.epochHeight := by
  intro D
  induction D with
  | zero =>
    intro c1 hc1 E hEpos hlt hD
    omega
  | succ D ih =>
    intro c1 hc1 E hEpos hlt hD
    obtain ⟨bd, hbdb, hbde, -, hbda, bb, hbbt, hbbh⟩ :=
      cert1_crosses_boundary tree N hcfg hres hheights hh c1.view.toNat c1 (Nat.le_refl _) hc1 E
        hEpos hlt
    have hbdstep : bd.data.epoch.toNat + 1 = c1.data.epoch.toNat :=
      congrArg EpochNumber.toNat hbde
    by_cases hE : bd.data.epoch = E
    · exact ⟨bd, hbdb, hE, hbda, bb, hbbt, by rw [hbbh, hE]⟩
    · obtain ⟨bd1, hbd1b, -, hbd1h, hbd1e⟩ := cert2_implies_cert1 cfg N hcfg hbdb
      have hne : bd.data.epoch.toNat ≠ E.toNat := fun heq => hE (EpochNumber.ext heq)
      obtain ⟨r, hrb, hre, hra, bb', hbbt', hbbh'⟩ :=
        ih bd1 hbd1b E hEpos (by rw [hbd1e]; omega) (by rw [hbd1e]; omega)
      exact ⟨r, hrb, hre, Ancestor.trans (hbd1h ▸ hra) hbda, bb', hbbt', hbbh'⟩

/--
A `Cert2` is before every certified block at a later view.

The trichotomy on the epochs, and the result no-fork is read off.

* One epoch is `cert2_ancestor_epoch`.
* A later epoch on the certified side: `cert1_reaches_epoch` produces the
  decided last block of `c`'s own epoch, and `lastBlock_unique` or
  `cert2_ancestor_epoch` places `c` against it.
* An earlier epoch on the certified side cannot happen. The same route puts a
  block of `c1`'s epoch before `c`'s block, so `ancestor_view_le` puts it at an
  earlier view than `c`'s; that leaves `c` and `c1` at one view, and
  `ancestor_view_lt` then forces them to be one block — which would put them in
  one epoch.
-/
theorem cert2_ancestor {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hroot : AnchorRooted tree cfg) (hcoh : TreeCoherent tree)
    (hcf : CollisionFree)
    (hres : Resolves tree N)
    (hheights : HeightSucceedsParent tree N)
    {c : Cert2} (hc2 : Network.ValidCert2 cfg N c)
    {c1 : Cert1} (hc1 : Network.ValidCert1 cfg N c1) (hview : c.view ≤ c1.view) :
    Ancestor tree c.data.blockHash c1.data.blockHash := by
  obtain ⟨cc, hccb, hccv, hcch, hcce⟩ := cert2_implies_cert1 cfg N hcfg hc2
  obtain ⟨cp, hcpv, hcph, hcpe, hcpwf, hcpt, -, -⟩ := cert1_proposal tree N hres hccb
  obtain ⟨c1p, hc1pv, hc1ph, hc1pe, hc1pwf, hc1pt, -, -⟩ := cert1_proposal tree N hres hc1
  have hviewN : c.view.toNat ≤ c1.view.toNat := hview
  have hcct : tree cc.data.blockHash = some cp := by rw [hcph]; exact hcpt
  have hc1t : tree c1.data.blockHash = some c1p := by rw [hc1ph]; exact hc1pt
  rcases Nat.lt_trichotomy c.data.epoch.toNat c1.data.epoch.toNat with hlt | heq | hgt
  · -- `c1` is in a later epoch: reach back to `c`'s own.
    have hh : cfg.epochHeight ≠ 0 := by
      intro h0
      rw [← hcce, hcpe, hcpwf.2.2, hc1pe, hc1pwf.2.2] at hlt
      simp [epochOf, h0] at hlt
    obtain ⟨r, hrb, hre, hra, bb, hbbt, hbbh⟩ :=
      cert1_reaches_epoch tree N hcfg hres hheights hh
        (c1.data.epoch.toNat - c.data.epoch.toNat) c1 hc1 c.data.epoch
        (by rw [← hcce, hcpe, hcpwf.2.2]; exact Nat.pos_of_ne_zero (epochOf_pos hh)) hlt
        (Nat.le_refl _)
    rcases Nat.lt_or_ge c.view.toNat r.view.toNat with hrv | hrv
    · obtain ⟨r1, hr1b, hr1v, hr1h, hr1e⟩ := cert2_implies_cert1 cfg N hcfg hrb
      refine Ancestor.trans (b := r1.data.blockHash) ?_ (by rw [hr1h]; exact hra)
      exact cert2_ancestor_epoch tree N hcfg hroot hcoh hcf hres hheights
        c.data.epoch.toNat c hc2 rfl r1.view.toNat r1 (Nat.le_refl _) hr1b
        (by rw [hr1e, hre]) (by rw [hr1v]; exact Nat.le_of_lt hrv)
    · have hsame := lastBlock_unique tree N hcfg hroot hcoh hcf hres hheights hh hrb hbbt
        (by rw [hbbh, hre]) hccb (by rw [hre, hcce]) (by rw [hccv]; exact hrv)
      rw [← hcch, ← hsame]
      exact hra
  · exact cert2_ancestor_epoch tree N hcfg hroot hcoh hcf hres hheights
      c.data.epoch.toNat c hc2 rfl c1.view.toNat c1 (Nat.le_refl _) hc1
      (EpochNumber.ext heq) hview
  · -- An earlier epoch on the certified side is impossible.
    exfalso
    have hh : cfg.epochHeight ≠ 0 := by
      intro h0
      rw [← hcce, hcpe, hcpwf.2.2, hc1pe, hc1pwf.2.2] at hgt
      simp [epochOf, h0] at hgt
    obtain ⟨r, hrb, hre, hra, bb, hbbt, hbbh⟩ :=
      cert1_reaches_epoch tree N hcfg hres hheights hh
        (cc.data.epoch.toNat - c1.data.epoch.toNat) cc hccb c1.data.epoch
        (by rw [hc1pe, hc1pwf.2.2]; exact Nat.pos_of_ne_zero (epochOf_pos hh))
        (by rw [hcce]; omega) (Nat.le_refl _)
    obtain ⟨r1, hr1b, hr1v, hr1h, -⟩ := cert2_implies_cert1 cfg N hcfg hrb
    obtain ⟨rp, hrpv, hrph, -, -, hrpt, -, -⟩ := cert1_proposal tree N hres hr1b
    have ht1 : tree r1.data.blockHash = some rp := by rw [hrph]; exact hrpt
    have ht2 : tree r1.data.blockHash = some bb := by rw [hr1h]; exact hbbt
    have hrpbb : rp = bb := Option.some_inj.mp (ht1.symm.trans ht2)
    have hrbb : r.view = bb.viewNumber := by rw [← hr1v, hrpv, hrpbb]
    have hcpview : cp.viewNumber = c.view := by rw [← hcpv, hccv]
    have hrview : r.view.toNat ≤ c.view.toNat := by
      have h := ancestor_view_le tree N hcfg hroot hcoh hcf hres cc.view.toNat cc
        (Nat.le_refl _) hccb
        r.data.blockHash bb cp hra hbbt hcct
      rw [hrbb, ← hcpview]
      exact h
    rcases Nat.lt_or_ge c1.view.toNat r.view.toNat with hrv | hrv
    · omega
    · have hsame := lastBlock_unique tree N hcfg hroot hcoh hcf hres hheights hh hrb hbbt
        (by rw [hbbh, hre]) hc1 hre hrv
      by_cases hblk : c1.data.blockHash = cc.data.blockHash
      · have hpp : c1p = cp := hcf c1p cp (by rw [← hc1ph, hblk, hcph])
        have heq : c1.data.epoch = c.data.epoch := by rw [hc1pe, hpp, ← hcpe, hcce]
        exact absurd (congrArg EpochNumber.toNat heq) (by omega)
      · have hstrict := ancestor_view_lt tree N hcfg hroot hcoh hcf hres hccb
          (by rw [← hsame]; exact hra) hblk hc1t hcct
        have hc1view : c1p.viewNumber = c1.view := hc1pv.symm
        rw [hc1view, hcpview] at hstrict
        have h2 : c1.view.toNat < c.view.toNat := hstrict
        omega

end NewProtocol
