module

public import NewProtocolSpec.Proofs.Certificates

/-!
# No fork

The walk down a certified branch, and `noFork`. Inside one epoch, `cert1_step` keeps
a branch from stepping over a committed view (`cert2_ancestor_epoch`). Across epochs,
the branch passes through a decided last block (`cert1_crosses_boundary`), and the
two are joined by a case split on the epochs (`cert2_ancestor`). No argument ever
intersects two committees' quorums.

Re-votes certify an epoch's last block again at later views, so `Cert2`s are ordered
by epoch first (`CertNotAfter`): the outgoing committee may re-vote its last block
at a view after the next epoch committed blocks of its own.

Ported from the previous specification's proof: only the facts about votes
(`NewProtocolSpec.Proofs.Certificates`) are new. The chain arithmetic is unchanged.
-/

@[expose] public section

namespace NewProtocol

variable (tree : BlockTable)

/-- Ancestry composes. -/
theorem Ancestor.trans {tree : BlockTable} {a b c : BlockHash}
    (hab : Ancestor cfg tree a b) (hbc : Ancestor cfg tree b c) : Ancestor cfg tree a c := by
  induction hbc with
  | refl => exact hab
  | step ht _ hgen ih => exact Ancestor.step ht ih hgen

/--
A certified block's height is one more than its parent's.

What makes `epochOf` mean anything. Without it a chain could cross a boundary
without passing through it, and a block's epoch would say nothing about the
epochs of the blocks before it.

`heightSucceedsParent` proves it, from the height a certificate carries
(`Vote1Data.blockNumber`) and the clause of `ProposalWellFormed` that ties a
proposal's height to its parent certificate's, for a coherent tree that resolves
the honest nodes' blocks. A lemma that has those takes the property from it; one
that does not takes the property itself.

Asked only of *certified* blocks, and that restriction is the whole of what
makes it provable. `Resolves` puts every proposal an honest node holds into
the tree, and a node holds proposals it never voted for; nothing constrains
their heights, so a tree-wide version would be false. A certified block is one
an honest node voted for, and a node votes only on a well-formed proposal — so
this is exactly the range the rules reach.
-/
def HeightSucceedsParent {C : Committee} (N : Network cfg C) : Prop :=
  ∀ c1 : Cert1, Cert1Backed N.trace c1 → ∀ b parent : Block,
    tree c1.data.blockHash = some b →
    tree b.parentCert.data.blockHash = some parent →
    b.blockHeader.blockNumber = parent.blockHeader.blockNumber + 1

/--
**A certified block's height is one more than its parent's**, which
`HeightSucceedsParent` is the name of.

Every certificate carries the height of the block it certifies
(`Vote1Data.blockNumber`), put there by the quorum that formed it, and
`ProposalWellFormed` makes a proposal's height one more than the height its
parent certificate names. `CollisionFree` is what says
the block the tree answers with is the block that quorum voted on, and the two
together are the whole argument.

The anchor is where the chain of certificates stops, and there the configuration
stands in for a quorum: nothing in a run votes at the anchor's view, so
`ConfigCoherent.anchorCertBlockNumber` is what says the anchor certificate and the
anchor block agree on the height.

The implementation checks the same thing on the same path, in the proposal
validation rather than in the state validation.
-/
theorem heightSucceedsParent {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hcoh : TreeCoherent tree) (hcf : CollisionFree) (hres : Resolves cfg tree N) :
    HeightSucceedsParent tree N := by
  intro c1 hc1 b parent htb htp
  obtain ⟨p, -, hhash, -, hwf, -, hpar, -, -, -⟩ := cert1_proposal tree N hres hc1
  have hbp : b = p := hcf b p (by rw [hcoh _ b htb, hhash])
  rw [hbp] at htp ⊢
  have hpn : parent.blockHeader.blockNumber = p.parentCert.data.blockNumber := by
    rcases hpar with hvalid | hanchor
    · obtain ⟨g, -, hgh, -, hgn⟩ := cert1Backed_block hvalid
      have hg : parent = g := hcf parent g (by rw [hcoh _ parent htp, hgh])
      rw [hg, hgn]
    · have ha : parent = cfg.anchorBlock :=
        hcf parent cfg.anchorBlock
          (by rw [hcoh _ parent htp, hanchor, hcfg.anchorCertBlock])
      rw [ha, hanchor, hcfg.anchorCertBlockNumber]
  rw [hpn]
  exact hwf.height.symm

/--
A well-formed proposal on the anchor or a backed certificate is not at the anchor's
view: its parent is at the anchor's view or later, and strictly before it.
-/
theorem view_ne_anchor {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg) {p : Proposal}
    (hwf : ProposalWellFormed cfg p)
    (hpar : Cert1Backed N.trace p.parentCert ∨ p.parentCert = cfg.anchorCert) :
    p.viewNumber ≠ cfg.anchorView := fun hz => by
  have h1 : p.parentCert.view.toNat < p.viewNumber.toNat := hwf.parentEarlier
  have h2 : cfg.anchorView.toNat ≤ p.parentCert.view.toNat := by
    rcases hpar with hb | ha
    · exact Nat.le_of_lt (cert1_after_anchor N hcfg _ _ (Nat.le_refl _) hb)
    · rw [ha, hcfg.anchorCertView]; exact Nat.le_refl _
  rw [hz] at h1
  omega

/--
Nothing is before the anchor.

`Ancestor` never steps back from a block at the anchor's view.
-/
theorem ancestor_anchor {tree : BlockTable} (hcoh : TreeCoherent tree) (hcf : CollisionFree) {a : BlockHash}
    (h : Ancestor cfg tree a (blockHash cfg.anchorBlock)) :
    a = blockHash cfg.anchorBlock := by
  cases h with
  | refl => rfl
  | step ht _hrest hgen =>
    rename_i b
    have hb : b = cfg.anchorBlock := hcf b cfg.anchorBlock (hcoh _ b ht)
    exact absurd (by rw [hb]) hgen

/--
The parent of a certified block is itself certified, or is the anchor.

The two cases a walk back along a chain can end in, with the view that separates
them: a certified parent is at a later view than genesis
(`cert1_not_at_anchor`), and the anchor is at genesis. Every induction that
steps from a block to the one before it splits here.
-/
theorem parent_cases {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg) {pc : Cert1}
    (hpar : Cert1Backed N.trace pc ∨ pc = cfg.anchorCert) :
    (Cert1Backed N.trace pc ∧ pc.view ≠ cfg.anchorView)
      ∨ (pc = cfg.anchorCert ∧ pc.view = cfg.anchorView) := by
  rcases hpar with hb | ha
  · exact Or.inl ⟨hb, cert1_not_at_anchor N hcfg hb⟩
  · exact Or.inr ⟨ha, by rw [ha]; exact hcfg.anchorCertView⟩

/--
A certified block in a later epoch has a block an earlier epoch decided before
it.

The epoch-crossing half of no-fork. Walking
back from `c1`, each step either stays in the epoch or crosses a boundary, and
nothing else is possible (`epochOf_succ`, `epochOf_one`). A crossing is exactly
where the block before it is its epoch's last, which is exactly where an honest
vote1 needs a `Cert2` over that block (`OpensEpochJustified`). So the walk
reaches a decided block of a strictly earlier epoch, and the argument never
intersects two committees' quorums.

The walk terminates on the view, which strictly decreases at every step; at
genesis there is nothing certified to step to (`cert1_not_at_anchor`).
-/
theorem cert1_crosses_boundary {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hres : Resolves cfg tree N)
    (hheights : HeightSucceedsParent tree N) (hh : cfg.epochHeight ≠ 0) :
    ∀ n, ∀ c1 : Cert1, c1.view.toNat ≤ n → Cert1Backed N.trace c1 →
      cfg.startEpoch < c1.data.epoch →
        ∃ bc : Cert2, Cert2Backed N.trace bc ∧ bc.data.epoch + 1 = c1.data.epoch
          ∧ bc.view < c1.view
          ∧ Ancestor cfg tree bc.data.blockHash c1.data.blockHash
          ∧ ∃ bb, tree bc.data.blockHash = some bb
              ∧ bb.blockHeader.blockNumber.toNat = bc.data.epoch.toNat * cfg.epochHeight := by
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1 _
    exact absurd (cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) hc1)
      (by show ¬ cfg.anchorView.toNat < c1.view.toNat; omega)
  | succ n ih =>
    intro c1 hle hc1 hlt
    obtain ⟨p, hview, hhash, hepc, hwf, htree, hpar, hbd, -, -⟩ := cert1_proposal tree N hres hc1
    rcases parent_cases N hcfg hpar with ⟨hpar', hgen⟩ | ⟨hanchor, -⟩
    case inr =>
      -- The block before it is the anchor, so it is in the epoch the run starts in,
      -- and there is no earlier one to reach.
      exfalso
      have hanct : tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock :=
        anchor_in_tree tree N hres hc1
      have hsucc : p.blockHeader.blockNumber = cfg.anchorBlock.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 p cfg.anchorBlock (hhash ▸ htree)
          (by rw [hanchor, hcfg.anchorCertBlock]; exact hanct)
      have hstart : c1.data.epoch = cfg.startEpoch := by
        rw [hepc, hwf.epoch, hsucc, epochOf_after_anchor hcfg]
      have h2 : cfg.startEpoch.toNat < c1.data.epoch.toNat := hlt
      rw [hstart] at h2
      omega
    obtain ⟨pp, hppv, hpph, hppe, hppwf, hpptree, hpppar, -, -, -⟩ := cert1_proposal tree N hres hpar'
    have hsucc : p.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
      hheights c1 hc1 p pp (hhash ▸ htree) (hpph ▸ hpptree)
    have hviewlt : p.parentCert.view.toNat < c1.view.toNat := Nat.lt_of_lt_of_le hwf.1 hview
    by_cases hlast : IsLastBlock pp.blockHeader.blockNumber cfg.epochHeight
    · -- The block before it is its epoch's last, so this proposal had to carry
      -- the certificate that decided it.
      obtain ⟨bc, hbcb, hbcv, hbch, hbce⟩ := hbd (by
        show IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
        rw [hsucc]; simpa using hlast)
      -- The anchor's `Cert2` is not over `pp`, a certified block after the anchor.
      replace hbcb : Cert2Backed N.trace bc := hbcb.resolve_right fun hbca => by
        have hppa : pp = cfg.anchorBlock := by
          have h1 : blockHash pp = blockHash cfg.anchorBlock := by
            rw [← hpph, ← hbch, hbca]; exact hcfg.anchorCertBlock
          have hanct : tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock :=
            anchor_in_tree tree N hres hc1
          rw [h1] at hpptree
          exact Option.some_inj.mp (hpptree.symm.trans hanct)
        exact view_ne_anchor N hcfg hppwf hpppar (by rw [hppa])
      refine ⟨bc, hbcb, ?_, ?_, ?_, pp, ?_, ?_⟩
      · rw [hbce, hppe, hepc, hwf.epoch, hppwf.epoch, hsucc,
          epochOf_succ _ _ hh (isLastBlock_iff.mp hlast).1, ite_eq_left hlast]
      · exact Nat.lt_of_lt_of_le hbcv hview
      · refine Ancestor.step (hhash ▸ htree) ?_ (view_ne_anchor N hcfg hwf hpar)
        rw [hbch]
        exact Ancestor.refl _
      · rw [hbch, hpph]; exact hpptree
      · rw [hbce, hppe]
        exact lastBlock_height hlast (by rw [← hppwf.epoch])
    · -- The epoch is unchanged one block back, so keep walking.
      have hsame : p.epoch = pp.epoch := by
        rw [hwf.epoch, hppwf.epoch, hsucc]
        by_cases hz : pp.blockHeader.blockNumber.toNat = 0
        · rw [BlockNumber.ext hz]; exact epochOf_one _ hh
        · rw [epochOf_succ _ _ hh hz, ite_eq_right hlast]
      obtain ⟨bc, hbcb, hbce, hbcv, hbca, bb, hbbt, hbbl⟩ :=
        ih p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar'
          (by rw [hppe, ← hsame, ← hepc]; exact hlt)
      refine ⟨bc, hbcb, ?_, ?_, ?_, bb, hbbt, hbbl⟩
      · rw [hepc, hsame, ← hppe]; exact hbce
      · exact Nat.lt_trans hbcv hviewlt
      · exact Ancestor.step (hhash ▸ htree) hbca (view_ne_anchor N hcfg hwf hpar)

/--
A block's height is at most that of any certified block after it.

Stated for a chain that ends at a certified block, because that is what makes
well-formedness available at every step: a node may hold a proposal it never
voted for, `Resolves` puts it in the tree, and nothing makes it well-formed. A condition on the tree
saying otherwise could not be met alongside `Resolves`, which would leave
no-fork vacuously true.
-/
theorem ancestor_height_le {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves cfg tree N) :
    ∀ n (c1 : Cert1), c1.view.toNat ≤ n → Cert1Backed N.trace c1 →
      ∀ (a : BlockHash) (ba bc : Block), Ancestor cfg tree a c1.data.blockHash →
        tree a = some ba → tree c1.data.blockHash = some bc →
        ba.blockHeader.blockNumber.toNat ≤ bc.blockHeader.blockNumber.toNat := by
  have hheights := heightSucceedsParent tree N hcfg hcoh hcf hres
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1
    exact absurd (cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) hc1)
      (by show ¬ cfg.anchorView.toNat < c1.view.toNat; omega)
  | succ n ih =>
    intro c1 hle hc1 a ba bc hanc hba hbc
    obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -, -, -⟩ := cert1_proposal tree N hres hc1
    have hpc : bc = p := by
      rw [hhash] at hbc; exact Option.some_inj.mp (hbc.symm.trans htree)
    cases hanc with
    | refl =>
      rw [show ba = bc from Option.some_inj.mp (hba.symm.trans hbc)]
      exact Nat.le_refl _
    | step hstep hrest _hgen =>
      -- The step's block is the certified one, since the tree answers once.
      have hb : ∀ b, tree c1.data.blockHash = some b → b = p := fun b hb =>
        Option.some_inj.mp ((hhash ▸ hb).symm.trans htree)
      rename_i b
      have hbp : b = p := hb b hstep
      subst hbp
      rcases parent_cases N hcfg hpar with ⟨hpar', hgen⟩ | ⟨hanchor, -⟩
      · obtain ⟨pp, hppv, hpph, -, -, hpptree, -, -, -, -⟩ := cert1_proposal tree N hres hpar'
        have hsucc : b.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
          hheights c1 hc1 b pp hstep (hpph ▸ hpptree)
        have hviewlt : b.parentCert.view.toNat < c1.view.toNat := Nat.lt_of_lt_of_le hwf.1 hview
        have := ih b.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar'
          a ba pp hrest hba (hpph ▸ hpptree)
        rw [hpc, hsucc, BlockNumber.toNat_add]
        omega
      · -- The walk reached the anchor, which is the block's parent.
        have haa : a = blockHash cfg.anchorBlock :=
          ancestor_anchor hcoh hcf
          (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest)
        have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
        have hanct : tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock := by
          rw [← haa, ← hba']; exact hba
        have hsucc : b.blockHeader.blockNumber = cfg.anchorBlock.blockHeader.blockNumber + 1 :=
          hheights c1 hc1 b cfg.anchorBlock hstep
            (by rw [hanchor, hcfg.anchorCertBlock]; exact hanct)
        rw [hpc, hsucc, hba', BlockNumber.toNat_add]
        omega

/--
A block before a certified block is at the certificate's view or an earlier one.

Stated for a chain that ends at a certified block, as `ancestor_height_le` is:
each step is to the parent of a block a quorum voted for, which is well formed,
so the parent is at an earlier view than the block's certificate.
-/
theorem ancestor_view_le {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hcoh : TreeCoherent tree) (hcf : CollisionFree) (hres : Resolves cfg tree N) :
    ∀ n (c1 : Cert1), c1.view.toNat ≤ n → Cert1Backed N.trace c1 →
      ∀ (a : BlockHash) (ba : Block), Ancestor cfg tree a c1.data.blockHash →
        tree a = some ba → ba.viewNumber ≤ c1.view := by
  intro n
  induction n with
  | zero =>
    intro c1 hle hc1
    exact absurd (cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) hc1)
      (by show ¬ cfg.anchorView.toNat < c1.view.toNat; omega)
  | succ n ih =>
    intro c1 hle hc1 a ba hanc hba
    obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -, -, -⟩ := cert1_proposal tree N hres hc1
    cases hanc with
    | refl =>
      rw [show ba = p from Option.some_inj.mp (hba.symm.trans (hhash ▸ htree))]
      exact hview
    | step hstep hrest _hgen =>
      rename_i b
      have hbp : b = p := Option.some_inj.mp ((hhash ▸ hstep).symm.trans htree)
      subst hbp
      have hviewlt : b.parentCert.view.toNat < c1.view.toNat := Nat.lt_of_lt_of_le hwf.1 hview
      rcases parent_cases N hcfg hpar with ⟨hpar', -⟩ | ⟨hanchor, hav⟩
      · have := ih b.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar' a ba hrest hba
        exact Nat.le_of_lt (Nat.lt_of_le_of_lt this hviewlt)
      · have haa : a = blockHash cfg.anchorBlock :=
          ancestor_anchor hcoh hcf (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest)
        have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
        rw [hba']
        exact Nat.le_of_lt (cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) hc1)

/--
A different block before a certified block has a strictly smaller height.

The form the argument uses: two certified blocks of one height, one before the
other, are the same block. That is what rules out a second last block of an
epoch, which is what an epoch boundary would otherwise let a fork hide behind.
-/
theorem ancestor_height_lt {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves cfg tree N) {c1 : Cert1}
    (hc1 : Cert1Backed N.trace c1) {a : BlockHash} {ba bc : Block}
    (hanc : Ancestor cfg tree a c1.data.blockHash) (hne : a ≠ c1.data.blockHash)
    (hba : tree a = some ba) (hbc : tree c1.data.blockHash = some bc) :
    ba.blockHeader.blockNumber.toNat < bc.blockHeader.blockNumber.toNat := by
  have hheights := heightSucceedsParent tree N hcfg hcoh hcf hres
  obtain ⟨p, hview, hhash, -, hwf, htree, hpar, -, -, -⟩ := cert1_proposal tree N hres hc1
  have hpc : bc = p := by
    rw [hhash] at hbc; exact Option.some_inj.mp (hbc.symm.trans htree)
  cases hanc with
  | refl => exact absurd rfl hne
  | step hstep hrest _hgen =>
    rename_i b
    have hbp : b = p := Option.some_inj.mp ((hhash ▸ hstep).symm.trans htree)
    subst hbp
    rcases parent_cases N hcfg hpar with ⟨hpar', hgen⟩ | ⟨hanchor, -⟩
    · obtain ⟨pp, hppv, hpph, -, -, hpptree, -, -, -, -⟩ := cert1_proposal tree N hres hpar'
      have hsucc : b.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 b pp hstep (hpph ▸ hpptree)
      have hle := ancestor_height_le tree N hcfg hcoh hcf hres
        b.parentCert.view.toNat
        b.parentCert (Nat.le_refl _) hpar' a ba pp hrest hba (hpph ▸ hpptree)
      rw [hpc, hsucc, BlockNumber.toNat_add]
      omega
    · -- The walk reached the anchor: `b` is the block after it, at height one.
      have haa : a = blockHash cfg.anchorBlock :=
        ancestor_anchor hcoh hcf
          (by rw [hanchor, hcfg.anchorCertBlock] at hrest; exact hrest)
      have hba' : ba = cfg.anchorBlock := hcf ba cfg.anchorBlock (by rw [hcoh a ba hba, haa])
      have hanct : tree (blockHash cfg.anchorBlock) = some cfg.anchorBlock := by
        rw [← haa, ← hba']; exact hba
      have hsucc : b.blockHeader.blockNumber = cfg.anchorBlock.blockHeader.blockNumber + 1 :=
        hheights c1 hc1 b cfg.anchorBlock hstep
          (by rw [hanchor, hcfg.anchorCertBlock]; exact hanct)
      rw [hpc, hsucc, hba', BlockNumber.toNat_add]
      omega

/--
A block of the same height as a certified block after it is that block.
-/
theorem ancestor_height_eq {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hcoh : TreeCoherent tree) (hcf : CollisionFree)
    (hres : Resolves cfg tree N) {c1 : Cert1}
    (hc1 : Cert1Backed N.trace c1) {a : BlockHash} {ba bc : Block}
    (hanc : Ancestor cfg tree a c1.data.blockHash)
    (hba : tree a = some ba) (hbc : tree c1.data.blockHash = some bc)
    (heq : ba.blockHeader.blockNumber.toNat = bc.blockHeader.blockNumber.toNat) :
    a = c1.data.blockHash := by
  have hheights := heightSucceedsParent tree N hcfg hcoh hcf hres
  by_cases hne : a = c1.data.blockHash
  · exact hne
  · exact absurd heq (Nat.ne_of_lt
      (ancestor_height_lt tree N hcfg hcoh hcf hres hc1 hanc hne hba hbc))

/-- The block a backed `Cert2` is over is at its view or an earlier one. -/
theorem cert2_block_view {C : Committee} (N : Network cfg C) (hcfg : ConfigCoherent cfg)
    (hres : Resolves cfg tree N) {c : Cert2} (hc2 : Cert2Backed N.trace c) {b : Block}
    (hb : tree c.data.blockHash = some b) : b.viewNumber ≤ c.view := by
  obtain ⟨cc, hccb, hccv, hcch, -⟩ := cert2_implies_cert1 cfg N hcfg hc2
  obtain ⟨p, hpv, hph, -, -, hpt, -, -, -, -⟩ := cert1_proposal tree N hres hccb
  have hpb : p = b := Option.some_inj.mp (hpt.symm.trans (by rw [← hph, hcch]; exact hb))
  rw [← hpb, ← hccv]
  exact hpv

/--
A `Cert2` is before every certified block of its own epoch at a later view.

The intra-epoch half of no-fork. Induction on the epoch, and inside it on the
view: each step back from the later certificate (`cert1_step`) skips no committed
view, so the walk reaches `c.view` itself, where `cert2_implies_cert1` and
`cert1_unique` identify the two blocks. A step is a proposal's parent link, or a
re-vote's, which stays on the same block; and where the timeout evidence's lock is
a later certificate over the parent's block, the walk continues from that lock.

What the induction on the epoch pays for is that the epoch cannot change on the
way. A step that crossed a boundary would leave from a proposal opening the epoch,
which names its parent at the parent's own view (`OpensEpochJustified`). `c` itself
has a decided last block of the earlier epoch before it, strictly before `c.view`
(`cert1_crosses_boundary`). Both are last blocks of one epoch, so the earlier
epoch's case of this result and `ancestor_height_eq` make them one block, whose
view would then be both before `c.view` and at or after it.
-/
theorem cert2_ancestor_epoch {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hcoh : TreeCoherent tree)
    (hcf : CollisionFree)
    (hres : Resolves cfg tree N) :
    ∀ E, ∀ c : Cert2, Cert2Backed N.trace c → c.data.epoch.toNat = E →
      ∀ n, ∀ c1 : Cert1, c1.view.toNat ≤ n → Cert1Backed N.trace c1 →
        c.data.epoch = c1.data.epoch → c.view ≤ c1.view →
          Ancestor cfg tree c.data.blockHash c1.data.blockHash := by
  have hheights := heightSucceedsParent tree N hcfg hcoh hcf hres
  intro E
  induction E using Nat.strongRecOn with
  | _ E ih =>
    intro c hc2 hE n
    induction n with
    | zero =>
      intro c1 hle hc1 _ _
      exact absurd (cert1_after_anchor N hcfg _ c1 (Nat.le_refl _) hc1)
        (by show ¬ cfg.anchorView.toNat < c1.view.toNat; omega)
    | succ n ihn =>
      intro c1 hle hc1 hep hview
      rcases Nat.eq_or_lt_of_le hview with heqv | hltv
      · -- One view, one epoch: the two certificates name one block.
        obtain ⟨cc, hccb, hccv, hcch, hcce⟩ := cert2_implies_cert1 cfg N hcfg hc2
        have heq := cert1_unique cfg N hc1 hccb
          (by rw [← hep, ← hcce]) (by rw [hccv]; exact (ViewNumber.ext heqv).symm)
        rw [← hcch, ← heq]
        exact Ancestor.refl _
      · -- A later view: step back once.
        -- The walk continues from a certificate over the same block as `c1`'s.
        have hsame : ∀ T : Cert1, T.data.blockHash = c1.data.blockHash → T.view.toNat < c1.view.toNat →
            Cert1Backed N.trace T → T.data.epoch = c.data.epoch → c.view ≤ T.view →
            Ancestor cfg tree c.data.blockHash c1.data.blockHash := fun T hTh hTv hTb hTe hTle => by
          rw [← hTh]
          exact ihn T (by omega) hTb hTe.symm hTle
        rcases cert1_step tree N hcfg hres hc1 with
          ⟨p, hview1, hhash, hepc, hwf, htree, hpar, hopen, hng⟩ | ⟨pc, hpcb, hpcd, hpcv, -, hng⟩
        case inr =>
          -- A re-vote: the same block at an earlier view.
          rcases hng c hc2 hep hltv with hgap | ⟨T, hTb, hTd, hTe, hTle, hTlt⟩
          · exact hsame pc (by rw [hpcd]) hpcv hpcb (by rw [hpcd, ← hep]) hgap
          · exact hsame T (by rw [hTd, hpcd]) hTlt hTb hTe hTle
        have hviewlt : p.parentCert.view.toNat < c1.view.toNat := by
          rw [hview1]; exact hwf.1
        -- From an ancestor of the parent's block, one link reaches `c1`'s.
        have hlink : Ancestor cfg tree c.data.blockHash p.parentCert.data.blockHash →
            Ancestor cfg tree c.data.blockHash c1.data.blockHash := fun h =>
          Ancestor.step (hhash ▸ htree) h (view_ne_anchor N hcfg hwf hpar)
        rcases hng c hc2 hep hltv with hgap | ⟨T, hTb, hTd, hTe, hTle, hTlt⟩
        case inr =>
          apply hlink
          rw [← show T.data.blockHash = p.parentCert.data.blockHash by rw [hTd]]
          exact ihn T (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hTlt hle)) hTb hTe.symm hTle
        rcases parent_cases N hcfg hpar with ⟨hpar', hgen⟩ | ⟨-, hgenv⟩
        case inr =>
          -- The walk reached the anchor, so `c` would have to be at the anchor's view or before.
          exfalso
          have h0 : c.view.toNat ≤ cfg.anchorView.toNat := by
            have : c.view.toNat ≤ p.parentCert.view.toNat := hgap
            rw [hgenv] at this; exact this
          have : cfg.anchorView.toNat < c.view.toNat := cert2_after_anchor N hc2
          omega
        obtain ⟨pp, hppv, hpph, hppe, hppwf, hpptree, -, -, -, -⟩ := cert1_proposal tree N hres hpar'
        have hsucc : p.blockHeader.blockNumber = pp.blockHeader.blockNumber + 1 :=
          hheights c1 hc1 p pp (hhash ▸ htree) (hpph ▸ hpptree)
        have hsameE : c.data.epoch = p.parentCert.data.epoch := by
          by_cases hlast : IsLastBlock pp.blockHeader.blockNumber cfg.epochHeight
          · -- A boundary here would put the parent both before `c.view` and at or after it.
            exfalso
            have hent : EntersEpoch cfg p := by
              show IsLastBlock (p.blockHeader.blockNumber - 1) cfg.epochHeight
              rw [hsucc]; simpa using hlast
            obtain ⟨j, hj, m, q, -, hq, hqv, hqh, -⟩ := hopen hent
            have hqt : tree p.parentCert.data.blockHash = some q := by rw [hqh]; exact hres j hj m q hq
            have hqpp : q = pp := Option.some_inj.mp (hqt.symm.trans (by rw [hpph]; exact hpptree))
            have hppvw : pp.viewNumber = p.parentCert.view := hqpp ▸ hqv
            obtain ⟨cc, hccb, hccv, -, hcce⟩ := cert2_implies_cert1 cfg N hcfg hc2
            have hstep : pp.epoch + 1 = c.data.epoch := by
              rw [hep, hepc, hwf.epoch, hppwf.epoch, hsucc,
                epochOf_succ _ _ hlast.2.1 (isLastBlock_iff.mp hlast).1, ite_eq_left hlast]
            -- `c` is in the later epoch, so a decided last block is before it.
            obtain ⟨bd, hbdb, hbde, hbdv, -, bb, hbbt, hbbh⟩ :=
              cert1_crosses_boundary tree N hcfg hres hheights hlast.2.1 cc.view.toNat cc
                (Nat.le_refl _) hccb (Nat.lt_of_le_of_lt
                  (show cfg.startEpoch.toNat ≤ pp.epoch.toNat by
                    rw [← hppe]; exact cert1_epoch_after_start N hcfg _ _ (Nat.le_refl _) hpar')
                  (show pp.epoch.toNat < cc.data.epoch.toNat by
                    rw [hcce, ← hstep]; exact Nat.lt_succ_self _))
            have hbdep : bd.data.epoch = pp.epoch :=
              EpochNumber.ext (by
                have hs : bd.data.epoch + 1 = pp.epoch + 1 := by rw [hbde, hcce, hstep]
                have h2 : bd.data.epoch.toNat + 1 = pp.epoch.toNat + 1 :=
                  congrArg EpochNumber.toNat hs
                omega)
            have hppheight :
                pp.blockHeader.blockNumber.toNat = bd.data.epoch.toNat * cfg.epochHeight :=
              lastBlock_height hlast (by rw [hbdep, ← hppwf.epoch])
            have hbdlt : bd.view.toNat < c.view.toNat := by
              have : bd.view.toNat < cc.view.toNat := hbdv
              rw [hccv] at this; exact this
            have hstepN : pp.epoch.toNat + 1 = E := by
              have h2 : pp.epoch.toNat + 1 = c.data.epoch.toNat :=
                congrArg EpochNumber.toNat hstep
              omega
            -- The earlier epoch's case puts the decided last block before the parent.
            have hord := ih pp.epoch.toNat (by omega) bd hbdb (by rw [hbdep])
              p.parentCert.view.toNat p.parentCert (Nat.le_refl _) hpar'
              (by rw [hbdep, hppe])
              (show bd.view.toNat ≤ p.parentCert.view.toNat by
                have : c.view.toNat ≤ p.parentCert.view.toNat := hgap
                omega)
            have hsameblock : bd.data.blockHash = p.parentCert.data.blockHash :=
              ancestor_height_eq tree N hcfg hcoh hcf hres hpar' hord hbbt
                (by rw [hpph]; exact hpptree) (by rw [hbbh, hppheight])
            -- One block: its view is at or before `bd`'s, and at the parent's.
            have hbbpp : bb = pp :=
              Option.some_inj.mp (hbbt.symm.trans (by rw [hsameblock, hpph]; exact hpptree))
            have hppbd : pp.viewNumber.toNat ≤ bd.view.toNat :=
              hbbpp ▸ cert2_block_view tree N hcfg hres hbdb hbbt
            have : c.view.toNat ≤ p.parentCert.view.toNat := hgap
            rw [← hppvw] at this
            omega
          · rw [hep, hepc, hppe, hwf.epoch, hppwf.epoch, hsucc]
            by_cases hh : cfg.epochHeight = 0
            · simp [epochOf_eq, hh]
            · by_cases hz : pp.blockHeader.blockNumber.toNat = 0
              · rw [BlockNumber.ext hz]; exact epochOf_one _ hh
              · rw [epochOf_succ _ _ hh hz, ite_eq_right hlast]
        exact hlink (ihn p.parentCert (Nat.le_of_lt_succ (Nat.lt_of_lt_of_le hviewlt hle)) hpar'
          hsameE hgap)

/--
An epoch's decided last block is the only certified block of its epoch at or
after it.

Two last blocks of one epoch are at one height (`lastBlock_height`), no block of
the epoch is past that height (`epochOf_height_le`), and a block before another
at the same height is that block (`ancestor_height_eq`).
-/
theorem lastBlock_unique {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hcoh : TreeCoherent tree)
    (hcf : CollisionFree)
    (hres : Resolves cfg tree N) (hh : cfg.epochHeight ≠ 0)
    {bd : Cert2} (hbdb : Cert2Backed N.trace bd) {bb : Block}
    (hbbt : tree bd.data.blockHash = some bb)
    (hbbh : bb.blockHeader.blockNumber.toNat = bd.data.epoch.toNat * cfg.epochHeight)
    {c1 : Cert1} (hc1 : Cert1Backed N.trace c1)
    (hepq : bd.data.epoch = c1.data.epoch) (hview : bd.view ≤ c1.view) :
    bd.data.blockHash = c1.data.blockHash := by
  have hheights := heightSucceedsParent tree N hcfg hcoh hcf hres
  obtain ⟨p, hview1, hhash, hepc, hwf, htree, -, -, -, -⟩ := cert1_proposal tree N hres hc1
  have hord := cert2_ancestor_epoch tree N hcfg hcoh hcf hres
    bd.data.epoch.toNat bd hbdb rfl c1.view.toNat c1 (Nat.le_refl _) hc1 hepq hview
  have hle := ancestor_height_le tree N hcfg hcoh hcf hres c1.view.toNat c1
    (Nat.le_refl _) hc1
    bd.data.blockHash bb p hord hbbt (hhash ▸ htree)
  have hpbound : p.blockHeader.blockNumber.toNat ≤ bd.data.epoch.toNat * cfg.epochHeight := by
    rw [hepq, hepc, hwf.epoch]
    exact epochOf_height_le hh
  exact ancestor_height_eq tree N hcfg hcoh hcf hres hc1 hord hbbt
    (hhash ▸ htree) (by omega)

/--
A certified block has, before it, the decided last block of every earlier epoch.

`cert1_crosses_boundary` steps back one epoch; this iterates it until the epoch
asked for is the one reached.
-/
theorem cert1_reaches_epoch {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hres : Resolves cfg tree N)
    (hheights : HeightSucceedsParent tree N) (hh : cfg.epochHeight ≠ 0) :
    ∀ D, ∀ c1 : Cert1, Cert1Backed N.trace c1 → ∀ E : EpochNumber,
      cfg.startEpoch.toNat ≤ E.toNat →
      E.toNat < c1.data.epoch.toNat → c1.data.epoch.toNat - E.toNat ≤ D →
        ∃ bd : Cert2, Cert2Backed N.trace bd ∧ bd.data.epoch = E
          ∧ Ancestor cfg tree bd.data.blockHash c1.data.blockHash
          ∧ ∃ bb, tree bd.data.blockHash = some bb
              ∧ bb.blockHeader.blockNumber.toNat = E.toNat * cfg.epochHeight := by
  intro D
  induction D with
  | zero =>
    intro c1 hc1 E hEpos hlt hD
    omega
  | succ D ih =>
    intro c1 hc1 E hEpos hlt hD
    obtain ⟨bd, hbdb, hbde, -, hbda, bb, hbbt, hbbh⟩ :=
      cert1_crosses_boundary tree N hcfg hres hheights hh c1.view.toNat c1 (Nat.le_refl _) hc1
        (Nat.lt_of_le_of_lt hEpos hlt)
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
A `Cert2` is before every certified block that is later: of a later epoch, or of
its own epoch at a later view.

The result no-fork is read off.

* One epoch is `cert2_ancestor_epoch`.
* A later epoch on the certified side: `cert1_reaches_epoch` produces the
  decided last block of `c`'s own epoch, and `lastBlock_unique` or
  `cert2_ancestor_epoch` places `c` against it.
-/
theorem cert2_ancestor {C : Committee} (N : Network cfg C)
    (hcfg : ConfigCoherent cfg) (hcoh : TreeCoherent tree)
    (hcf : CollisionFree)
    (hres : Resolves cfg tree N)
    {c : Cert2} (hc2 : Cert2Backed N.trace c)
    {c1 : Cert1} (hc1 : Cert1Backed N.trace c1)
    (horder : EpochViewLE c.data.epoch c.view c1.data.epoch c1.view) :
    Ancestor cfg tree c.data.blockHash c1.data.blockHash := by
  have hheights := heightSucceedsParent tree N hcfg hcoh hcf hres
  obtain ⟨cc, hccb, hccv, hcch, hcce⟩ := cert2_implies_cert1 cfg N hcfg hc2
  obtain ⟨cp, -, -, hcpe, hcpwf, -, -, -, -, -⟩ := cert1_proposal tree N hres hccb
  obtain ⟨c1p, -, -, hc1pe, hc1pwf, -, -, -, -, -⟩ := cert1_proposal tree N hres hc1
  rcases (horder : EpochViewLE _ _ _ _) with hlt | ⟨heq, hview⟩
  · -- `c1` is in a later epoch: reach back to `c`'s own.
    have hlt : c.data.epoch.toNat < c1.data.epoch.toNat := hlt
    have hh : cfg.epochHeight ≠ 0 := by
      intro h0
      rw [← hcce, hcpe, hcpwf.epoch, hc1pe, hc1pwf.epoch] at hlt
      simp [epochOf_eq, h0] at hlt
    obtain ⟨r, hrb, hre, hra, bb, hbbt, hbbh⟩ :=
      cert1_reaches_epoch tree N hcfg hres hheights hh
        (c1.data.epoch.toNat - c.data.epoch.toNat) c1 hc1 c.data.epoch
        (by rw [← hcce]; exact cert1_epoch_after_start N hcfg _ _ (Nat.le_refl _) hccb) hlt
        (Nat.le_refl _)
    rcases Nat.lt_or_ge c.view.toNat r.view.toNat with hrv | hrv
    · obtain ⟨r1, hr1b, hr1v, hr1h, hr1e⟩ := cert2_implies_cert1 cfg N hcfg hrb
      refine Ancestor.trans (b := r1.data.blockHash) ?_ (by rw [hr1h]; exact hra)
      exact cert2_ancestor_epoch tree N hcfg hcoh hcf hres
        c.data.epoch.toNat c hc2 rfl r1.view.toNat r1 (Nat.le_refl _) hr1b
        (by rw [hr1e, hre]) (by rw [hr1v]; exact Nat.le_of_lt hrv)
    · have hsame := lastBlock_unique tree N hcfg hcoh hcf hres hh hrb hbbt
        (by rw [hbbh, hre]) hccb (by rw [hre, hcce]) (by rw [hccv]; exact hrv)
      rw [← hcch, ← hsame]
      exact hra
  · exact cert2_ancestor_epoch tree N hcfg hcoh hcf hres
      c.data.epoch.toNat c hc2 rfl c1.view.toNat c1 (Nat.le_refl _) hc1 heq hview

/--
**No fork.** Of any two blocks with a backed `Cert2`, the one certified earlier, by
epoch and then by view (`CertNotAfter`), is an ancestor of the other.
-/
theorem noFork (cfg : Config) (C : Committee) : NoFork cfg C := by
  intro N tree hcfg htc hcf hres c c' hc hc' hle
  obtain ⟨c1, hb1, hv1, hh1, he1⟩ := cert2_implies_cert1 cfg N hcfg hc'
  have := cert2_ancestor tree N hcfg htc hcf hres hc hb1
    (by rw [he1, hv1]; exact hle)
  rw [hh1] at this
  exact this

end NewProtocol
