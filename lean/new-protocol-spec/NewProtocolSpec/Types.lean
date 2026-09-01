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

/-- The part of a block header the core consensus logic reads. -/
structure BlockHeader where
  /-- The commitment to the block's payload. -/
  payloadCommit : PayloadCommit

  /-- The block's height in the chain. -/
  blockNumber : Nat
deriving DecidableEq, Repr, Inhabited

/--
What a vote1 (quorum) vote signs.

The block, and the epoch whose committee is entitled to certify it. The epoch
is signed rather than derived because it is what a verifier needs *before* it
has the block: a certificate is checked against the stake table its own epoch
names. `SafetySpec.vote1Justified` is what ties it to the proposal voted for,
and `ProposalWellFormed` ties that to the block number.
-/
structure Vote1Data where
  /-- The block being voted for. -/
  blockHash : BlockHash

  /-- The epoch whose committee certifies this vote. -/
  epoch : EpochNumber
deriving DecidableEq, Repr

/--
What a vote2 signs.

The same data as `Vote1Data`, but deliberately a distinct type: it keeps `Cert1`
and `Cert2` apart at the type level, so no rule can confuse a vote of one round
for a vote of the other.
-/
structure Vote2Data where
  /-- The block being voted for. -/
  blockHash : BlockHash

  /-- The epoch whose committee certifies this vote. -/
  epoch : EpochNumber
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

/--
What a timeout vote signs.

Only the epoch: the view timed out is the vote's own `Vote.view`, and there is
no block. The epoch is here for the same reason it is on `Vote1Data` — it names
the committee entitled to form the certificate, and a verifier needs it before
it has anything to derive it from.
-/
structure TimeoutData where
  /-- The epoch whose committee certifies this vote. -/
  epoch : EpochNumber
deriving DecidableEq, Repr, Inhabited

/--
Certificate that a view timed out.

`view` is the view that timed out; the certificate is stored under the view it
advances *into*, i.e. `view + 1`. Several rules read it that way
(`StepSpec.timeoutCertProvenance`, `StepSpec.timeoutCertIngested`,
`ParentCertJustified`).
-/
abbrev TimeoutCert := Certificate TimeoutData

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

  /-- The epoch whose committee governs this block. -/
  epoch : EpochNumber

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

`opaque`, which is what keeps this an abstraction rather than a definition: no proof can see
through to the projection, so every rule relates `blockHash` images without depending on how
they arise. The body is still there for the compiler, so the machine can be executed
(`NewProtocolImpl.Demo`) and hence driven against a real node.

`@[irreducible] def` will not do, and was tried: it blocks reduction at default transparency
but leaves the equation lemma, so `unfold blockHash` reaches the projection. That is enough to
exhibit two blocks differing only in view with one identity, which refutes `CollisionFree`
outright and makes every result guarded by it vacuous.

Hashes the block a proposal carries.
-/
opaque blockHash (b : Block) : BlockHash := b.identity

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

/-- Timeout vote. -/
abbrev TimeoutVote := Vote TimeoutData

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
