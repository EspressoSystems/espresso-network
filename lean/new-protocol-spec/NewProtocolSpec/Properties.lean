module

public import NewProtocolSpec.Timing

/-!
# What the protocol guarantees

The results the rules are for, each a named proposition. `noFork`,
`decideAgreement` and `decidesValid` prove `NoFork`, `DecideAgreement` and
`DecidesValid` (`NewProtocolSpec.Proofs.Safety`, `NewProtocolSpec.Proofs.Decide`). `Liveness.chainGrows`
(`NewProtocolSpec.Proofs.Liveness.Epochs`) proves `ChainGrows` for every epoch
height.

* `NoFork`: of any two blocks with a `Cert2`, the one certified earlier, by epoch
  and then by view, is an ancestor of the other.
* `DecideAgreement`: what honest nodes decide lies on one chain.
* `DecidesValid`: what honest nodes decide is valid.
* `ChainGrows`: after GST, decides keep coming, and every steady node keeps
  deciding new views; so does every node honest in infinitely many epochs, when
  epochs have blocks.

`NoFork`, `DecideAgreement` and `DecidesValid` read the signing rules and the
verification premises of `Network` only. `ChainGrows` also reads the rest of the protocol, the
promptness bound and the synchrony assumptions.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config)

/-! ## The block tree -/

section Tree

variable (tree : BlockTable)

/--
Ancestry between blocks, by hash, following parent certificates through `tree`.

The walk never steps back from a block at the anchor's view. Only the anchor sits
there among the blocks it reaches, and the anchor's parent link is outside the run.
-/
inductive Ancestor : BlockHash → BlockHash → Prop where
  /-- Every block is its own ancestor. -/
  | refl (h : BlockHash) : Ancestor h h
  /-- An ancestor of the parent is an ancestor of the block, unless the block is at the anchor's view. -/
  | step {a c : BlockHash} {b : Block} : tree c = some b →
      Ancestor a b.parentCert.data.blockHash → b.viewNumber ≠ cfg.anchorView → Ancestor a c

/-- `tree` answers only with a block of the hash it was asked about. -/
def TreeCoherent : Prop :=
  ∀ h b, tree h = some b → blockHash b = h

/--
The hash identifies its block.

A cryptographic assumption: no run exhibits a collision. It can be stated of the
function only because `blockHash` is opaque. The body Lean compiles for it, the
block's `identity`, does collide, but no proof can see that body, so the
assumption is consistent; it is a hypothesis about runs, not a claim about that
body.
-/
def CollisionFree : Prop :=
  ∀ b b' : Block, blockHash b = blockHash b' → b = b'

/-- `tree` has every proposal an honest node holds. -/
def Resolves {C : Committee} (N : Network cfg C) : Prop :=
  ∀ k h n b, ((N.trace k h).history n).HasProposal cfg b → tree (blockHash b) = some b

end Tree

/-! ## Safety -/

/-- `c` is no later than `c'` (`EpochViewLE`). -/
def CertNotAfter (c c' : Cert2) : Prop := EpochViewLE c.data.epoch c.view c'.data.epoch c'.view

/--
Of any two blocks with a `Cert2`, the one whose `Cert2` is no later (`CertNotAfter`)
is an ancestor of the other.
-/
def NoFork (C : Committee) : Prop :=
  ∀ (N : Network cfg C) (tree : BlockTable), ConfigCoherent cfg → TreeCoherent tree →
    CollisionFree → Resolves cfg tree N →
    ∀ c c', Cert2Backed N.trace c → Cert2Backed N.trace c' → CertNotAfter c c' →
      Ancestor cfg tree c.data.blockHash c'.data.blockHash

/-- Node `k` delivered block `b` to the application at some step, on a `Cert2` of an epoch it is honest in. -/
def DecidedBlock {C : Committee} (N : Network cfg C) (k : PubKey) (h : C.Honest k) (b : Block) : Prop :=
  ∃ n blocks c1 c2, Output.decided blocks c1 c2 ∈ (N.trace k h n).output ∧ b ∈ blocks
    ∧ C.honest c2.data.epoch k

/--
Of any two blocks any two honest nodes decide, one is an ancestor of the other.

A decide counts when its node is honest in the epoch of the `Cert2` it decides on
(`DecidedBlock`).
-/
def DecideAgreement (C : Committee) : Prop :=
  ∀ (N : Network cfg C) (tree : BlockTable), ConfigCoherent cfg → TreeCoherent tree →
    CollisionFree → Resolves cfg tree N →
    ∀ k h k' h' b b', DecidedBlock cfg N k h b → DecidedBlock cfg N k' h' b' →
      Ancestor cfg tree (blockHash b) (blockHash b') ∨ Ancestor cfg tree (blockHash b') (blockHash b)

/--
Every block an honest node decides is valid.

A decide counts as in `DecidedBlock`. Validity is the application's (`BlockValid`):
the rules let an honest node vote1 only for a valid block (`SafeHistory.vote1Justified`),
so this follows from every decided block having an honest vote1. How a node learns
validity is not the rules' concern; the reference machine takes the validity
reports it receives as truthful.
-/
def DecidesValid (C : Committee) : Prop :=
  ∀ (N : Network cfg C), ConfigCoherent cfg → CollisionFree →
    ∀ k h b, DecidedBlock cfg N k h b → BlockValid b

/-! ## Liveness -/

/--
After GST, decides keep coming, and every steady node keeps deciding.

Chain growth: for every time `t` some node decides, after `t`, a view it had not
decided by `t`, on a `Cert2` of an epoch it is honest in (`TimedNetwork.DecidesAfter`).
There are finitely many honest nodes, so one of them decides ever later views, and
with `DecideAgreement` what honest nodes decide lies on one chain: the chain keeps
growing. And a node honest in every epoch keeps deciding itself. When epochs have
blocks, so does a node honest in infinitely many epochs (`Committee.HonestOften`):
the chain passes through every epoch, and the last block of each epoch the node is
honest in reaches it as an epoch change (`Synchrony.epochChange`), which it
decides. With epoch height zero there is one epoch, the start epoch: a node honest
in it keeps deciding, and a node not honest in it is promised nothing.

A node is not promised every block: a decide is owed only on a `Cert2` it holds and
for views past its floor (`History.AfterFloor`), and may skip ancestors it does not
hold. A block committed only through a later block's `Cert2` is never owed.

The premises are:
* the network's: its members honest in each epoch are a quorum of the epoch's
  committee (`TimedNetwork.honestQuorum`), finitely many nodes are honest
  (`Committee.honestFinite`), and certificates are genuine (`Network`);
* the full protocol (`TimedNetwork.protocol`) and promptness within `δ` (`Prompt`);
* synchrony after GST with bound `Δ`, and truthful validity reports
  (`Synchrony`), and a view timer `τ` long enough for a view to complete;
* a leader schedule that keeps giving every epoch honest leaders (`LeaderRotation`);
* a coherent configuration (`ConfigCoherent`) and no hash collisions
  (`CollisionFree`).

The timer has to outlast a view with an honest leader, which `8 * Δ + 3 * δ < τ`
counts. From the first honest node entering the view: every honest node is in it
by `2Δ`, since a timeout certificate may take that long to spread; the leader can
lock on what put it there by `3Δ`, has a header by `4Δ` and proposes by `4Δ + δ`;
the members have the proposal and its validity report by `6Δ + δ` and vote1 by
`6Δ + 2δ`; every honest node has the `Cert1` by `7Δ + 2δ`, every member can lock on it by
`8Δ + 2δ` and votes2 by `8Δ + 3δ`. No honest node times the view out before `τ`
has passed since it entered.
-/
def ChainGrows (leader : EpochNumber → ViewNumber → Option PubKey) (C : Committee) : Prop :=
  ∀ (N : TimedNetwork cfg leader C) (GST Δ δ τ : Nat), ConfigCoherent cfg → CollisionFree →
    Synchrony N GST Δ τ → Prompt N δ → 8 * Δ + 3 * δ < τ → LeaderRotation C leader →
    (∀ t, ∃ k, ∃ h : C.Honest k, N.DecidesAfter k h t)
      ∧ ∀ t k (h : C.Honest k),
        (C.Steady k ∨ (cfg.epochHeight = 0 ∧ C.honest cfg.startEpoch k)
          ∨ (cfg.epochHeight ≠ 0 ∧ C.HonestOften k)) →
        N.DecidesAfter k h t

end NewProtocol
