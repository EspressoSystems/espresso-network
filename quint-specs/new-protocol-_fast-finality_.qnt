// New Protocol (Fast Finality) — V0_6 BFT consensus
//
// Models the V0_6 fast-finality consensus state machine of
// `crates/hotshot/new-protocol/src/` at the level of votes, quorum/timeout
// certificates, view changes, epochs, and cutover from the legacy stack.
//
// The abstraction is tuned to REPLAY faithfully against the crate's
// single-node `ConsensusHarness` tests (tests/consensus.rs, tests/cutover.rs):
// each logged transition is one observable event of ONE consensus instance.
//
// Scope / abstractions (see the "does NOT cover" note at the bottom):
//   * The message transport is an abstract broadcast soup (`network`); we do
//     NOT model cliquenet's x25519 encryption, framing, or ordering.
//   * A block/leaf is abstracted to `{ view, block }` with `block` an opaque
//     content id. The instrumentation logs a constant block for the canonical
//     chain; equivocation (two blocks at one view) is reachable in simulation
//     but honest tests never drive it.
//   * Signatures/stake are abstracted: a certificate is a fact that a quorum
//     (> 2/3 of total stake, distinct signers) voted for the exact (view,
//     block). A single node only ever emits its OWN vote and RECEIVES the
//     assembled certificate, so `form_qc1`/`form_qc2`/`form_tc` self-supply the
//     quorum of votes the cert stands for — this is what folds the code's
//     `VoteCollector` accumulate side and `is_valid_cert` verify side into one
//     fact, and keeps the supermajority property (`certs_need_supermajority`)
//     true by construction.
//   * A view starts from GENESIS (view 0 decided) or from a CUTOVER seed; both
//     leave view 0 anchored. DRB, VID reconstruction, persistence-gated sends
//     and restart recovery are abstracted — they gate WHEN a node emits, not
//     which certificates/decisions are possible.
//   * Epoch membership is fixed (same validator set every epoch); only the
//     epoch counter advancing is modelled. Dynamic stake-table changes across
//     epochs are out of scope of this model.

module fastFinality {
  import basicSpells.* from "./spells/basicSpells"

  // ─── Parameters (fixed; kept as pure vals so the module runs directly) ───

  /// The validator set. N = 4 is the minimal BFT size: f = 1, quorum = 3.
  pure val VALIDATORS: Set[int] = Set(0, 1, 2, 3)

  /// Possible opaque block contents. Two values let a byzantine leader
  /// equivocate at a single view (simulation only; instrumentation logs 1).
  pure val BLOCKS: Set[int] = Set(1, 2)

  /// Highest view the model explores. Covers the single-node tests' range
  /// (consensus.rs drives up to ~view 5-6).
  pure val MAX_VIEW: int = 8

  /// Views the model ranges over (0 is the genesis / legacy anchor).
  pure val VIEWS: Set[int] = 0.to(MAX_VIEW)

  /// Number of views per epoch (epoch boundary derived from view/block number).
  pure val EPOCH_LENGTH: int = 2

  /// The genesis / legacy handover anchor: view 0 is decided at start.
  pure val LEGACY_ANCHOR: int = 0
  pure val CUTOVER_VIEW: int = 1
  /// The (opaque) block the anchor leaf carries.
  pure val LEGACY_BLOCK: int = 9

  // ─── Types ───

  type ViewNum = int

  /// A leaf/block, abstracted to its view and opaque content.
  type Leaf = { view: ViewNum, block: int }

  /// Wire messages and certificates carried by the abstract transport.
  /// Certificates (QC1/QC2/TC) are facts placed on the network once a quorum
  /// of votes exists, standing in for the code's SimpleCertificate.
  type Msg =
    | Proposal({ view: ViewNum, block: int, leader: int })
    | Vote1({ view: ViewNum, block: int, voter: int })
    | Vote2({ view: ViewNum, block: int, voter: int })
    | TimeoutVote({ view: ViewNum, voter: int })
    | QC1({ view: ViewNum, block: int })
    | QC2({ view: ViewNum, block: int })
    | TC({ view: ViewNum })

  // ─── State (one consensus instance's view of the network) ───

  var seeded: bool                // has the cutover seed been applied?
  var network: Set[Msg]           // abstract broadcast soup (messages + certs)
  var curView: ViewNum            // current_view
  var curEpoch: int               // current_epoch
  var lockedView: ViewNum         // view of locked_cert (the "high QC" / lock)
  var lockedBlock: Option[int]    // block the lock commits to
  var timeoutView: ViewNum        // high-water timeout view: views <= this are dead for voting/proposing
  var decided: Set[Leaf]          // committed leaves (grows only)
  var lastDecidedView: ViewNum    // committed high-water mark (advances only)

  // ─── Pure logic ───

  /// Leader of a view: deterministic rotation over the validator set.
  pure def leaderOf(view: ViewNum): int = view % VALIDATORS.size()

  /// Quorum = strictly more than 2/3 of total stake. Stake is 1 per validator,
  /// so this is `3 * |signers| > 2 * N` (i.e. >= 3 of 4).
  pure def meetsQuorum(signers: Set[int]): bool =
    3 * signers.size() > 2 * VALIDATORS.size()

  /// Every validator's vote of each kind (the quorum a received cert stands for).
  pure def allVote1(v: ViewNum, b: int): Set[Msg] =
    VALIDATORS.map(x => Vote1({ view: v, block: b, voter: x }))
  pure def allVote2(v: ViewNum, b: int): Set[Msg] =
    VALIDATORS.map(x => Vote2({ view: v, block: b, voter: x }))
  pure def allTimeout(v: ViewNum): Set[Msg] =
    VALIDATORS.map(x => TimeoutVote({ view: v, voter: x }))

  pure def vote1Signers(net: Set[Msg], v: ViewNum, b: int): Set[int] =
    net.filter(m => match m { | Vote1(d) => d.view == v and d.block == b | _ => false })
       .map(m => match m { | Vote1(d) => d.voter | _ => -1 })
  pure def vote2Signers(net: Set[Msg], v: ViewNum, b: int): Set[int] =
    net.filter(m => match m { | Vote2(d) => d.view == v and d.block == b | _ => false })
       .map(m => match m { | Vote2(d) => d.voter | _ => -1 })
  pure def timeoutSigners(net: Set[Msg], v: ViewNum): Set[int] =
    net.filter(m => match m { | TimeoutVote(d) => d.view == v | _ => false })
       .map(m => match m { | TimeoutVote(d) => d.voter | _ => -1 })

  pure def hasQC1(net: Set[Msg], v: ViewNum, b: int): bool =
    net.contains(QC1({ view: v, block: b }))
  pure def hasQC2(net: Set[Msg], v: ViewNum, b: int): bool =
    net.contains(QC2({ view: v, block: b }))
  pure def hasTC(net: Set[Msg], v: ViewNum): bool =
    net.contains(TC({ view: v }))

  /// No QC1/QC2 exists at `v` for a block other than `b` (one block per view).
  pure def noConflictingQC1(net: Set[Msg], v: ViewNum, b: int): bool =
    not(net.exists(m => match m { | QC1(d) => d.view == v and d.block != b | _ => false }))
  pure def noConflictingQC2(net: Set[Msg], v: ViewNum, b: int): bool =
    not(net.exists(m => match m { | QC2(d) => d.view == v and d.block != b | _ => false }))

  pure def lastViewOfEpoch(e: int): ViewNum = (e + 1) * EPOCH_LENGTH - 1

  // ─── Actions ───

  action init: bool = all {
    seeded' = false,
    // Genesis: view 0 is decided and carries its quorum certificates.
    network' = Set(
      QC1({ view: LEGACY_ANCHOR, block: LEGACY_BLOCK }),
      QC2({ view: LEGACY_ANCHOR, block: LEGACY_BLOCK })),
    curView' = 0,
    curEpoch' = 0,
    lockedView' = LEGACY_ANCHOR,
    lockedBlock' = Some(LEGACY_BLOCK),
    timeoutView' = 0,
    decided' = Set({ view: LEGACY_ANCHOR, block: LEGACY_BLOCK }),
    lastDecidedView' = LEGACY_ANCHOR,
  }

  /// Apply the cutover seed (apply_pre_cutover_seed): mark the handover from
  /// the legacy stack. The anchor coincides with genesis (view 0) in this
  /// model, so seeding only records that a cutover occurred. Idempotent — the
  /// real seed may be applied repeatedly (with the same or an older seed)
  /// without regressing state.
  action apply_cutover_seed: bool = all {
    seeded' = true,
    network' = network,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// The leader of `view` broadcasts a proposal (ConsensusOutput::SendProposal).
  /// A byzantine leader may fire this again with a different block.
  action propose(view: ViewNum, block: int): bool = all {
    view > timeoutView,
    not(network.contains(Proposal({ view: view, block: block, leader: leaderOf(view) }))),
    network' = network.union(Set(Proposal({ view: view, block: block, leader: leaderOf(view) }))),
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// Cast a phase-1 vote (ConsensusOutput::SendVote1). Suppressed for
  /// timed-out views, matching maybe_vote_1's `view > timeout_view` guard.
  action vote1(view: ViewNum, block: int): bool = all {
    view > timeoutView,
    network' = network.union(allVote1(view, block)),
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// A phase-1 certificate is received/verified (handle_certificate1). It
  /// stands for a quorum of distinct signers, supplied here.
  action form_qc1(view: ViewNum, block: int): bool = all {
    noConflictingQC1(network, view, block),
    network' = network.union(allVote1(view, block)).union(Set(QC1({ view: view, block: block }))),
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// On a QC1, update the lock and cast a phase-2 vote
  /// (maybe_vote_2_and_update_lock; ConsensusOutput::SendVote2). Vote2 is
  /// allowed even for timed-out views. The lock only ever advances.
  action vote2_and_lock(view: ViewNum, block: int): bool =
    val advances = view > lockedView
    all {
      hasQC1(network, view, block),
      network' = network.union(allVote2(view, block)),
      seeded' = seeded,
      curView' = if (advances) max(curView, view + 1) else curView,
      curEpoch' = curEpoch,
      lockedView' = if (advances) view else lockedView,
      lockedBlock' = if (advances) Some(block) else lockedBlock,
      timeoutView' = timeoutView,
      decided' = decided,
      lastDecidedView' = lastDecidedView,
    }

  /// A phase-2 certificate is received/verified (handle_certificate2).
  /// Independent of a local QC1 — cert1 and cert2 arrive separately over the
  /// network, and a node may receive a cert2 for a view (e.g. an ancestor being
  /// gap-filled) whose cert1 it never handled. `decide` still requires both.
  action form_qc2(view: ViewNum, block: int): bool = all {
    noConflictingQC2(network, view, block),
    network' = network.union(allVote2(view, block)).union(Set(QC2({ view: view, block: block }))),
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// Decide (commit) a leaf once both its QC1 and QC2 exist (maybe_decide, at
  /// the triggering view). The committed high-water mark advances only forward;
  /// the decided set only grows, so a committed leaf is never reverted.
  action decide(view: ViewNum, block: int): bool = all {
    hasQC1(network, view, block),
    hasQC2(network, view, block),
    not(decided.contains({ view: view, block: block })),
    network' = network,
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided.union(Set({ view: view, block: block })),
    lastDecidedView' = if (view > lastDecidedView) view else lastDecidedView,
  }

  /// The view fails to make progress (handle_timeout): broadcast a timeout vote
  /// and bump the timeout high-water mark, killing propose/vote1 for that view.
  action timeout(view: ViewNum): bool = all {
    network' = network.union(allTimeout(view)),
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = max(timeoutView, view),
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// A timeout certificate is received/verified (handle_timeout_certificate).
  action form_tc(view: ViewNum): bool = all {
    network' = network.union(allTimeout(view)).union(Set(TC({ view: view }))),
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// Advance past a failed view via its timeout certificate
  /// (handle_timeout_certificate): the only non-QC way current_view moves.
  action advance_on_tc(view: ViewNum): bool = all {
    hasTC(network, view),
    network' = network,
    seeded' = seeded,
    curView' = max(curView, view + 1),
    curEpoch' = curEpoch,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = max(timeoutView, view),
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  /// Advance the epoch once the last view of the current epoch is decided
  /// (SendEpochChange / handle_epoch_change).
  action advance_epoch: bool = all {
    curEpoch < 2,
    decided.exists(l => l.view >= lastViewOfEpoch(curEpoch)),
    network' = network,
    seeded' = seeded,
    curView' = curView,
    curEpoch' = curEpoch + 1,
    lockedView' = lockedView,
    lockedBlock' = lockedBlock,
    timeoutView' = timeoutView,
    decided' = decided,
    lastDecidedView' = lastDecidedView,
  }

  action step: bool = any {
    apply_cutover_seed,
    advance_epoch,
    { nondet view = VIEWS.oneOf() timeout(view) },
    { nondet view = VIEWS.oneOf() form_tc(view) },
    { nondet view = VIEWS.oneOf() advance_on_tc(view) },
    { nondet view = VIEWS.oneOf() nondet block = BLOCKS.oneOf() propose(view, block) },
    { nondet view = VIEWS.oneOf() nondet block = BLOCKS.oneOf() vote1(view, block) },
    { nondet view = VIEWS.oneOf() nondet block = BLOCKS.oneOf() form_qc1(view, block) },
    { nondet view = VIEWS.oneOf() nondet block = BLOCKS.oneOf() vote2_and_lock(view, block) },
    { nondet view = VIEWS.oneOf() nondet block = BLOCKS.oneOf() form_qc2(view, block) },
    { nondet view = VIEWS.oneOf() nondet block = BLOCKS.oneOf() decide(view, block) },
  }

  // ─── Observations (positive; the behaviors worth measuring coverage for) ───

  /// The cutover seed has been applied (handover from the legacy stack).
  val cutover_seeded: bool = seeded

  /// A phase-1 vote has been cast in a new-protocol view.
  val vote1_cast: bool =
    network.exists(m => match m { | Vote1(d) => d.view > LEGACY_ANCHOR | _ => false })

  /// A quorum certificate (QC1) formed for a new-protocol view.
  val qc1_formed: bool =
    network.exists(m => match m { | QC1(d) => d.view > LEGACY_ANCHOR | _ => false })

  /// A second-phase certificate (QC2) formed for a new-protocol view.
  val qc2_formed: bool =
    network.exists(m => match m { | QC2(d) => d.view > LEGACY_ANCHOR | _ => false })

  /// The lock advanced past the legacy anchor.
  val lock_advanced: bool = lockedView > LEGACY_ANCHOR

  /// A new-protocol leaf was decided (committed) beyond the legacy handover.
  val leaf_decided: bool = decided.exists(l => l.view > LEGACY_ANCHOR)

  /// The committed chain extended past the legacy anchor.
  val chain_extends_legacy: bool = lastDecidedView > LEGACY_ANCHOR

  /// A view failed and a timeout certificate formed.
  val view_timed_out: bool =
    network.exists(m => match m { | TC(_) => true | _ => false })

  /// The current view advanced via a timeout certificate.
  val timeout_recovered: bool =
    curView > CUTOVER_VIEW and hasTC(network, curView - 1)

  /// The network advanced into a later epoch.
  val epoch_changed: bool = curEpoch > 0

  /// A byzantine leader equivocated: two different blocks proposed at one view.
  val conflicting_proposals: bool =
    VIEWS.exists(v =>
      network.filter(m => match m { | Proposal(d) => d.view == v | _ => false })
             .map(m => match m { | Proposal(d) => d.block | _ => -1 })
             .size() > 1)

  // ─── Safety invariants ───

  /// Agreement: no two conflicting leaves are decided at the same view.
  val agreement: bool =
    decided.forall(a => decided.forall(b =>
      (a.view == b.view) implies (a.block == b.block)))

  /// A view commits at most one block: no two QC2 differ at the same view.
  val one_block_per_view: bool =
    VIEWS.forall(v => BLOCKS.union(Set(LEGACY_BLOCK)).forall(b1 =>
      BLOCKS.union(Set(LEGACY_BLOCK)).forall(b2 =>
        (hasQC2(network, v, b1) and hasQC2(network, v, b2)) implies (b1 == b2))))

  /// Every new-protocol certificate is backed by a quorum (>2/3 stake) of
  /// distinct signers for that exact (view, block). The anchor certs are
  /// grandfathered by genesis/cutover, so they are exempt.
  val certs_need_supermajority: bool =
    network.forall(m => match m {
      | QC1(d) => (d.view <= LEGACY_ANCHOR) or meetsQuorum(vote1Signers(network, d.view, d.block))
      | QC2(d) => (d.view <= LEGACY_ANCHOR) or meetsQuorum(vote2Signers(network, d.view, d.block))
      | TC(d)  => meetsQuorum(timeoutSigners(network, d.view))
      | _ => true
    })

  /// The committed high-water mark never drops below the handover point.
  val no_regression_below_anchor: bool =
    lastDecidedView >= LEGACY_ANCHOR

  /// Cutover has no gap at the boundary: if any new-protocol leaf is decided,
  /// the anchor leaf is decided too — the new chain extends the legacy decided
  /// chain rather than starting a disconnected one.
  val cutover_no_gap: bool =
    decided.exists(l => l.view > LEGACY_ANCHOR)
      implies decided.contains({ view: LEGACY_ANCHOR, block: LEGACY_BLOCK })
}
