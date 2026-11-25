module

public import NewProtocolSpec.Base
public import NewProtocolSpec.Types

/-!
# Interface

The configuration, and the inputs and outputs of one node. A node's history is a
sequence of steps, each an input and the outputs it produced
(`NewProtocolSpec.History`), and the rules are predicates on that history. This is
also the shape of a recorded trace, which is what lets a trace of an
implementation be checked against the rules directly.
-/

@[expose] public section

namespace NewProtocol

/-- Static configuration, agreed before the network starts. -/
structure Config where
  /--
  The anchor: the decided block every node starts from, at any view and height.

  Nothing in a run votes on it. It is the genesis block, or a block an earlier run
  decided; the chain before it is not modelled. It exists so the first proposal has
  a parent to name.
  -/
  anchorBlock : Block

  /--
  The certificate over `Config.anchorBlock`.

  Nothing in a run votes at the anchor's view, so no quorum of the run stands behind
  it; the configuration vouches for it instead (`ConfigCoherent`).
  -/
  anchorCert : Cert1

  /--
  How many views before its latest decide a node still owes anything.

  What lets a node with bounded memory meet the obligations: it may forget views
  earlier than that, and nothing is owed there.
  -/
  decideBuffer : Nat := 20

  /--
  Blocks to an epoch.

  Fixes where every epoch boundary falls, via `epochOf`. Zero means epochs are not
  in use: every block reports epoch zero and no boundary falls.
  -/
  epochHeight : Nat := 0
deriving Repr

/--
The anchor's view. No view of a run is earlier; the run starts in the view after
it, which the anchor's certificate is grounds for.
-/
abbrev Config.anchorView (cfg : Config) : ViewNumber := cfg.anchorBlock.viewNumber

/--
The anchor's `Cert2`.

The anchor is decided, so the configuration vouches for a `Cert2` over it, as it
does for its `Cert1`. A proposal on the anchor that opens an epoch comes behind it
(`OpensEpochJustified`).
-/
def Config.anchorCert2 (cfg : Config) : Cert2 := ⟨cfg.anchorCert.data.toVote2, cfg.anchorCert.view⟩

/--
The epoch a run starts in: the anchor's, or the next one if the anchor is the last
block of its epoch. The anchor is decided, so its epoch has then ended.
-/
def Config.startEpoch (cfg : Config) : EpochNumber :=
  if IsLastBlock cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight then cfg.anchorCert.data.epoch + 1
  else cfg.anchorCert.data.epoch

/-- The anchor's certificate describes the anchor block. -/
structure ConfigCoherent (cfg : Config) : Prop where
  /-- The certificate is at the anchor's view. -/
  anchorCertView : cfg.anchorCert.view = cfg.anchorView

  /-- It names the anchor block. -/
  anchorCertBlock : cfg.anchorCert.data.blockHash = blockHash cfg.anchorBlock

  /-- And its height. -/
  anchorCertBlockNumber : cfg.anchorCert.data.blockNumber = cfg.anchorBlock.blockHeader.blockNumber

  /-- And the epoch that height falls in. -/
  anchorCertEpoch :
    cfg.anchorCert.data.epoch = epochOf cfg.anchorBlock.blockHeader.blockNumber cfg.epochHeight

  /-- The anchor block names that epoch too. -/
  anchorBlockEpoch : cfg.anchorBlock.epoch = cfg.anchorCert.data.epoch

/--
An input to one step of a node.

Certificates arriving here are already verified. Whether a node assembled a
certificate from votes itself or was handed it is not distinguished: in a full
mesh every honest node receives every honest vote, and both come in as the same
input.
-/
inductive Input where
  /-- The node has the payload of the block at view `v`, rebuilt from VID shares or built by itself. -/
  | blockReconstructed (v : ViewNumber) (c : PayloadCommit)
  /-- A `Cert1`. -/
  | certificate1 (c : Cert1)
  /-- A `Cert2`. -/
  | certificate2 (c : Cert2)
  /-- Evidence that an epoch ended: the two certificates over its last block, and that block. -/
  | epochChange (c1 : Cert1) (c2 : Cert2) (p : Proposal)
  /-- Block content is available for this node to propose at `v` on the parent `parent`. -/
  | headerBuilt (v : ViewNumber) (parent : BlockHash) (h : BlockHeader)
  /--
  A proposal received from `sender`, with this node's VID share of it, if any.

  A member of the proposal's committee gets its share from the leader with the
  proposal. A node outside the committee gets none, a member may get the proposal
  before its share, and a proposal fetched from a peer comes without one: each is
  `share = none`. Holding the proposal is what lets a node extend or decide the
  block; voting on it needs the share.
  -/
  | proposal (sender : PubKey) (p : Proposal) (share : Option VidShare)
  /-- A request from `sender` to vote again on an epoch's last block (`RevoteRequest`). -/
  | revote (sender : PubKey) (r : RevoteRequest)
  /-- The block with hash `h` at view `v` is valid. -/
  | blockValidated (v : ViewNumber) (h : BlockHash)
  /-- The node's timer for view `v` fired. -/
  | timeout (v : ViewNumber)
  /-- A timeout certificate. -/
  | timeoutCertificate (c : TimeoutCert)
  /-- Timeout votes for view `v` reached the one-honest threshold. -/
  | timeoutOneHonest (v : ViewNumber)
deriving DecidableEq, Repr

/--
A message a node sends.

No message names a recipient. Who receives what, and when, is the network's part,
stated as assumptions of the liveness result (`Synchrony`).
-/
inductive Message where
  /-- The block the node proposes for a view it leads. -/
  | proposal (p : Proposal)
  /-- The node's request, as the leader of its view, to vote again on an epoch's last block. -/
  | revote (r : RevoteRequest)
  /-- The node's vote1. -/
  | vote1 (v : Vote1)
  /-- The node's vote2. -/
  | vote2 (v : Vote2)
  /-- The node's vote to give up a view. -/
  | timeoutVote (v : TimeoutVote)
  /-- A timeout certificate, grounds for the view after the one it certifies. -/
  | timeoutCert (c : TimeoutCert)
  /-- A `Cert1`. -/
  | cert1 (c : Cert1)
  /-- A `Cert2`. -/
  | cert2 (c : Cert2)
  /-- The evidence that an epoch ended; see `Input.epochChange`. -/
  | epochChange (c1 : Cert1) (c2 : Cert2) (p : Proposal)
  /-- The node's own VID share, so peers can rebuild the payload. -/
  | vidShare (s : VidShare)
deriving DecidableEq, Repr

/-- The view a message is a leader's for: a proposal's, or a re-vote request's. -/
def Message.leaderView : Message → Option ViewNumber
  | .proposal p => some p.viewNumber
  | .revote r => some r.view
  | _ => none

/-- The epoch a leader's message is for: a proposal's, or that of the block a re-vote request votes on again. -/
def Message.leaderEpoch : Message → Option EpochNumber
  | .proposal p => some p.epoch
  | .revote r => some r.cert.data.epoch
  | _ => none

/--
Who sent an input that names its sender, and the message it sent: a proposal or a
re-vote request.
-/
def Input.sentBy : Input → Option (PubKey × Message)
  | .proposal l p _ => some (l, .proposal p)
  | .revote l r => some (l, .revote r)
  | _ => none

/--
An output of one step: a message to peers, or blocks decided.

A decide lists blocks newest first. `c2` commits the newest and `c1` certifies it;
each older block is the parent the next one's parent certificate names.
-/
inductive Output where
  /-- Send a message to peers. -/
  | send (m : Message)
  /-- Blocks decided, delivered to the application. -/
  | decided (blocks : List Block) (c1 : Cert1) (c2 : Cert2)
deriving DecidableEq, Repr

end NewProtocol
