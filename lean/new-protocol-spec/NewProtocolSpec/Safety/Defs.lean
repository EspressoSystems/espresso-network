module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Network

/-!
# The block tree, and what the conclusion is phrased with

`Ancestor`, which follows the `parentCert` links through a `BlockTable`, and
`Resolves`, the condition that the table holds the blocks the network does.
-/

@[expose] public section

namespace NewProtocol

variable (tree : BlockTable)

/--
Ancestry between blocks, by hash.

`Ancestor tree a c` holds when the block identified by `a` is reachable from
the block identified by `c` by following `parentCert` links downwards (reflexively).
-/
inductive Ancestor : BlockHash → BlockHash → Prop where
  /-- Every block is its own ancestor. -/
  | refl (h : BlockHash) : Ancestor h h
  /-- And an ancestor of the parent is an ancestor of the block. -/
  | step {a c : BlockHash} {b : Block} : tree c = some b →
      Ancestor a b.parentCert.data.blockHash → Ancestor a c

/-- Ancestry composes: a block before `b`, with `b` before `c`, is before `c`. -/
theorem Ancestor.trans {tree : BlockTable} {a b c : BlockHash}
    (hab : Ancestor tree a b) (hbc : Ancestor tree b c) : Ancestor tree a c := by
  induction hbc with
  | refl => exact hab
  | step ht hrest ih => exact Ancestor.step ht ih

/--
The tree resolves every block an honest node holds.

Ancestry is walked through `tree`, so a chain can only be traced if the tree
actually contains its blocks. Certified blocks are held by the honest nodes
that voted for them, so this is a property of a faithful `tree`, not an extra
assumption about the protocol.
-/
def Resolves {C : Committee} (N : Network cfg C) : Prop :=
  ∀ k (h : C.honest k) n v b,
    (Run.state (N.run k h) n).proposals v = some b → tree (blockHash b) = some b

/--
A certified block's height is one more than its parent's.

What makes `epochOf` mean anything. Without it a chain could cross a boundary
without passing through it, and a block's epoch would say nothing about the
epochs of the blocks before it.

Asked only of *certified* blocks, and that restriction is the whole of what
makes it satisfiable. `Resolves` puts every proposal an honest node holds into
the tree, and a node holds proposals it never admitted; nothing constrains
their heights, so a tree-wide version could not be met alongside `Resolves` and
would leave no-fork vacuously true. A certified block is one an honest node
voted for, and a node votes only after the state validation that checks the
height — so this is exactly the range over which the implementation establishes
it.

Not a rule, because consensus does not check it: the check sits in the state
validation, outside the component modelled here, which is why it arrives as a
condition rather than a clause.

Only the anchor is exempt, and by identity rather than by view. Exempting every
block whose parent link sits at genesis would exempt the anchor's *children*
too, leaving the first real block's height unconstrained — and the anchor's own
link names itself (`ConfigCoherent.anchorParentBlock`), so without an exemption
it would have to be one more than its own.
-/
def HeightSucceedsParent {C : Committee} (N : Network cfg C) : Prop :=
  ∀ c1 : Cert1, Network.ValidCert1 cfg N c1 → ∀ b parent : Block,
    tree c1.data.blockHash = some b → b ≠ cfg.anchorBlock →
    tree b.parentCert.data.blockHash = some parent →
    b.blockHeader.blockNumber = parent.blockHeader.blockNumber + 1

end NewProtocol
