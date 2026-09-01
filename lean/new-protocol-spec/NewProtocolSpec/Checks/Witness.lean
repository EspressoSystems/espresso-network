module

public import NewProtocolSpec.Safety
public import NewProtocolSpec.Gc

/-!
# A network the premises are satisfied by

`NewProtocolSpec.Checks.Examples` exhibits states that owe an action, so that no
obligation's guards are checked only in prose. This is the same idea one level
higher: it exhibits a `Network`, a `BlockTable` and a `Cert1` satisfying every
premise `decideSafety` reads, *and* certifying a block.

Why it is worth the space. Every premise here is a hypothesis of the no-fork
result, and a hypothesis nothing can satisfy makes the result true for no
reason. That has happened three times in this specification, each time hidden by
the last: a `Network.parentCertValid` with no exemption for the anchor made
certificates impossible; a `blockHash` that reduced made `CollisionFree`
refutable; and an anchor condition demanding a fixed point of `blockHash` made
the configuration unbuildable. None was caught by a build, and the first two
survived a review. A witness fails immediately under all three.

Two hypotheses are taken rather than proved, and could not be otherwise:
`CollisionFree` and `BlockValid` are the cryptographic and application notions
the model deliberately abstracts, and both are `opaque` here.
-/

@[expose] public section

namespace NewProtocol
namespace Witness

/-! ## The configuration -/

/-- The single node. -/
def me : PubKey := ⟨1⟩

/-- Genesis: block zero, at the genesis view. -/
def anchorB : Block :=
  ⟨⟨⟨0⟩, 0⟩, ViewNumber.genesis, epochOf 0 0,
    ⟨⟨⟨0⟩, ⟨0⟩⟩, ViewNumber.genesis⟩, none, ⟨7⟩⟩

/--
Epochs are switched off, which is the static-committee configuration; the epoch
extension has to leave that case working, and here it is.
-/
def cfg : Config where
  anchorBlock := anchorB
  anchorCert := ⟨⟨blockHash anchorB, epochOf 0 0⟩, ViewNumber.genesis⟩
  decideBuffer := 20
  epochHeight := 0

theorem cfg_coherent : ConfigCoherent cfg where
  anchorBlockView := rfl
  anchorCertView := rfl
  anchorBlockNumber := rfl
  anchorCertBlock := rfl
  anchorParentView := rfl

/-- The block the run certifies: the first proposal, on the anchor. -/
def blk : Block :=
  ⟨⟨⟨3⟩, 1⟩, ⟨1⟩, epochOf 1 0, cfg.anchorCert, none, ⟨9⟩⟩

/-- This node's share of its payload. -/
def share : VidShare := ⟨⟨1⟩, ⟨3⟩⟩

/-- The vote the run casts, and the certificate its quorum forms. -/
def theVote : Vote1 := ⟨⟨blockHash blk, blk.epoch⟩, ⟨1⟩, me⟩

def theCert : Cert1 := ⟨theVote.data, ⟨1⟩⟩

/-! ## The run

The machine of `new-protocol-impl` cannot be used: sealing `blockHash` as
`opaque`, which is what keeps `CollisionFree` from being refutable, also stops
the kernel reducing the machine's guards, so its steps cannot be evaluated in a
proof. A `Network` asks only for runs obeying the safety clauses, so the run is
given directly: ingest the proposal, then the validity report, casting the vote
with it; collect for ever after.
-/

/-- After the proposal: held, admitted, and its share in hand. -/
def st1 : NodeState :=
  { NodeState.initial cfg with
    proposals := fun v => if v = ⟨1⟩ then some blk
      else if v = ViewNumber.genesis then some cfg.anchorBlock else none
    admitted := fun v => if v = ⟨1⟩ then some blk else none
    vidShares := fun v => if v = ⟨1⟩ then some share else none }

/-- After the validity report, with the vote1 cast. -/
def st2 : NodeState :=
  { st1 with
    validated := fun v => if v = ⟨1⟩ then some (blockHash blk) else none
    voted1Views := fun v => v = ⟨1⟩
    vote1Branches := fun v => if v = ⟨1⟩ then some blk.parentCert.view else none }

/-- The states the run passes through. -/
def wstate : Nat → NodeState
  | 0 => NodeState.initial cfg
  | 1 => st1
  | _ => st2

/-- And the events between them. -/
def wevent : Nat → Event
  | 0 => .consensus (Input.proposal me blk share) []
  | 1 => .consensus (Input.blockValidated ⟨1⟩ (blockHash blk)) [Output.send (.vote1 theVote)]
  | _ => .collect

/-- Collecting nothing is a collection. -/
theorem gc_id (s : NodeState) : GcSpec cfg s s where
  shrinks :=
    { proposals := fun _ _ h => h
      admitted := fun _ _ h => h
      vidShares := fun _ _ h => h
      validated := fun _ _ h => h
      blocksReconstructed := fun _ _ h => h
      headers := fun _ _ _ h => h
      cert1s := fun _ _ h => h
      cert2s := fun _ _ h => h
      timeoutCerts := fun _ _ h => h
      vote1Branches := fun _ _ h => h }
  barredViewMono := Nat.le_refl _
  barredViewJustified := fun h => absurd rfl h
  keepsDecideAboveFloor := fun _ _ =>
    { proposals := fun _ h => h
      cert1s := fun _ h => h
      cert2s := fun _ h => h
      blocksReconstructed := fun _ h => h }
  keepsVoteAboveBar := fun _ _ =>
    { admitted := fun _ h => h
      vidShares := fun _ h => h
      validated := fun _ h => h
      headers := fun _ _ h => h
      timeoutCerts := fun _ h => h }
  floorStable := fun _ h => h
  decidedRetained := fun _ _ h => h
  decidedSound := fun _ h => h
  voted1Retained := fun _ _ h => h
  voted2Retained := fun _ _ h => h
  proposedRetained := fun _ _ h => h
  vote1BranchesRetained := fun _ _ _ h => h
  voted1Sound := fun _ h => h
  voted2Sound := fun _ h => h
  proposedSound := fun _ h => h
  lockSame := rfl
  currentViewSame := rfl
  timeoutViewSame := rfl

/-- Ingesting the proposal: content arrives, nothing is emitted. -/
theorem step0 : SafetySpec cfg me (NodeState.initial cfg)
    (Input.proposal me blk share) [] st1 where
  proposalProvenance := by
    intro v p hp
    simp only [st1] at hp
    by_cases hvv : v = (⟨1⟩ : ViewNumber)
    · rw [if_pos hvv] at hp
      obtain rfl : p = blk := (Option.some_inj.mp hp).symm
      exact Or.inr ⟨⟨me, share, rfl⟩, hvv.symm, ⟨by decide, Or.inl rfl, rfl⟩⟩
    · rw [if_neg hvv] at hp
      exact Or.inl hp
  admissionJustified := by
    intro v p hp
    simp only [st1] at hp
    by_cases hvv : v = (⟨1⟩ : ViewNumber)
    · rw [if_pos hvv] at hp
      obtain rfl : p = blk := (Option.some_inj.mp hp).symm
      subst hvv
      exact Or.inr ⟨me, share, rfl, rfl, by decide, trivial,
        ⟨by decide, Or.inl rfl, rfl⟩, ⟨rfl, rfl⟩,
        by simp [st1], by simp [st1]⟩
    · rw [if_neg hvv] at hp; exact absurd hp (by simp)
  cert1Provenance := fun v c hc => Or.inl (by simpa [st1] using hc)
  barredViewUnchanged := rfl
  vote1NotBarred := by intro v h; exact absurd h (by simp)
  vote2NotBarred := by intro v h; exact absurd h (by simp)
  lockMono := fun lock hl => absurd hl (by simp [NodeState.initial])
  decidedRetained := fun _ h => h
  voted1Retained := fun _ h => h
  vote1BranchesRetained := fun _ _ h => h
  voted2Retained := fun _ h => h
  vote1Once := by intro v h; exact absurd h (by simp)
  vote1Justified := by intro v h; exact absurd h (by simp)
  vote1Records := by intro v h; exact absurd h (by simp)
  vote2Once := by intro v h; exact absurd h (by simp)
  vote2Justified := by intro v h; exact absurd h (by simp)
  vote2LockOrdered := by intro v h; exact absurd h (by simp)
  vote2NotInSkippedView := by intro v h; exact absurd h (by simp)
  vote2AboveFloor := by intro v h; exact absurd h (by simp)
  lockJustified := fun lock hl => Or.inl (by simpa [st1] using hl)
  timeoutVoteSound := by intro v e h; exact absurd h (by simp)

/--
The validity report arrives, and the vote goes out with it.

`Vote1Justification` holds by construction: the block is admitted, its share is
held, nothing is locked so `SafeToExtend` is trivial, and the parent is the
anchor at genesis, which `parentLinked` exempts.
-/
theorem step1 (hv : ∀ b, BlockValid b) :
    SafetySpec cfg me st1 (Input.blockValidated ⟨1⟩ (blockHash blk))
      [Output.send (.vote1 theVote)] st2 where
  proposalProvenance := fun v p hp => Or.inl hp
  admissionJustified := fun v p hp => Or.inl hp
  cert1Provenance := fun v c hc => Or.inl hc
  barredViewUnchanged := rfl
  vote1NotBarred := by
    intro v h
    simp only [List.mem_singleton] at h
    injection h with h1
    injection h1 with h2
    subst h2
    decide
  vote2NotBarred := by intro v h; exact absurd h (by simp)
  lockMono := fun lock hl => absurd hl (by simp [st1, NodeState.initial])
  decidedRetained := fun _ h => h
  voted1Retained := by
    intro v h
    exact absurd h (by simp [st1, NodeState.initial])
  vote1BranchesRetained := by
    intro v u h
    exact absurd h (by simp [st1, NodeState.initial])
  voted2Retained := fun _ h => h
  vote1Once := by
    intro v h
    simp only [List.mem_singleton] at h
    injection h with h1
    injection h1 with h2
    subst h2
    exact ⟨by simp [st1, NodeState.initial], rfl⟩
  vote1Justified := by
    intro v h
    simp only [List.mem_singleton] at h
    injection h with h1
    injection h1 with h2
    subst h2
    refine ⟨blk, ?_, by simp [theVote, blk], rfl, rfl, rfl⟩
    exact { proposalAdmitted := by simp [st2, st1, blk]
            blockValid := hv blk
            vidShare := by simp [st2, st1, blk]
            safeToExtend := by simp [SafeToExtend, st2, st1, NodeState.initial]
            parentLinked := by
              intro hne
              exact absurd rfl hne }
  vote1Records := by
    intro v h
    simp only [List.mem_singleton] at h
    injection h with h1
    injection h1 with h2
    subst h2
    exact ⟨blk, by simp [st2, st1, theVote], by simp [st2, theVote]⟩
  vote2Once := by intro v h; exact absurd h (by simp)
  vote2Justified := by intro v h; exact absurd h (by simp)
  vote2LockOrdered := by intro v h; exact absurd h (by simp)
  vote2NotInSkippedView := by intro v h; exact absurd h (by simp)
  vote2AboveFloor := by intro v h; exact absurd h (by simp)
  lockJustified := fun lock hl => Or.inl (by simpa [st2] using hl)
  timeoutVoteSound := by intro v e h; exact absurd h (by simp)

/-- The run: two consensus steps, then collections for ever. -/
def wrun (hv : ∀ b, BlockValid b) : Run cfg (SafetySpec cfg me) where
  state := wstate
  event := wevent
  transition n := by
    match n with
    | 0 => exact Transition.step step0
    | 1 => exact Transition.step (step1 hv)
    | m + 2 => exact Transition.collect (gc_id st2)

/-! ## The committee and the network -/

/-- One honest node, and the only quorum is that node. -/
def com : Committee where
  honest k := k = me
  Quorum _ q := ∀ k, q k ↔ k = me
  intersect := fun _ _ _ hq hq' => ⟨me, (hq me).mpr rfl, (hq' me).mpr rfl, rfl⟩

/-- Steps are ordered by index; there is one node, so that is all the order. -/
def before (a b : NodeStep com) : Prop := a.index < b.index

theorem before_wf : WellFounded before :=
  InvImage.wf (fun s : NodeStep com => s.index) Nat.lt_wfRel.wf

theorem no_input_after (hv : ∀ b, BlockValid b) {n : Nat} {i : Input}
    (h : Run.Consumes (wrun hv) n i) : n = 0 ∨ n = 1 := by
  obtain ⟨o, ho⟩ := h
  match n with
  | 0 => exact Or.inl rfl
  | 1 => exact Or.inr rfl
  | m + 2 => exact absurd ho (by simp [wrun, wevent])

/-- The network: one honest node, running that run. -/
def net (hv : ∀ b, BlockValid b) : Network cfg com where
  run _ hk := hk ▸ wrun hv
  start k hk := by cases hk; rfl
  Before := before
  beforeNext _ _ n := Nat.lt_succ_self n
  beforeTrans := Nat.lt_trans
  beforeWF := before_wf
  evidenceValid := by
    rintro k rfl n sender p vid tc hcons hte
    obtain ⟨o, ho⟩ := hcons
    match n with
    | 0 =>
      simp only [wrun, wevent] at ho
      injection ho with h1 _
      injection h1 with _ h3 _
      rw [← h3] at hte
      exact absurd hte (by simp [blk])
    | 1 => simp only [wrun, wevent] at ho; injection ho with h1 _; exact absurd h1 (by simp)
    | m + 2 => exact absurd ho (by simp [wrun, wevent])
  timeoutOneHonestBacked := by
    rintro k rfl n v hcons
    obtain ⟨o, ho⟩ := hcons
    match n with
    | 0 => simp only [wrun, wevent] at ho; injection ho with h1 _; exact absurd h1 (by simp)
    | 1 => simp only [wrun, wevent] at ho; injection ho with h1 _; exact absurd h1 (by simp)
    | m + 2 => exact absurd ho (by simp [wrun, wevent])
  cert1Delivered := by
    rintro k rfl n c - hdel
    rcases hdel with ⟨o, ho⟩ | ⟨o, ho⟩ <;>
      match n with
      | 0 => simp only [wrun, wevent] at ho; injection ho with h1 _; exact absurd h1 (by simp)
      | 1 => simp only [wrun, wevent] at ho; injection ho with h1 _; exact absurd h1 (by simp)
      | m + 2 => exact absurd ho (by simp [wrun, wevent])
  parentCertValid := by
    rintro k rfl n sender p vid hcons
    obtain ⟨o, ho⟩ := hcons
    match n with
    | 0 =>
      simp only [wrun, wevent] at ho
      injection ho with h1 _
      injection h1 with _ h3 _
      exact Or.inr (by rw [← h3]; rfl)
    | 1 => simp only [wrun, wevent] at ho; injection ho with h1 _; exact absurd h1 (by simp)
    | m + 2 => exact absurd ho (by simp [wrun, wevent])
  boundaryDecided := by
    rintro k rfl n sender p vid - he
    exact absurd he.2.1 (by simp [cfg])

/-! ## The block tree, and the certificate -/

/-- The tree holds the anchor and the certified block, and nothing else. -/
def wtree : BlockTable := fun h =>
  if h = blockHash blk then some blk
  else if h = blockHash cfg.anchorBlock then some cfg.anchorBlock else none

theorem wtree_coherent : TreeCoherent wtree := by
  intro h b hb
  simp only [wtree] at hb
  by_cases h1 : h = blockHash blk
  · rw [if_pos h1] at hb; rw [← Option.some_inj.mp hb, h1]
  · rw [if_neg h1] at hb
    by_cases h2 : h = blockHash cfg.anchorBlock
    · rw [if_pos h2] at hb; rw [← Option.some_inj.mp hb, h2]
    · rw [if_neg h2] at hb; exact absurd hb (by simp)

/--
The anchor's parent link is not resolved, given that it does not collide with
either block's hash. `blockHash` is `opaque`, so nothing inside the model can
rule a collision out; it is an assumption of the same kind as `CollisionFree`.
-/
theorem wtree_rooted
    (h1 : cfg.anchorBlock.parentCert.data.blockHash ≠ blockHash blk)
    (h2 : cfg.anchorBlock.parentCert.data.blockHash ≠ blockHash cfg.anchorBlock) :
    AnchorRooted wtree cfg := by
  show (if _ then _ else if _ then _ else _) = none
  rw [if_neg h1, if_neg h2]

/-- **A network satisfying every premise, in which a block really is certified.** -/
theorem certificate_exists (hv : ∀ b, BlockValid b) :
    Network.ValidCert1 cfg (net hv) theCert :=
  ⟨fun k => k = me, fun _ => Iff.rfl, fun k hk hh => by
    cases hh
    exact ⟨1, Output.send (.vote1 theVote), by simp [net, wrun, wevent, Event.outputs], rfl⟩⟩

end Witness
end NewProtocol
