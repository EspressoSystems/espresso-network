module

public import NewProtocolSpec.History

/-!
# The rules

What an honest node's history must satisfy.

* `SafeHistory` is when a node may sign a vote and what a decide must show. It is
  all the no-fork result reads. A vote1 after a timeout extends no block the
  timeout certificate's lock rules out (`LockAllows`), a timeout vote's lock
  covers the node's vote2s, and no vote2 follows a timeout of its view: together
  these keep a committed view from being skipped.
* `ProtocolHistory` adds the rest of what a node may do: when it may time out and
  what it may propose. These are what the liveness result needs of a node beyond
  safety.
* `Owed` is what a node must eventually do. `OwedIn` restricts it to the epochs
  the node is honest in (`History.restrict`), and the timing layer
  (`NewProtocolSpec.Timing`) asks that nothing stay owed for long (`Prompt`).

Each rule reads the history up to and including the step it constrains, so a node
may act on an input in the step it arrives. The timeout rules are the exception:
they read the history before the step, because a timer or one-honest indication
is answered from the view and epoch the node was in when it arrived.
-/

@[expose] public section

namespace NewProtocol

variable (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey) (node : PubKey)

open History

/-! ## Signing -/

/--
Timeout evidence lets certificate `c` through, for a block of epoch `e`: the
timeout certificate is of epoch `e`, and its lock allows `c` (`LockAllows`).

So nothing skips a view a quorum may have committed. Without timeout evidence,
a proposal extends the view before its own and skips nothing. With it, a quorum
of epoch `e` that committed a view `x` and the quorum behind the timeout
certificate share a node honest in `e`, which voted2 at `x` before it timed out,
so the certificate's lock is at `x` or later.
-/
def SafeEvidence (ev : Option TimeoutCert) (c : Cert1) (e : EpochNumber) : Prop :=
  ∀ tc, ev = some tc → tc.data.epoch = e ∧ LockAllows tc.data.lock c e

/-- A proposal's parent is safe to build on, given its timeout evidence. -/
def SafeParent (p : Proposal) : Prop := SafeEvidence p.timeoutEvidence p.parentCert p.epoch

/-- A re-vote request's certificate is safe to vote on again, given its timeout evidence. -/
def SafeRevote (r : RevoteRequest) : Prop := SafeEvidence r.timeoutEvidence r.cert r.cert.data.epoch

/--
A proposal opening an epoch names its parent by the parent's own `Cert1`, at the
parent's view, and comes behind a `Cert2` over the parent at an earlier view. The
node holds the parent and the `Cert2`, to check both; the anchor's `Cert2` is the
configuration's (`Config.anchorCert2`).

The parent is at the certificate's view exactly. Elsewhere a held parent may be at
an earlier view than the certificate (`ProposalReady.parent`, `ParentReady`),
since a re-vote certifies a block again at a later view; here that is what is
ruled out.

A re-vote's certificate is over the same block at a later view, and the outgoing
committee can form one after the next epoch began. Building the first block of an
epoch on such a certificate could start the epoch a second time. The `Cert2` is
what makes every honest voter of the epoch's first block one that can decide the
previous epoch's last block, so the epoch never moves on without honest nodes
learning that it ended.
-/
def OpensEpochJustified (h : History) (p : Proposal) : Prop :=
  EntersEpoch cfg p → (∃ parent, h.HasProposal cfg parent ∧ parent.viewNumber = p.parentCert.view
    ∧ p.parentCert.data.blockHash = blockHash parent)
    ∧ ∃ c2, (h.HasCert2 c2 ∨ c2 = cfg.anchorCert2) ∧ c2.view < p.viewNumber
      ∧ c2.data = p.parentCert.data.toVote2

/--
When a node may sign, and what its decides show, for the epochs it is honest in.

Every field is a permission. A node that never acts satisfies all of them.

`honestIn` holds of the epochs the node is honest in (`Committee.honest`). A rule
applies when the node is honest in the epoch of every message it relates: the
vote it constrains, and any vote or timeout vote it reads. A node honest in every
epoch takes `fun _ => True`.
-/
structure SafeHistory (honestIn : EpochNumber → Prop) (h : History) : Prop where
  /--
  A vote1 is for a well-formed, valid proposal the node received, with a safe
  parent, or answers a well-formed, safe re-vote request the node received.
  -/
  vote1Justified : ∀ n vote, h.SentAt n (.vote1 vote) →
    honestIn vote.data.epoch → vote.signer = node
      ∧ ((∃ sender p vid, (h.upTo (n + 1)).Received (.proposal sender p (some vid))
          ∧ ProposalWellFormed cfg p ∧ BlockValid p ∧ SafeParent p
          ∧ OpensEpochJustified cfg (h.upTo (n + 1)) p ∧ Vote1For vote p)
        ∨ ∃ sender r, (h.upTo (n + 1)).Received (.revote sender r)
          ∧ RevoteWellFormed cfg r ∧ SafeRevote r ∧ Vote1Again vote r)

  /--
  At most one vote1 per epoch and view.

  Per epoch, because quorums intersect only within one: a node in two committees
  may answer a re-vote request of the outgoing epoch and vote for the next epoch's
  first block in the same view.
  -/
  vote1Once : ∀ vote vote', h.Sent (.vote1 vote) → h.Sent (.vote1 vote') →
    honestIn vote.data.epoch → vote.data.epoch = vote'.data.epoch → vote.view = vote'.view → vote = vote'

  /--
  A vote2 is for a block whose proposal and payload the node holds, under a `Cert1`
  over it that the node holds.

  The payload is what makes a `Cert2` mean the block is available: every honest
  signer had it.
  -/
  vote2Justified : ∀ n vote, h.SentAt n (.vote2 vote) →
    honestIn vote.data.epoch → vote.signer = node ∧ cfg.anchorView < vote.view
      ∧ ∃ c b, (h.upTo (n + 1)).HasCert1 cfg c ∧ (h.upTo (n + 1)).HasProposal cfg b ∧ Certifies c b
        ∧ (h.upTo (n + 1)).HasPayload cfg b.viewNumber b.payloadCommit
        ∧ vote.view = c.view ∧ vote.data = c.data.toVote2

  /-- At most one vote2 per epoch and view. -/
  vote2Once : ∀ vote vote', h.Sent (.vote2 vote) → h.Sent (.vote2 vote') →
    honestIn vote.data.epoch → vote.data.epoch = vote'.data.epoch → vote.view = vote'.view → vote = vote'

  /--
  No vote2 at a view the node has timed out, or at an earlier one.

  So a node's timeout vote for a view comes after any vote2 it casts at that view
  or an earlier one, and its lock covers them (`timeoutLock`).
  -/
  vote2BeforeTimeout : ∀ n vote, h.SentAt n (.vote2 vote) →
    honestIn vote.data.epoch → ∀ tv : TimeoutVote, (h.upTo (n + 1)).Sent (.timeoutVote tv) →
      honestIn tv.data.epoch → tv.view < vote.view

  /--
  A timeout vote names a lock no earlier, in epoch and view order (`EpochViewLE`),
  than every vote2 the node sent before it.

  What makes `SafeParent` safe: a timeout certificate's lock is then no earlier
  than any view its honest signers voted2 in before timing out.
  -/
  timeoutLock : ∀ n vote, h.SentAt n (.timeoutVote vote) →
    honestIn vote.data.epoch → ∀ v2 : Vote2, (h.upTo n).Sent (.vote2 v2) → honestIn v2.data.epoch →
      EpochViewLE v2.data.epoch v2.view vote.data.lock.data.epoch vote.data.lock.view

  /--
  A decide delivers a linked chain of blocks whose proposals the node holds, the
  newest with a `Cert2` and a `Cert1` over it that the node holds.

  A decide is the node's when it is honest in the epoch of the `Cert2`. Each decide
  is checkable against its own contents: the newest block by its `Cert2`, each older
  one by the next one's parent certificate. No decided block is
  at the anchor's view: the anchor is where the chain starts, and a walk that went
  past it would reach blocks nothing certified.
  -/
  decideJustified : ∀ n st blocks c1 c2, h[n]? = some st → Output.decided blocks c1 c2 ∈ st.output →
    honestIn c2.data.epoch → ∃ head rest, blocks = head :: rest ∧ (h.upTo (n + 1)).HasCert2 c2 ∧ Commits c2 head
      ∧ (h.upTo (n + 1)).HasCert1 cfg c1 ∧ Certifies c1 head ∧ ChainLinked blocks
      ∧ ∀ b ∈ blocks, (h.upTo (n + 1)).HasProposal cfg b ∧ cfg.anchorView < b.viewNumber

  /--
  A decide delivers each view at most once: its blocks are at distinct views, and
  none is at a view the node decided before, in an earlier step or in an earlier
  output of the same step.

  The application's half of `decideJustified`. Order is not promised: a decide may
  skip blocks the node does not hold, so a later decide can deliver an earlier view.
  -/
  decideOnce : ∀ n (st : Step) (i : Nat) (blocks : List Block) (c1 : Cert1) (c2 : Cert2), h[n]? = some st →
    st.output[i]? = some (.decided blocks c1 c2) →
    honestIn c2.data.epoch → (blocks.map (·.viewNumber)).Nodup
      ∧ ∀ b ∈ blocks, ¬ (h.upTo n ++ [st.take i]).DecidedView b.viewNumber

/-! ## Timing out and proposing -/

/--
Epoch `e` is no earlier than the node's own.

A node does not vote for an epoch it has left. Otherwise the outgoing committee,
voting again on its last block, could take honest vote1s from the views of the
next epoch.
-/
def NotBehind (h : History) (e : EpochNumber) : Prop :=
  ∀ e', h.InEpoch cfg e' → e' ≤ e

/--
The certificate a proposal may build on, or a re-vote request vote on again,
given the timeout evidence it carries.

Without evidence, a certificate the node can build on: it holds the certificate
and the block's proposal, though not necessarily the payload. After a timeout, a
certificate the node holds over the block it was locked on at some point after it
received the timeout certificate, or over the last block of an epoch it held a
`Cert2` over by then. Usually that is the lock itself; for the last block of an
epoch the lock may be a re-vote's certificate, or a stale certificate of the epoch
over another block, and the node names the committed block's first one.
-/
def CertJustified (h : History) (c : Cert1) : Option TimeoutCert → Prop
  | none => h.Buildable cfg c
  | some tc => h.HasCert1 cfg c ∧ ∃ m, (h.upTo m).Received (.timeoutCertificate tc)
      ∧ ((∃ l, (h.upTo m).LockedOn cfg l ∧ l.data = c.data)
        ∨ (IsLastBlock c.data.blockNumber cfg.epochHeight
          ∧ ∃ c2, (h.upTo m).HasCert2 c2 ∧ c2.data = c.data.toVote2))

/--
The parent a proposal may name.

Without timeout evidence, the certificate of the view before, which the node can
build on: it holds the certificate and the block, so it can build the header the
moment it enters the view.
After a timeout, a certificate the node was locked on at some point after it
received the timeout certificate for the view before, or the first certificate
of an epoch's last block it held a `Cert2` over by then (`CertJustified`).

The lock is not required to still be the node's when the proposal goes out. An
implementation may decide on the proposal, then wait, for example for storage,
before sending it, and its lock can move in between.
-/
def ParentJustified (h : History) (p : Proposal) : Prop :=
  CertJustified cfg h p.parentCert p.timeoutEvidence

/--
The node may propose `p`, once it has a header for it.

It leads `p`'s view in `p`'s epoch, `p` is well formed, its parent is justified,
held and safe, and it is for the node's epoch and a view the node has reached.
-/
structure ProposalReady (h : History) (p : Proposal) : Prop where
  /-- The node leads the view, in the proposal's epoch. -/
  leads : leader p.epoch p.viewNumber = some node

  /-- The proposal is well formed. -/
  wellFormed : ProposalWellFormed cfg p

  /-- The parent certificate is one the node may build on. -/
  justified : ParentJustified cfg h p

  /--
  The node holds the parent.

  Without timeout evidence `justified` already says so (`History.Buildable`); after
  a timeout the certificate it names may be one whose block the node has not
  fetched yet.
  -/
  parent : ∃ parent, h.HasProposal cfg parent ∧ parent.viewNumber ≤ p.parentCert.view
    ∧ p.parentCert.data.blockHash = blockHash parent

  /--
  A proposal opening an epoch names its parent at the parent's view, behind a
  `Cert2` over it the node holds.
  -/
  opens : OpensEpochJustified cfg h p

  /-- Honest nodes may vote1 on it: its parent is safe. -/
  safe : SafeParent p

  /-- It is for the node's epoch or a later one. -/
  current : NotBehind cfg h p.epoch

  /-- It is for a view the node has reached. -/
  reached : ∃ u, h.ViewGround cfg u ∧ p.viewNumber ≤ u

/-- The node may propose `p`: it is ready to, and its header was built for that view and parent. -/
structure ProposalJustified (h : History) (p : Proposal) : Prop
    extends ProposalReady cfg leader node h p where
  /-- The header was built for the view and the parent. -/
  built : h.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash p.blockHeader)

/--
The node may send re-vote request `r`.

It leads the request's view in the epoch of the block voted on again, the request
is well formed and safe, and the certificate is one it may vote on again
(`CertJustified`).

A request is justified as soon as the leader can lock on the block, whether or not
its `Cert2` is on its way. It is owed from then (`Owed`), so an implementation
may wait for the `Cert2` up to `δ` (`Prompt`), and propose the next epoch's first
block instead if it arrives. Waiting longer is not allowed: asking for a timeout
first admits a liveness attack at the boundary.
-/
structure RevoteJustified (h : History) (r : RevoteRequest) : Prop where
  /-- The node leads the view, in the epoch of the block voted on again. -/
  leads : leader r.cert.data.epoch r.view = some node

  /-- The request is well formed. -/
  wellFormed : RevoteWellFormed cfg r

  /--
  The certificate is one the node may vote on again.

  Without timeout evidence this follows from `lockable`, since a certificate the
  node can lock on is one it can build on.
  -/
  justified : CertJustified cfg h r.cert r.timeoutEvidence

  /--
  Without timeout evidence, the node can lock on the certificate: it has the
  block's payload as well, which a proposal does not need.
  -/
  lockable : r.timeoutEvidence = none → h.Lockable cfg r.cert

  /-- Honest nodes may vote1 on it. -/
  safe : SafeRevote r

  /-- It is for the node's epoch or a later one. -/
  current : NotBehind cfg h r.cert.data.epoch

  /-- It is for a view the node has reached. -/
  reached : ∃ u, h.ViewGround cfg u ∧ r.view ≤ u

/--
The node sent a proposal or a re-vote request of epoch `e` for view `v`.

A leader sends one of the two per epoch and view, never both. At a boundary it may
ask for a re-vote of the outgoing epoch's last block and propose the next epoch's
first block in the same view: each is its epoch's one message for the view.
-/
def ProposedIn (h : History) (e : EpochNumber) (v : ViewNumber) : Prop :=
  (∃ p, h.Sent (.proposal p) ∧ p.viewNumber = v ∧ p.epoch = e)
    ∨ ∃ r, h.Sent (.revote r) ∧ r.view = v ∧ r.cert.data.epoch = e

/--
`SafeHistory`, and the rest of what an honest node may do, for the epochs it is
honest in.

The added fields are what liveness needs: a node that timed out at will could stop
every view, and one that answered no timer could keep a failed view open for ever.
As for the signing rules, a rule applies when the node is honest in the epoch of
the messages it relates. A timer is answered when the node is honest in the epoch
it is in and a member of its committee (`memberIn`): a timeout vote counts towards
no certificate of an epoch the node is not a member of.
-/
structure ProtocolHistory (honestIn memberIn : EpochNumber → Prop) (h : History) : Prop
    extends SafeHistory cfg node honestIn h where
  /--
  A vote1 is on a proposal, or answers a re-vote request, that the leader of its
  view sent, of the node's epoch or a later one, and in a view the node has
  reached.

  The network is authenticated, so who sent a proposal is known; without this, a
  faulty node's proposal could take the node's one vote1 in the view.

  The proposal or request here is the one `vote1Justified` names: `Vote1For` and
  `Vote1Again` fix the block's hash, epoch, height and view.
  -/
  vote1Leader : ∀ n vote, h.SentAt n (.vote1 vote) →
    honestIn vote.data.epoch → (∃ v, (h.upTo (n + 1)).ViewGround cfg v ∧ vote.view ≤ v)
    ∧ ((∃ sender p vid, (h.upTo (n + 1)).Received (.proposal sender p (some vid))
      ∧ leader p.epoch p.viewNumber = some sender ∧ Vote1For vote p
      ∧ NotBehind cfg (h.upTo (n + 1)) p.epoch)
    ∨ ∃ sender r, (h.upTo (n + 1)).Received (.revote sender r)
      ∧ leader r.cert.data.epoch r.view = some sender ∧ Vote1Again vote r
      ∧ NotBehind cfg (h.upTo (n + 1)) r.cert.data.epoch)

  /--
  A timeout vote answers a timer or the one-honest indication, names the node's
  epoch, and names as its lock a certificate the node holds.

  The timer fires for the view the node is in. The one-honest indication may be
  for a later view, since the node is following others who already left. Both are
  read from the history before the step (`h.upTo n`), not after it.
  -/
  timeoutJustified : ∀ n st vote, h[n]? = some st → Output.send (.timeoutVote vote) ∈ st.output →
    honestIn vote.data.epoch → vote.signer = node ∧ (h.upTo n).InEpoch cfg vote.data.epoch
      ∧ (h.upTo n).HasCert1 cfg vote.data.lock
      ∧ ((st.input = .timeout vote.view ∧ (h.upTo n).InView cfg vote.view)
        ∨ (st.input = .timeoutOneHonest vote.view
            ∧ ∃ v, (h.upTo n).InView cfg v ∧ v ≤ vote.view))

  /--
  Such a timer or indication is answered, in the step it arrives, in an epoch the
  node is honest in and a member of.
  -/
  timeoutAnswered : ∀ n st v, h[n]? = some st →
    ((st.input = .timeout v ∧ (h.upTo n).InView cfg v)
      ∨ (st.input = .timeoutOneHonest v ∧ ∃ w, (h.upTo n).InView cfg w ∧ w ≤ v)) →
    (∀ e, (h.upTo n).InEpoch cfg e → honestIn e ∧ memberIn e) →
    ∃ e L, (h.upTo n).InEpoch cfg e
      ∧ Output.send (.timeoutVote { data := { epoch := e, lock := L }, view := v, signer := node }) ∈ st.output

  /-- A proposal is one the node may make. -/
  proposeJustified : ∀ n p, h.SentAt n (.proposal p) →
    honestIn p.epoch → ProposalJustified cfg leader node (h.upTo (n + 1)) p

  /-- A re-vote request is one the node may send. -/
  revoteJustified : ∀ n r, h.SentAt n (.revote r) →
    honestIn r.cert.data.epoch → RevoteJustified cfg leader node (h.upTo (n + 1)) r

  /--
  At most one proposal or re-vote request per epoch and view (`Message.leaderView`,
  `Message.leaderEpoch`), of the epochs it is honest in.
  -/
  proposeOnce : ∀ m m' v e, h.Sent m → h.Sent m' → m.leaderView = some v → m'.leaderView = some v →
    m.leaderEpoch = some e → m'.leaderEpoch = some e → honestIn e → m = m'

/-! ## What a node owes -/

/--
The node holds what a vote1 on `p` reads of its parent.

The parent and its payload, unless the parent is the anchor or `p` opens an epoch.
The exemption is there because VID shares go to the committee of a block's own
epoch, so a node new to the incoming committee cannot rebuild the outgoing
epoch's last block.
-/
def ParentReady (h : History) (p : Proposal) : Prop :=
  p.parentCert.view = cfg.anchorView ∨ EntersEpoch cfg p
    ∨ ∃ parent, h.HasProposal cfg parent ∧ parent.viewNumber ≤ p.parentCert.view
        ∧ p.parentCert.data.blockHash = blockHash parent
        ∧ h.HasPayload cfg parent.viewNumber parent.payloadCommit

/--
The node owes a vote1 on `p`: it received `p` from the view's leader with its
share, `p` is well formed, reported valid, its parent ready and safe, and the node
has not yet voted1 for the epoch and view, nor timed it out, and is in it.
-/
structure OwedVote1 (h : History) (p : Proposal) : Prop where
  /-- The node received `p` from the leader of its view, with a share of it. -/
  received : ∃ sender vid, h.Received (.proposal sender p (some vid)) ∧ leader p.epoch p.viewNumber = some sender
    ∧ ShareMatches p vid

  /-- `p` is well formed. -/
  wellFormed : ProposalWellFormed cfg p

  /-- `p` was reported valid. -/
  validated : h.Received (.blockValidated p.viewNumber (blockHash p))

  /-- The node holds what a vote1 reads of the parent. -/
  parent : ParentReady cfg h p

  /-- The parent is safe, given the timeout evidence. -/
  safe : SafeParent p

  /-- If `p` opens an epoch, it is behind a `Cert2` over its parent. -/
  opens : OpensEpochJustified cfg h p

  /-- `p` is for the node's epoch or a later one. -/
  current : NotBehind cfg h p.epoch

  /-- The node has not timed the view out. -/
  notTimedOut : ¬ h.TimedOut p.viewNumber

  /-- The node has not voted1 for the epoch and view. -/
  notVoted : ∀ vote : Vote1, h.Sent (.vote1 vote) → vote.data.epoch = p.epoch → vote.view ≠ p.viewNumber

  /-- The node is in the view. -/
  inView : h.InView cfg p.viewNumber

/--
The node owes a vote1 answering `r`: it received `r` from the view's leader, `r` is
well formed and safe, the node holds the block's proposal and payload, and it has not yet
voted1 for the epoch and view, nor timed it out, and is in it.
-/
structure OwedVote1Again (h : History) (r : RevoteRequest) : Prop where
  /-- The node received `r` from the leader of its view. -/
  received : ∃ sender, h.Received (.revote sender r) ∧ leader r.cert.data.epoch r.view = some sender

  /-- `r` is well formed. -/
  wellFormed : RevoteWellFormed cfg r

  /-- The certificate is safe to vote on again, given the timeout evidence. -/
  safe : SafeRevote r

  /-- The node holds the block's proposal and payload. -/
  payload : ∃ b, h.HasProposal cfg b ∧ Certifies r.cert b ∧ h.HasPayload cfg b.viewNumber b.payloadCommit

  /-- `r` is for the node's epoch or a later one. -/
  current : NotBehind cfg h r.cert.data.epoch

  /-- The node has not timed the view out. -/
  notTimedOut : ¬ h.TimedOut r.view

  /-- The node has not voted1 for the epoch and view. -/
  notVoted : ∀ vote : Vote1, h.Sent (.vote1 vote) → vote.data.epoch = r.cert.data.epoch → vote.view ≠ r.view

  /-- The node is in the view. -/
  inView : h.InView cfg r.view

/-- An action a node may owe. -/
inductive Obligation where
  /-- A vote1 on this proposal. -/
  | vote1 (p : Proposal)
  /-- A vote1 answering this re-vote request. -/
  | vote1Again (r : RevoteRequest)
  /-- A vote2 on the block this `Cert1` is over. -/
  | vote2 (c : Cert1)
  /-- A decide of the block this `Cert2` is over. -/
  | decide (c : Cert2)
  /-- A proposal, or a re-vote request, of this epoch for this view. -/
  | propose (e : EpochNumber) (v : ViewNumber)

/--
The node owes a vote2 on the block `c` is over: it holds `c`, the block's proposal
and payload, has not yet voted2 for the epoch and view, holds no `Cert2` there,
has not given the view up, and the view is after its floor.
-/
structure OwedVote2 (h : History) (c : Cert1) : Prop where
  /-- The node holds `c`, the block it is over, and the block's payload. -/
  holds : ∃ b, h.HasCert1 cfg c ∧ h.HasProposal cfg b ∧ Certifies c b
    ∧ h.HasPayload cfg b.viewNumber b.payloadCommit

  /-- The node has not voted2 for the epoch and view. -/
  notVoted : ∀ vote : Vote2, h.Sent (.vote2 vote) → vote.data.epoch = c.data.epoch → vote.view ≠ c.view

  /-- The node holds no `Cert2` for the epoch and view. -/
  notCommitted : ∀ c2, h.HasCert2 c2 → c2.data.epoch = c.data.epoch → c2.view ≠ c.view

  /-- The node has not given the view up. -/
  notPast : ¬ h.PastView c.view

  /-- The view is after the floor (`History.AfterFloor`), so after the anchor's. -/
  afterFloor : h.AfterFloor cfg c.view

/--
The node owes a decide of the block `c` commits: it holds `c`, the block's
proposal and a `Cert1` over it, has not decided the block's view, and the view is
after its floor.

A decide is owed once the node holds the block's `Cert1` as well as its `Cert2`:
the `Cert2` formed after the `Cert1`, so the node receives both, and a decide
delivers the two together.
-/
structure OwedDecide (h : History) (c : Cert2) : Prop where
  /-- The node holds `c`, the block it commits, and a `Cert1` over it. -/
  holds : ∃ b c1, h.HasCert2 c ∧ h.HasProposal cfg b ∧ Commits c b ∧ h.HasCert1 cfg c1
    ∧ Certifies c1 b ∧ ¬ h.DecidedView b.viewNumber ∧ h.AfterFloor cfg b.viewNumber

/--
The node owes the action.

Each case is the action being justified by what the node holds, not yet taken, and
not overtaken. A vote1 and a proposal are owed only in the view the node is in, and
until it times the view out; a vote2 until the node gives the certificate's view
up, holds its `Cert2`, or the view falls behind the floor; a decide until the
block's view is decided or falls behind the floor. So every obligation ends, and
none asks for anything the node would have to fetch, or for a view earlier than
the floor (`Config.decideBuffer`).
-/
def Owed (h : History) : Obligation → Prop
  | .vote1 p => OwedVote1 cfg leader h p
  | .vote1Again r => OwedVote1Again cfg leader h r
  | .vote2 c => OwedVote2 cfg h c
  | .decide c => OwedDecide cfg h c
  | .propose e v =>
      ((∃ p, ProposalJustified cfg leader node h p ∧ p.viewNumber = v ∧ p.epoch = e)
        ∨ ∃ r, RevoteJustified cfg leader node h r ∧ r.view = v ∧ r.cert.data.epoch = e)
      ∧ ¬ ProposedIn h e v ∧ ¬ h.TimedOut v ∧ h.InView cfg v

/-! ## For the epochs a node is honest in -/

/--
What the node signed for an epoch `honestIn` holds of, or decided on a `Cert2` of
one. A certificate it sends on is not its own signature, and always counts.
-/
def Output.SignedIn (honestIn : EpochNumber → Prop) : Output → Prop
  | .send (.proposal p) => honestIn p.epoch
  | .send (.revote r) => honestIn r.cert.data.epoch
  | .send (.vote1 v) => honestIn v.data.epoch
  | .send (.vote2 v) => honestIn v.data.epoch
  | .send (.timeoutVote v) => honestIn v.data.epoch
  | .send _ => True
  | .decided _ _ c2 => honestIn c2.data.epoch

open Classical in
/--
The history as the epochs `honestIn` holds of see it: every input, and of the
outputs only what the node signed or decided for those epochs.

What a node signs for an epoch it is not honest in is unconstrained, so it does not
count against what it owes in an epoch it is honest in. A timeout vote for a late
view, signed for another epoch, would otherwise end its vote1s and vote2s.
-/
noncomputable def History.restrict (honestIn : EpochNumber → Prop) (h : History) : History :=
  h.map fun st => ⟨st.input, st.output.filter fun o => decide (o.SignedIn honestIn)⟩

/-- The epoch an obligation is for: that of the proposal, request or certificate it is about. -/
def Obligation.epoch : Obligation → EpochNumber
  | .vote1 p => p.epoch
  | .vote1Again r => r.cert.data.epoch
  | .vote2 c => c.data.epoch
  | .decide c => c.data.epoch
  | .propose e _ => e

/--
The node owes the action, for the epochs `honestIn` holds of: the action is for one
of them (`Obligation.epoch`), and `Owed` holds of what the node did in them
(`History.restrict`). A node honest in every epoch owes what `Owed` says.
-/
def OwedIn (honestIn : EpochNumber → Prop) (h : History) (o : Obligation) : Prop :=
  honestIn o.epoch ∧ Owed cfg leader node (h.restrict honestIn) o

end NewProtocol
