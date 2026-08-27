module

public import NewProtocolSpec.Base

/-!
# Protocol data types

The data a node sends, receives and stores.
-/

@[expose] public section

namespace NewProtocol

/--
Hash identifying a whole block — header, view, parent certificate.

This is what votes sign, certificates certify and chain links point to.
Distinct from `PayloadCommit`, which covers only the transaction bytes.-/
structure BlockHash where
  /-- The value. Never read: hashes are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

/--
VID commitment to a block *payload* — the transaction bytes that are
erasure-coded and dispersed as shares.

Not the commitment of the whole block (that is `BlockHash`): the payload
is the only large part of a block and the only part travelling via VID, so
it has its own commitment for verifying shares and reconstructions. The
header contains it, and `BlockHash` covers the header, so the block hash
commits to the payload transitively.-/
structure PayloadCommit where
  /-- The value. Never read: commitments are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

/-- Public key identifying a node. -/
structure PubKey where
  /-- The value. Never read: keys are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Ord, Inhabited

/--
The part of a block header the core consensus logic reads.

The block height is not carried: it only feeds the epoch arithmetic, which returns with
the epoch extension.
-/
structure BlockHeader where
  /-- The commitment to the block's payload. -/
  payloadCommit : PayloadCommit
deriving DecidableEq, Repr, Inhabited

/--
What a vote1 (quorum) vote signs.

Reduced to the block hash; the epoch and block
number belong to the epoch machinery.
-/
structure Vote1Data where
  /-- The block being voted for. -/
  blockHash : BlockHash
deriving DecidableEq, Repr

/--
What a vote2 signs.

Reduced like `Vote1Data`, but deliberately a distinct type: it keeps `Cert1`
and `Cert2` apart at the type level, so no rule can confuse a vote of one round
for a vote of the other.
-/
structure Vote2Data where
  /-- The block being voted for. -/
  blockHash : BlockHash
deriving DecidableEq, Repr

/--
A certificate over vote data `α`, formed in view `view`.

The aggregate signature is not modelled. A certificate's validity — that a
quorum really signed `data` in `view` — is `Network.ValidCert1` and
`Network.ValidCert2` in
`NewProtocolSpec.Network.Defs`.
-/
structure Certificate (α : Type) where
  /-- What the quorum signed. -/
  data : α

  /-- The view the certificate was formed in. -/
  view : ViewNumber
deriving DecidableEq, Repr

/--
Quorum certificate over vote1s (QC).-/
abbrev Cert1 := Certificate Vote1Data

/--
`Cert2`; a block with a `Cert2` is decided.-/
abbrev Cert2 := Certificate Vote2Data

/-- Certificate that a view timed out. -/
abbrev TimeoutCert := Certificate Unit

/--
Proposal to append a block to the chain.

Carries what the protocol reads. The fields a real message adds for the epoch
machinery, light-client certification and version upgrades are not modelled.
-/
structure Proposal where
  /-- The block header to append. -/
  blockHeader : BlockHeader

  /-- The view this proposal is made in. -/
  viewNumber : ViewNumber

  /--
  Certificate for the parent block this proposal extends.

  The chain link: ancestry is followed through these, so the branch a proposal
  belongs to is exactly the chain of `parentCert`s below it. A `Cert1`, not a
  `Cert2` — a parent must be certified to be extended, not decided.
  -/
  parentCert : Cert1

  /--
  Proof that the views between `parentCert` and this proposal timed out.

  Required whenever `parentCert` is not for the immediately preceding view.
  -/
  timeoutEvidence : Option TimeoutCert

  /--
  The identity the network assigned this block, which `blockHash` reads.

  Carried rather than computed, because the model cannot compute it: a real
  commitment covers a serialised form over fields this type does not have.
  Carrying is also what makes the comparisons work at all — every hash test in
  the protocol puts this against an identity arriving inside a certificate
  (`SafeToExtend`, `StepSpec.decideJustified`, `Vote1Justification.parentLinked`),
  so the two must be values of the same provenance.

  For a proposal a node *builds*, no identity has been assigned yet and this
  field is meaningless. Nothing reads it: a proposer emits its proposal without
  storing or hashing it, and every rule that hashes reaches for a block the
  node *holds* — which arrived, and so carries a real one.

  That the identity is honest and distinguishes blocks is `CollisionFree`
  (`NewProtocolSpec.Assumptions`).
  -/
  identity : BlockHash
deriving DecidableEq, Repr

/--
A block of the chain.

A proposal and the block it proposes are one object here, and
carries the same data, so the model identifies the two.
-/
abbrev Block := Proposal

/--
Which block a hash identifies.

The blocks produced across a network form a tree: after timeouts or a leader
proposing twice, two proposals can name the same parent, so one block can have
more than one child. This is that tree's node table — it says which block a
hash names, and each block's `parentCert` supplies the edge to its parent.
`Ancestor` walks those edges.

It is not part of a node's state and not something an implementation builds. It
exists so that the safety statement has something to mean "ancestor" *in*: an
individual node holds only the blocks it happens to have, which is not enough to
trace a chain. What ties it to reality are two conditions on it, `Resolves` —
that it contains the blocks honest nodes actually hold — and `TreeCoherent` —
that it answers with a block of the hash it was asked about.
-/
abbrev BlockTable := BlockHash → Option Block

/-- The proposed block's payload commitment. -/
def Proposal.payloadCommit (p : Proposal) : PayloadCommit :=
  p.blockHeader.payloadCommit

/--
The hash identifying the proposed block.

`@[irreducible]`, which is what keeps this an abstraction rather than a definition: no proof
can see through to the projection, so every rule relates `blockHash` images without depending
on how they arise — exactly as when this was `opaque`. What the body buys is that the machine
can be executed (`NewProtocolImpl.Demo`), and hence driven against a real node.

Hashes the block a proposal carries.
-/
@[irreducible] def blockHash (b : Block) : BlockHash := b.identity

/--
The block is valid in the application's sense.

Opaque, and deliberately so: consensus carries blocks it does not interpret, so
validity is not something any rule here can compute. Whatever the application
means by it — typically that a block's requests are admissible and consistent
with the requests in its ancestors, recursively — is a black box to this
specification.

It appears in exactly one rule, `Vote1Justification.blockValid`: a node may not
cast a vote1 for an invalid block, so a `Cert1` is quorum evidence that
the block passed the application's own check. Nothing *obliges* a vote until the
node has been told (`Vote1Enabled` reads `NodeState.validated`, which is
what a node can act on), and what ties the two together is the one thing about
validity that has to be believed rather than proved; see
`NewProtocolSpec.Assumptions`.
-/
opaque BlockValid : Block → Prop

/--
This node's VID share of a block payload.

Reduced to the fields consensus reads.
-/
structure VidShare where
  /-- The view of the block this share belongs to. -/
  view : ViewNumber

  /-- The payload this is a share of. -/
  payloadCommit : PayloadCommit
deriving DecidableEq, Repr

/--
A single node's vote over data `α`.

Data, view and the voter's key. A signature would come too (the signature
is dropped).
-/
structure Vote (α : Type) where
  /-- What is signed. -/
  data : α

  /-- The view voted in. -/
  view : ViewNumber

  /-- Who cast it. A certificate needs a quorum of distinct signers. -/
  signer : PubKey
deriving DecidableEq, Repr

/-- Vote1. -/
abbrev Vote1 := Vote Vote1Data

/-- Vote2.-/
abbrev Vote2 := Vote Vote2Data

/--
Timeout vote.

See `TimeoutCert` for why the data is trivial.
-/
abbrev TimeoutVote := Vote Unit

/--
The highest certificate a node holds.

Attached to timeout votes so peers stuck on stale views can re-converge.-/
inductive CatchupEvidence where
  /-- The highest `Cert1` held: the peer may enter the view past it. -/
  | cert1 (cert : Cert1)

  /-- A timeout certificate, for a view that produced no `Cert1` at all. -/
  | timeout (cert : TimeoutCert)
deriving DecidableEq, Repr

end NewProtocol
