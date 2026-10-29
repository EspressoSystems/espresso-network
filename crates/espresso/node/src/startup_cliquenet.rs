//! The node's cliquenet during the startup fill.
#![expect(
    dead_code,
    unused_variables,
    reason = "skeleton: bodies and wiring follow"
)]

use std::time::Duration;

use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::network::Cliquenet;
use hotshot_types::signature_key::BLSPrivKey;
use vec1::Vec1;

use crate::startup_catchup::{
    BootstrapPeers, ObservedCertificate, StartupCliquenet, StartupRequest, StartupResponse,
};

/// The node's `Cliquenet`, borrowed by the fill before the coordinator owns it.
///
/// Sends with `Sender::unicast` and reads with `receive()` only. It never calls `client_api()`,
/// `apply_epoch`, or touches `shared.epoch`. Messages it consumes never reach the coordinator, and
/// inbound requests from other peers are dropped. Each request fans out to `k = 5` bootstrap peers
/// not yet asked, peers that answered before first.
pub(crate) struct CliquenetBeforeConsensus<'a> {
    net: &'a mut Cliquenet<SeqTypes>,
    peers: Vec<PubKey>,
    public_key: PubKey,
    private_key: BLSPrivKey,
}

impl<'a> CliquenetBeforeConsensus<'a> {
    /// Borrows the instance `init` created from the factory, no parties yet. `public_key` is
    /// derived from `private_key`, not passed, so it cannot disagree with
    /// `StartupCatchupParams::own_key`.
    pub fn new(net: &'a mut Cliquenet<SeqTypes>, private_key: BLSPrivKey) -> Self {
        todo!()
    }
}

#[async_trait]
impl StartupCliquenet for CliquenetBeforeConsensus<'_> {
    /// `Cliquenet::add_peers(Role::Active, ..)` with each key converted inline: `NetAddr` from
    /// `hotshot_new_protocol::network`, the x25519 key through `.into()` from `PeerConnectInfo`.
    fn dial(&mut self, peers: &BootstrapPeers) -> anyhow::Result<()> {
        todo!()
    }

    fn accepted_inbound_connections(&self) -> usize {
        todo!()
    }

    /// See the trait. Peers with no answer, an error answer, or an answer failing verification log
    /// at DEBUG; per-stage answered and unanswered counts go to `startup catchup done`.
    async fn fetch_concurrently(
        &mut self,
        requests: &Vec1<StartupRequest>,
        wait: Duration,
    ) -> Vec<anyhow::Result<StartupResponse>> {
        todo!()
    }

    async fn next_observed_certificate(&mut self, wait: Duration) -> Option<ObservedCertificate> {
        todo!()
    }
}
