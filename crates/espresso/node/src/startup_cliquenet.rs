//! The node's cliquenet during the startup fill.

use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use espresso_types::{Leaf2, PubKey, SeqTypes};
use hotshot_new_protocol::{
    message::{ConsensusMessage, Message, MessageType, Unchecked, Validated},
    network::{Cliquenet, Role},
    utils::CertifiedLeafChain,
};
use hotshot_types::{
    data::ViewNumber, signature_key::BLSPrivKey, traits::signature_key::SignatureKey,
};
use rand::seq::SliceRandom;
use request_response::{
    RequestHash, Serializable,
    message::{Message as RequestResponseMessage, MessageFrame, RequestMessage},
};
use tokio::time::{Instant, timeout_at};
use vec1::Vec1;

use crate::{
    external_event_handler::{ExternalMessage, frame_external_message},
    request_response::request::{Request, Response},
    startup_catchup::{
        BootstrapPeers, ObservedCertificate, STARTUP_FAN_OUT, StartupCliquenet, StartupRequest,
        StartupResponse,
    },
};

/// Certificates kept for `next_observed_certificate` while a batch waits for answers.
const BUFFERED_CERTIFICATES_LIMIT: usize = 256;

/// The node's `Cliquenet`, borrowed by the fill before the coordinator owns it.
///
/// Sends with `Sender::unicast` and reads with `receive()` only. It never calls `client_api()`,
/// `apply_epoch`, or touches `shared.epoch`. Messages it consumes never reach the coordinator, and
/// inbound requests from other peers are dropped. Each request goes to `STARTUP_FAN_OUT` random
/// bootstrap peers per attempt.
pub(crate) struct CliquenetBeforeConsensus<'a> {
    net: &'a mut Cliquenet<SeqTypes>,
    peers: Vec<PubKey>,
    public_key: PubKey,
    private_key: BLSPrivKey,
    /// Certificates read while waiting for answers.
    buffered: VecDeque<ObservedCertificate>,
}

impl<'a> CliquenetBeforeConsensus<'a> {
    /// Borrows the instance `init` created from the factory, no parties yet. `public_key` is
    /// derived from `private_key`, not passed, so it cannot disagree with
    /// `StartupCatchupParams::own_key`.
    pub fn new(net: &'a mut Cliquenet<SeqTypes>, private_key: BLSPrivKey) -> Self {
        Self {
            net,
            peers: Vec::new(),
            public_key: PubKey::from_private(&private_key),
            private_key,
            buffered: VecDeque::new(),
        }
    }

    /// Signs `wire`, frames it as `external_event_handler` frames request-response payloads and
    /// sends it to `peer`.
    fn send(&self, peer: &PubKey, wire: &Request) -> anyhow::Result<()> {
        let request = RequestMessage::new_signed(&self.public_key, &self.private_key, wire)?;
        let payload = RequestResponseMessage::<Request, PubKey>::Request(request).to_bytes()?;
        let message = Message::<SeqTypes, Validated> {
            sender: self.public_key,
            message_type: MessageType::External(frame_external_message(self.public_key, payload)?),
        };
        self.net
            .sender()
            .unicast(ViewNumber::genesis(), peer, &message)?;
        Ok(())
    }

    /// Sends `wire` to `STARTUP_FAN_OUT` random peers.
    fn fan_out(&self, wire: &Request) {
        let peers = self
            .peers
            .choose_multiple(&mut rand::thread_rng(), STARTUP_FAN_OUT);
        for peer in peers {
            if let Err(err) = self.send(peer, wire) {
                tracing::debug!(%peer, "failed to send a startup request: {err:#}");
            }
        }
    }

    /// Starts `slot`'s current phase: registers its hash and sends the request unless another slot
    /// already waits for the same bytes.
    fn start(
        &mut self,
        index: usize,
        slots: &mut [Slot],
        waiting: &mut HashMap<RequestHash, Vec<usize>>,
    ) {
        let wire = slots[index].wire();
        let hash = wire_hash(&wire);
        slots[index].wire_hash = Some(hash);
        let waiters = waiting.entry(hash).or_default();
        waiters.push(index);
        if waiters.len() == 1 {
            self.fan_out(&wire);
        }
    }

    /// Routes one inbound message: responses to their waiters, certificates to the buffer, the
    /// rest (including inbound requests) dropped.
    fn route(
        &mut self,
        message: Message<SeqTypes, Unchecked>,
        slots: &mut [Slot],
        waiting: &mut HashMap<RequestHash, Vec<usize>>,
    ) {
        match message.message_type {
            MessageType::External(data) => {
                self.route_external(message.sender, &data, slots, waiting)
            },
            MessageType::Consensus(consensus) => self.buffer(consensus),
            _ => {},
        }
    }

    fn route_external(
        &mut self,
        sender: PubKey,
        data: &[u8],
        slots: &mut [Slot],
        waiting: &mut HashMap<RequestHash, Vec<usize>>,
    ) {
        let Ok(ExternalMessage::RequestResponse(payload)) = bincode::deserialize(data) else {
            return;
        };
        let Ok(MessageFrame::Response { request_hash, body }) = MessageFrame::parse(&payload)
        else {
            return;
        };
        let Some(waiters) = waiting.get(&request_hash).cloned() else {
            return;
        };
        let response = match Response::from_bytes(body) {
            Ok(response) => response,
            Err(err) => {
                tracing::debug!(%sender, "undecodable startup response: {err:#}");
                return;
            },
        };
        for index in waiters {
            if slots[index].wire_hash != Some(request_hash) {
                continue;
            }
            match slots[index].accept(&response) {
                Ok(next) => {
                    if next {
                        self.start(index, slots, waiting);
                    }
                },
                Err(err) => {
                    tracing::debug!(%sender, "startup response not accepted: {err:#}");
                },
            }
        }
    }

    fn buffer(&mut self, message: ConsensusMessage<SeqTypes, Unchecked>) {
        let certificate = match message {
            ConsensusMessage::Certificate1(cert, _) => ObservedCertificate::Cert1(cert),
            ConsensusMessage::Certificate2(cert, _) => ObservedCertificate::Cert2(cert),
            _ => return,
        };
        if self.buffered.len() >= BUFFERED_CERTIFICATES_LIMIT {
            self.buffered.pop_front();
        }
        self.buffered.push_back(certificate);
    }
}

#[async_trait]
impl StartupCliquenet for CliquenetBeforeConsensus<'_> {
    /// `Cliquenet::add_peers(Role::Active, ..)` with each key converted inline: `NetAddr` from
    /// `hotshot_new_protocol::network`, the x25519 key through `.into()` from `PeerConnectInfo`.
    fn dial(&mut self, peers: &BootstrapPeers) -> anyhow::Result<()> {
        let targets = peers
            .iter()
            .filter(|(key, _)| **key != self.public_key)
            .map(|(key, info)| (*key, (info.x25519_key.into(), info.p2p_addr.clone())))
            .collect::<Vec<_>>();
        self.peers = targets.iter().map(|(key, _)| *key).collect();
        self.net.add_peers(Role::Active, targets)?;
        Ok(())
    }

    /// See the trait. Peers with no answer, an error answer, or an answer of the wrong shape log
    /// at DEBUG; per-stage answered and unanswered counts go to `startup catchup done`.
    async fn fetch_concurrently(
        &mut self,
        requests: &Vec1<StartupRequest>,
        wait: Duration,
    ) -> Vec<anyhow::Result<StartupResponse>> {
        let deadline = Instant::now() + wait;
        let mut slots: Vec<Slot> = requests.iter().cloned().map(Slot::new).collect();
        let mut waiting = HashMap::new();
        for index in 0..slots.len() {
            self.start(index, &mut slots, &mut waiting);
        }

        while slots.iter().any(|slot| !slot.is_done()) {
            match timeout_at(deadline, self.net.receive()).await {
                Err(_) => break,
                Ok(Ok(message)) => self.route(message, &mut slots, &mut waiting),
                Ok(Err(err)) if err.is_critical() => {
                    tracing::debug!("cliquenet failed during a startup batch: {err:#}");
                    break;
                },
                Ok(Err(err)) => tracing::debug!("skipping a cliquenet message: {err:#}"),
            }
        }

        for slot in slots.iter().filter(|slot| !slot.is_done()) {
            tracing::debug!(request = ?slot.request, "no valid answer");
        }
        slots.into_iter().map(Slot::finish).collect()
    }

    async fn next_observed_certificate(&mut self, wait: Duration) -> Option<ObservedCertificate> {
        let deadline = Instant::now() + wait;
        loop {
            if let Some(certificate) = self.buffered.pop_front() {
                return Some(certificate);
            }
            match timeout_at(deadline, self.net.receive()).await {
                Err(_) => return None,
                Ok(Ok(message)) => {
                    if let MessageType::Consensus(consensus) = message.message_type {
                        self.buffer(consensus);
                    }
                },
                Ok(Err(err)) if err.is_critical() => return None,
                Ok(Err(err)) => tracing::debug!("skipping a cliquenet message: {err:#}"),
            }
        }
    }
}

fn wire_hash(wire: &Request) -> RequestHash {
    blake3::hash(&wire.to_bytes().expect("requests serialize"))
}

/// One startup request on its way: the wire request of its current phase and what it collected.
struct Slot {
    request: StartupRequest,
    phase: Phase,
    /// Hash of the wire request in flight.
    wire_hash: Option<RequestHash>,
}

enum Phase {
    /// Waiting for `Leaf(height)` or `HeaderProof`.
    First,
    /// Waiting for the cert2 at `height` of the leaf chain `leaves`.
    Cert2 {
        leaves: Vec<Leaf2>,
        height: u64,
    },
    Done(StartupResponse),
}

impl Slot {
    fn new(request: StartupRequest) -> Self {
        Self {
            request,
            phase: Phase::First,
            wire_hash: None,
        }
    }

    fn is_done(&self) -> bool {
        matches!(self.phase, Phase::Done(_))
    }

    /// The wire request of the current phase.
    fn wire(&self) -> Request {
        match (&self.request, &self.phase) {
            (
                StartupRequest::RootHeaderProof {
                    anchor_height,
                    root_height,
                    ..
                },
                _,
            ) => Request::HeaderProof {
                root: *anchor_height,
                height: *root_height,
            },
            (_, Phase::Cert2 { height, .. }) => Request::Cert2(*height),
            (
                StartupRequest::VerifiedLeaf { height, .. }
                | StartupRequest::DrbLeaf { height, .. },
                _,
            ) => Request::Leaf(*height),
        }
    }

    /// Takes `response` if it answers the current phase. `Ok(true)` when a further request is
    /// due.
    fn accept(&mut self, response: &Response) -> anyhow::Result<bool> {
        match (&self.request, &self.phase, response) {
            (
                StartupRequest::RootHeaderProof { root_height, .. },
                Phase::First,
                Response::HeaderProof(proof),
            ) => {
                anyhow::ensure!(
                    proof.header().height() == *root_height,
                    "header proof is for height {}, expected {root_height}",
                    proof.header().height()
                );
                self.phase = Phase::Done(StartupResponse::HeaderProof(proof.clone()));
                Ok(false)
            },
            (
                StartupRequest::VerifiedLeaf { height, .. }
                | StartupRequest::DrbLeaf { height, .. },
                Phase::First,
                Response::Leaf(leaves),
            ) => {
                let first = leaves.iter().map(Leaf2::height).min();
                let last = leaves.iter().map(Leaf2::height).max();
                let (Some(first), Some(last)) = (first, last) else {
                    return Err(anyhow!("empty leaf chain"));
                };
                anyhow::ensure!(
                    first == *height,
                    "leaf chain starts at {first}, not {height}"
                );
                self.phase = Phase::Cert2 {
                    leaves: leaves.clone(),
                    height: last,
                };
                Ok(true)
            },
            (_, Phase::Cert2 { leaves, height }, Response::Cert2(cert2)) => {
                anyhow::ensure!(
                    cert2.data.block_number == *height,
                    "cert2 is at height {}, expected {height}",
                    cert2.data.block_number
                );
                self.phase =
                    Phase::Done(StartupResponse::LeafChain(Box::new(CertifiedLeafChain {
                        leaves: leaves.clone(),
                        cert2: cert2.clone(),
                    })));
                Ok(false)
            },
            _ => Err(anyhow!("answer of the wrong kind")),
        }
    }

    fn finish(self) -> anyhow::Result<StartupResponse> {
        match self.phase {
            Phase::Done(response) => Ok(response),
            _ => Err(anyhow!("no valid answer for {:?}", self.request)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, TcpListener},
        sync::{Arc, Mutex},
    };

    use alloy::primitives::U256;
    use espresso_types::{AuthenticatedValidatorMap, EpochCommittees, v0_3::Fetcher};
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_new_protocol::message::MessageType;
    use hotshot_query_service::node::BlockId;
    use hotshot_types::{
        PeerConfig, PeerConnectInfo, ValidatorConfig,
        addr::NetAddr,
        data::EpochNumber,
        epoch_membership::EpochMembershipCoordinator,
        message::UpgradeLock,
        traits::{election::Membership, metrics::NoMetrics},
        x25519,
    };
    use indexmap::IndexMap;
    use light_client::{client::Client, testing::TestClient};
    use request_response::message::ResponseMessage;
    use tokio::task::JoinHandle;
    use tracing_subscriber::fmt::MakeWriter;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    type Answer = Arc<dyn Fn(&Request) -> Option<Response> + Send + Sync>;

    /// A peer that answers startup requests with `answer`, or stays silent when it returns `None`.
    struct TestResponder {
        config: ValidatorConfig<SeqTypes>,
        info: PeerConnectInfo,
        task: JoinHandle<()>,
    }

    impl Drop for TestResponder {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn free_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn validator(
        seed: u8,
        index: u64,
    ) -> (ValidatorConfig<SeqTypes>, PeerConnectInfo, x25519::Keypair) {
        let config = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
            [seed; 32],
            index,
            U256::from(100),
            true,
        );
        let keypair = x25519::Keypair::generate().unwrap();
        let info = PeerConnectInfo {
            x25519_key: keypair.public_key(),
            p2p_addr: NetAddr::from((Ipv4Addr::LOCALHOST, free_port())),
        };
        (config, info, keypair)
    }

    async fn network(
        config: &ValidatorConfig<SeqTypes>,
        info: &PeerConnectInfo,
        keypair: x25519::Keypair,
        parties: Vec<(PubKey, PeerConnectInfo)>,
    ) -> Cliquenet<SeqTypes> {
        Cliquenet::create(
            "startup-test",
            config.public_key,
            keypair,
            info.p2p_addr.clone(),
            parties,
            None,
            UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION)),
            Box::new(NoMetrics),
        )
        .await
        .unwrap()
    }

    impl TestResponder {
        async fn spawn(
            seed: u8,
            index: u64,
            requester: (PubKey, PeerConnectInfo),
            answer: Answer,
        ) -> Self {
            Self::spawn_pushing(seed, index, requester, answer, None).await
        }

        /// Like `spawn`, and from `push_after` on sends the requester an unsolicited message every
        /// 100 ms.
        async fn spawn_pushing(
            seed: u8,
            index: u64,
            requester: (PubKey, PeerConnectInfo),
            answer: Answer,
            push_after: Option<Duration>,
        ) -> Self {
            let (config, info, keypair) = validator(seed, index);
            let mut net = network(&config, &info, keypair, vec![requester.clone()]).await;
            let key = config.public_key;
            let task = tokio::spawn(async move {
                let started = tokio::time::Instant::now();
                let mut tick = tokio::time::interval(Duration::from_millis(100));
                loop {
                    tokio::select! {
                        received = net.receive() => {
                            let Ok(message) = received else { break };
                            if let Some(reply) = respond(key, &answer, message) {
                                let _ = net.sender().unicast(ViewNumber::genesis(), &requester.0, &reply);
                            }
                        },
                        _ = tick.tick() => {
                            if push_after.is_some_and(|after| started.elapsed() >= after) {
                                let push = external(key, b"push".to_vec());
                                let _ = net.sender().unicast(ViewNumber::genesis(), &requester.0, &push);
                            }
                        },
                    }
                }
            });
            Self { config, info, task }
        }

        fn key(&self) -> PubKey {
            self.config.public_key
        }

        fn peer_config(&self) -> PeerConfig<SeqTypes> {
            let mut config = self.config.public_config();
            config.connect_info = Some(self.info.clone());
            config
        }
    }

    /// An external message of `key` carrying `payload`.
    fn external(key: PubKey, payload: Vec<u8>) -> Message<SeqTypes, Validated> {
        Message {
            sender: key,
            message_type: MessageType::External(frame_external_message(key, payload).unwrap()),
        }
    }

    /// The answer to a request-response request in `message`, if `answer` has one.
    fn respond(
        key: PubKey,
        answer: &Answer,
        message: Message<SeqTypes, Unchecked>,
    ) -> Option<Message<SeqTypes, Validated>> {
        let MessageType::External(data) = message.message_type else {
            return None;
        };
        let ExternalMessage::RequestResponse(payload) = bincode::deserialize(&data).ok()?;
        let RequestResponseMessage::<Request, PubKey>::Request(request) =
            RequestResponseMessage::from_bytes(&payload).ok()?
        else {
            return None;
        };
        let response = RequestResponseMessage::<Request, PubKey>::Response(ResponseMessage {
            request_hash: wire_hash(&request.request),
            response: answer(&request.request)?,
        })
        .to_bytes()
        .ok()?;
        Some(external(key, response))
    }

    /// The requester and the peers it dials.
    struct Setup {
        net: Cliquenet<SeqTypes>,
        private_key: BLSPrivKey,
        peers: BootstrapPeers,
    }

    async fn setup(
        responders: &[&TestResponder],
        requester: &(ValidatorConfig<SeqTypes>, PeerConnectInfo, x25519::Keypair),
    ) -> Setup {
        let (config, info, keypair) = requester;
        let net = network(config, info, keypair.clone(), vec![]).await;
        let voting: IndexMap<_, _> = responders
            .iter()
            .map(|responder| (responder.key(), responder.peer_config()))
            .collect();
        Setup {
            net,
            private_key: config.private_key.clone(),
            peers: BootstrapPeers::new(&AuthenticatedValidatorMap::default(), &voting),
        }
    }

    async fn header_proof_answer(root: u64) -> Answer {
        let client = TestClient::default();
        client.leaf(root as usize).await;
        let proof = client.header_proof(root, BlockId::Number(3)).await.unwrap();
        Arc::new(move |request| match request {
            Request::HeaderProof { .. } => Some(Response::HeaderProof(Box::new(proof.clone()))),
            _ => None,
        })
    }

    fn header_proof_request() -> Vec1<StartupRequest> {
        Vec1::new(StartupRequest::RootHeaderProof {
            epoch: EpochNumber::new(5),
            anchor_height: 10,
            root_height: 3,
        })
    }

    #[derive(Clone, Default)]
    struct LogBuffer(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for LogBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogBuffer {
        type Writer = LogBuffer;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[tokio::test]
    async fn fan_out_first_valid_answer_ok() {
        let requester = validator(1, 0);
        let me = (requester.0.public_key, requester.1.clone());
        let silent: Answer = Arc::new(|_| None);
        let answer = header_proof_answer(10).await;
        let a = TestResponder::spawn(2, 0, me.clone(), silent.clone()).await;
        let b = TestResponder::spawn(2, 1, me.clone(), answer).await;
        let c = TestResponder::spawn(2, 2, me, silent).await;
        let mut setup = setup(&[&a, &b, &c], &requester).await;
        let mut net = CliquenetBeforeConsensus::new(&mut setup.net, setup.private_key);
        net.dial(&setup.peers).unwrap();

        let results = net
            .fetch_concurrently(&header_proof_request(), Duration::from_secs(10))
            .await;

        assert!(matches!(results[0], Ok(StartupResponse::HeaderProof(_))));
    }

    #[tokio::test]
    async fn non_answering_peers_debug_only_ok() {
        let logs = LogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(logs.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);
        let requester = validator(1, 0);
        let me = (requester.0.public_key, requester.1.clone());
        let silent: Answer = Arc::new(|_| None);
        let a = TestResponder::spawn(2, 0, me.clone(), silent.clone()).await;
        let b = TestResponder::spawn(2, 1, me, silent).await;
        let mut setup = setup(&[&a, &b], &requester).await;
        let mut net = CliquenetBeforeConsensus::new(&mut setup.net, setup.private_key);
        net.dial(&setup.peers).unwrap();

        let results = net
            .fetch_concurrently(&header_proof_request(), Duration::from_millis(500))
            .await;

        assert!(results[0].is_err());
        let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
        assert!(output.contains("no valid answer"), "{output}");
        assert!(!output.contains(" WARN "), "{output}");
    }

    #[tokio::test]
    async fn startup_ignores_inbound_requests_ok() {
        let requester = validator(1, 0);
        let me = (requester.0.public_key, requester.1.clone());
        let answer = header_proof_answer(10).await;
        let peer = TestResponder::spawn(2, 0, me, answer).await;
        let mut setup = setup(&[&peer], &requester).await;
        let mut net = CliquenetBeforeConsensus::new(&mut setup.net, setup.private_key);
        net.dial(&setup.peers).unwrap();
        // Wait for the peer's answer to a request of ours: it proves the connection works. Then the
        // peer asks us, which is dropped without a reply and without an observed certificate.
        let results = net
            .fetch_concurrently(&header_proof_request(), Duration::from_secs(10))
            .await;
        assert!(results[0].is_ok());

        assert!(
            net.next_observed_certificate(Duration::from_millis(300))
                .await
                .is_none()
        );
    }

    /// A membership whose window at `first_epoch` is only `window_peer`.
    fn window_of(
        window_peer: &TestResponder,
        first_epoch: u64,
    ) -> EpochMembershipCoordinator<SeqTypes> {
        let peers = vec![window_peer.peer_config()];
        let committees =
            EpochCommittees::new_stake(peers.clone(), peers, None, Fetcher::mock(), 10u64);
        let coordinator =
            EpochMembershipCoordinator::new(committees, 10, &TestStorage::<SeqTypes>::default());
        coordinator
            .membership()
            .set_first_epoch(EpochNumber::new(first_epoch), [0; 32]);
        coordinator
    }

    /// After the window is applied, messages of the in-window peer arrive and, over the same
    /// stretch in which both peers push, none of the candidate (its connection is removed).
    async fn assert_only_window_peers_remain(
        net: &mut Cliquenet<SeqTypes>,
        in_window: &TestResponder,
        candidate: &TestResponder,
    ) {
        let mut window_messages = 0;
        let deadline = Instant::now() + Duration::from_secs(5);
        while window_messages < 5 {
            match timeout_at(deadline, net.receive()).await {
                Err(_) => break,
                Ok(Ok(message)) => {
                    assert_ne!(message.sender, candidate.key(), "candidate accepted");
                    window_messages += u32::from(message.sender == in_window.key());
                },
                Ok(Err(hotshot_new_protocol::network::NetworkError::InvalidSender {
                    src, ..
                })) => assert_eq!(src, candidate.info.x25519_key),
                Ok(Err(err)) => panic!("unexpected error {err:#}"),
            }
        }
        assert!(
            window_messages >= 5,
            "too few messages from the window peer"
        );
    }

    async fn pushing_peers(
        requester: &(ValidatorConfig<SeqTypes>, PeerConnectInfo, x25519::Keypair),
    ) -> (TestResponder, TestResponder) {
        let me = (requester.0.public_key, requester.1.clone());
        let silent: Answer = Arc::new(|_| None);
        let push_after = Some(Duration::from_secs(1));
        let in_window =
            TestResponder::spawn_pushing(2, 0, me.clone(), silent.clone(), push_after).await;
        let candidate = TestResponder::spawn_pushing(2, 1, me, silent, push_after).await;
        (in_window, candidate)
    }

    #[tokio::test]
    async fn apply_epoch_drops_candidate_peers_ok() {
        let requester = validator(1, 0);
        let (in_window, candidate) = pushing_peers(&requester).await;
        let mut setup = setup(&[&in_window, &candidate], &requester).await;
        CliquenetBeforeConsensus::new(&mut setup.net, setup.private_key.clone())
            .dial(&setup.peers)
            .unwrap();

        setup
            .net
            .apply_epoch(EpochNumber::new(1), &window_of(&in_window, 1))
            .unwrap();

        assert_only_window_peers_remain(&mut setup.net, &in_window, &candidate).await;
    }

    /// The fill neither calls `apply_epoch` nor sets the network epoch, so the caller's
    /// `apply_epoch` is not an early return and drops the candidates.
    #[tokio::test]
    async fn fill_leaves_network_epoch_unset_ok() {
        let requester = validator(1, 0);
        let (in_window, candidate) = pushing_peers(&requester).await;
        let mut setup = setup(&[&in_window, &candidate], &requester).await;
        let mut fill = CliquenetBeforeConsensus::new(&mut setup.net, setup.private_key.clone());
        fill.dial(&setup.peers).unwrap();
        let _ = fill
            .fetch_concurrently(&header_proof_request(), Duration::from_millis(100))
            .await;
        let _ = fill
            .next_observed_certificate(Duration::from_millis(50))
            .await;

        setup
            .net
            .apply_epoch(EpochNumber::new(1), &window_of(&in_window, 1))
            .unwrap();

        assert_only_window_peers_remain(&mut setup.net, &in_window, &candidate).await;
    }
}
