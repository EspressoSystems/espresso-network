//! Optional per-view tracing of the leader's duty.
//!
//! With no tracer registered each event site costs one `Option` check. Timestamps are
//! wall-clock unix-epoch ns, the same clock as the bench's `MetricsCollector`, so the streams join on `view + ts_ns`.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::Path,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use parking_lot::Mutex;
use time::OffsetDateTime;
use tracing::warn;

pub type LeaderTracerHandle = Arc<dyn LeaderTracer>;

pub trait LeaderTracer: Send + Sync + 'static {
    fn record(&self, view: u64, event: LeaderEvent, ts_ns: i128);
}

/// Per-view events in rough order. Most fire on every node; the proposal and block builder events
/// fire on the leader of the view only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderEvent {
    // V-1 events that trigger V duty.
    ProposalValidatedVMinus1,
    RequestBlockHeaderQueued,

    // Block builder, per request.
    /// The block builder accepts a request and spawns its task, before the empty-block delay.
    BlockRequestStarted,
    /// The block builder task is past the empty-block delay, before it builds the payload.
    BlockPayloadStart,
    /// The block builder starts `block::block_commitments`: payload bytes, then the VID encode in
    /// parallel with transaction hashes and builder commitment. Built path only.
    BlockCommitmentsStart,
    /// `block_commitments` returned, `Ok` or not. Same span as `block_commitments_duration`.
    BlockCommitmentsEnd,

    // Coordinator applies the header and block.
    HeaderCreatedApplied,
    BlockBuiltApplied,

    // Disperse the block and unicast the shares. `NsDisperse*` when the disperser encodes
    // (`Dispersal::Encode`), `VidFragments*` when the block builder did (`Dispersal::Built`).
    /// The disperser starts encoding the block. Encode path only.
    NsDisperseStart,
    /// The last bucket is encoded and its fragments assembled. Encode path only.
    NsDisperseEnd,
    /// The disperser starts on the block builder's shares: sign, bucket, assemble. Built path only.
    VidFragmentsStart,
    /// The last bucket's fragments are assembled. Built path only.
    VidFragmentsEnd,
    VidSharesUnicastStart,
    VidSharesUnicastEnd,

    // Replica work for V-1, in parallel.
    Vote1VMinus1Arrived,
    ThresholdShareReachedVMinus1,
    /// Stamped on every attempt; with the first stamp kept, the recover span includes failed
    /// attempts and the retry.
    RecoverVMinus1Start,
    /// Erasure decoding done; the rest until `RecoverVMinus1End` is the
    /// single-threaded `from_bytes` and transaction commitments.
    RecoverVMinus1DecodeEnd,
    RecoverVMinus1End,

    // cert1[V-1] formation; gates V's proposal.
    /// Consensus first stores cert1 of V-1, however it was formed or received.
    Cert1VMinus1Obtained,
    Vote2VMinus1Queued,
    // cert2 formation and finality.
    /// Consensus first stores cert2 of V-1, however it was formed or received.
    Cert2VMinus1Obtained,
    LeafDecided,

    // Build and sign V's proposal.
    /// The proposal is assembled, before its commit is computed and signed.
    ProposalAssembled,
    ProposalQueued,

    // Network sends of V's proposal and V-1 votes and certs.
    ProposalBroadcastStart,
    ProposalBroadcastEnd,
    Vote2VMinus1BroadcastStart,
    Vote2VMinus1BroadcastEnd,
    Cert1VMinus1BroadcastStart,
    Cert1VMinus1BroadcastEnd,
    Vote1VMinus1BroadcastStart,
    Vote1VMinus1BroadcastEnd,
}

impl LeaderEvent {
    /// CSV event name. Downstream tools match on it; do not rename.
    pub fn name(self) -> &'static str {
        use LeaderEvent::*;
        match self {
            ProposalValidatedVMinus1 => "proposal_validated_v_minus_1",
            RequestBlockHeaderQueued => "request_block_header_queued",
            BlockRequestStarted => "block_request_started",
            BlockPayloadStart => "block_payload_start",
            BlockCommitmentsStart => "block_commitments_start",
            BlockCommitmentsEnd => "block_commitments_end",
            HeaderCreatedApplied => "header_created_applied",
            BlockBuiltApplied => "block_built_applied",
            NsDisperseStart => "ns_disperse_start",
            NsDisperseEnd => "ns_disperse_end",
            VidFragmentsStart => "vid_fragments_start",
            VidFragmentsEnd => "vid_fragments_end",
            VidSharesUnicastStart => "vid_shares_unicast_start",
            VidSharesUnicastEnd => "vid_shares_unicast_end",
            Vote1VMinus1Arrived => "vote1_v_minus_1_arrived",
            ThresholdShareReachedVMinus1 => "threshold_share_reached_v_minus_1",
            RecoverVMinus1Start => "recover_v_minus_1_start",
            RecoverVMinus1DecodeEnd => "recover_v_minus_1_decode_end",
            RecoverVMinus1End => "recover_v_minus_1_end",
            Cert1VMinus1Obtained => "cert1_v_minus_1_obtained",
            Vote2VMinus1Queued => "vote2_v_minus_1_queued",
            Cert2VMinus1Obtained => "cert2_v_minus_1_obtained",
            LeafDecided => "leaf_decided",
            ProposalAssembled => "proposal_assembled",
            ProposalQueued => "proposal_queued",
            ProposalBroadcastStart => "proposal_broadcast_start",
            ProposalBroadcastEnd => "proposal_broadcast_end",
            Vote2VMinus1BroadcastStart => "vote2_v_minus_1_broadcast_start",
            Vote2VMinus1BroadcastEnd => "vote2_v_minus_1_broadcast_end",
            Cert1VMinus1BroadcastStart => "cert1_v_minus_1_broadcast_start",
            Cert1VMinus1BroadcastEnd => "cert1_v_minus_1_broadcast_end",
            Vote1VMinus1BroadcastStart => "vote1_v_minus_1_broadcast_start",
            Vote1VMinus1BroadcastEnd => "vote1_v_minus_1_broadcast_end",
        }
    }
}

#[inline(always)]
pub fn now_ns() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos()
}

const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

/// `LeaderTracer` appending `view,node_id,event,ts_ns` rows to a CSV file.
///
/// A background thread flushes every second, so a SIGKILL loses at most about 1s
/// of rows. Appends to an existing file. The first I/O error disables the tracer.
pub struct CsvLeaderTracer {
    node_id: u64,
    shared: Arc<Shared>,
}

struct Shared {
    writer: Mutex<BufWriter<File>>,
    failed: AtomicBool,
}

impl Shared {
    fn fail(&self, err: io::Error) {
        if !self.failed.swap(true, Ordering::Relaxed) {
            warn!(%err, "leader trace write failed, disabling leader trace");
        }
    }

    fn flush(&self) {
        if self.failed.load(Ordering::Relaxed) {
            return;
        }
        let result = self.writer.lock().flush();
        if let Err(err) = result {
            self.fail(err);
        }
    }
}

impl Drop for CsvLeaderTracer {
    fn drop(&mut self) {
        self.shared.flush();
    }
}

impl CsvLeaderTracer {
    pub fn new(node_id: u64, path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        if file.metadata()?.len() == 0 {
            file.write_all(b"view,node_id,event,ts_ns\n")?;
        }
        let shared = Arc::new(Shared {
            writer: Mutex::new(BufWriter::new(file)),
            failed: AtomicBool::new(false),
        });
        spawn_flusher(Arc::downgrade(&shared))?;
        Ok(Self { node_id, shared })
    }
}

fn spawn_flusher(shared: Weak<Shared>) -> io::Result<()> {
    thread::Builder::new()
        .name("leader-trace-flush".into())
        .spawn(move || {
            loop {
                thread::sleep(FLUSH_INTERVAL);
                let Some(shared) = shared.upgrade() else {
                    return;
                };
                shared.flush();
            }
        })?;
    Ok(())
}

impl LeaderTracer for CsvLeaderTracer {
    fn record(&self, view: u64, event: LeaderEvent, ts_ns: i128) {
        if self.shared.failed.load(Ordering::Relaxed) {
            return;
        }
        let row = format!("{view},{},{},{ts_ns}\n", self.node_id, event.name());
        let result = self.shared.writer.lock().write_all(row.as_bytes());
        if let Err(err) = result {
            self.shared.fail(err);
        }
    }
}

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

/// Records `$event` for `$view` when `$tracer` is `Some`.
#[macro_export]
macro_rules! trace_leader_event {
    ($tracer:expr, $view:expr, $event:expr) => {
        if let ::core::option::Option::Some(ref t) = $tracer {
            let v = $crate::leader_trace::AsViewU64::as_view_u64(&$view);
            t.record(v, $event, $crate::leader_trace::now_ns());
        }
    };
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    const HEADER: &str = "view,node_id,event,ts_ns\n";

    #[test]
    fn row_reaches_disk_without_flush_or_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/trace.csv");
        let tracer = CsvLeaderTracer::new(7, &path).unwrap();
        tracer.record(1, LeaderEvent::NsDisperseStart, 100);
        let expected = format!("{HEADER}1,7,ns_disperse_start,100\n");
        let deadline = Instant::now() + Duration::from_secs(10);
        while fs::read_to_string(&path).unwrap() != expected {
            assert!(Instant::now() < deadline, "row not flushed within 10s");
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn reopen_appends_without_second_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trace.csv");
        let first = CsvLeaderTracer::new(0, &path).unwrap();
        first.record(1, LeaderEvent::NsDisperseStart, 1);
        drop(first);
        let second = CsvLeaderTracer::new(0, &path).unwrap();
        second.record(2, LeaderEvent::VidSharesUnicastEnd, 2);
        drop(second);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!("{HEADER}1,0,ns_disperse_start,1\n2,0,vid_shares_unicast_end,2\n")
        );
    }
}
