use std::{collections::BTreeMap, fs::File, path::Path, time::Instant};

use hotshot_example_types::node_types::TestTypes;
use hotshot_new_protocol::consensus::{ConsensusInput, ConsensusOutput};
use hotshot_types::vote::HasViewNumber;
use serde::Serialize;

/// Per-view timing measurements.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ViewMetrics {
    pub view: u64,
    pub is_leader: bool,
    pub proposal_recv_ns: Option<u128>,
    pub state_validated_ns: Option<u128>,
    pub vote1_sent_ns: Option<u128>,
    pub cert1_formed_ns: Option<u128>,
    pub vote2_sent_ns: Option<u128>,
    pub cert2_formed_ns: Option<u128>,
    pub leaf_decided_ns: Option<u128>,
    pub proposal_sent_ns: Option<u128>,
    pub vid_disperse_ns: Option<u128>,
    pub block_reconstructed_ns: Option<u128>,
}

/// Collects per-view timing metrics.
pub struct MetricsCollector {
    start: Instant,
    views: BTreeMap<u64, ViewMetrics>,
    node_id: u64,
    current_view: u64,
}

impl MetricsCollector {
    pub fn new(node_id: u64) -> Self {
        Self {
            start: Instant::now(),
            views: BTreeMap::new(),
            node_id,
            current_view: 0,
        }
    }

    fn elapsed_ns(&self) -> u128 {
        self.start.elapsed().as_nanos()
    }

    fn view_mut(&mut self, view: u64) -> &mut ViewMetrics {
        self.views.entry(view).or_insert_with(|| ViewMetrics {
            view,
            ..Default::default()
        })
    }

    /// Record a consensus input event.
    pub fn on_input(&mut self, input: &ConsensusInput<TestTypes>) {
        let ts = self.elapsed_ns();
        match input {
            ConsensusInput::Proposal(p) => {
                let v = *p.view_number();
                self.view_mut(v).proposal_recv_ns = Some(ts);
            },
            ConsensusInput::StateValidated(resp) => {
                let v = *resp.view;
                self.view_mut(v).state_validated_ns = Some(ts);
            },
            ConsensusInput::Certificate1(cert) => {
                let v = *cert.view_number();
                self.view_mut(v).cert1_formed_ns = Some(ts);
            },
            ConsensusInput::Certificate2(cert) => {
                let v = *cert.view_number();
                self.view_mut(v).cert2_formed_ns = Some(ts);
            },
            ConsensusInput::VidDisperseCreated(view, _) => {
                let v = **view;
                self.view_mut(v).vid_disperse_ns = Some(ts);
            },
            ConsensusInput::BlockReconstructed(view, _) => {
                let v = **view;
                self.view_mut(v).block_reconstructed_ns = Some(ts);
            },
            _ => {},
        }
    }

    /// Record a consensus output event.
    pub fn on_output(&mut self, output: &ConsensusOutput<TestTypes>) {
        let ts = self.elapsed_ns();
        match output {
            ConsensusOutput::SendVote1(_) => {
                self.view_mut(self.current_view).vote1_sent_ns = Some(ts);
            },
            ConsensusOutput::SendVote2(_) => {
                self.view_mut(self.current_view).vote2_sent_ns = Some(ts);
            },
            ConsensusOutput::SendProposal(..) => {
                let v = self.current_view;
                let m = self.view_mut(v);
                m.proposal_sent_ns = Some(ts);
                m.is_leader = true;
            },
            ConsensusOutput::LeafDecided(_) => {
                self.view_mut(self.current_view).leaf_decided_ns = Some(ts);
            },
            ConsensusOutput::ViewChanged(view, _epoch) => {
                self.current_view = **view;
            },
            _ => {},
        }
    }

    /// Return the highest decided view number (based on `leaf_decided_ns` being set).
    pub fn max_decided_view(&self) -> u64 {
        self.views
            .values()
            .rev()
            .find(|m| m.leaf_decided_ns.is_some())
            .map_or(0, |m| m.view)
    }

    /// Write collected metrics to a CSV file.
    pub fn write_csv(&self, path: &Path) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut wtr = csv::Writer::from_writer(file);
        for m in self.views.values() {
            wtr.serialize(m)?;
        }
        wtr.flush()?;
        tracing::info!(node_id = self.node_id, path = %path.display(), "metrics written");
        Ok(())
    }
}
