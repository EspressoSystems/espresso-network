module

public import NewProtocolSpec.Proofs.Safety

/-!
# Agreement on decides, and their validity

What honest nodes deliver to the application lies on one chain, and is valid. Each
decide carries a `Cert2` the node holds, which is backed, and every block it
delivers is before that certificate's block; `noFork` orders the certificates.
Each block it delivers has a backed `Cert1` over it, and an honest signer of that
voted for the block, which it may do only for a valid one (`cert1_valid`).
-/

@[expose] public section

namespace NewProtocol

/-- Two blocks before a third are ordered: each block has one parent to step back to. -/
theorem Ancestor.comparable {tree : BlockTable} {x y z : BlockHash}
    (hx : Ancestor cfg tree x z) (hy : Ancestor cfg tree y z) :
    Ancestor cfg tree x y ∨ Ancestor cfg tree y x := by
  induction hx generalizing y with
  | refl => exact Or.inr hy
  | step hb hrest hgen ih =>
    cases hy with
    | refl => exact Or.inl (Ancestor.step hb hrest hgen)
    | step hb' hrest' =>
      rw [hb] at hb'
      cases hb'
      exact ih hrest'

/-- Every block of a linked chain the tree resolves, none at genesis, is before its head. -/
theorem ChainLinked.ancestor {tree : BlockTable} :
    ∀ {head : Block} {rest : List Block},
      (∀ b ∈ head :: rest, tree (blockHash b) = some b) →
      (∀ b ∈ head :: rest, b.viewNumber ≠ cfg.anchorView) → ChainLinked (head :: rest) →
        ∀ b ∈ head :: rest, Ancestor cfg tree (blockHash b) (blockHash head)
  | head, [], _, _, _, b, hb => by
    obtain rfl : b = head := by simpa using hb
    exact .refl _
  | head, b' :: rest, htree, hgen, hlinked, b, hb => by
    obtain ⟨-, hph, hrest⟩ := hlinked
    have hstep : Ancestor cfg tree (blockHash b') (blockHash head) :=
      .step (htree head (List.mem_cons_self ..)) (hph ▸ .refl _) (hgen head (List.mem_cons_self ..))
    rcases List.mem_cons.mp hb with rfl | hb
    · exact .refl _
    · exact (ChainLinked.ancestor (fun c hc => htree c (List.mem_cons_of_mem _ hc))
        (fun c hc => hgen c (List.mem_cons_of_mem _ hc)) hrest b hb).trans hstep

/-- A block a node decides, on a `Cert2` of an epoch it is honest in, is before the block of a backed `Cert2`. -/
theorem decided_certified {cfg : Config} {C : Committee} (tree : BlockTable) (N : Network cfg C)
    (hres : Resolves cfg tree N) {k : PubKey} {h : C.Honest k} {b : Block}
    (hd : DecidedBlock cfg N k h b) :
    ∃ c2 : Cert2, Cert2Backed N.trace c2 ∧ Ancestor cfg tree (blockHash b) c2.data.blockHash := by
  obtain ⟨n, blocks, c1, c2, hmem, hb, he⟩ := hd
  obtain ⟨head, rest, rfl, hc2, hcommit, -, -, hlinked, hall⟩ :=
    (N.safe_at k h n).decideJustified n (N.trace k h n) blocks c1 c2
      (Trace.history_getElem? _ (Nat.lt_succ_self n)) hmem he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hc2 hall
  have htree : ∀ x ∈ head :: rest, tree (blockHash x) = some x :=
    fun x hx => hres k h (n + 1) x (hall x hx).1
  have hgen : ∀ x ∈ head :: rest, x.viewNumber ≠ cfg.anchorView :=
    fun x hx hz => absurd (hz ▸ (hall x hx).2) (Nat.lt_irrefl _)
  refine ⟨c2, cert2_held_backed N hc2, ?_⟩
  rw [show c2.data.blockHash = blockHash head from congrArg Vote2Data.blockHash hcommit.2]
  exact ChainLinked.ancestor htree hgen hlinked b hb

/-- Of two `Cert2`s, one is no later than the other. -/
theorem CertNotAfter.total (c c' : Cert2) : CertNotAfter c c' ∨ CertNotAfter c' c := by
  unfold CertNotAfter EpochViewLE
  rcases Nat.lt_trichotomy c.data.epoch.toNat c'.data.epoch.toNat with h | h | h
  · exact Or.inl (Or.inl h)
  · have he : c.data.epoch = c'.data.epoch := EpochNumber.ext h
    rcases Nat.le_total c.view.toNat c'.view.toNat with hv | hv
    · exact Or.inl (Or.inr ⟨he, hv⟩)
    · exact Or.inr (Or.inr ⟨he.symm, hv⟩)
  · exact Or.inr (Or.inl h)

/-- **Agreement on decides.** Of any two blocks any two honest nodes decide, one is an ancestor of the other. -/
theorem decideAgreement (cfg : Config) (C : Committee) : DecideAgreement cfg C := by
  intro N tree hcfg htc hcf hres k h k' h' b b' hd hd'
  obtain ⟨c2, hc2, hanc⟩ := decided_certified tree N hres hd
  obtain ⟨c2', hc2', hanc'⟩ := decided_certified tree N hres hd'
  rcases CertNotAfter.total c2 c2' with hle | hle
  · have hc := noFork cfg C N tree hcfg htc hcf hres c2 c2' hc2 hc2' hle
    exact Ancestor.comparable (hanc.trans hc) hanc'
  · have hc := noFork cfg C N tree hcfg htc hcf hres c2' c2 hc2' hc2 hle
    exact Ancestor.comparable hanc (hanc'.trans hc)

/-! ## Validity of decides -/

/--
A backed `Cert1` is over a valid block: an honest signer voted for the block, which
it may only do for a valid one (`cert1_origin`).
-/
theorem cert1_valid {cfg : Config} {C : Committee} (N : Network cfg C) (n : Nat) (c : Cert1)
    (hle : c.view.toNat ≤ n) (hc : Cert1Backed N.trace c) :
    ∃ p, BlockValid p ∧ c.data.blockHash = blockHash p :=
  let ⟨_, _, _, _, p, _, _, _, hval, hdata, _⟩ := cert1_origin N n c hle hc
  ⟨p, hval, congrArg Vote1Data.blockHash hdata⟩

section Valid

variable {cfg : Config} {C : Committee} (N : Network cfg C) (hcf : CollisionFree)
include hcf

/-- The block a backed `Cert1` is over is valid. -/
theorem valid_of_backed {c : Cert1} {x : Block} (hc : Cert1Backed N.trace c)
    (hx : c.data.blockHash = blockHash x) : BlockValid x := by
  obtain ⟨p, hval, hp⟩ := cert1_valid N _ c (Nat.le_refl _) hc
  rw [hcf x p (hx.symm.trans hp)]
  exact hval

/-- The block a backed `Cert1` is over names a parent by a backed `Cert1`, or by the anchor's. -/
theorem parent_of_backed {c : Cert1} {x : Block} (hc : Cert1Backed N.trace c)
    (hx : c.data.blockHash = blockHash x) :
    x.parentCert = cfg.anchorCert ∨ Cert1Backed N.trace x.parentCert := by
  obtain ⟨k, hk, m, sender, p, vid, hin, -, -, hdata, -, -⟩ := cert1_origin N _ c (Nat.le_refl _) hc
  have hxp : x = p := hcf x p (hx.symm.trans (congrArg Vote1Data.blockHash hdata))
  subst hxp
  exact N.parentGenuine k hk m sender x vid hin

/-- Every block of a linked chain after genesis whose newest block has a backed `Cert1` over it is valid. -/
theorem chain_valid (hcfg : ConfigCoherent cfg) :
    ∀ (l : List Block) (x : Block), ChainLinked (x :: l) →
      (∀ y ∈ x :: l, cfg.anchorView < y.viewNumber) →
      (∃ c, Cert1Backed N.trace c ∧ c.data.blockHash = blockHash x) →
      ∀ y ∈ x :: l, BlockValid y := by
  intro l
  induction l with
  | nil =>
    intro x _ _ ⟨c, hc, hh⟩ y hy
    rw [List.mem_singleton.mp hy]
    exact valid_of_backed N hcf hc hh
  | cons x' l ih =>
    intro x hl hg ⟨c, hc, hh⟩ y hy
    rcases List.mem_cons.mp hy with rfl | hy
    · exact valid_of_backed N hcf hc hh
    · obtain ⟨-, hlink, hl'⟩ := hl
      refine ih x' hl' (fun z hz => hg z (List.mem_cons_of_mem _ hz)) ?_ y hy
      rcases parent_of_backed N hcf hc hh with hanc | hpb
      · exfalso
        have hh' : blockHash x' = blockHash cfg.anchorBlock := by
          rw [← hlink, hanc, hcfg.anchorCertBlock]
        have hx' : x' = cfg.anchorBlock := hcf _ _ hh'
        have hgx := hg x' (List.mem_cons_of_mem _ (List.mem_cons_self))
        rw [hx'] at hgx
        exact Nat.lt_irrefl _ hgx
      · exact ⟨x.parentCert, hpb, hlink⟩

end Valid

/-- **Decides are valid.** Every block an honest node decides is valid. -/
theorem decidesValid (cfg : Config) (C : Committee) : DecidesValid cfg C := by
  intro N hcfg hcf k h b hd
  obtain ⟨n, blocks, c1, c2, hmem, hb, he⟩ := hd
  obtain ⟨head, rest, rfl, hc2, hcommit, -, -, hlinked, hall⟩ :=
    (N.safe_at k h n).decideJustified n (N.trace k h n) blocks c1 c2
      (Trace.history_getElem? _ (Nat.lt_succ_self n)) hmem he
  rw [Trace.history_upTo _ (Nat.le_refl _)] at hc2
  obtain ⟨c, hcb, -, hch, -⟩ := cert2_implies_cert1 cfg N hcfg (cert2_held_backed N hc2)
  exact chain_valid N hcf hcfg rest head hlinked (fun y hy => (hall y hy).2)
    ⟨c, hcb, hch.trans (congrArg Vote2Data.blockHash hcommit.2)⟩ b hb

end NewProtocol
