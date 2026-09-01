module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types
public import NewProtocolSpec.Interface

/-!
# What is assumed

The premises the safety argument takes rather than proves — the part that has to
be believed, so it is kept short and alone. The free-standing ones are here; the
rest are fields of `Committee` and `Network`, which is where a premise about an
object has to live (`NewProtocolSpec.Network.Defs`).

One more lives with the results that consume it: `HeightSucceedsParent`
(`NewProtocolSpec.Safety.Defs`) says a certified block's height is one more than
its parent's. It is stated there rather than here because it is a condition on
the block tree and on the network, like `Resolves`, and cannot be phrased before
either exists.

The progress results add none. `LiveNetwork` holds a `Network` and its premises
unchanged, and asks two things more of it — that its nodes obey all of `StepSpec`
rather than `SafetySpec` alone, and `WeaklyFair` — both of which `Conforms`
obliges an implementation to exhibit. Everything else the results of
`NewProtocolSpec.Progress` and `NewProtocolSpec.Deadlock` need is a hypothesis of
the statement rather than a standing premise: that the environment delivers, and
that nothing overtakes the view while the node has yet to act.
-/

@[expose] public section

namespace NewProtocol

variable (tree : BlockTable)

/--
The environment reports validity truthfully.

What an implementation supplies as the `envOk` of `Implements`: whenever it is handed
`Input.blockValidated`, the block named really is `BlockValid`. Quantified
over every block with the given hash, because that is all the input carries —
under `CollisionFree` there is only one.

It is unfalsifiable here by construction: consensus does not interpret blocks,
so `BlockValid` is opaque and this is the sole bridge to it. Note what rests on
it — building a conforming node, and nothing else. `DecideSafety` never reads
validity, so no-fork is independent of it.
-/
def ValidityReported (i : Input) : Prop :=
  ∀ v h, i = Input.blockValidated v h → ∀ b : Block, blockHash b = h → BlockValid b

/-- `tree` only maps a hash to a block that actually hashes to it. -/
def TreeCoherent : Prop :=
  ∀ (h : BlockHash) (b : Block), tree h = some b → blockHash b = h

/--
The hash identifies its block.

The model compares `blockHash` images wherever the implementation compares
commitments, so every argument that concludes "same hash, therefore same
block" needs this.

It is stated of the function, and could not be stated of a run: `blockHash` is
`opaque`, so nothing here can say which blocks a concrete hash sends together.
A real hash is not injective, and what is assumed is that no run exhibits a
collision — a cryptographic break rather than a protocol failure. The reason
`blockHash` must stay `opaque` is this premise: give it a visible body and the
premise is refutable, and everything guarded by it holds vacuously.
-/
def CollisionFree : Prop :=
  ∀ b b' : Block, blockHash b = blockHash b' → b = b'

end NewProtocol
