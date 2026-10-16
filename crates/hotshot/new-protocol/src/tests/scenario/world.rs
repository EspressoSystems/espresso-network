//! The real blocks and certificates a scenario's abstract ones stand for.
//!
//! A scenario names a block by an identity and a certificate by the block it is
//! over, its epoch, height and view (`super::format`). This builds each from
//! those, the first time it is named, and keeps it, so a block named twice is
//! the same proposal both times. Certificates are signed by every test key, as
//! the other tests' certificates are: what consensus checks of them is the
//! committee's threshold, not who signed.
//!
//! What a real proposal carries and the specification does not model is filled
//! in as the Rust node expects it: the parent's `Certificate2` for the first
//! block of an epoch, the state certificate after an epoch root, and the DRB
//! result in an epoch's transition blocks, which is the harness's
//! `TEST_DRB_RESULT` throughout.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use bitvec::bitvec;
use committable::{Commitment, Committable};
use futures::future::BoxFuture;
use hotshot::types::{BLSPrivKey, BLSPubKey};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, VidCommitment, VidDisperse, VidDisperse2, VidDisperseShare2, ViewNumber,
    },
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{TimeoutCertificate3, TimeoutEvidence},
    simple_vote::{
        LockView, QuorumData2, SimpleVote, TimeoutData3, VersionedVoteData, Vote2Data, Voteable,
    },
    stake_table::{HSStakeTable, StakeTableEntries},
    traits::{
        BlockPayload,
        block_contents::BlockHeader,
        election::Membership,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    utils::{epoch_from_block_number, is_epoch_root, is_epoch_transition, is_last_block},
    vote::{Certificate, Vote},
};

use super::format::{self, Cert, TimeoutCert};
use crate::{
    helpers::{test_timeout_epoch_lock, test_upgrade_lock},
    message::{Certificate1, Certificate2, Proposal},
    tests::common::{
        coordinator_builder::build_genesis_cert1,
        utils::{
            MockBlock, STAKE_TABLE_CAPACITY, StakeTableSchedule, TEST_DRB_RESULT,
            build_state_cert_for_test, extract_vid_shares,
            mock_membership_with_client_and_schedule,
        },
    },
};

type StateCert = hotshot_types::simple_certificate::LightClientStateUpdateCertificateV2<TestTypes>;

/// The anchor's identity in every scenario.
pub const ANCHOR: u64 = 1;

/// A block built for a scenario.
#[derive(Clone)]
pub struct Built {
    pub proposal: SignedProposal<TestTypes, Proposal<TestTypes>>,
    /// The leaf's commitment, which certificates over the block name.
    pub commit: Commitment<Leaf2<TestTypes>>,
    pub payload: TestBlockPayload,
    pub vid: VidDisperse2<TestTypes>,
    pub shares: Vec<VidDisperseShare2<TestTypes>>,
    pub leader: BLSPubKey,
}

impl Built {
    /// The VID share for `recipient`.
    pub fn share_for(&self, recipient: &BLSPubKey) -> VidDisperseShare2<TestTypes> {
        self.shares
            .iter()
            .find(|s| s.recipient_key == *recipient)
            .expect("a share for every member")
            .clone()
    }
}

/// What building costs most, kept across the scenarios of a test run: VID
/// dispersals, certificates and the nodes' keys. Each is a function of what it
/// is keyed by and the committees, so a scenario of another node or another run
/// with the same committees that names the same reuses it.
#[derive(Default)]
pub struct Cache {
    /// The node count and committees the rest was built for.
    committees: Option<(u64, Committees)>,
    /// By payload (`None` for the harness's own block), view and epoch.
    vids: BTreeMap<(Option<u64>, u64, u64), VidDisperse2<TestTypes>>,
    /// By the block's leaf commitment, epoch, height and view.
    cert1s: BTreeMap<CertKey, Certificate1<TestTypes>>,
    cert2s: BTreeMap<CertKey, Certificate2<TestTypes>>,
    state_certs: BTreeMap<CertKey, StateCert>,
    /// By view, epoch and lock, the anchor's being `None`.
    timeout_certs: BTreeMap<(u64, u64, Option<CertKey>), TimeoutCertificate3<TestTypes>>,
    /// Each node's keys, by its number less one.
    keys: Vec<(BLSPubKey, BLSPrivKey)>,
}

type CertKey = (Commitment<Leaf2<TestTypes>>, u64, u64, u64);

/// Committees by the epoch they start from, each node by its number from 1.
pub type Committees = Vec<(u64, Vec<u64>)>;

pub struct World {
    cache: Cache,
    nodes: u64,
    epoch_height: u64,
    membership: EpochMembershipCoordinator<TestTypes>,
    keys: BTreeMap<BLSPubKey, BLSPrivKey>,
    /// Each node's keys, by its number less one.
    node_keys: Vec<(BLSPubKey, BLSPrivKey)>,
    genesis: Leaf2<TestTypes>,
    genesis_cert: Certificate1<TestTypes>,
    defs: HashMap<u64, format::Block>,
    built: HashMap<u64, Arc<Built>>,
    /// The blocks the node itself proposed (`World::adopt_own`).
    own: std::collections::HashSet<u64>,
    /// The `Certificate2`s the node was given, by the block they are over.
    cert2s: HashMap<u64, Vec<Cert>>,
}

impl World {
    pub async fn new(
        nodes: u64,
        epoch_height: u64,
        committees: &Committees,
        blocks: &[format::Block],
        mut cache: Cache,
    ) -> Self {
        let setup = Some((nodes, committees.clone()));
        if cache.committees != setup {
            cache = Cache {
                committees: setup,
                ..Cache::default()
            };
        }
        if cache.keys.len() as u64 != nodes {
            cache.keys = (0..nodes)
                .map(|i| BLSPubKey::generated_from_seed_indexed([0u8; 32], i))
                .collect();
        }
        let node_keys = cache.keys.clone();
        let membership = scheduled_membership(nodes, epoch_height, committees);
        let genesis = Leaf2::<TestTypes>::genesis(
            &TestValidatedState::default(),
            &TestInstanceState::default(),
            TEST_VERSIONS.vid2.base,
        )
        .await;
        let genesis_cert = build_genesis_cert1(&genesis);
        Self {
            cache,
            nodes,
            epoch_height,
            membership,
            keys: node_keys.iter().cloned().collect(),
            node_keys,
            genesis,
            genesis_cert,
            defs: blocks.iter().map(|b| (b.id, *b)).collect(),
            built: HashMap::new(),
            own: std::collections::HashSet::new(),
            cert2s: HashMap::new(),
        }
    }

    /// The cache, for the next scenario's world.
    pub fn into_cache(self) -> Cache {
        self.cache
    }

    /// Node `node`'s keys; nodes are numbered from 1.
    pub fn key(&self, node: u64) -> (BLSPubKey, BLSPrivKey) {
        self.node_keys[(node - 1) as usize].clone()
    }

    /// The leader of `view` in `epoch`, with its private key.
    pub fn leader(&self, epoch: u64, view: u64) -> (BLSPubKey, BLSPrivKey) {
        let leader = self
            .membership
            .membership_for_epoch(Some(EpochNumber::new(epoch)))
            .expect("the epoch's committee is known")
            .leader(ViewNumber::new(view))
            .expect("every view has a leader");
        let key = self
            .keys
            .get(&leader)
            .expect("the leader is a test key")
            .clone();
        (leader, key)
    }

    /// The node a key belongs to, numbered from 1.
    pub fn node_of(&self, key: &BLSPubKey) -> Option<u64> {
        (1..=self.nodes).find(|&n| self.node_keys[(n - 1) as usize].0 == *key)
    }

    /// The block with the identity `id`, built on first use.
    pub fn block(&mut self, id: u64) -> BoxFuture<'_, Arc<Built>> {
        Box::pin(async move {
            if let Some(b) = self.built.get(&id) {
                return b.clone();
            }
            let def = *self
                .defs
                .get(&id)
                .unwrap_or_else(|| panic!("block {id} is in the scenario's block list"));
            let b = Arc::new(self.build(def).await);
            self.built.insert(id, b.clone());
            b
        })
    }

    /// The block a payload the scenario names was proposed with.
    pub fn block_with_payload(&self, view: u64, payload: u64) -> Option<u64> {
        self.defs
            .values()
            .find(|b| b.view == view && b.payload == payload)
            .map(|b| b.id)
    }

    /// Use the proposal the node itself sent for the block `id`, if it is on the
    /// parent the scenario names; the node's own block stands for it from then on.
    pub async fn adopt_own(
        &mut self,
        id: u64,
        proposal: SignedProposal<TestTypes, Proposal<TestTypes>>,
    ) -> bool {
        let Some(def) = self.defs.get(&id).copied() else {
            return false;
        };
        if self.built.contains_key(&id) {
            return false;
        }
        if proposal.data.justify_qc.data.leaf_commit != self.leaf_commit(def.parent.block).await
            || proposal.data.view_number != ViewNumber::new(def.view)
        {
            return false;
        }
        let mock = MockBlock::new();
        let epoch = proposal.data.epoch;
        let vid = self
            .disperse(None, &mock.block, &mock.metadata, def.view, epoch)
            .await;
        let leader = self.leader(*epoch, def.view).0;
        let leaf = Leaf2::from(proposal.data.clone());
        if is_epoch_root(def.height, self.epoch_height) {
            self.register_epoch_root(&proposal.data.block_header).await;
        }
        self.own.insert(id);
        self.built.insert(
            id,
            Arc::new(Built {
                proposal,
                commit: leaf.commit(),
                payload: mock.block,
                shares: extract_vid_shares(&vid),
                vid,
                leader,
            }),
        );
        true
    }

    /// The identity of the built block with this leaf commitment, if any.
    pub fn block_with_commitment(&self, commitment: &Commitment<Leaf2<TestTypes>>) -> Option<u64> {
        self.built
            .iter()
            .find(|(_, b)| b.commit == *commitment)
            .map(|(id, _)| *id)
    }

    /// Whether the block `id` passes the validity check.
    pub fn is_valid(&self, id: u64) -> bool {
        self.defs.get(&id).is_none_or(|b| b.valid)
    }

    /// Whether the block `id` is one the node itself proposed.
    pub fn is_own(&self, id: u64) -> bool {
        self.own.contains(&id)
    }

    /// Note a `Certificate2` the node was given, for the first block of the next
    /// epoch to name.
    pub fn saw_cert2(&mut self, c: Cert) {
        self.cert2s.entry(c.block).or_default().push(c);
    }

    async fn leaf_commit(&mut self, id: u64) -> Commitment<Leaf2<TestTypes>> {
        if id == ANCHOR {
            return self.genesis.commit();
        }
        self.block(id).await.commit
    }

    pub async fn cert1(&mut self, c: Cert) -> Certificate1<TestTypes> {
        if c.block == ANCHOR && c.view == 0 {
            return self.genesis_cert.clone();
        }
        let key = self.cert_key(c).await;
        if let Some(cert) = self.cache.cert1s.get(&key) {
            return cert.clone();
        }
        let data = QuorumData2 {
            leaf_commit: key.0,
            epoch: Some(EpochNumber::new(c.epoch)),
            block_number: Some(c.height),
        };
        let cert: Certificate1<TestTypes> = self.certify(data, c.epoch, c.view);
        self.cache.cert1s.insert(key, cert.clone());
        cert
    }

    async fn cert_key(&mut self, c: Cert) -> CertKey {
        (self.leaf_commit(c.block).await, c.epoch, c.height, c.view)
    }

    pub async fn cert2(&mut self, c: Cert) -> Certificate2<TestTypes> {
        let key = self.cert_key(c).await;
        if let Some(cert) = self.cache.cert2s.get(&key) {
            return cert.clone();
        }
        let data = Vote2Data {
            leaf_commit: key.0,
            epoch: EpochNumber::new(c.epoch),
            block_number: c.height,
        };
        let cert: Certificate2<TestTypes> = self.certify(data, c.epoch, c.view);
        self.cache.cert2s.insert(key, cert.clone());
        cert
    }

    pub async fn timeout_cert(&mut self, tc: TimeoutCert) -> TimeoutCertificate3<TestTypes> {
        let anchor = tc.lock.block == ANCHOR && tc.lock.view == 0;
        let lock_key = if anchor {
            None
        } else {
            Some(self.cert_key(tc.lock).await)
        };
        let key = (tc.view, tc.epoch, lock_key);
        if let Some(cert) = self.cache.timeout_certs.get(&key) {
            return cert.clone();
        }
        let lock = if anchor {
            None
        } else {
            Some(self.cert1(tc.lock).await)
        };
        let cert = self.sign_timeout_cert(tc.view, tc.epoch, lock);
        self.cache.timeout_certs.insert(key, cert.clone());
        cert
    }

    /// A certificate over `data` at `view`, signed by every member of `epoch`'s
    /// committee.
    fn certify<D, C>(&self, data: D, epoch: u64, view: u64) -> C
    where
        D: Voteable<TestTypes> + 'static,
        C: Certificate<TestTypes, D, Voteable = D>,
    {
        let membership = self.committee(epoch);
        let view = ViewNumber::new(view);
        let upgrade_lock = test_upgrade_lock::<TestTypes>();
        let entries = StakeTableEntries::<TestTypes>::from(C::stake_table(&membership)).0;
        let signatures: Vec<_> = entries
            .iter()
            .map(|e| self.vote(data.clone(), view, &e.public_key(), &upgrade_lock))
            .collect();
        let params = BLSPubKey::public_parameter(&entries, C::threshold(&membership));
        let signature = BLSPubKey::assemble(
            &params,
            bitvec![1; entries.len()].as_bitslice(),
            &signatures,
        );
        let commitment = VersionedVoteData::new(data.clone(), view, &upgrade_lock)
            .expect("versioned vote data")
            .commit();
        C::create_signed_certificate(commitment, data, signature, view)
    }

    /// A timeout certificate for `view` in `epoch`, signed by every member of the
    /// committee, all holding `lock`.
    fn sign_timeout_cert(
        &self,
        view: u64,
        epoch: u64,
        lock: Option<Certificate1<TestTypes>>,
    ) -> TimeoutCertificate3<TestTypes> {
        let membership = self.committee(epoch);
        let (view, epoch) = (ViewNumber::new(view), EpochNumber::new(epoch));
        let upgrade_lock = test_timeout_epoch_lock::<TestTypes>();
        let entries = StakeTableEntries::<TestTypes>::from_iter(membership.stake_table()).0;
        let lock_view = lock.as_ref().map(LockView::of);
        let data = TimeoutData3 {
            view,
            epoch,
            lock: lock_view,
        };
        let votes: Vec<_> = entries
            .iter()
            .map(|e| {
                let key = e.public_key();
                let signature = self.vote(data.clone(), view, &key, &upgrade_lock);
                (lock_view, key, signature)
            })
            .collect();
        TimeoutCertificate3::assemble(view, epoch, &entries, votes, lock)
            .expect("assemble timeout certificate")
    }

    /// The signature of `key`'s vote for `data` at `view`.
    fn vote<D: Voteable<TestTypes> + 'static>(
        &self,
        data: D,
        view: ViewNumber,
        key: &BLSPubKey,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> <BLSPubKey as SignatureKey>::PureAssembledSignatureType {
        let private_key = self.keys.get(key).expect("every member is a test key");
        SimpleVote::create_signed_vote(data, view, key, private_key, upgrade_lock)
            .expect("sign vote")
            .signature()
    }

    fn committee(&self, epoch: u64) -> hotshot_types::epoch_membership::EpochMembership<TestTypes> {
        self.membership
            .membership_for_epoch(Some(EpochNumber::new(epoch)))
            .unwrap_or_else(|e| panic!("the committee of epoch {epoch} is known: {e}"))
    }

    /// The dispersal of a payload, the scenario's `payload` or, for `None`, the
    /// harness's own block.
    async fn disperse(
        &mut self,
        id: Option<u64>,
        payload: &TestBlockPayload,
        metadata: &TestMetadata,
        view: u64,
        epoch: EpochNumber,
    ) -> VidDisperse2<TestTypes> {
        let key = (id, view, *epoch);
        if let Some(vid) = self.cache.vids.get(&key) {
            return vid.clone();
        }
        let disperse = VidDisperse::calculate_vid_disperse(
            payload,
            &self.membership,
            ViewNumber::new(view),
            Some(epoch),
            Some(epoch),
            metadata,
            &test_upgrade_lock(),
        )
        .await
        .expect("dispersal succeeds");
        let VidDisperse::V2(vid) = disperse.disperse else {
            panic!("a V2 dispersal");
        };
        self.cache.vids.insert(key, vid.clone());
        vid
    }

    /// The epoch a block's height puts it in, given its parent, whatever epoch the
    /// proposal names: the one whose committee disperses and signs for it.
    fn epoch_of(&self, def: &format::Block) -> u64 {
        if is_last_block(def.parent.height, self.epoch_height) {
            def.parent.epoch + 1
        } else {
            def.parent.epoch.max(1)
        }
    }

    async fn build(&mut self, def: format::Block) -> Built {
        let epoch = EpochNumber::new(def.epoch);
        let committee_epoch = EpochNumber::new(self.epoch_of(&def));
        let justify_qc = self.cert1(def.parent).await;
        let transactions = vec![TestTransaction::new(def.payload.to_le_bytes().to_vec())];
        let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
            &transactions,
            &TestValidatedState::default(),
            &TestInstanceState::default(),
        )
        .await
        .expect("a payload from one transaction");
        let builder_commitment =
            <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&payload, &metadata);
        let vid = self
            .disperse(
                Some(def.payload),
                &payload,
                &metadata,
                def.view,
                committee_epoch,
            )
            .await;
        let block_header = TestBlockHeader {
            block_number: def.height,
            payload_commitment: VidCommitment::V2(vid.payload_commitment),
            builder_commitment,
            metadata,
            timestamp: def.view,
            timestamp_millis: def.view * 1_000,
            random: def.id,
            version: TEST_VERSIONS.test.base,
        };
        let next_epoch_justify_qc = if is_last_block(def.parent.height, self.epoch_height) {
            Some(self.parent_cert2(def.parent).await)
        } else {
            None
        };
        let state_cert = self.state_cert(def.parent).await;
        let next_drb_result = (epoch > EpochNumber::genesis()
            && is_epoch_transition(def.height, self.epoch_height))
        .then_some(TEST_DRB_RESULT);
        let view_change_evidence = match def.evidence {
            Some(tc) => Some(TimeoutEvidence::V3(self.timeout_cert(tc).await)),
            None => None,
        };
        let proposal = Proposal {
            block_header,
            view_number: ViewNumber::new(def.view),
            epoch,
            justify_qc,
            next_epoch_justify_qc,
            upgrade_certificate: None,
            view_change_evidence,
            next_drb_result,
            state_cert,
        };
        let leaf = Leaf2::from(proposal.clone());
        let (leader, leader_key) = self.leader(*committee_epoch, def.view);
        let signature = <BLSPubKey as SignatureKey>::sign(&leader_key, leaf.commit().as_ref())
            .expect("the leader signs the leaf");
        if is_epoch_root(def.height, self.epoch_height) {
            self.register_epoch_root(&proposal.block_header).await;
        }
        Built {
            commit: leaf.commit(),
            proposal: SignedProposal {
                data: proposal,
                signature,
                _pd: std::marker::PhantomData,
            },
            payload,
            shares: extract_vid_shares(&vid),
            vid,
            leader,
        }
    }

    /// The light-client state certificate that goes with a `Cert1` over an epoch
    /// root, at the certificate's view, as the node's state-vote tally pairs them;
    /// none for any other block.
    pub async fn state_cert(&mut self, c: Cert) -> Option<StateCert> {
        if !is_epoch_root(c.height, self.epoch_height) {
            return None;
        }
        let key = self.cert_key(c).await;
        if let Some(cert) = self.cache.state_certs.get(&key) {
            return Some(cert.clone());
        }
        let root = self.block(c.block).await;
        let next = self
            .committee(c.epoch)
            .next_epoch_stake_table()
            .expect("the next epoch's committee is known");
        let next_state = HSStakeTable::from_iter(next.stake_table())
            .commitment(STAKE_TABLE_CAPACITY)
            .expect("a stake table commitment");
        let members = self.committee(c.epoch).stake_table().len() as u64;
        let cert = build_state_cert_for_test(
            &root.proposal.data.block_header,
            ViewNumber::new(c.view),
            EpochNumber::new(c.epoch),
            &next_state,
            (2 * members) / 3 + 1,
        );
        self.cache.state_certs.insert(key, cert.clone());
        Some(cert)
    }

    /// The `Certificate2` over an epoch's last block that the first block of the
    /// next one names: the latest the node was given at the parent's view or
    /// later, else one at the parent's view.
    async fn parent_cert2(&mut self, parent: Cert) -> Certificate2<TestTypes> {
        let seen = self
            .cert2s
            .get(&parent.block)
            .and_then(|cs| {
                cs.iter()
                    .filter(|c| c.view >= parent.view)
                    .max_by_key(|c| c.view)
            })
            .copied();
        self.cert2(seen.unwrap_or(parent)).await
    }

    /// Register the epochs an epoch root decides, as the harness does once the
    /// root is decided, so the committee that signs the later epochs' certificates
    /// is known here too.
    async fn register_epoch_root(&mut self, header: &TestBlockHeader) {
        let height = <TestBlockHeader as BlockHeader<TestTypes>>::block_number(header);
        self.membership
            .add_epoch_root(header.clone())
            .await
            .expect("an epoch root registers");
        let target = EpochNumber::new(epoch_from_block_number(height, self.epoch_height) + 2);
        self.membership
            .membership()
            .add_drb_result(target, TEST_DRB_RESULT);
    }
}

/// The test membership of `nodes` nodes with `committees`. The first committee
/// is the genesis one, and the test membership fixes it for the first two epochs.
pub fn scheduled_membership(
    nodes: u64,
    epoch_height: u64,
    committees: &Committees,
) -> EpochMembershipCoordinator<TestTypes> {
    let indices = |members: &[u64]| members.iter().map(|&n| (n - 1) as usize).collect();
    let (initial, changes) = match committees.split_first() {
        Some(((_, first), rest)) => (
            indices(first),
            rest.iter()
                .map(|(e, members)| (*e, indices(members)))
                .collect(),
        ),
        None => ((0..nodes as usize).collect(), Vec::new()),
    };
    assert!(
        changes.iter().all(|(e, _)| *e >= 3),
        "the test membership fixes the committee of the first two epochs"
    );
    let schedule = StakeTableSchedule {
        initial,
        changes,
        addr_overrides: Vec::new(),
    };
    let (pk0, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    mock_membership_with_client_and_schedule(
        nodes as usize,
        epoch_height,
        pk0,
        TestStorage::default(),
        &schedule,
        &[],
    )
    .0
}
