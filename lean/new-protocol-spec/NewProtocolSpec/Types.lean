module

public import NewProtocolSpec.Base

/-!
# Protocol data

What nodes send each other and what a certificate is made of. Everything here is
content: nothing says when a node may send it, which is `NewProtocolSpec.Rules`.
-/

@[expose] public section

namespace NewProtocol

/--
Hash identifying a whole block: header, view, parent certificate and timeout evidence.

What votes sign, certificates certify and chain links point to. Distinct from
`PayloadCommit`, which covers only the transaction bytes.
-/
structure BlockHash where
  /-- The value. Never read: hashes are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

/--
Commitment to a block's payload, the transaction bytes that are erasure-coded
and dispersed as VID shares.

The header contains it and the block hash covers the header, so the block hash
commits to the payload too.
-/
structure PayloadCommit where
  /-- The value. Never read: commitments are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Inhabited, Ord

/-- Public key identifying a node. -/
structure PubKey where
  /-- The value. Never read: keys are only compared and stored. -/
  toNat : Nat
deriving DecidableEq, Repr, Ord, Inhabited

/-- The part of a block header consensus reads. -/
structure BlockHeader where
  /-- The commitment to the block's payload. -/
  payloadCommit : PayloadCommit

  /-- The block's height in the chain. -/
  blockNumber : BlockNumber
deriving DecidableEq, Repr, Inhabited

/--
What a vote1 signs: the block, the epoch whose committee certifies it, and its
height.

The epoch is signed because a verifier needs it before it has the block: a
certificate is checked against the stake table of the epoch it names. The height
is signed because a certificate is often all a node has of a block, and the
epoch arithmetic is about heights.
-/
structure Vote1Data where
  /-- The block voted for. -/
  blockHash : BlockHash

  /-- The epoch whose committee certifies the vote. -/
  epoch : EpochNumber

  /-- The height of the block voted for. -/
  blockNumber : BlockNumber
deriving DecidableEq, Repr

/--
What a vote2 signs: the same fields as `Vote1Data`, as a distinct type.

Keeping the two apart means no rule can mistake a vote of one round for a vote of
the other.
-/
structure Vote2Data where
  /-- The block voted for. -/
  blockHash : BlockHash

  /-- The epoch whose committee certifies the vote. -/
  epoch : EpochNumber

  /-- The height of the block voted for. -/
  blockNumber : BlockNumber
deriving DecidableEq, Repr

/-- The same fields, as `Vote2Data`. -/
def Vote1Data.toVote2 (d : Vote1Data) : Vote2Data := ⟨d.blockHash, d.epoch, d.blockNumber⟩

/--
A certificate over data `α`, formed in view `view`.

The aggregate signature is not modelled. That a quorum really signed `data` in
`view` is what `Cert1Backed` and its companions say. A timeout certificate is the
exception: its signers' votes differ in their locks, and its `data` names a lock at
least as late as each of them (`TimeoutCertBacked`).
-/
structure Certificate (α : Type) where
  /-- What the quorum signed; for a timeout certificate, see `TimeoutData`. -/
  data : α

  /-- The view the certificate was formed in. -/
  view : ViewNumber
deriving DecidableEq, Repr

/-- A certificate over vote1s: the block certificate. -/
abbrev Cert1 := Certificate Vote1Data

/-- A certificate over vote2s: the commit certificate. A block with one is decided. -/
abbrev Cert2 := Certificate Vote2Data

/--
What a timeout vote signs: the epoch whose committee is asked to form the
certificate, and the sender's lock.

The view timed out is the vote's own view. A timeout certificate carries the same
fields, but its signers signed different locks: its `lock` is at least as late as
each of them, and names the block a proposal after the timeout must not skip
(`SafeParent`).
-/
structure TimeoutData where
  /-- The epoch whose committee certifies the vote. -/
  epoch : EpochNumber

  /-- The sender's lock; in a certificate, one at least as late as each of its signers'. -/
  lock : Cert1
deriving DecidableEq, Repr

/-- A certificate that view `view` timed out. It lets nodes enter `view + 1`. -/
abbrev TimeoutCert := Certificate TimeoutData

/--
A proposal to append a block to the chain.

Carries what consensus reads. Fields a real proposal adds for light clients,
state certificates and version upgrades are not modelled.
-/
structure Proposal where
  /-- The block header. -/
  blockHeader : BlockHeader

  /-- The view the proposal is made in. -/
  viewNumber : ViewNumber

  /-- The epoch whose committee governs the block. -/
  epoch : EpochNumber

  /--
  The certificate over the parent block.

  The chain link: ancestry follows these, so the branch a block belongs to is the
  chain of parent certificates before it.
  -/
  parentCert : Cert1

  /-- The timeout certificate for the views the proposal skips, if it skips any. -/
  timeoutEvidence : Option TimeoutCert

  /--
  The identity the network assigned the block, which `blockHash` reads.

  Carried rather than computed: a real commitment covers a serialised form over
  fields this type does not have. For a proposal a node builds, no identity has
  been assigned yet and nothing reads this field.
  -/
  identity : BlockHash
deriving DecidableEq, Repr

/--
A block of the chain, by its proposal.

Three words are kept apart. A *proposal* is what a leader sends and what a node
holds of a block: its header, view, epoch, parent certificate and timeout
evidence, without the payload. The *payload* is the block's contents, which a
node rebuilds from VID shares; the specification never reads it, and has it only
as a commitment (`BlockHeader.payloadCommit`) and as the fact that a node rebuilt
it (`History.HasPayload`). A *block* is the element of the chain a proposal
proposes, header and payload together, named by its hash (`blockHash`): what a
certificate is over, what is decided, what has a height and is the last of an
epoch.

So a node holds a proposal (`History.HasProposal`) and a payload, never a block,
and the type is the proposal's: the specification reads a block only through its
proposal.
-/
abbrev Block := Proposal

/--
A request to vote again on the last block of an epoch, in a later view.

It is for when the last block of an epoch has a `Cert1` but no `Cert2`, so the
next epoch cannot start. The outgoing committee votes on that block once more, at
view `view`: vote1s over `cert`'s data at `view` form a `Cert1` over the same block
at the later view, and vote2s over that a `Cert2`, which commits the block. No new
block is built. No rule checks that the `Cert2` is missing; a leader may wait for it
before asking (`RevoteJustified`).
-/
structure RevoteRequest where
  /-- The certificate over the block voted on again: its first `Cert1`, or an earlier re-vote's. -/
  cert : Cert1

  /-- The view the votes are cast in. -/
  view : ViewNumber

  /-- The timeout certificate for the view before, if the request follows a timeout. -/
  timeoutEvidence : Option TimeoutCert
deriving DecidableEq, Repr

/--
Which block a hash names.

Not something a node keeps: it exists so the safety statement has something to
mean "ancestor" in. `Resolves` ties it to the proposals honest nodes hold.
-/
abbrev BlockTable := BlockHash → Option Block

/-- The block's payload commitment. -/
def Proposal.payloadCommit (p : Proposal) : PayloadCommit :=
  p.blockHeader.payloadCommit

/--
The hash identifying a block.

`opaque`, so no proof can see through to the projection and every rule relates
`blockHash` images without depending on how they arise. That is what keeps
`CollisionFree` consistent: a visible body would let two blocks differing only in
view share an identity, which refutes it. The body is there for the compiler, so
a machine built on this can run.
-/
opaque blockHash (b : Block) : BlockHash := b.identity

/--
The block is valid in the application's sense.

Opaque: consensus carries blocks it does not interpret. A node learns validity
from `Input.blockValidated`, and the rules forbid a vote1 on an invalid block.
-/
opaque BlockValid : Block → Prop

/-- A node's VID share of a block payload, reduced to what consensus reads. -/
structure VidShare where
  /-- The view of the block the share belongs to. -/
  view : ViewNumber

  /-- The payload it is a share of. -/
  payloadCommit : PayloadCommit
deriving DecidableEq, Repr

/-- A node's vote over data `α`: the data, the view, and who cast it. -/
structure Vote (α : Type) where
  /-- What is signed. -/
  data : α

  /-- The view voted in. -/
  view : ViewNumber

  /-- Who cast it. -/
  signer : PubKey
deriving DecidableEq, Repr

/-- A vote1. -/
abbrev Vote1 := Vote Vote1Data

/-- A vote2. -/
abbrev Vote2 := Vote Vote2Data

/-- A timeout vote. -/
abbrev TimeoutVote := Vote TimeoutData

end NewProtocol
