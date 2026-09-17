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

end NewProtocol
