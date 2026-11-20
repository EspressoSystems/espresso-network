//! What the ingester publishes for the API instances besides blocks and state: the stake table and
//! the discovery records of each epoch.
//!
//! The stake tables come from membership, as the chain follower and the gap filler hand it epoch
//! roots. The records come from the deployment's consensus node: when the newest stored height
//! enters an epoch, the publisher asks it for the epoch's records and stores the valid ones.
//! Validators send their records as the epoch starts, so while some validators of the epoch have
//! none stored, it asks again every [`RECORDS_RETRY`].

use std::{collections::BTreeSet, net::SocketAddr, time::Duration};

use chain_api::Tables;
use chain_store::ChainStore;
use consensus_api::{
    DecideLogClient, discovery::SignedServiceRecord, proto::ServiceRecordsRequest,
};
use espresso_types::SeqTypes;
use hotshot_types::data::EpochNumber;
use membership::RETAINED_EPOCHS;
use tokio::time::{Instant, sleep};
use tonic::{Code, transport::Channel};

use crate::{membership::Membership, peers::valid_records};

/// The least time between two passes.
const PASS_INTERVAL: Duration = Duration::from_secs(1);

/// How long to wait before asking consensus again for the records of the epoch, while some of
/// its validators have none.
const RECORDS_RETRY: Duration = Duration::from_secs(30);

pub struct Publisher {
    store: ChainStore,
    tables: Tables,
    membership: Membership,

    /// The channel to the deployment's consensus node.
    consensus: Channel,

    /// The epochs whose stake tables are stored, from the earliest still published on.
    stake_tables: BTreeSet<EpochNumber>,

    /// The epoch whose records were last asked for, whether every validator of the epoch had one,
    /// and when.
    records: Option<(EpochNumber, bool, Instant)>,
}

impl Publisher {
    pub fn new(store: ChainStore, membership: Membership, consensus_api: SocketAddr) -> Self {
        Self {
            tables: Tables::new(store.clone()),
            store,
            membership,
            consensus: consensus_api::channel(consensus_api),
            stake_tables: BTreeSet::new(),
            records: None,
        }
    }

    /// Publishes whenever the store changes, at most once per [`PASS_INTERVAL`], and at least
    /// once per [`RECORDS_RETRY`]. Returns when the store's changes end.
    pub async fn run(mut self) {
        let mut changes = self.store.changes();
        loop {
            let started = Instant::now();
            changes.borrow_and_update();
            if let Err(err) = self.pass().await {
                tracing::warn!("publishing failed: {err:#}");
            }
            sleep(PASS_INTERVAL.saturating_sub(started.elapsed())).await;
            tokio::select! {
                changed = changes.changed() => if changed.is_err() {
                    return;
                },
                () = sleep(RECORDS_RETRY) => {},
            }
        }
    }

    async fn pass(&mut self) -> anyhow::Result<()> {
        let Some(newest) = self.store.newest().await? else {
            return Ok(());
        };
        let epoch = self.membership.epoch_of(newest);
        self.publish_stake_tables(epoch).await?;
        self.publish_records(epoch).await
    }

    /// Stores the stake tables membership has, of the epochs from [`RETAINED_EPOCHS`] before
    /// `epoch` to the latest one it can know, two after `epoch`.
    async fn publish_stake_tables(&mut self, epoch: EpochNumber) -> anyhow::Result<()> {
        let from = self
            .membership
            .first_epoch()
            .max(EpochNumber::new(epoch.saturating_sub(RETAINED_EPOCHS)));
        self.stake_tables = self.stake_tables.split_off(&from);
        for e in *from..=*epoch + 2 {
            let e = EpochNumber::new(e);
            if self.stake_tables.contains(&e) {
                continue;
            }
            if let Some(table) = self.membership.stake_table(e) {
                self.tables.put_stake_table(e, &table.0).await?;
                self.stake_tables.insert(e);
            }
        }
        Ok(())
    }

    /// Stores the valid records consensus holds for `epoch`, unless they are stored already and
    /// every validator of the epoch has one, or they were asked for less than [`RECORDS_RETRY`]
    /// ago.
    async fn publish_records(&mut self, epoch: EpochNumber) -> anyhow::Result<()> {
        if let Some((asked, complete, at)) = self.records
            && asked == epoch
            && (complete || at.elapsed() < RECORDS_RETRY)
        {
            return Ok(());
        }
        // The records are checked against it.
        let Some(table) = self.membership.stake_table(epoch) else {
            return Ok(());
        };
        let records = service_records(self.consensus.clone(), epoch).await?;
        let valid = valid_records(records, epoch, &self.membership);
        let complete = valid.len() >= table.0.len();
        if !valid.is_empty() {
            let keep_from = EpochNumber::new(epoch.saturating_sub(RETAINED_EPOCHS));
            self.tables
                .put_service_records(epoch, &valid, keep_from)
                .await?;
            tracing::info!(%epoch, records = valid.len(), "stored the records of the epoch");
        }
        self.records = Some((epoch, complete, Instant::now()));
        Ok(())
    }
}

/// The records the consensus node behind `consensus` holds for `epoch`; those that do not decode
/// are left out.
async fn service_records(
    consensus: Channel,
    epoch: EpochNumber,
) -> anyhow::Result<Vec<SignedServiceRecord<SeqTypes>>> {
    let mut client = DecideLogClient::new(consensus);
    let response = match client
        .get_service_records(ServiceRecordsRequest {
            epoch: Some(*epoch),
        })
        .await
    {
        Ok(response) => response.into_inner(),
        Err(status) if status.code() == Code::NotFound => return Ok(Vec::new()),
        Err(status) => anyhow::bail!("asking consensus for records: {status}"),
    };
    Ok(response
        .records
        .iter()
        .filter_map(|bytes| {
            bincode::deserialize::<SignedServiceRecord<SeqTypes>>(bytes)
                .inspect_err(|err| tracing::warn!(%err, "an undecodable service record"))
                .ok()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::{
        net::Ipv4Addr,
        ops::Range,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use async_trait::async_trait;
    use bytes::Bytes;
    use chain_store::{Item, Parts, mem::MemBackend};
    use consensus_api::{Error, Source, discovery::ServiceRecord, proto::Decided};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use tokio::sync::watch;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;
    use crate::membership::testing::*;

    /// A consensus node that holds service records only, and counts how often it is asked.
    struct Records {
        epoch: u64,
        records: Vec<Bytes>,
        asked: AtomicUsize,
        changes: watch::Sender<u64>,
    }

    #[async_trait]
    impl Source for Records {
        async fn decided(&self, _: u64, _: usize) -> Result<Vec<Decided>, Error> {
            Ok(Vec::new())
        }

        async fn earliest(&self) -> Result<Option<u64>, Error> {
            Ok(None)
        }

        fn changes(&self) -> watch::Receiver<u64> {
            self.changes.subscribe()
        }

        async fn ack(&self, _: Vec<Range<u64>>) -> Result<(), Error> {
            Ok(())
        }

        async fn payload(&self, _: u64) -> Result<Option<Bytes>, Error> {
            Ok(None)
        }

        async fn service_records(
            &self,
            epoch: Option<u64>,
        ) -> Result<Option<(u64, Vec<Bytes>)>, Error> {
            self.asked.fetch_add(1, Ordering::Relaxed);
            Ok((epoch == Some(self.epoch)).then(|| (self.epoch, self.records.clone())))
        }
    }

    /// A record of `epoch`, signed by the test validator `signer`.
    fn record(signer: u64, epoch: u64) -> Bytes {
        let (public, private) = bls_key(signer);
        let record: SignedServiceRecord<SeqTypes> = ServiceRecord {
            epoch: EpochNumber::new(epoch),
            api_url: format!("http://127.0.0.1:{}", 10 + signer),
            api_key: None,
            sequence: 1,
        }
        .sign(public, &private)
        .unwrap();
        bincode::serialize(&record).unwrap().into()
    }

    async fn serve(source: Arc<Records>) -> SocketAddr {
        let port = test_utils::reserve_tcp_port().unwrap();
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(consensus_api::server(source))
                .serve(addr),
        );
        addr
    }

    /// The stake tables membership has and the valid records of the newest height's epoch are
    /// stored; consensus is not asked again soon while validators lack records.
    #[tokio::test]
    async fn publishes_stake_tables_and_valid_records() {
        let membership = membership();
        let epoch = FIRST_EPOCH + 1;
        let source = Arc::new(Records {
            epoch,
            records: vec![
                record(0, epoch),
                record(1, epoch + 1),
                record(SIGNERS + 1, epoch),
            ],
            asked: AtomicUsize::new(0),
            changes: watch::channel(0).0,
        });
        let addr = serve(source.clone()).await;
        let store = ChainStore::new(MemBackend::default());
        let leaf = leaf(membership.heights_of(EpochNumber::new(epoch)).start + 1).await;
        let cert1 = QuorumCertificate2::genesis(
            &Default::default(),
            &espresso_types::NodeState::mock(),
            Upgrade::trivial(NEW_PROTOCOL_VERSION),
        )
        .await;
        store
            .put(&Item { leaf, cert1 }, &Parts::default())
            .await
            .unwrap();
        let tables = Tables::new(store.clone());
        let mut publisher = Publisher::new(store, membership, addr);

        tokio::time::timeout(Duration::from_secs(10), async {
            while publisher.pass().await.is_err() {
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("consensus answers");
        for e in [FIRST_EPOCH, epoch] {
            let table = tables.stake_table(EpochNumber::new(e)).await.unwrap();
            assert_eq!(table, Some(committee()), "epoch {e}");
        }
        assert_eq!(
            tables
                .stake_table(EpochNumber::new(epoch + 1))
                .await
                .unwrap(),
            None,
            "membership derives no later stake table without L1"
        );
        let (stored_epoch, records) = tables.service_records(None).await.unwrap().unwrap();
        assert_eq!(*stored_epoch, epoch);
        let signers: Vec<_> = records.iter().map(|record| record.signer).collect();
        assert_eq!(signers, vec![bls_key(0).0]);

        let asked = source.asked.load(Ordering::Relaxed);
        publisher.pass().await.unwrap();
        assert_eq!(source.asked.load(Ordering::Relaxed), asked);
    }
}
