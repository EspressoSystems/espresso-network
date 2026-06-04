//! Optional fine-grained tracing of the leader's per-view duty.
//!
//! Production builds register `None` and pay one branch (~ns) per event site.
//! The bench binary registers a real tracer and writes the captured stream to
//! disk for offline timeline reconstruction.
//!
//! Events are wall-clock unix-epoch ns, matching `MetricsCollector::now_ns()`
//! in the bench so a downstream tool can join the streams on `view + ts_ns`.

use std::sync::Arc;

use time::OffsetDateTime;

pub type LeaderTracerHandle = Arc<dyn LeaderTracer>;

pub trait LeaderTracer: Send + Sync + 'static {
    /// `view` is the raw u64 view number (matches `MetricsCollector`'s key type).
    /// Call sites pass `*ViewNumber` so the macro stays one-liner.
    fn record(&self, view: u64, event: LeaderEvent, ts_ns: i128);
}

/// Closed event enum spanning the leader's V-1→V duty.
///
/// Variants are grouped by phase but assigned to a single flat enum so call
/// sites stay short and a downstream tool can match on a string name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderEvent {
    // Phase 0 - V-1 events that trigger V duty.
    ProposalValidatedVMinus1,
    RequestBlockHeaderQueued,
    HeaderCreatedApplied,
    BlockBuiltApplied,
    RequestVidDisperseQueued,

    // Phase 2 - ns_disperse on the background task.
    NsDisperseStart,
    NsDisperseEnd,

    // Phase 3 - post-disperse outbox push on the consensus task.
    BlockPushSigned,
    BlockPushQueued,
    ShareSignLoopStart,
    ShareSignLoopEnd,
    VidSharesQueued,

    // Phase 4 - outbox drain (network I/O).
    BlockPushUnicastStart,
    BlockPushUnicastEnd,
    VidSharesUnicastStart,
    VidSharesUnicastEnd,

    // Phase 5 - replica-of-V-1 work running in parallel.
    Vote1VMinus1Arrived,
    BlockPushVMinus1Received,
    VerifyBlockVMinus1Start,
    VerifyBlockVMinus1End,
    ThresholdShareReachedVMinus1,
    RecoverVMinus1Start,
    /// Emitted right after `AvidmGf2Scheme::recover` returns the decoded
    /// payload bytes, BEFORE `from_bytes` and `transaction_commitments`.
    /// The decode→end interval is the (single-threaded) `transaction_commitments`
    /// Keccak256 of the payload — split out so the parallel AvidM work and the
    /// serial post-processing can be measured separately.
    RecoverVMinus1DecodeEnd,
    RecoverVMinus1End,

    // Phase 6 - cert1[V-1] formation gates Phase 7.
    Cert1VMinus1ThresholdReached,
    Cert1VMinus1Aggregated,
    Cert1VMinus1InputDispatched,
    Vote2VMinus1Signed,
    Vote2VMinus1Queued,
    // Phase 6b - cert2 formation (the QC2 round) and finality.
    Cert2VMinus1InputDispatched,
    LeafDecided,

    // Phase 7 - build + sign V's proposal.
    MaybeProposeEntered,
    Leaf2CommitComputed,
    ProposalSigned,
    ProposalQueued,

    // Phase 8 - outbox drain II.
    ProposalBroadcastStart,
    ProposalBroadcastEnd,
    Vote2VMinus1BroadcastStart,
    Vote2VMinus1BroadcastEnd,
    Cert1VMinus1BroadcastStart,
    Cert1VMinus1BroadcastEnd,
}

impl LeaderEvent {
    /// Stable string name for CSV emission.
    pub fn name(self) -> &'static str {
        use LeaderEvent::*;
        match self {
            ProposalValidatedVMinus1 => "proposal_validated_v_minus_1",
            RequestBlockHeaderQueued => "request_block_header_queued",
            HeaderCreatedApplied => "header_created_applied",
            BlockBuiltApplied => "block_built_applied",
            RequestVidDisperseQueued => "request_vid_disperse_queued",
            NsDisperseStart => "ns_disperse_start",
            NsDisperseEnd => "ns_disperse_end",
            BlockPushSigned => "block_push_signed",
            BlockPushQueued => "block_push_queued",
            ShareSignLoopStart => "share_sign_loop_start",
            ShareSignLoopEnd => "share_sign_loop_end",
            VidSharesQueued => "vid_shares_queued",
            BlockPushUnicastStart => "block_push_unicast_start",
            BlockPushUnicastEnd => "block_push_unicast_end",
            VidSharesUnicastStart => "vid_shares_unicast_start",
            VidSharesUnicastEnd => "vid_shares_unicast_end",
            Vote1VMinus1Arrived => "vote1_v_minus_1_arrived",
            BlockPushVMinus1Received => "block_push_v_minus_1_received",
            VerifyBlockVMinus1Start => "verify_block_v_minus_1_start",
            VerifyBlockVMinus1End => "verify_block_v_minus_1_end",
            ThresholdShareReachedVMinus1 => "threshold_share_reached_v_minus_1",
            RecoverVMinus1Start => "recover_v_minus_1_start",
            RecoverVMinus1DecodeEnd => "recover_v_minus_1_decode_end",
            RecoverVMinus1End => "recover_v_minus_1_end",
            Cert1VMinus1ThresholdReached => "cert1_v_minus_1_threshold_reached",
            Cert1VMinus1Aggregated => "cert1_v_minus_1_aggregated",
            Cert1VMinus1InputDispatched => "cert1_v_minus_1_input_dispatched",
            Vote2VMinus1Signed => "vote2_v_minus_1_signed",
            Vote2VMinus1Queued => "vote2_v_minus_1_queued",
            Cert2VMinus1InputDispatched => "cert2_v_minus_1_input_dispatched",
            LeafDecided => "leaf_decided",
            MaybeProposeEntered => "maybe_propose_entered",
            Leaf2CommitComputed => "leaf2_commit_computed",
            ProposalSigned => "proposal_signed",
            ProposalQueued => "proposal_queued",
            ProposalBroadcastStart => "proposal_broadcast_start",
            ProposalBroadcastEnd => "proposal_broadcast_end",
            Vote2VMinus1BroadcastStart => "vote2_v_minus_1_broadcast_start",
            Vote2VMinus1BroadcastEnd => "vote2_v_minus_1_broadcast_end",
            Cert1VMinus1BroadcastStart => "cert1_v_minus_1_broadcast_start",
            Cert1VMinus1BroadcastEnd => "cert1_v_minus_1_broadcast_end",
        }
    }
}

/// Wall-clock unix-epoch ns. Same source as the bench's `MetricsCollector`.
#[inline(always)]
pub fn now_ns() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos()
}

/// Emit an event through an optional tracer with a single `is_some` check.
/// Internal helper: convert anything ViewNumber-shaped (`ViewNumber`, `&ViewNumber`, `u64`) into u64.
pub trait AsViewU64 {
    fn as_view_u64(&self) -> u64;
}

impl AsViewU64 for hotshot_types::data::ViewNumber {
    fn as_view_u64(&self) -> u64 {
        **self
    }
}

impl AsViewU64 for u64 {
    fn as_view_u64(&self) -> u64 {
        *self
    }
}

#[macro_export]
macro_rules! trace_leader_event {
    ($tracer:expr, $view:expr, $event:expr) => {
        if let ::core::option::Option::Some(ref t) = $tracer {
            let v = $crate::leader_trace::AsViewU64::as_view_u64(&$view);
            t.record(v, $event, $crate::leader_trace::now_ns());
        }
    };
}
