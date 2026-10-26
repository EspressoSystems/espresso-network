module

public import NewProtocolSpec.Validity

/-!
# Histories

A node is described by its history: the inputs it received and the outputs it
produced, step by step. Everything the rules read is derived from it: what the
node holds, what it is locked on, which view and epoch it is in. None of these is
a field an implementation must keep. They are facts about what it has seen, and an
implementation may store them however it likes, or forget what no rule reads any
more.
-/

@[expose] public section

namespace NewProtocol

/-- One step of a node: an input, and the outputs produced in response. -/
structure Step where
  /-- What arrived. -/
  input : Input

  /-- What the node emitted in response. -/
  output : List Output
deriving DecidableEq, Repr

/-- The step with only its first `i` outputs: what it had produced before output `i`. -/
abbrev Step.take (st : Step) (i : Nat) : Step := { st with output := st.output.take i }

/-- A node's steps so far, oldest first. -/
abbrev History := List Step

namespace History

variable (cfg : Config) (h : History)

/-- The first `n` steps of the history. -/
def upTo (n : Nat) : History := h.take n

/-! ## What arrived and what went out -/

/-- The node received input `i`. -/
def Received (i : Input) : Prop := ∃ st ∈ h, st.input = i

/-- The node sent message `m`. -/
def Sent (m : Message) : Prop := ∃ st ∈ h, Output.send m ∈ st.output

/-- The node sent message `m` in step `n`. -/
def SentAt (n : Nat) (m : Message) : Prop := ∃ st, h[n]? = some st ∧ Output.send m ∈ st.output

/-- The node decided view `v`: some block it delivered to the application is at `v`. -/
def DecidedView (v : ViewNumber) : Prop :=
  ∃ st ∈ h, ∃ blocks c1 c2 b, Output.decided blocks c1 c2 ∈ st.output ∧ b ∈ blocks
    ∧ b.viewNumber = v

/-- The node sent a timeout vote for view `v` or a later one. -/
def TimedOut (v : ViewNumber) : Prop :=
  ∃ vote : TimeoutVote, Sent h (.timeoutVote vote) ∧ v ≤ vote.view

/-! ## What the node holds -/

/--
The node holds a `Cert1`: the anchor's, one it received on its own, or the one an
epoch change carried.

A `Cert1` carried inside any other message is not held until the implementation
records it as an `Input.certificate1`.
-/
def HasCert1 (c : Cert1) : Prop :=
  c = cfg.anchorCert ∨ Received h (.certificate1 c) ∨ ∃ c2 p, Received h (.epochChange c c2 p)

/--
The node holds a `Cert2`: one it received on its own, or the one an epoch change
carried.

A `Cert2` carried inside any other message is not held until the implementation
records it as an `Input.certificate2`.
-/
def HasCert2 (c : Cert2) : Prop :=
  Received h (.certificate2 c) ∨ ∃ c1 p, Received h (.epochChange c1 c p)

/--
The node holds proposal `p`: the anchor's, or one it received, with or without its
share, or in an epoch change.

Holding a proposal is not holding the block's payload (`History.HasPayload`).
-/
def HasProposal (p : Proposal) : Prop :=
  p = cfg.anchorBlock ∨ (∃ sender share, Received h (.proposal sender p share))
    ∨ ∃ c1 c2, Received h (.epochChange c1 c2 p)

/-- The node has the payload with commitment `pc` of the block at view `v`. -/
def HasPayload (v : ViewNumber) (pc : PayloadCommit) : Prop :=
  (v = cfg.anchorView ∧ pc = cfg.anchorBlock.payloadCommit)
    ∨ Received h (.blockReconstructed v pc)

/-- The node took an epoch change: well-formed evidence that the epoch of `c2` ended. -/
def TookEpochChange (c1 : Cert1) (c2 : Cert2) (p : Proposal) : Prop :=
  Received h (.epochChange c1 c2 p) ∧ EpochChangeWellFormed cfg c1 c2 p

/-! ## The lock

The lock is not state here. It is the latest certificate the node could lock on,
and what it could lock on is fixed by what it holds.
-/

/--
The node could lock on `c`.

The anchor's certificate, or a `Cert1` over a block whose proposal and payload
the node holds, or the `Cert1` of an epoch change it took. The last is what lets a node
new to the incoming committee lock on the boundary block without its payload:
it holds no share of that block, and the `Cert2` beside the certificate shows a
quorum had the payload.
-/
def Lockable (c : Cert1) : Prop :=
  c = cfg.anchorCert
    ∨ (HasCert1 cfg h c
        ∧ ∃ b, HasProposal cfg h b ∧ Certifies c b ∧ HasPayload cfg h b.viewNumber b.payloadCommit)
    ∨ ∃ c2 p, TookEpochChange cfg h c c2 p

/--
The node could build on `c`: the anchor's certificate, or a `Cert1` it holds over
a block whose proposal it holds.

What a leader needs to propose on `c` without a timeout. Unlike `History.Lockable`,
it does not need the payload: a leader proposes as soon as it enters the view, and
the members of the epoch rebuild the payload from their shares (`Synchrony.lockSpread`).
-/
def Buildable (c : Cert1) : Prop :=
  c = cfg.anchorCert ∨ (HasCert1 cfg h c ∧ ∃ b, HasProposal cfg h b ∧ Certifies c b)

/-- The node is locked on `c`: it could lock on `c`, and on nothing later in lock order (`LockLE`). -/
def LockedOn (c : Cert1) : Prop :=
  Lockable cfg h c ∧ ∀ c', Lockable cfg h c' → LockLE c' c

/-! ## Where the node is

The view and the epoch are also derived: each is the latest one the node has
grounds for.
-/

/--
The node has grounds to be in view `v`.

The view after a `Cert1` it holds, after a timeout certificate, or after an epoch
change it took. The anchor's certificate is always held, so every node has grounds
for the view after the anchor's.

A certificate alone is grounds: a node shown it follows, whether or not it can
rebuild the block's payload. Locking on it, and voting on it, are what need the
payload.
-/
def ViewGround (v : ViewNumber) : Prop :=
  (∃ c, HasCert1 cfg h c ∧ v = c.view + 1)
    ∨ (∃ tc, Received h (.timeoutCertificate tc) ∧ v = tc.view + 1)
    ∨ ∃ c1 c2 p, TookEpochChange cfg h c1 c2 p ∧ v = c2.view + 1

/-- The node is in view `v`: the latest it has grounds for. -/
def InView (v : ViewNumber) : Prop :=
  ViewGround cfg h v ∧ ∀ v', ViewGround cfg h v' → v' ≤ v

/--
The node has grounds to be in epoch `e`.

The epoch the run starts in (`Config.startEpoch`), the epoch after one an epoch
change ended, the epoch of a timeout certificate, or the epoch of a `Cert1` it
holds, when that epoch is the one the certificate's height falls in.

Each names an epoch a quorum acted in. None is the epoch of a block after a
certified one: a `Cert1` over an epoch's last block does not make it final, so it
leaves the node in the outgoing epoch.
-/
def EpochGround (e : EpochNumber) : Prop :=
  e = cfg.startEpoch
    ∨ (∃ c1 c2 p, TookEpochChange cfg h c1 c2 p ∧ e = c2.data.epoch + 1)
    ∨ (∃ tc, Received h (.timeoutCertificate tc) ∧ e = tc.data.epoch)
    ∨ ∃ c, HasCert1 cfg h c
        ∧ c.data.epoch = epochOf c.data.blockNumber cfg.epochHeight ∧ e = c.data.epoch

/-- The node is in epoch `e`: the latest it has grounds for. -/
def InEpoch (e : EpochNumber) : Prop :=
  EpochGround cfg h e ∧ ∀ e', EpochGround cfg h e' → e' ≤ e

/--
The node has given up view `v`: it timed out on `v` or a later view, or holds a
timeout certificate for one.

Moving ahead on a later certificate does not count. A node can be shown a later
certificate before its own view's arrives, and should still vote2 there.
-/
def PastView (v : ViewNumber) : Prop :=
  h.TimedOut v ∨ ∃ tc, h.Received (.timeoutCertificate tc) ∧ v ≤ tc.view

/--
View `v` is after the decide floor: later than the anchor's, and later than
`w - Config.decideBuffer` for every view `w` the node decided.

At the floor and before it nothing is owed, which is what lets a node forget.
-/
def AfterFloor (v : ViewNumber) : Prop :=
  cfg.anchorView < v ∧ ∀ w, DecidedView h w → w - cfg.decideBuffer < v

end History

end NewProtocol
