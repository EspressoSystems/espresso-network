//! Signing the light client state of each decided block for the state relay server.
//!
//! The light client contract on L1 holds a recent finalized Espresso state, which a prover
//! updates with a proof that enough stake signed it. Every validator signs the state of each
//! decided block with its Schnorr key and sends the signature to the relay server, which collects
//! them for the prover. A lost signature is not sent again: the relay server takes the signatures
//! of a later block instead. The states of an epoch's last blocks, from its root on, are left out;
//! consensus signs those itself, in the state certificate of the epoch root.

use alloy::primitives::FixedBytes;
use espresso_types::{Header, SeqTypes};
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::{
        LCV3StateSignatureRequestBody, LightClientState, StakeTableState, StateKeyPair,
        StateSignKey, StateVerKey,
    },
    signature_key::SchnorrPubKey,
    stake_table::HSStakeTable,
    traits::{
        block_contents::BlockHeader,
        election::Membership as _,
        signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, StakeTableEntryType},
    },
    utils::{is_ge_epoch_root, option_epoch_from_block_number},
};
use http_client::{Client, error::ClientErr};
use tokio::sync::broadcast;
use url::Url;
use vbs::version::StaticVersion;

/// The API version the relay server speaks.
type RelayApiVersion = StaticVersion<0, 1>;

/// The newest leaf of a decide, as far as signing goes.
#[derive(Clone, Debug)]
pub struct Decided {
    pub header: Header,
    pub view: ViewNumber,
}

/// Signs the state of each decided block and sends the signature to the relay server.
pub struct StateSigner {
    sign_key: StateSignKey,
    ver_key: StateVerKey,
    relay: Client<ClientErr, RelayApiVersion>,
    membership: EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    stake_table_capacity: usize,

    /// The stake table that signs the current states: its epoch, its commitment, and whether this
    /// node is in it.
    voting: Option<(Option<EpochNumber>, StakeTableState, bool)>,
}

impl StateSigner {
    pub fn new(
        sign_key: StateSignKey,
        relay: Url,
        membership: EpochMembershipCoordinator<SeqTypes>,
        epoch_height: u64,
        stake_table_capacity: usize,
    ) -> Self {
        let ver_key = StateKeyPair::from_sign_key(sign_key.clone()).ver_key();
        Self {
            sign_key,
            ver_key,
            relay: Client::new(relay),
            membership,
            epoch_height,
            stake_table_capacity,
            voting: None,
        }
    }

    /// Signs the decides received on `decided` until it closes. A signer that falls behind skips
    /// the oldest, since the relay server wants the newest.
    pub async fn run(mut self, mut decided: broadcast::Receiver<Decided>) {
        loop {
            match decided.recv().await {
                Ok(leaf) => self.sign(&leaf).await,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "the state signer fell behind")
                },
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    async fn sign(&mut self, leaf: &Decided) {
        match self.request(leaf) {
            Ok(Some(body)) => {
                let height = body.state.block_height;
                let sent = match self.relay.post::<()>("api/state").body_binary(&body) {
                    Ok(request) => request.send().await.map_err(|err| format!("{err:?}")),
                    Err(err) => Err(format!("{err:?}")),
                };
                if let Err(err) = sent {
                    tracing::warn!(height, "sending the state signature failed: {err}");
                }
            },
            Ok(None) => {},
            Err(err) => tracing::warn!(
                height = leaf.header.height(),
                "cannot sign the light client state: {err:#}"
            ),
        }
    }

    /// The signed state of `leaf`, or `None` if this node does not sign it.
    pub fn request(
        &mut self,
        leaf: &Decided,
    ) -> anyhow::Result<Option<LCV3StateSignatureRequestBody>> {
        let state = leaf.header.get_light_client_state(leaf.view)?;
        let height = state.block_height;
        // Consensus signs these, in the state certificate of the epoch root.
        if is_ge_epoch_root(height, self.epoch_height) {
            return Ok(None);
        }
        let epoch = option_epoch_from_block_number(true, height, self.epoch_height);
        let (stake_table_state, in_stake_table) = self.voting_stake_table(epoch)?;
        if !in_stake_table {
            return Ok(None);
        }
        let auth_root = leaf.header.auth_root()?;
        Ok(Some(self.signed(state, stake_table_state, auth_root)?))
    }

    /// The commitment of the stake table that signs the states of `epoch`, and whether this node
    /// is in it.
    fn voting_stake_table(
        &mut self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<(StakeTableState, bool)> {
        if let Some((held, state, in_it)) = &self.voting
            && *held == epoch
        {
            return Ok((*state, *in_it));
        }
        let table = match epoch {
            // States from before epochs are signed by the genesis stake table.
            None => {
                let genesis = self.membership.membership().genesis_stake_tables();
                HSStakeTable::from_iter(genesis.stake_table())
            },
            Some(epoch) => {
                let membership = self
                    .membership
                    .stake_table_for_epoch(epoch)
                    .map_err(|err| anyhow::anyhow!("no stake table of epoch {epoch}: {err}"))?;
                HSStakeTable::from_iter(membership.stake_table())
            },
        };
        let state = table.commitment(self.stake_table_capacity)?;
        let in_it = table.0.iter().any(|peer| {
            peer.state_ver_key == self.ver_key && !peer.stake_table_entry.stake().is_zero()
        });
        self.voting = Some((epoch, state, in_it));
        Ok((state, in_it))
    }

    fn signed(
        &self,
        state: LightClientState,
        next_stake: StakeTableState,
        auth_root: FixedBytes<32>,
    ) -> anyhow::Result<LCV3StateSignatureRequestBody> {
        let digest = derive_signed_state_digest(&state, &next_stake, &auth_root);
        let signature =
            <SchnorrPubKey as LCV3StateSignatureKey>::sign_state(&self.sign_key, digest)?;
        let v2_signature = <SchnorrPubKey as LCV2StateSignatureKey>::sign_state(
            &self.sign_key,
            &state,
            &next_stake,
        )?;
        Ok(LCV3StateSignatureRequestBody {
            key: self.ver_key.clone(),
            state,
            next_stake,
            signature,
            v2_signature,
            auth_root,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroU64, sync::Arc, time::Duration};

    use alloy::primitives::U256;
    use async_lock::Mutex;
    use axum::{Router, body::Bytes, extract::State, routing::post};
    use espresso_types::{
        EpochCommittees, L1ClientOptions, Leaf2, NodeState, PubKey, ValidatedState,
        v0_3::{ChainConfig, Fetcher},
    };
    use hotshot_types::{PeerConfig, traits::signature_key::SignatureKey};
    use membership::{MembershipStore, null_catchup::NullStateCatchup};
    use tokio::{net::TcpListener, sync::mpsc};
    use vbs::{BinarySerializer, Serializer};
    use versions::NEW_PROTOCOL_VERSION;

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;
    const SIGNERS: u64 = 4;

    fn state_key(i: u64) -> StateKeyPair {
        StateKeyPair::generate_from_seed_indexed([0; 32], i)
    }

    /// A membership on a fixed committee for the first two epochs, which reads nothing from L1.
    async fn membership() -> (tempfile::TempDir, EpochMembershipCoordinator<SeqTypes>) {
        let committee: Vec<_> = (0..SIGNERS)
            .map(|i| PeerConfig {
                stake_table_entry: PubKey::generated_from_seed_indexed([0; 32], i)
                    .0
                    .stake_table_entry(U256::from(100)),
                state_ver_key: state_key(i).ver_key(),
                connect_info: None,
            })
            .collect();
        let dir = tempfile::tempdir().unwrap();
        let (store, _) = crate::store::open(dir.path().join("store")).await.unwrap();
        let store = MembershipStore::new(store);
        let l1 = L1ClientOptions::default()
            .connect(vec!["http://127.0.0.1:1".parse().unwrap()])
            .unwrap();
        let fetcher = Fetcher::new(
            Arc::new(NullStateCatchup::default()),
            Arc::new(Mutex::new(store.clone())),
            l1,
            ChainConfig::default(),
        );
        let committees = EpochCommittees::new(
            committee.clone(),
            committee,
            None,
            fetcher,
            EPOCH_HEIGHT,
            EpochNumber::new(1),
        )
        .unwrap();
        let length = NonZeroU64::new(EPOCH_HEIGHT).unwrap();
        (
            dir,
            EpochMembershipCoordinator::new(committees, length, &store),
        )
    }

    async fn decided(height: u64) -> Decided {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let mut leaf = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
        *leaf.block_header_mut().height_mut() = height;
        Decided {
            header: leaf.block_header().clone(),
            view: ViewNumber::new(height + 1),
        }
    }

    fn signer(
        key: u64,
        membership: &EpochMembershipCoordinator<SeqTypes>,
        relay: Url,
    ) -> StateSigner {
        StateSigner::new(
            state_key(key).sign_key(),
            relay,
            membership.clone(),
            EPOCH_HEIGHT,
            SIGNERS as usize,
        )
    }

    /// A member signs the state of a block before the epoch root, with both signatures over the
    /// state and the commitment of its epoch's stake table.
    #[tokio::test]
    async fn signs_the_state_of_a_decided_block() {
        let (_dir, membership) = membership().await;
        let relay: Url = "http://127.0.0.1:1".parse().unwrap();
        let leaf = decided(3).await;
        let body = signer(0, &membership, relay.clone())
            .request(&leaf)
            .unwrap()
            .expect("a member signs");
        assert_eq!(
            body.state,
            leaf.header.get_light_client_state(leaf.view).unwrap()
        );
        let digest = derive_signed_state_digest(&body.state, &body.next_stake, &body.auth_root);
        assert!(<SchnorrPubKey as LCV3StateSignatureKey>::verify_state_sig(
            &body.key,
            &body.signature,
            digest
        ));
        assert!(<SchnorrPubKey as LCV2StateSignatureKey>::verify_state_sig(
            &body.key,
            &body.v2_signature,
            &body.state,
            &body.next_stake
        ));

        // Consensus signs the last blocks of an epoch, from its root on.
        let root = hotshot_types::utils::root_block_in_epoch(1, EPOCH_HEIGHT);
        let mut signer0 = signer(0, &membership, relay.clone());
        assert!(signer0.request(&decided(root).await).unwrap().is_none());
        // A key outside the stake table signs nothing.
        assert!(
            signer(9, &membership, relay)
                .request(&leaf)
                .unwrap()
                .is_none()
        );
    }

    /// The signature reaches the relay server as the relay server decodes it.
    #[tokio::test]
    async fn sends_the_signature_to_the_relay_server() {
        let (_dir, membership) = membership().await;
        let (received_tx, mut received) = mpsc::unbounded_channel::<Bytes>();
        let app = Router::new()
            .route(
                "/api/state",
                post(
                    |State(tx): State<mpsc::UnboundedSender<Bytes>>, body: Bytes| async move {
                        let _ = tx.send(body);
                    },
                ),
            )
            .with_state(received_tx);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let relay: Url = format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });

        let (tx, rx) = broadcast::channel(4);
        tokio::spawn(signer(1, &membership, relay).run(rx));
        let leaf = decided(4).await;
        tx.send(leaf.clone()).unwrap();
        let body = tokio::time::timeout(Duration::from_secs(10), received.recv())
            .await
            .expect("the relay server receives a signature")
            .unwrap();
        let body: LCV3StateSignatureRequestBody =
            Serializer::<RelayApiVersion>::deserialize(&body).unwrap();
        assert_eq!(body.key, state_key(1).ver_key());
        assert_eq!(body.state.block_height, 4);
    }
}
