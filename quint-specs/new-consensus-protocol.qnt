// -*- mode: Bluespec; -*-
//
// New Consensus Protocol (HotShot V0_6 fast finality).
//
// Models ONE node's `Consensus<T>` state machine
// (crates/hotshot/new-protocol/src/consensus.rs). Every action below is either
// an input the node's `apply(ConsensusInput, &mut Outbox<ConsensusOutput>)`
// dispatcher accepts (consensus.rs:637), or one of the guarded `maybe_*` rules
// `apply` re-runs after each input.
//
// COVERED
//   two-phase voting (`maybe_vote_1` 1993, `maybe_vote_2_and_update_lock` 2159),
//   locking and the safety-or-liveness rule (`is_safe` 2274), the decide rule
//   and its ancestor gap-fill walk (`maybe_decide` 1740), proposing
//   (`maybe_propose` 1548), timeouts (`handle_timeout` 1320) and timeout
//   certificates (`handle_timeout_certificate` 1431), and the storage gating
//   that holds votes back until their facts are persisted (`handle_stored`
//   1883, `release_vote1` 1913, `release_vote2` 1936).
//
// NOT COVERED (adjacent components / deliberate abstractions)
//   VID dispersal and reconstruction, cliquenet networking, storage internals,
//   header/state validation and the legacy cutover are abstract inputs
//   (`block_reconstructed`, `state_validated`, `stored`) rather than modelled
//   machinery. Signatures, stake tables and quorum arithmetic are abstract: a
//   certificate is a stake-threshold FACT the environment may hand the node
//   (`cert_verifier.rs` establishes validity, `vote.rs` / `vote/accumulate.rs`
//   establish formation). Leader election is likewise abstract: `propose` is
//   guarded on the proposal inputs being ready, not on this node holding the
//   slot, so the model over-approximates who may propose. Epochs, DRB results,
//   epoch-root state certificates, the epoch-change message, restart barring
//   and the proposal/VID pairing buffers are out of scope; `staked_in_epoch`
//   is always true, a proposal always arrives paired with this node's VID
//   share, and the leader's block and header are always available.
//
//   Storage is collapsed to two facts, because their ORDER is what the
//   protocol depends on: `stored` confirms a view's proposal, VID share and
//   Vote action together, while `stored_high_qc` confirms the lock separately.
//
// AGREEMENT ACROSS BRANCHES
//   Certificates are abstract facts here, so nothing stops the environment
//   from handing this node certificates on two conflicting branches at
//   different views. Cross-branch agreement rests on quorum intersection,
//   which lives outside a single-node model; what this spec states is the
//   per-view form: the node never decides two conflicting leaves for the same
//   view, and never votes for a leaf its view's Cert1 does not certify.
//
// LEAF COMMITMENTS
//   A leaf commitment hashes the whole proposal, so it determines the leaf's
//   view AND the leaf its justify_qc certifies. Here a leaf is the integer
//   `2 * view + branch`, so the view is intrinsic, and the justify_qc link is
//   LEARNED from each accepted proposal into `parents` rather than fixed by
//   the model. `branch` distinguishes conflicting proposals at one view;
//   collision freedom is assumed, not modelled.
module new_consensus_protocol {
  import basicSpells.* from "./spells/basicSpells"

  /// A leaf commitment, encoded as `2 * view + branch`. `GENESIS` is the
  /// anchor leaf at view 0.
  type Leaf = int

  pure val GENESIS: Leaf = 0

  /// Highest view the model ranges over.
  pure val MAX_VIEW: int = 6

  /// The two leaves of each view 1..MAX_VIEW: `2v` (branch 0, the canonical
  /// chain) and `2v + 1` (branch 1, a conflicting proposal at the same view).
  pure val LEAVES: Set[Leaf] = 2.to(2 * MAX_VIEW + 1)

  /// What a proposal's justify_qc may certify: any leaf, or the anchor.
  pure val PARENT_LEAVES: Set[Leaf] = LEAVES.union(Set(GENESIS))

  /// Views the rules act in. A timeout certificate for MAX_VIEW enters
  /// MAX_VIEW + 1.
  pure val VIEWS: Set[int] = 1.to(MAX_VIEW + 1)

  /// One entry per view, so an ancestor walk can never run short. The values
  /// are unused; only the length matters.
  pure val WALK_STEPS: List[int] = List(1, 2, 3, 4, 5, 6)

  /// consensus.rs:197 — decide inputs are retained this far behind the
  /// watermark, so an older gap view can still decide.
  pure val DECIDE_BUFFER: int = 20

  pure def view_of(leaf: Leaf): int = leaf / 2

  // ------------------------------------------------------------- state ------

  /// The protocol state the rules read and write (consensus.rs:202).
  /// `parents` is the learned justify_qc link: leaf -> the leaf its
  /// justify_qc certifies, recorded when the proposal is accepted.
  type ProtocolState = {
    proposals: int -> Leaf,
    parents: Leaf -> Leaf,
    certs: int -> Leaf,
    certs2: int -> Leaf,
    timeoutCerts: Set[int],
    lockedCert: Option[Leaf],
    decidedViews: Set[int],
    lastDecidedView: int,
    voted1: Set[Leaf],
    voted2: Set[Leaf],
    // Views whose block payload this node holds. The code keys this by
    // (view, payload commitment); a payload has no leaf identity until the
    // view's proposal arrives, so the model keys it by view alone and does
    // not cover a reconstructed payload that mismatches the stored proposal.
    blocksReconstructed: Set[int],
    statesVerified: int -> Leaf,
    vidShares: Set[int],
    currentView: int,
    timeoutView: int,
    proposedViews: Set[int],
  }

  /// A phase-2 vote parked until its lock is persisted (`pending_vote2`,
  /// consensus.rs:244): the voted leaf, and the locked-QC view whose
  /// persistence releases it.
  type PendingVote2 = { leaf: Leaf, required: int }

  /// Durability facts, and the votes parked waiting on them
  /// (consensus.rs:232-245).
  type StorageState = {
    storedViews: Set[int],
    storedHighQc: Option[int],
    pendingVote1: int -> Leaf,
    pendingVote2: int -> PendingVote2,
  }

  /// Not protocol state: the outputs the node released and the watermarks the
  /// properties compare against. Nothing here gates a rule.
  type History = {
    decidedLeaves: Set[Leaf],
    sentVote1: Set[Leaf],
    sentVote2: Set[Leaf],
    decideTriggers: Set[int],
    lateDecides: Set[int],
    droppedVote1: Set[int],
    proposedAfterTimeout: Set[int],
    rejectedUnsafe: Set[Leaf],
    maxDecideBatch: int,
    lockAdvances: int,
    prevLockedView: int,
    prevLastDecidedView: int,
  }

  type NodeState = {
    proto: ProtocolState,
    storage: StorageState,
    history: History,
  }

  var node: NodeState

  pure def with_proto(n: NodeState, p: ProtocolState): NodeState = { ...n, proto: p }
  pure def with_storage(n: NodeState, s: StorageState): NodeState = { ...n, storage: s }
  pure def with_history(n: NodeState, h: History): NodeState = { ...n, history: h }

  // ---------------------------------------------------------------- reads --

  /// The leaf this leaf's justify_qc certifies, as learned when its proposal
  /// was accepted.
  pure def parent_of(n: NodeState, leaf: Leaf): Leaf =
    n.proto.parents.getOrElse(leaf, GENESIS)

  pure def locked_view(n: NodeState): int =
    match n.proto.lockedCert {
      | Some(leaf) => view_of(leaf)
      | None => 0
    }

  /// consensus.rs:628 — newest view that can no longer be decided.
  pure def decide_floor(n: NodeState): int =
    max(n.proto.lastDecidedView - DECIDE_BUFFER, 0)

  /// consensus.rs:439 — whether the lock at `required` is confirmed persisted.
  pure def high_qc_persisted(n: NodeState, required: int): bool =
    match n.storage.storedHighQc {
      | Some(stored) => stored >= required
      | None => false
    }

  /// consensus.rs:1955 — the view's Vote action and VID share are persisted.
  pure def vote2_persisted(n: NodeState, view: int): bool =
    n.storage.storedViews.contains(view)

  /// consensus.rs:448 — either we hold the parent's payload, or our lock
  /// certifies exactly this parent leaf (a lock is only taken on a
  /// reconstructed block).
  /// NOTE: nothing below may bind a name the replay picks use (`leaf`,
  /// `parent`, `view`) except the actions' own parameters — a shadowing
  /// binding evaluated during a step makes the oracle unable to pin that pick.
  pure def parent_reconstructed(n: NodeState, qcLeaf: Leaf): bool =
    n.proto.blocksReconstructed.contains(view_of(qcLeaf))
      or n.proto.lockedCert == Some(qcLeaf)

  /// consensus.rs:2274 — the safety-or-liveness lock rule, over the proposal's
  /// leaf and the leaf its justify_qc certifies.
  pure def is_safe(n: NodeState, leaf: Leaf, parent: Leaf): bool =
    match n.proto.lockedCert {
      | None => true
      | Some(locked) =>
        if (view_of(locked) == view_of(leaf)) locked == leaf
        else parent == locked or view_of(parent) > view_of(locked)
    }

  /// consensus.rs:898 — a proposal is wanted unless the lock is newer than it
  /// or its view is already fully processed.
  pure def wants_proposal(n: NodeState, leaf: Leaf): bool =
    locked_view(n) <= view_of(leaf) and not(n.proto.proposals.has(view_of(leaf)))

  /// consensus.rs:1556 — the parent certificate a proposal at `view` builds on:
  /// the lock when a timeout certificate carried us into the view, else the
  /// previous view's Cert1.
  pure def propose_parent(n: NodeState, view: int): Option[Leaf] =
    if (n.proto.timeoutCerts.contains(view)) n.proto.lockedCert
    else if (n.proto.certs.has(view - 1)) Some(n.proto.certs.get(view - 1))
    else None

  // ----------------------------------------------------------- guards -------

  /// A justify_qc always certifies an earlier view than the proposal it backs.
  pure def parent_precedes(leaf: Leaf, parent: Leaf): bool =
    view_of(parent) < view_of(leaf)

  pure def can_receive_proposal(n: NodeState, leaf: Leaf, parent: Leaf): bool = and {
    parent_precedes(leaf, parent),
    wants_proposal(n, leaf),
    is_safe(n, leaf, parent),
  }

  pure def can_reject_unsafe(n: NodeState, leaf: Leaf, parent: Leaf): bool = and {
    parent_precedes(leaf, parent),
    wants_proposal(n, leaf),
    not(is_safe(n, leaf, parent)),
    not(n.history.rejectedUnsafe.contains(leaf)),
  }

  /// consensus.rs:1213 — a Cert1 is kept only above the decide floor, and the
  /// first one at a view wins.
  pure def can_receive_cert1(n: NodeState, leaf: Leaf): bool =
    view_of(leaf) > decide_floor(n) and not(n.proto.certs.has(view_of(leaf)))

  pure def can_receive_cert2(n: NodeState, leaf: Leaf): bool =
    view_of(leaf) > decide_floor(n) and not(n.proto.certs2.has(view_of(leaf)))

  pure def can_reconstruct_block(n: NodeState, view: int): bool =
    not(n.proto.blocksReconstructed.contains(view))

  pure def can_validate_state(n: NodeState, leaf: Leaf): bool =
    n.proto.statesVerified.getOrElse(view_of(leaf), -1) != leaf

  /// consensus.rs:1326 — a timeout for a stale view is ignored.
  pure def can_timeout(n: NodeState, view: int): bool =
    view >= n.proto.currentView and view > n.proto.timeoutView

  /// consensus.rs:1436 — a timeout certificate for the timed-out `view` carries
  /// the node into `view + 1`.
  pure def can_receive_timeout_cert(n: NodeState, view: int): bool = {
    val entered = view + 1
    entered >= n.proto.currentView and not(n.proto.timeoutCerts.contains(entered))
  }

  /// consensus.rs:2045 — the parent chain check: the parent's stored proposal
  /// matches the justify_qc and its block counts as reconstructed. Skipped when
  /// the justify_qc is the genesis QC.
  pure def parent_chain_ok(n: NodeState, child: Leaf): bool = {
    val qcLeaf = parent_of(n, child)
    val qcView = view_of(qcLeaf)
    if (qcView == 0) true
    else if (n.proto.proposals.getOrElse(qcView, -1) != qcLeaf) false
    else parent_reconstructed(n, qcLeaf)
  }

  /// consensus.rs:1993.
  pure def can_vote_1(n: NodeState, view: int): bool =
    if (not(n.proto.proposals.has(view)) or not(n.proto.statesVerified.has(view))) false
    else {
      val votedLeaf = n.proto.proposals.get(view)
      and {
        view > n.proto.timeoutView,
        not(n.proto.voted1.contains(votedLeaf)),
        n.proto.vidShares.contains(view),
        n.proto.statesVerified.get(view) == votedLeaf,
        parent_chain_ok(n, votedLeaf),
      }
    }

  /// consensus.rs:2171 — Cert1, proposal and reconstructed block all present
  /// and agreeing.
  pure def cert1_ready(n: NodeState, view: int): bool =
    if (not(n.proto.certs.has(view)) or not(n.proto.proposals.has(view))) false
    else {
      val certifiedLeaf = n.proto.proposals.get(view)
      n.proto.certs.get(view) == certifiedLeaf and n.proto.blocksReconstructed.contains(view)
    }

  /// consensus.rs:2216 — the lock only ever moves forward.
  pure def lock_advances(n: NodeState, view: int): bool = locked_view(n) < view

  /// consensus.rs:2230 — a view already certified in phase 2 or decided is not
  /// voted on again.
  pure def can_vote_2_now(n: NodeState, view: int): bool = and {
    not(n.proto.certs2.has(view)),
    not(n.proto.decidedViews.contains(view)),
    view > decide_floor(n),
  }

  pure def can_vote_2_and_update_lock(n: NodeState, view: int): bool =
    if (not(cert1_ready(n, view))) false
    else and {
      not(n.proto.voted2.contains(n.proto.proposals.get(view))),
      lock_advances(n, view) or can_vote_2_now(n, view),
    }

  /// consensus.rs:1744 — a Cert2 matching the stored proposal, with the view's
  /// Cert1 in hand, decides the view.
  pure def can_decide(n: NodeState, view: int): bool = and {
    view > decide_floor(n),
    not(n.proto.decidedViews.contains(view)),
    n.proto.certs2.getOrElse(view, -1) == n.proto.proposals.getOrElse(view, -2),
    n.proto.certs.has(view),
  }

  pure def can_propose(n: NodeState, view: int): bool = and {
    view > n.proto.timeoutView,
    not(n.proto.proposedViews.contains(view)),
    match propose_parent(n, view) {
      | None => false
      | Some(parent) => n.proto.proposals.getOrElse(view_of(parent), -1) == parent
    },
  }

  pure def can_store(n: NodeState, view: int): bool =
    n.proto.proposals.has(view) and not(n.storage.storedViews.contains(view))

  /// `PersistHighQc` is emitted with the lock, so a confirmation is always for
  /// a lock this node held — but the lock may have moved on before storage
  /// reports back, hence `>=` rather than `==`.
  pure def can_store_high_qc(n: NodeState, view: int): bool = and {
    locked_view(n) >= view,
    view > 0,
    not(high_qc_persisted(n, view)),
  }

  // ----------------------------------------------------------- updates ------

  pure def apply_receive_proposal(n: NodeState, leaf: Leaf, parent: Leaf): NodeState =
    n.with_proto({ ...n.proto,
      proposals: n.proto.proposals.put(view_of(leaf), leaf),
      parents: n.proto.parents.put(leaf, parent),
      vidShares: n.proto.vidShares.union(Set(view_of(leaf))),
    })

  pure def apply_reject_unsafe(n: NodeState, leaf: Leaf): NodeState =
    n.with_history({ ...n.history,
      rejectedUnsafe: n.history.rejectedUnsafe.union(Set(leaf)),
    })

  pure def apply_receive_cert1(n: NodeState, leaf: Leaf): NodeState =
    n.with_proto({ ...n.proto, certs: n.proto.certs.put(view_of(leaf), leaf) })

  pure def apply_receive_cert2(n: NodeState, leaf: Leaf): NodeState =
    n.with_proto({ ...n.proto, certs2: n.proto.certs2.put(view_of(leaf), leaf) })

  pure def apply_reconstruct_block(n: NodeState, view: int): NodeState =
    n.with_proto({ ...n.proto,
      blocksReconstructed: n.proto.blocksReconstructed.union(Set(view)),
    })

  pure def apply_validate_state(n: NodeState, leaf: Leaf): NodeState =
    n.with_proto({ ...n.proto,
      statesVerified: n.proto.statesVerified.put(view_of(leaf), leaf),
    })

  pure def apply_timeout(n: NodeState, view: int): NodeState =
    n.with_proto({ ...n.proto, timeoutView: max(n.proto.timeoutView, view) })

  pure def apply_receive_timeout_cert(n: NodeState, view: int): NodeState = {
    val entered = view + 1
    n.with_proto({ ...n.proto,
      timeoutCerts: n.proto.timeoutCerts.union(Set(entered)),
      currentView: max(n.proto.currentView, entered),
    })
  }

  pure def apply_vote_1(n: NodeState, view: int): NodeState = {
    val votedLeaf = n.proto.proposals.get(view)
    val voted = n.with_proto({ ...n.proto, voted1: n.proto.voted1.union(Set(votedLeaf)) })
    if (n.storage.storedViews.contains(view))
      voted.with_history({ ...voted.history,
        sentVote1: voted.history.sentVote1.union(Set(votedLeaf)),
      })
    else
      voted.with_storage({ ...voted.storage,
        pendingVote1: voted.storage.pendingVote1.put(view, votedLeaf),
      })
  }

  /// consensus.rs:1913 — a parked phase-1 vote goes out once its view's facts
  /// are stored, unless the view timed out meanwhile.
  pure def release_pending_vote1(n: NodeState, view: int): NodeState =
    if (not(n.storage.pendingVote1.has(view)) or not(n.storage.storedViews.contains(view))) n
    else {
      val parkedLeaf = n.storage.pendingVote1.get(view)
      val popped = n.with_storage({ ...n.storage,
        pendingVote1: n.storage.pendingVote1.mapRemove(view),
      })
      if (view <= n.proto.timeoutView)
        popped.with_history({ ...popped.history,
          droppedVote1: popped.history.droppedVote1.union(Set(view)),
        })
      else
        popped.with_history({ ...popped.history,
          sentVote1: popped.history.sentVote1.union(Set(parkedLeaf)),
        })
    }

  /// consensus.rs:1936 — every parked phase-2 vote whose gates now pass. A view
  /// that certified in phase 2 meanwhile drops its vote instead of sending it.
  pure def release_pending_vote2(n: NodeState): NodeState = {
    val ready = n.storage.pendingVote2.keys().filter(v => or {
      n.proto.certs2.has(v),
      vote2_persisted(n, v) and high_qc_persisted(n, n.storage.pendingVote2.get(v).required),
    })
    val sent =
      ready.filter(v => not(n.proto.certs2.has(v)))
           .map(v => n.storage.pendingVote2.get(v).leaf)
    n.with_storage({ ...n.storage, pendingVote2: n.storage.pendingVote2.mapRemoveAll(ready) })
     .with_history({ ...n.history, sentVote2: n.history.sentVote2.union(sent) })
  }

  pure def apply_vote_2_and_update_lock(n: NodeState, view: int): NodeState = {
    val votedLeaf = n.proto.proposals.get(view)
    val locked =
      if (lock_advances(n, view))
        n.with_proto({ ...n.proto,
          lockedCert: Some(votedLeaf),
          currentView: max(n.proto.currentView, view + 1),
        }).with_history({ ...n.history, lockAdvances: n.history.lockAdvances + 1 })
      else n
    if (not(can_vote_2_now(locked, view))) locked
    else {
      val required = locked_view(locked)
      val voted = locked.with_proto({ ...locked.proto,
        voted2: locked.proto.voted2.union(Set(votedLeaf)),
      })
      if (vote2_persisted(voted, view) and high_qc_persisted(voted, required))
        voted.with_history({ ...voted.history,
          sentVote2: voted.history.sentVote2.union(Set(votedLeaf)),
        })
      else
        voted.with_storage({ ...voted.storage,
          pendingVote2: voted.storage.pendingVote2.put(view, {
            leaf: votedLeaf,
            required: required,
          }),
        })
    }
  }

  /// consensus.rs:1796 — walk the justify_qc chain below `view`, collecting
  /// every ancestor whose stored proposal matches the child's justify_qc. A
  /// missing or mismatching ancestor is a gap and stops the walk.
  pure def decide_chain(n: NodeState, view: int): Set[Leaf] = {
    val floor = decide_floor(n)
    val start = n.proto.proposals.get(view)
    val walked = WALK_STEPS.foldl(
      { cursor: parent_of(n, start), leaves: Set(start), stopped: false },
      (st, _) =>
        if (st.stopped) st
        else {
          val parentView = view_of(st.cursor)
          if (and {
            parentView > floor,
            not(n.proto.decidedViews.contains(parentView)),
            n.proto.proposals.getOrElse(parentView, -1) == st.cursor,
          })
            { cursor: parent_of(n, st.cursor),
              leaves: st.leaves.union(Set(st.cursor)),
              stopped: false }
          else { ...st, stopped: true }
        }
    )
    walked.leaves
  }

  pure def apply_decide(n: NodeState, view: int): NodeState = {
    val leaves = decide_chain(n, view)
    n.with_proto({ ...n.proto,
      decidedViews: n.proto.decidedViews.union(leaves.map(l => view_of(l))),
      lastDecidedView: max(n.proto.lastDecidedView, view),
    }).with_history({ ...n.history,
      decidedLeaves: n.history.decidedLeaves.union(leaves),
      decideTriggers: n.history.decideTriggers.union(Set(view)),
      lateDecides:
        if (view <= n.proto.lastDecidedView) n.history.lateDecides.union(Set(view))
        else n.history.lateDecides,
      maxDecideBatch: max(n.history.maxDecideBatch, leaves.size()),
    })
  }

  pure def apply_propose(n: NodeState, view: int): NodeState =
    n.with_proto({ ...n.proto, proposedViews: n.proto.proposedViews.union(Set(view)) })
     .with_history({ ...n.history,
       proposedAfterTimeout:
         if (n.proto.timeoutCerts.contains(view))
           n.history.proposedAfterTimeout.union(Set(view))
         else n.history.proposedAfterTimeout,
     })

  /// consensus.rs:1883 — a storage confirmation for a view, which re-checks
  /// both parked votes.
  pure def apply_store(n: NodeState, view: int): NodeState = {
    val stored = n.with_storage({ ...n.storage,
      storedViews: n.storage.storedViews.union(Set(view)),
    })
    release_pending_vote2(release_pending_vote1(stored, view))
  }

  /// consensus.rs:1898 — a newly persisted lock can unblock phase-2 votes
  /// across many views, so all parked ones are re-checked.
  pure def apply_store_high_qc(n: NodeState, view: int): NodeState = {
    val stored = n.with_storage({ ...n.storage,
      storedHighQc:
        match n.storage.storedHighQc {
          | Some(cur) => Some(max(cur, view))
          | None => Some(view)
        },
    })
    release_pending_vote2(stored)
  }

  // ----------------------------------------------------------- actions ------

  /// Commit the next state, recording the pre-transition watermarks the
  /// monotonicity invariants compare against.
  action commit(updated: NodeState): bool = node' = updated.with_history({
    ...updated.history,
    prevLockedView: locked_view(node),
    prevLastDecidedView: node.proto.lastDecidedView,
  })

  /// A validated proposal (paired with this node's VID share) passes the lock
  /// rule and is stored. Code: `handle_proposal_with_vid_share`
  /// (consensus.rs:1048), logged where it returns `Protocol::Continue`.
  action receive_proposal(leaf: Leaf, parent: Leaf): bool = all {
    can_receive_proposal(node, leaf, parent),
    commit(apply_receive_proposal(node, leaf, parent)),
  }

  /// A validated proposal is discarded because it fails `is_safe`
  /// (consensus.rs:1081).
  action reject_unsafe_proposal(leaf: Leaf, parent: Leaf): bool = all {
    can_reject_unsafe(node, leaf, parent),
    commit(apply_reject_unsafe(node, leaf)),
  }

  /// `ConsensusInput::Certificate1` — a phase-1 quorum certificate for `leaf`.
  action receive_cert1(leaf: Leaf): bool = all {
    can_receive_cert1(node, leaf),
    commit(apply_receive_cert1(node, leaf)),
  }

  /// `ConsensusInput::Certificate2` — a phase-2 quorum certificate for `leaf`.
  action receive_cert2(leaf: Leaf): bool = all {
    can_receive_cert2(node, leaf),
    commit(apply_receive_cert2(node, leaf)),
  }

  /// `ConsensusInput::BlockReconstructed` — the block payload for `view` is
  /// available locally.
  action block_reconstructed(view: int): bool = all {
    can_reconstruct_block(node, view),
    commit(apply_reconstruct_block(node, view)),
  }

  /// `ConsensusInput::StateValidated` — the state transition for `leaf` checks
  /// out.
  action state_validated(leaf: Leaf): bool = all {
    can_validate_state(node, leaf),
    commit(apply_validate_state(node, leaf)),
  }

  /// `ConsensusInput::Timeout` — the view timer fired; bar the view and send a
  /// timeout vote (consensus.rs:1320).
  action timeout(view: int): bool = all {
    can_timeout(node, view),
    commit(apply_timeout(node, view)),
  }

  /// `ConsensusInput::TimeoutCertificate` for the timed-out `view`; the node
  /// enters `view + 1` (consensus.rs:1431).
  action timeout_certificate(view: int): bool = all {
    can_receive_timeout_cert(node, view),
    commit(apply_receive_timeout_cert(node, view)),
  }

  /// `maybe_vote_1` (consensus.rs:1993): cast the phase-1 vote, sending it now
  /// if the view's storage facts are already confirmed and parking it otherwise.
  action vote_1(view: int): bool = all {
    can_vote_1(node, view),
    commit(apply_vote_1(node, view)),
  }

  /// `maybe_vote_2_and_update_lock` (consensus.rs:2159): move the lock to this
  /// view's Cert1 and cast the phase-2 vote, which waits on the new lock being
  /// persisted.
  action vote_2_and_update_lock(view: int): bool = all {
    can_vote_2_and_update_lock(node, view),
    commit(apply_vote_2_and_update_lock(node, view)),
  }

  /// `maybe_decide` (consensus.rs:1740): decide the view and every ancestor the
  /// justify_qc chain reaches.
  action decide(view: int): bool = all {
    can_decide(node, view),
    commit(apply_decide(node, view)),
  }

  /// `maybe_propose` (consensus.rs:1548): propose on the previous view's Cert1,
  /// or on the lock when a timeout certificate carried us here.
  action propose(view: int): bool = all {
    can_propose(node, view),
    commit(apply_propose(node, view)),
  }

  /// `ConsensusInput::Stored` — the view's proposal, VID share and Vote action
  /// are durably recorded, releasing any vote parked on them.
  action stored(view: int): bool = all {
    can_store(node, view),
    commit(apply_store(node, view)),
  }

  /// `StorageOutput::HighQc` — the lock is durably recorded, releasing any
  /// phase-2 vote waiting on it.
  action stored_high_qc(view: int): bool = all {
    can_store_high_qc(node, view),
    commit(apply_store_high_qc(node, view)),
  }

  action init: bool = node' = {
    proto: {
      proposals: Map(0 -> GENESIS),
      parents: Map(),
      certs: Map(0 -> GENESIS),
      certs2: Map(),
      timeoutCerts: Set(),
      lockedCert: None,
      decidedViews: Set(),
      lastDecidedView: 0,
      voted1: Set(),
      voted2: Set(),
      blocksReconstructed: Set(),
      statesVerified: Map(),
      vidShares: Set(),
      currentView: 1,
      timeoutView: 0,
      proposedViews: Set(),
    },
    storage: {
      storedViews: Set(),
      storedHighQc: None,
      pendingVote1: Map(),
      pendingVote2: Map(),
    },
    history: {
      decidedLeaves: Set(),
      sentVote1: Set(),
      sentVote2: Set(),
      decideTriggers: Set(),
      lateDecides: Set(),
      droppedVote1: Set(),
      proposedAfterTimeout: Set(),
      rejectedUnsafe: Set(),
      maxDecideBatch: 0,
      lockAdvances: 0,
      prevLockedView: 0,
      prevLastDecidedView: 0,
    },
  }

  /// Three picks per step, named after the arguments the instrumentation logs:
  /// the leaf under discussion, the leaf a proposal's justify_qc certifies, and
  /// the view a view-indexed rule acts in.
  ///
  /// `parent` is currently NOT logged: see doc/quint-oracle-replay-blocker.md.
  /// It is resolved by replay search instead, which later steps discriminate.
  action step: bool = {
    nondet leaf = LEAVES.oneOf()
    nondet parent = PARENT_LEAVES.oneOf()
    nondet view = VIEWS.oneOf()
    any {
      receive_proposal(leaf, parent),
      reject_unsafe_proposal(leaf, parent),
      receive_cert1(leaf),
      receive_cert2(leaf),
      state_validated(leaf),
      block_reconstructed(view),
      timeout(view),
      timeout_certificate(view),
      vote_1(view),
      vote_2_and_update_lock(view),
      decide(view),
      propose(view),
      stored(view),
      stored_high_qc(view),
    }
  }

  // --------------------------------------------------------- invariants -----

  /// Decided leaves form a single chain: two conflicting proposals are never
  /// both decided.
  val no_conflicting_decides: bool =
    node.history.decidedLeaves.forall(a =>
      node.history.decidedLeaves.forall(b => view_of(a) == view_of(b) implies a == b))

  /// A view decides only when its Cert1 exists and its Cert2 matches the stored
  /// proposal's commitment.
  val decide_requires_matching_certs: bool =
    node.history.decideTriggers.forall(v => and {
      node.proto.certs.has(v),
      node.proto.certs2.getOrElse(v, -1) == node.proto.proposals.getOrElse(v, -2),
    })

  /// At most one phase-1 vote is released per view.
  val one_vote1_per_view: bool =
    node.history.sentVote1.forall(a =>
      node.history.sentVote1.forall(b => view_of(a) == view_of(b) implies a == b))

  /// At most one phase-2 vote is released per view.
  val one_vote2_per_view: bool =
    node.history.sentVote2.forall(a =>
      node.history.sentVote2.forall(b => view_of(a) == view_of(b) implies a == b))

  /// The locked certificate's view is non-decreasing.
  val locked_view_non_decreasing: bool = locked_view(node) >= node.history.prevLockedView

  /// The lock is always a certificate this node holds.
  val lock_is_certified: bool =
    match node.proto.lockedCert {
      | None => true
      | Some(leaf) => node.proto.certs.getOrElse(view_of(leaf), -1) == leaf
    }

  /// `last_decided_view` is non-decreasing: a gap-fill decide of an older view
  /// never moves the watermark backward.
  val last_decided_view_non_decreasing: bool =
    node.proto.lastDecidedView >= node.history.prevLastDecidedView

  /// A phase-2 vote is released only after the lock it was cast under is
  /// persisted.
  val vote2_after_high_qc_persisted: bool =
    node.history.sentVote2.forall(leaf => high_qc_persisted(node, view_of(leaf)))

  /// Every phase-2 vote is for the leaf its view's Cert1 certifies.
  val vote2_matches_cert1: bool =
    node.proto.voted2.forall(leaf => node.proto.certs.getOrElse(view_of(leaf), -1) == leaf)

  // -------------------------------------------------------- observations ----

  /// A validated proposal passed the lock rule and was stored.
  val proposal_accepted: bool = node.proto.proposals.keys().exclude(Set(0)) != Set()

  /// A proposal was rejected by the safety-or-liveness lock rule.
  val unsafe_proposal_rejected: bool = node.history.rejectedUnsafe != Set()

  /// A phase-1 vote was released.
  val vote1_sent: bool = node.history.sentVote1 != Set()

  /// A phase-1 vote was held back waiting on its storage confirmations.
  val vote1_deferred_on_storage: bool = node.storage.pendingVote1.keys() != Set()

  /// A parked phase-1 vote was lost to its view timing out. Once the view is at
  /// or below `timeoutView` the vote can no longer be released — `release_vote1`
  /// drops it and `maybe_vote_1` will not re-cast it — so the pending case is
  /// the same behavior as the recorded drop, one step earlier.
  val vote1_lost_to_timeout: bool = or {
    node.history.droppedVote1 != Set(),
    node.storage.pendingVote1.keys().exists(v => v <= node.proto.timeoutView),
  }

  /// The node took a lock on a Cert1.
  val lock_taken: bool = node.proto.lockedCert != None

  /// The lock moved forward more than once.
  val lock_advanced_repeatedly: bool = node.history.lockAdvances >= 2

  /// A phase-2 vote was released.
  val vote2_sent: bool = node.history.sentVote2 != Set()

  /// A phase-2 vote was held back waiting on the new lock to be persisted.
  val vote2_deferred_on_high_qc: bool = node.storage.pendingVote2.keys() != Set()

  /// A view was decided.
  val view_decided: bool = node.proto.decidedViews != Set()

  /// Two or more views are decided.
  val two_views_decided: bool = node.proto.decidedViews.size() >= 2

  /// One decide emitted a chain of more than one leaf (the ancestor gap-fill
  /// walk).
  val chain_decided_in_one_step: bool = node.history.maxDecideBatch > 1

  /// A view older than the watermark decided late, leaving the watermark put.
  val older_view_decided_late: bool = node.history.lateDecides != Set()

  /// A Cert2 arrived for a view whose Cert1 the node does not hold yet.
  val cert2_before_cert1: bool =
    node.proto.certs2.keys().exists(v => not(node.proto.certs.has(v)))

  /// A view timer fired and the node sent a timeout vote.
  val timeout_vote_sent: bool = node.proto.timeoutView > 0

  /// A timeout certificate carried the node into the next view.
  val view_advanced_by_timeout_certificate: bool = node.proto.timeoutCerts != Set()

  /// The node proposed.
  val proposal_made: bool = node.proto.proposedViews != Set()

  /// The leader proposed in a view a timeout certificate carried it into
  /// (liveness after a failed view).
  val proposed_after_timeout_certificate: bool = node.history.proposedAfterTimeout != Set()
}
