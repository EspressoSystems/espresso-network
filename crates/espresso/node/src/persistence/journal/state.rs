//! In-memory consensus state shared by the write path and wal replay, and the wal record types
//! that mutate it. See the record-kind mapping table in the design doc for which trait method
//! produces each `Record` variant.

use std::collections::BTreeMap;

use espresso_types::{Header, Leaf2, Payload, SeqTypes};
use hotshot_new_protocol::message::Certificate2;
use hotshot_types::{
    data::{DaProposal2, QuorumProposalWrapper, VidDisperseShare},
    event::HotShotAction,
    message::Proposal,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        EncodeBytes as _,
        block_contents::{BlockHeader as _, BlockPayload as _},
    },
    vote::HasViewNumber,
};
use journal_lane::format;
use serde::{Deserialize, Serialize};

use crate::ViewNumber;

/// Gap-fill decides can finalize a state cert whose leaf view is behind the current anchor
/// (matches `sql.rs` `DECIDE_GAP_FILL_HORIZON`); pending certs older than this many views behind
/// the anchor are unreachable and dropped.
const DECIDE_GAP_FILL_HORIZON: u64 = 20;

/// Everything the wal stream can change. Cloned into a `Snapshot` record at every wal segment
/// roll and rebuilt by folding `apply` over records in LSN order after a restart.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub voted: Option<ViewNumber>,
    pub restart: Option<ViewNumber>,
    pub high_qc2: Option<QuorumCertificate2<SeqTypes>>,
    pub eqc: Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )>,
    pub next_epoch_qc: Option<NextEpochQuorumCertificate2<SeqTypes>>,
    pub upgrade: Option<UpgradeCertificate<SeqTypes>>,
    pub anchor: Option<(
        Leaf2,
        QuorumCertificate2<SeqTypes>,
        Option<NextEpochQuorumCertificate2<SeqTypes>>,
    )>,
    pub proposals: BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>,
    pub pending_state_certs: BTreeMap<ViewNumber, LightClientStateUpdateCertificateV2<SeqTypes>>,
    /// Set only on a query node. Recovery replays nothing older than the newest wal segment, so
    /// the query service's backlog has to live here to survive a restart.
    pub replay: Option<Replay>,
}

/// Decided data a query node has not yet replayed into its query service.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Replay {
    /// The newest replayed leaf's view and block height.
    pub cursor: Option<(ViewNumber, u64)>,
    pub leaves: BTreeMap<ViewNumber, ReplayLeaf>,
    pub cert2: BTreeMap<ViewNumber, Certificate2<SeqTypes>>,
}

impl Replay {
    fn is_replayed(&self, view: ViewNumber) -> bool {
        self.cursor.is_some_and(|(cursor, _)| view <= cursor)
    }
}

/// A decided leaf waiting to be replayed, with the state cert its decide finalized.
#[derive(Clone, Serialize, Deserialize)]
pub struct ReplayLeaf {
    pub leaf: Leaf2,
    pub cert: CertificatePair<SeqTypes>,
    pub state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
}

/// A state cert finalized by a `Leaf` record reaching its view, to be re-upserted into the side
/// `fs::Persistence` store (`insert_state_cert`).
pub struct Finalized {
    pub epoch: u64,
    pub cert: LightClientStateUpdateCertificateV2<SeqTypes>,
}

/// A decoded wal or data record.
#[derive(Clone)]
pub enum Record {
    /// Writer-only: opens every new wal segment, has no memory effect of its own.
    Snapshot(Box<State>),
    Action {
        view: ViewNumber,
        kind: HotShotAction,
    },
    HighQc2(QuorumCertificate2<SeqTypes>),
    Proposal(Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>),
    Leaf {
        leaf: Leaf2,
        qc: QuorumCertificate2<SeqTypes>,
        next_epoch_qc: Option<NextEpochQuorumCertificate2<SeqTypes>>,
    },
    Cert2 {
        view: ViewNumber,
        cert: Certificate2<SeqTypes>,
    },
    StateCert(LightClientStateUpdateCertificateV2<SeqTypes>),
    Upgrade(Option<UpgradeCertificate<SeqTypes>>),
    Eqc(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    ),
    NextEpochQc(NextEpochQuorumCertificate2<SeqTypes>),
    Vid(Proposal<SeqTypes, VidDisperseShare<SeqTypes>>),
    Da(Proposal<SeqTypes, DaProposal2<SeqTypes>>),
    /// The query service has ingested every decided leaf up to `view`, at block `height`.
    Processed {
        view: ViewNumber,
        height: u64,
    },
    /// A payload this node obtained for `view`. No longer written: a query node holds payloads
    /// in memory until their view is replayed. Still decoded so segments written before that
    /// recover.
    PendingPayload {
        view: ViewNumber,
        header: Header,
        payload: Payload,
    },
}

/// Tag identifying which `Record` variant a frame's payload decodes to. Discriminants are the
/// on-disk frame tags.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Snapshot = 1,
    Action = 2,
    HighQc2 = 3,
    Proposal = 4,
    Leaf = 5,
    Cert2 = 6,
    StateCert = 7,
    Upgrade = 8,
    Eqc = 9,
    NextEpochQc = 10,
    Vid = 11,
    Da = 12,
    Processed = 13,
    PendingPayload = 14,
}

// The lane layer recognizes snapshot frames by this tag.
const _: () = assert!(Kind::Snapshot as u8 == format::Kind::SNAPSHOT.0);

impl Kind {
    pub fn from_u8(b: u8) -> Option<Self> {
        Some(match b {
            1 => Self::Snapshot,
            2 => Self::Action,
            3 => Self::HighQc2,
            4 => Self::Proposal,
            5 => Self::Leaf,
            6 => Self::Cert2,
            7 => Self::StateCert,
            8 => Self::Upgrade,
            9 => Self::Eqc,
            10 => Self::NextEpochQc,
            11 => Self::Vid,
            12 => Self::Da,
            13 => Self::Processed,
            14 => Self::PendingPayload,
            _ => return None,
        })
    }

    /// `LaneConfig::known_kind` for the consensus journal lanes.
    pub fn is_known(b: u8) -> bool {
        Self::from_u8(b).is_some()
    }
}

impl From<Kind> for format::Kind {
    fn from(kind: Kind) -> Self {
        Self(kind as u8)
    }
}

impl Record {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Snapshot(_) => Kind::Snapshot,
            Self::Action { .. } => Kind::Action,
            Self::HighQc2(_) => Kind::HighQc2,
            Self::Proposal(_) => Kind::Proposal,
            Self::Leaf { .. } => Kind::Leaf,
            Self::Cert2 { .. } => Kind::Cert2,
            Self::StateCert(_) => Kind::StateCert,
            Self::Upgrade(_) => Kind::Upgrade,
            Self::Eqc(..) => Kind::Eqc,
            Self::NextEpochQc(_) => Kind::NextEpochQc,
            Self::Vid(_) => Kind::Vid,
            Self::Da(_) => Kind::Da,
            Self::Processed { .. } => Kind::Processed,
            Self::PendingPayload { .. } => Kind::PendingPayload,
        }
    }

    /// The frame header's `view` field. Not consensus-meaningful for `Snapshot`/`Upgrade`, which
    /// have no natural view; `0` there is only ever used for GC's max-view bound, which correctly
    /// treats it as a no-op.
    pub fn view(&self) -> u64 {
        match self {
            Self::Snapshot(_) | Self::Upgrade(_) => 0,
            Self::Action { view, .. } => view.u64(),
            Self::HighQc2(qc) => qc.view_number().u64(),
            Self::Proposal(p) => p.data.view_number().u64(),
            Self::Leaf { leaf, .. } => leaf.view_number().u64(),
            Self::Cert2 { view, .. } => view.u64(),
            Self::StateCert(c) => c.light_client_state.view_number,
            Self::Eqc(qc, _) => qc.view_number().u64(),
            Self::NextEpochQc(n) => n.view_number().u64(),
            Self::Vid(p) => p.data.view_number().u64(),
            Self::Da(p) => p.data.view_number().u64(),
            Self::Processed { view, .. } => view.u64(),
            Self::PendingPayload { view, .. } => view.u64(),
        }
    }

    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(match self {
            Self::Snapshot(s) => bincode::serialize(s)?,
            Self::Action { kind, .. } => bincode::serialize(kind)?,
            Self::HighQc2(qc) => bincode::serialize(qc)?,
            Self::Proposal(p) => bincode::serialize(p)?,
            Self::Leaf {
                leaf,
                qc,
                next_epoch_qc,
            } => bincode::serialize(&(leaf, qc, next_epoch_qc))?,
            Self::Cert2 { cert, .. } => bincode::serialize(cert)?,
            Self::StateCert(c) => bincode::serialize(c)?,
            Self::Upgrade(u) => bincode::serialize(u)?,
            Self::Eqc(qc, next) => bincode::serialize(&(qc, next))?,
            Self::NextEpochQc(n) => bincode::serialize(n)?,
            Self::Vid(p) => bincode::serialize(p)?,
            Self::Da(p) => bincode::serialize(p)?,
            Self::Processed { height, .. } => bincode::serialize(height)?,
            Self::PendingPayload {
                header, payload, ..
            } => bincode::serialize(&(header, payload.encode().as_ref()))?,
        })
    }

    /// `view` is the frame header's raw view, only needed to reconstruct the typed `view` field
    /// on variants that don't carry it in their own payload (`Action`, `Cert2`).
    pub fn decode(kind: Kind, view: u64, body: &[u8]) -> anyhow::Result<Self> {
        Ok(match kind {
            Kind::Snapshot => Self::Snapshot(bincode::deserialize(body)?),
            Kind::Action => Self::Action {
                view: ViewNumber::new(view),
                kind: bincode::deserialize(body)?,
            },
            Kind::HighQc2 => Self::HighQc2(bincode::deserialize(body)?),
            Kind::Proposal => Self::Proposal(bincode::deserialize(body)?),
            Kind::Leaf => {
                let (leaf, qc, next_epoch_qc) = bincode::deserialize(body)?;
                Self::Leaf {
                    leaf,
                    qc,
                    next_epoch_qc,
                }
            },
            Kind::Cert2 => Self::Cert2 {
                view: ViewNumber::new(view),
                cert: bincode::deserialize(body)?,
            },
            Kind::StateCert => Self::StateCert(bincode::deserialize(body)?),
            Kind::Upgrade => Self::Upgrade(bincode::deserialize(body)?),
            Kind::Eqc => {
                let (qc, next) = bincode::deserialize(body)?;
                Self::Eqc(qc, next)
            },
            Kind::NextEpochQc => Self::NextEpochQc(bincode::deserialize(body)?),
            Kind::Vid => Self::Vid(bincode::deserialize(body)?),
            Kind::Da => Self::Da(bincode::deserialize(body)?),
            Kind::Processed => Self::Processed {
                view: ViewNumber::new(view),
                height: bincode::deserialize(body)?,
            },
            Kind::PendingPayload => {
                let (header, payload) = decode_pending_payload(body)?;
                Self::PendingPayload {
                    view: ViewNumber::new(view),
                    header,
                    payload,
                }
            },
        })
    }
}

/// Body of a [`Record::PendingPayload`] frame. The payload is stored as its encoded bytes, which
/// only decode against the header's namespace table.
pub fn decode_pending_payload(body: &[u8]) -> anyhow::Result<(Header, Payload)> {
    let (header, payload) = bincode::deserialize::<(Header, Vec<u8>)>(body)?;
    let payload = Payload::from_bytes(&payload, header.metadata());
    Ok((header, payload))
}

impl State {
    /// Pure. Shared by the write path (under the state lock, before enqueueing) and replay.
    /// Returns a state cert finalized by a `Leaf` reaching its view, if any.
    pub fn apply(&mut self, rec: &Record) -> Option<Finalized> {
        match rec {
            Record::Snapshot(_)
            | Record::Vid(_)
            | Record::Da(_)
            | Record::PendingPayload { .. } => None,
            Record::Cert2 { view, cert } => {
                if let Some(replay) = &mut self.replay
                    && !replay.is_replayed(*view)
                {
                    replay.cert2.insert(*view, cert.clone());
                }
                None
            },
            Record::Processed { view, height } => {
                if let Some(replay) = &mut self.replay
                    && !replay.is_replayed(*view)
                {
                    replay.cursor = Some((*view, *height));
                    replay.leaves.retain(|v, _| v > view);
                    replay.cert2.retain(|v, _| v > view);
                }
                None
            },
            Record::Action { view, kind } => {
                match kind {
                    HotShotAction::Vote => {
                        self.voted = self.voted.max(Some(*view));
                        self.restart = self.restart.max(Some(*view + 1));
                    },
                    HotShotAction::Propose => {
                        self.voted = self.voted.max(Some(*view));
                    },
                    _ => {},
                }
                None
            },
            Record::HighQc2(qc) => {
                if self
                    .high_qc2
                    .as_ref()
                    .is_none_or(|cur| qc.view_number() > cur.view_number())
                {
                    self.high_qc2 = Some(qc.clone());
                }
                None
            },
            Record::Proposal(p) => {
                self.proposals.insert(p.data.view_number(), p.clone());
                None
            },
            Record::Leaf {
                leaf,
                qc,
                next_epoch_qc,
            } => {
                let leaf_view = leaf.view_number();
                if self
                    .anchor
                    .as_ref()
                    .is_none_or(|(a, ..)| leaf_view > a.view_number())
                {
                    self.anchor = Some((leaf.clone(), qc.clone(), next_epoch_qc.clone()));
                }
                if let Some(anchor_view) = self.anchor.as_ref().map(|(a, ..)| a.view_number()) {
                    self.proposals.retain(|v, _| *v > anchor_view);
                }
                // Finalize at the decided leaf's own view: a gap-fill decide can be older than
                // the anchor and still finalize a pending cert at its view.
                let state_cert = self.pending_state_certs.remove(&leaf_view);
                if let Some(replay) = &mut self.replay
                    && !replay.is_replayed(leaf_view)
                {
                    replay.leaves.insert(
                        leaf_view,
                        ReplayLeaf {
                            leaf: leaf.clone(),
                            cert: CertificatePair::new(qc.clone(), next_epoch_qc.clone()),
                            state_cert: state_cert.clone(),
                        },
                    );
                }
                state_cert.map(|cert| Finalized {
                    epoch: cert.epoch.u64(),
                    cert,
                })
            },
            Record::StateCert(cert) => {
                let view = ViewNumber::new(cert.light_client_state.view_number);
                self.pending_state_certs.insert(view, cert.clone());
                if let Some((anchor_leaf, ..)) = &self.anchor {
                    let bound = anchor_leaf
                        .view_number()
                        .u64()
                        .saturating_sub(DECIDE_GAP_FILL_HORIZON);
                    self.pending_state_certs.retain(|v, _| v.u64() >= bound);
                }
                None
            },
            Record::Upgrade(u) => {
                self.upgrade = u.clone();
                None
            },
            // Unconditional overwrite, matching fs/sql `store_eqc` (unlike `HighQc2`/`NextEpochQc`,
            // which only ever advance).
            Record::Eqc(qc, next) => {
                self.eqc = Some((qc.clone(), next.clone()));
                None
            },
            Record::NextEpochQc(next) => {
                if self
                    .next_epoch_qc
                    .as_ref()
                    .is_none_or(|cur| next.view_number() > cur.view_number())
                {
                    self.next_epoch_qc = Some(next.clone());
                }
                None
            },
        }
    }

    pub fn anchor_pair(&self) -> Option<(Leaf2, CertificatePair<SeqTypes>)> {
        self.anchor.as_ref().map(|(leaf, qc, next_epoch_qc)| {
            (
                leaf.clone(),
                CertificatePair::new(qc.clone(), next_epoch_qc.clone()),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use committable::Committable;
    use espresso_types::{NodeState, ValidatedState};
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_types::{
        data::QuorumProposal2, light_client::LightClientState, simple_vote::NextEpochQuorumData2,
    };

    use super::*;

    /// A genesis-derived leaf and matching QC at `view`, reusing the genesis header/QC so
    /// construction doesn't need a real block or signatures (mirrors
    /// `persistence.rs::consecutive_height_chain`).
    async fn leaf_and_qc(view: u64) -> (Leaf2, QuorumCertificate2<SeqTypes>) {
        let node_state = NodeState::mock();
        let genesis_leaf = Leaf2::genesis(
            &ValidatedState::default(),
            &node_state,
            TEST_VERSIONS.test.base,
        )
        .await;
        let base = QuorumCertificate2::genesis(
            &ValidatedState::default(),
            &node_state,
            TEST_VERSIONS.test,
        )
        .await;
        let proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                block_header: genesis_leaf.block_header().clone(),
                view_number: ViewNumber::new(view),
                justify_qc: base.clone(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch: None,
                state_cert: None,
            },
        };
        let leaf = Leaf2::from_quorum_proposal(&proposal);
        let mut qc = base;
        qc.view_number = leaf.view_number();
        qc.data.leaf_commit = Committable::commit(&leaf);
        (leaf, qc)
    }

    fn next_epoch_qc_at(
        base: &QuorumCertificate2<SeqTypes>,
        view: u64,
    ) -> NextEpochQuorumCertificate2<SeqTypes> {
        let data: NextEpochQuorumData2<SeqTypes> = base.data.into();
        NextEpochQuorumCertificate2::new(
            data.clone(),
            data.commit(),
            ViewNumber::new(view),
            None,
            PhantomData,
        )
    }

    fn state_cert(view: u64) -> LightClientStateUpdateCertificateV2<SeqTypes> {
        LightClientStateUpdateCertificateV2 {
            epoch: hotshot_types::data::EpochNumber::new(view),
            light_client_state: LightClientState {
                view_number: view,
                block_height: 0,
                block_comm_root: Default::default(),
            },
            next_stake_table_state: Default::default(),
            signatures: vec![],
            auth_root: Default::default(),
        }
    }

    #[tokio::test]
    async fn action_vote_updates_voted_and_restart_monotonically() {
        let mut s = State::default();
        s.apply(&Record::Action {
            view: ViewNumber::new(5),
            kind: HotShotAction::Vote,
        });
        assert_eq!(s.voted, Some(ViewNumber::new(5)));
        assert_eq!(s.restart, Some(ViewNumber::new(6)));

        // A lower vote never regresses voted/restart.
        s.apply(&Record::Action {
            view: ViewNumber::new(2),
            kind: HotShotAction::Vote,
        });
        assert_eq!(s.voted, Some(ViewNumber::new(5)));
        assert_eq!(s.restart, Some(ViewNumber::new(6)));

        // Propose only raises voted, never restart.
        s.apply(&Record::Action {
            view: ViewNumber::new(9),
            kind: HotShotAction::Propose,
        });
        assert_eq!(s.voted, Some(ViewNumber::new(9)));
        assert_eq!(s.restart, Some(ViewNumber::new(6)));
    }

    #[tokio::test]
    async fn action_ignores_other_kinds() {
        let mut s = State::default();
        s.apply(&Record::Action {
            view: ViewNumber::new(5),
            kind: HotShotAction::DaVote,
        });
        assert_eq!(s.voted, None);
    }

    #[tokio::test]
    async fn high_qc2_applies_as_max() {
        let (_, qc3) = leaf_and_qc(3).await;
        let (_, qc1) = leaf_and_qc(1).await;
        let mut s = State::default();
        s.apply(&Record::HighQc2(qc3));
        s.apply(&Record::HighQc2(qc1));
        assert_eq!(s.high_qc2.unwrap().view_number(), ViewNumber::new(3));
    }

    #[tokio::test]
    async fn eqc_overwrites_unconditionally_next_epoch_qc_applies_as_max() {
        let (_, qc5) = leaf_and_qc(5).await;
        let (_, qc2) = leaf_and_qc(2).await;
        let next5 = next_epoch_qc_at(&qc5, 5);
        let next2 = next_epoch_qc_at(&qc2, 2);

        let mut s = State::default();
        s.apply(&Record::Eqc(qc5, next5));
        s.apply(&Record::Eqc(qc2, next2));
        assert_eq!(s.eqc.as_ref().unwrap().0.view_number(), ViewNumber::new(2));

        let (_, qc4) = leaf_and_qc(4).await;
        let (_, qc1) = leaf_and_qc(1).await;
        s.apply(&Record::NextEpochQc(next_epoch_qc_at(&qc4, 4)));
        s.apply(&Record::NextEpochQc(next_epoch_qc_at(&qc1, 1)));
        assert_eq!(s.next_epoch_qc.unwrap().view_number(), ViewNumber::new(4));
    }

    #[tokio::test]
    async fn anchor_never_regresses() {
        let (leaf10, qc10) = leaf_and_qc(10).await;
        let (leaf3, qc3) = leaf_and_qc(3).await;
        let mut s = State::default();
        s.apply(&Record::Leaf {
            leaf: leaf10,
            qc: qc10,
            next_epoch_qc: None,
        });
        s.apply(&Record::Leaf {
            leaf: leaf3,
            qc: qc3,
            next_epoch_qc: None,
        });
        assert_eq!(
            s.anchor.unwrap().0.view_number(),
            ViewNumber::new(10),
            "an older leaf must never lower the anchor"
        );
    }

    #[tokio::test]
    async fn state_cert_finalizes_only_at_its_leaf_view() {
        let (leaf6, qc6) = leaf_and_qc(6).await;
        let (leaf7, qc7) = leaf_and_qc(7).await;
        let mut s = State::default();
        s.apply(&Record::StateCert(state_cert(7)));
        assert!(s.pending_state_certs.contains_key(&ViewNumber::new(7)));

        let finalized = s.apply(&Record::Leaf {
            leaf: leaf6,
            qc: qc6,
            next_epoch_qc: None,
        });
        assert!(finalized.is_none());
        assert!(s.pending_state_certs.contains_key(&ViewNumber::new(7)));

        let finalized = s.apply(&Record::Leaf {
            leaf: leaf7,
            qc: qc7,
            next_epoch_qc: None,
        });
        assert_eq!(finalized.unwrap().cert.light_client_state.view_number, 7);
        assert!(!s.pending_state_certs.contains_key(&ViewNumber::new(7)));
    }

    #[tokio::test]
    async fn gap_fill_decide_finalizes_without_lowering_anchor() {
        let (leaf10, qc10) = leaf_and_qc(10).await;
        let (leaf4, qc4) = leaf_and_qc(4).await;
        let mut s = State::default();
        s.apply(&Record::Leaf {
            leaf: leaf10,
            qc: qc10,
            next_epoch_qc: None,
        });
        s.apply(&Record::StateCert(state_cert(4)));

        let finalized = s.apply(&Record::Leaf {
            leaf: leaf4,
            qc: qc4,
            next_epoch_qc: None,
        });

        assert_eq!(finalized.unwrap().cert.light_client_state.view_number, 4);
        assert_eq!(s.anchor.unwrap().0.view_number(), ViewNumber::new(10));
    }

    #[test]
    fn record_encode_decode_roundtrips_action() {
        let rec = Record::Action {
            view: ViewNumber::new(11),
            kind: HotShotAction::Vote,
        };
        let body = rec.encode().unwrap();
        let decoded = Record::decode(Kind::Action, 11, &body).unwrap();
        assert_eq!(decoded.kind(), Kind::Action);
        assert_eq!(decoded.view(), 11);
    }
}
