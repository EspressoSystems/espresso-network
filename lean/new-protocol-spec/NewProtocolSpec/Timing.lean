module

public import NewProtocolSpec.Network

/-!
# Time

What liveness needs beyond the rules: when steps happen, how promptly a node acts,
and what the network delivers after it stabilises.

Partial synchrony: before an unknown time GST messages may be delayed without
bound; after it, a message sent at time `t` arrives by `max t GST + Δ`. The
delivery assumptions are stated per kind of input rather than per message,
because inputs are what a node's history records: a certificate arrives as one
input whether the node assembled it from votes or was handed it.

Each assumption says what arrives, not how. An implementation meets it however it
likes; that it does can be checked on its traces.
-/

@[expose] public section

namespace NewProtocol

open History

variable {cfg : Config} {leader : EpochNumber → ViewNumber → Option PubKey} {C : Committee}

/--
A network whose steps carry times, and whose nodes obey the whole protocol in the
epochs they are honest in.

The fault bound is per epoch: the members honest in an epoch are a quorum of its
committee. A node may be faulty in other epochs; what it signs there is
unconstrained, and does not count against what it owes in the epochs it is honest
in (`OwedIn`).
-/
structure TimedNetwork (cfg : Config) (leader : EpochNumber → ViewNumber → Option PubKey)
    (C : Committee) extends Network cfg C where
  /-- The members honest in an epoch are a quorum of its committee. -/
  honestQuorum : ∀ e, C.Quorum e fun k => C.members e k ∧ C.honest e k

  /-- When each step of each honest node happens. -/
  time : ∀ k, C.Honest k → Nat → Nat

  /-- A node's steps happen in order. -/
  timeMono : ∀ k h n, time k h n ≤ time k h (n + 1)

  /--
  Every prefix obeys the protocol, not only the signing rules, for the epochs the
  node is honest in, answering timers in those it is also a member of.
  -/
  protocol : ∀ k h n, ProtocolHistory cfg leader k (C.honest · k) (C.members · k) ((trace k h).history n)

  /--
  A timeout certificate an honest node is handed was formed from the timeout votes
  of nodes honest in its epoch, sent before it arrived, each with a lock no later
  than the certificate's.

  Strictly before, as on any real network, so that the first honest node to time a
  view out is well defined.
  -/
  timeoutCertCausal : ∀ k h n tc, (trace k h n).input = .timeoutCertificate tc →
    ∃ q, C.Quorum tc.data.epoch q ∧ ∀ k', q k' → ∀ hk' : C.honest tc.data.epoch k',
      ∃ m vote, TimeoutVoteFor k' tc vote ∧ Output.send (.timeoutVote vote) ∈ (trace k' (.of hk') m).output
        ∧ time k' (.of hk') m < time k h n

  /-- The one-honest indication for a view follows a timeout vote for it by a node honest in the vote's epoch. -/
  oneHonestCausal : ∀ k h n v, (trace k h n).input = .timeoutOneHonest v →
    ∃ k' e, ∃ hk' : C.honest e k', ∃ m vote, vote.data.epoch = e
      ∧ Output.send (.timeoutVote vote) ∈ (trace k' (.of hk') m).output
      ∧ vote.view = v ∧ time k' (.of hk') m < time k h n

  /--
  A message from a sender honest in its epoch is one it sent, before it arrived:
  the network is authenticated.
  -/
  authentic : ∀ k h n l msg, (trace k h n).input.sentBy = some (l, msg) →
    ∀ hl : C.Honest l, (Output.send msg).SignedIn (C.honest · l) →
      ∃ m, Output.send msg ∈ (trace l hl m).output ∧ time l hl m < time k h n

namespace TimedNetwork

variable (N : TimedNetwork cfg leader C)

/-- By time `t`, node `k`'s history satisfies `P`: some prefix of steps no later than `t` does. -/
def By (k : PubKey) (h : C.Honest k) (t : Nat) (P : History → Prop) : Prop :=
  ∃ n, (∀ i, i < n → N.time k h i ≤ t) ∧ P ((N.trace k h).history n)

/-- Node `k` sent `m` by time `t`. -/
def SentByTime (k : PubKey) (h : C.Honest k) (t : Nat) (m : Message) : Prop :=
  N.By k h t fun hist => hist.Sent m

/--
After time `t`, node `k` decides a view later than every view it had decided by
`t`, on a `Cert2` of an epoch it is honest in.

Decides on `Cert2`s of other epochs are not the node's own (`History.restrict`). A
node honest in every epoch makes only its own.
-/
def DecidesAfter (k : PubKey) (h : C.Honest k) (t : Nat) : Prop :=
  ∃ v, (∃ n, (((N.trace k h).history n).restrict (C.honest · k)).DecidedView v)
    ∧ ∀ n, (∀ i, i < n → N.time k h i ≤ t) →
      ∀ w, (((N.trace k h).history n).restrict (C.honest · k)).DecidedView w → w < v

/--
Once an honest node's history satisfies `P`, the history of every node honest in
epoch `e` or later satisfies `Q` within `Δ`, counted from GST if that is later.
-/
def Propagates (GST Δ : Nat) (e : EpochNumber) (P Q : History → Prop) : Prop :=
  ∀ k h n, P ((N.trace k h).history (n + 1)) →
    ∀ k' h', C.HonestFrom e k' → N.By k' h' (max (N.time k h n) GST + Δ) Q

/-- What one honest node holds of epoch `e`, every node honest in `e` or later holds within `Δ`. -/
abbrev Spreads (GST Δ : Nat) (e : EpochNumber) (P : History → Prop) : Prop := N.Propagates GST Δ e P P

end TimedNetwork

/--
No obligation a node has, for an epoch it is honest in, stays owed for more than `δ`.

Whatever a node owes after a step (`OwedIn`), it has discharged, or it has stopped
owing, by some step at most `δ` later. A node that acts in the step that creates
the obligation meets this with nothing to spare; one that batches or defers meets
it as long as it does not defer for longer than `δ`.
-/
def Prompt (N : TimedNetwork cfg leader C) (δ : Nat) : Prop :=
  ∀ k h n (o : Obligation), OwedIn cfg leader k (C.honest · k) ((N.trace k h).history (n + 1)) o →
    ∃ m, n ≤ m ∧ N.time k h m ≤ N.time k h n + δ
      ∧ ¬ OwedIn cfg leader k (C.honest · k) ((N.trace k h).history (m + 1)) o

/--
What the environment does after GST, as seen from the nodes' histories.

Each field is an assumption about the environment, not a rule for nodes. The
network owes nothing for what a node sends in an epoch it is not honest in: such a
message need not arrive, nor be valid, nor be one the node really sent. Nor does
it owe a node anything of an epoch after every epoch the node is honest in: a
message of epoch `e` reaches the nodes honest in `e` or later
(`Committee.HonestFrom`). A node retires once its last such epoch has ended. The
fields are of these kinds:

* what the network delivers within `Δ`: `proposal`, `revote`, `cert1`, `cert2`,
  `cert2Spread`, `certSpread`, `lockSpread`, `blockSpread`, `timeoutCert`,
  `timeoutLockSpread` and `epochChange`, and within `2Δ`, `timeoutCertSpread`;
* what a node's own modules deliver: the builder's blocks and headers, and the
  validity reports (`proposalValid`, `validatedSound`, `validated`, `header`). The same bound `Δ`
  covers the delays of these modules, so `Δ` is the larger of the network's bound
  and theirs;
* time, and the view timer of length `τ` (`timeUnbounded`, `timerNotEarly`,
  `timerFires`).
-/
structure Synchrony (N : TimedNetwork cfg leader C) (GST Δ τ : Nat) : Prop where
  /--
  A proposal from a leader honest in its epoch reaches every member of the epoch's
  committee honest in it or later, with that member's share.
  -/
  proposal : ∀ l hl n p, Output.send (.proposal p) ∈ (N.trace l hl n).output → C.honest p.epoch l →
    ∀ k hk, C.members p.epoch k → C.HonestFrom p.epoch k →
      N.By k hk (max (N.time l hl n) GST + Δ) fun hist =>
        ∃ vid, ShareMatches p vid ∧ hist.Received (.proposal l p (some vid))

  /--
  A re-vote request from a leader honest in the block's epoch reaches every member
  of that epoch's committee honest in it or later.
  -/
  revote : ∀ l hl n r, Output.send (.revote r) ∈ (N.trace l hl n).output → C.honest r.cert.data.epoch l →
    ∀ k hk, C.members r.cert.data.epoch k → C.HonestFrom r.cert.data.epoch k →
      N.By k hk (max (N.time l hl n) GST + Δ) fun hist => hist.Received (.revote l r)

  /--
  Vote1s from a quorum of nodes honest in the votes' epoch become a `Cert1` at every
  node honest in that epoch or later.
  -/
  cert1 : ∀ q (d : Vote1Data) v t, C.Quorum d.epoch q →
    (∀ k, q k → ∃ hk : C.honest d.epoch k, N.SentByTime k (.of hk) t (.vote1 ⟨d, v, k⟩)) →
    ∀ k hk, C.HonestFrom d.epoch k → N.By k hk (max t GST + Δ) fun hist => hist.Received (.certificate1 ⟨d, v⟩)

  /--
  Vote2s from a quorum of nodes honest in the votes' epoch become a `Cert2` at every
  node honest in that epoch or later.
  -/
  cert2 : ∀ q (d : Vote2Data) v t, C.Quorum d.epoch q →
    (∀ k, q k → ∃ hk : C.honest d.epoch k, N.SentByTime k (.of hk) t (.vote2 ⟨d, v, k⟩)) →
    ∀ k hk, C.HonestFrom d.epoch k → N.By k hk (max t GST + Δ) fun hist => hist.Received (.certificate2 ⟨d, v⟩)

  /-- A `Cert2` one honest node holds, every node honest in its epoch or later holds. -/
  cert2Spread : ∀ c, N.Spreads GST Δ c.data.epoch fun hist => hist.HasCert2 c

  /--
  A `Cert1` one honest node holds, every node honest in its epoch or later holds.

  How certificates spread is left to implementations: forwarding the certificate a
  node moved to its view on meets it. It is what keeps honest nodes in step, since a
  certificate alone is grounds for the view after it.
  -/
  certSpread : ∀ c, N.Spreads GST Δ c.data.epoch fun hist => hist.HasCert1 cfg c

  /--
  The members of a certificate's epoch, honest in it or later, can lock on it within
  `Δ` of an honest node holding it: they hold the proposal and have rebuilt the payload.

  One `Δ` is enough because what a member needs was sent before the certificate
  formed. A certificate an honest node holds is the anchor's or backed
  (`Network.cert1Genuine`), and a backed `Cert1` means a quorum of the epoch
  voted1 for the block. Before any vote, the leader sent the proposal to every
  member with that member's share. Each honest voter sends its own share with its
  vote1 (`Message.vidShare`), and the shares of a quorum's honest members suffice
  to rebuild the payload (what VID provides). So by the time an honest node holds
  the certificate, the block and the shares have all been sent, and after GST they
  arrive within `Δ`.

  A member that a faulty leader left out never received the proposal, and has to
  fetch the proposal: a request and its reply. The premise asks for it within
  `Δ` all the same. An implementation meets it by counting that fetch into `Δ`, or
  by having each voter send the proposal along with its share. Nodes outside the
  committee hold no share, and are not asked to lock.
  -/
  lockSpread : ∀ c k hk n, ((N.trace k hk).history (n + 1)).HasCert1 cfg c →
    ∀ k' hk', C.members c.data.epoch k' → C.HonestFrom c.data.epoch k' →
      N.By k' hk' (max (N.time k hk n) GST + Δ) fun hist => hist.Lockable cfg c

  /--
  A block one honest node holds with a certificate over it, every node honest in the
  certificate's epoch or later holds.

  The block, not its payload: with the certificate (`certSpread`), it is what a
  node outside the block's committee needs to decide it.
  -/
  blockSpread : ∀ c b, Certifies c b →
    N.Propagates GST Δ c.data.epoch (fun hist => hist.HasCert1 cfg c ∧ hist.HasProposal cfg b)
      fun hist => hist.HasProposal cfg b

  /--
  Timeout votes from a quorum of nodes honest in the votes' epoch become a timeout
  certificate at every node honest in that epoch or later, for their view and epoch.

  Its lock is whatever the votes it was formed from name.
  -/
  timeoutCert : ∀ e q v t, C.Quorum e q →
    (∀ k, q k → ∃ hk : C.honest e k, ∃ L, N.SentByTime k (.of hk) t (.timeoutVote ⟨⟨e, L⟩, v, k⟩)) →
    ∀ k hk, C.HonestFrom e k → N.By k hk (max t GST + Δ) fun hist =>
      ∃ tc, tc.data.epoch = e ∧ tc.view = v ∧ hist.Received (.timeoutCertificate tc)

  /--
  A timeout certificate one honest node holds, every node honest in its epoch or
  later holds one for the same view and epoch, within `2Δ`.

  It is grounds for the view after it. Any certificate for the view and epoch is,
  whatever lock it names; what the lock asks of the network is `timeoutLockSpread`.

  A node meets this by sending a timeout certificate on to every node when it first
  receives it. Within one epoch the one-honest indication would do without: the
  honest signers' timeout votes reach every node within `Δ`, and a node answers the
  indication with its own timeout vote, so the votes form a certificate everywhere
  within another `Δ`. At an epoch boundary it does not. A node answers only in its
  own epoch, and votes of two epochs never form one certificate, so a node already
  in the next epoch adds nothing towards a certificate of the epoch before. A node
  that formed such a certificate and moved past its view then times out only later
  views. The nodes behind it can be one vote short in each epoch, and wait for ever
  unless the certificate itself reaches them.
  -/
  timeoutCertSpread : ∀ tc, N.Propagates GST (2 * Δ) tc.data.epoch
    (fun hist => hist.Received (.timeoutCertificate tc)) fun hist =>
      ∃ tc', tc'.view = tc.view ∧ tc'.data.epoch = tc.data.epoch ∧ hist.Received (.timeoutCertificate tc')

  /--
  Within `Δ` of an honest node receiving a timeout certificate, every member of the
  lock's epoch that is honest in the certificate's epoch or later can lock on the
  certificate's lock.

  The lock is a `Cert1` the verifier checked (`TimeoutLockChecked`), so a quorum
  voted1 for its block, and one `Δ` is enough for the reasons given at
  `lockSpread`. It is what lets an honest leader build on the lock after a timeout,
  and the members vote1 for the block built on it.
  -/
  timeoutLockSpread : ∀ tc k hk n, ((N.trace k hk).history (n + 1)).Received (.timeoutCertificate tc) →
    ∀ k' hk', C.members tc.data.lock.data.epoch k' → C.HonestFrom tc.data.epoch k' →
      N.By k' hk' (max (N.time k hk n) GST + Δ) fun hist => hist.Lockable cfg tc.data.lock

  /--
  The last block of an epoch that an honest node holds with a `Cert2` over it
  reaches every node honest in that epoch or later as an epoch change, with that
  `Cert2` and the block's own `Cert1`. So a node honest in the epoch that ended
  learns that it ended, whether or not it is honest in the next.

  An implementation meets it by sending the epoch change once it holds the block's
  proposal and the `Cert2`, as the evidence that the epoch ended, fetching the block's
  `Cert1` if it does not hold it.
  -/
  epochChange : ∀ c2 b, Commits c2 b → IsLastBlock b.blockHeader.blockNumber cfg.epochHeight →
    N.Propagates GST Δ c2.data.epoch (fun hist => hist.HasProposal cfg b ∧ hist.HasCert2 c2)
      fun hist => ∃ c1, hist.TookEpochChange cfg c1 c2 b

  /--
  A block a leader honest in its epoch proposes is valid.

  The leader builds it from a header its builder handed it, and consensus does not
  interpret blocks, so this is an assumption about the builder.
  -/
  proposalValid : ∀ l hl n p, Output.send (.proposal p) ∈ (N.trace l hl n).output → C.honest p.epoch l →
    BlockValid p

  /--
  A block reported valid is valid.

  The report is what obliges a vote1 (`OwedVote1.validated`), while signing one
  needs the block to be valid (`SafeHistory.vote1Justified`), so a false report
  would oblige what no rule permits.
  -/
  validatedSound : ∀ k hk n v hash, (N.trace k hk n).input = .blockValidated v hash →
    ∀ b : Block, blockHash b = hash → BlockValid b

  /-- A valid block a node honest in its epoch or later received is reported valid. -/
  validated : ∀ k hk n sender p vid, (N.trace k hk n).input = .proposal sender p (some vid) →
    BlockValid p → C.HonestFrom p.epoch k →
      N.By k hk (max (N.time k hk n) GST + Δ) fun hist =>
        hist.Received (.blockValidated p.viewNumber (blockHash p))

  /--
  A node honest in `p`'s epoch that is ready to propose `p` except for its header
  (`ProposalReady`) is handed a header for `p`'s view and parent, at `p`'s height.

  Only in the view the node is in: a builder is asked for the block the node is
  about to propose, not for views the node has already left.
  -/
  header : ∀ k hk n p, C.honest p.epoch k → ((N.trace k hk).history (n + 1)).InView cfg p.viewNumber →
    ProposalReady cfg leader k ((N.trace k hk).history (n + 1)) p →
    N.By k hk (max (N.time k hk n) GST + Δ) fun hist =>
      ∃ hdr, hdr.blockNumber = p.blockHeader.blockNumber
        ∧ hist.Received (.headerBuilt p.viewNumber p.parentCert.data.blockHash hdr)

  /--
  Time passes: every honest node's steps go on to times as late as any.

  Without this a run could fit infinitely many steps, and every view, before some
  time, and nothing would be left to decide after it.
  -/
  timeUnbounded : ∀ k hk T, ∃ n, T < N.time k hk n

  /--
  The timer for a view does not fire before `τ` has passed since the node entered
  it, or since its first step if it starts in the view.
  -/
  timerNotEarly : ∀ k hk n m v, ((N.trace k hk).history (n + 1)).InView cfg v →
    (n = 0 ∨ ¬ ((N.trace k hk).history n).InView cfg v) → n ≤ m →
    (N.trace k hk m).input = .timeout v → N.time k hk n + τ ≤ N.time k hk m

  /--
  The timer for the node's view fires within `τ`, unless the node moves on to a
  later view first. `τ` is counted from when the node entered the view, from its
  first step if it starts in the view, or from the timer's last firing for it.

  So a node that stays in a view times out on it again every `τ`. That is what
  lets a view whose first timeout votes did not form a certificate recover: at an
  epoch boundary the first votes can name two epochs, and a later round names the
  one every honest node has reached by then.
  -/
  timerFires : ∀ k hk n v, ((N.trace k hk).history (n + 1)).InView cfg v →
    (n = 0 ∨ ¬ ((N.trace k hk).history n).InView cfg v ∨ (N.trace k hk n).input = .timeout v) →
    ∃ m, n < m ∧ N.time k hk m ≤ N.time k hk n + τ
      ∧ ((N.trace k hk m).input = .timeout v
        ∨ ∃ w, v < w ∧ ((N.trace k hk).history (m + 1)).InView cfg w)

/--
The leader schedule gives every epoch, from any view on, a later view whose leader
is a member of the epoch's committee honest in it.

Every epoch includes one that has ended: its last block may still need a re-vote
from its own committee, under a leader honest in that epoch.

One is enough: a view commits its own block, with both rounds of votes cast in
it, so no decide waits for the leader after it.
-/
def LeaderRotation (C : Committee) (leader : EpochNumber → ViewNumber → Option PubKey) : Prop :=
  ∀ e v, ∃ w, v ≤ w ∧ ∃ l, leader e w = some l ∧ C.honest e l ∧ C.members e l

end NewProtocol
