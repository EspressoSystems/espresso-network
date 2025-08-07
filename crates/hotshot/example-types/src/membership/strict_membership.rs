use std::{collections::HashSet, fmt::Debug, marker::PhantomData, sync::Arc, time::Duration};

use hotshot_types::{
    data::Leaf2,
    drb::DrbResult,
    stake_table::HSStakeTable,
    traits::{
        election::Membership,
        node_implementation::{NodeImplementation, NodeType, Versions},
        signature_key::SignatureKey,
    },
    PeerConfig,
};

use crate::membership::stake_table::TestStakeTable;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrictMembership<
    TYPES: NodeType,
    V: Versions,
    StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
> {
    inner: StakeTable,
    epochs: HashSet<TYPES::Epoch>,
    drbs: HashSet<TYPES::Epoch>,
    _phantom: PhantomData<V>,
}

impl<
        TYPES: NodeType,
        V: Versions,
        StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
    > StrictMembership<TYPES, V, StakeTable>
{
    fn assert_has_stake_table(&self, epoch: Option<TYPES::Epoch>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.epochs.contains(&epoch),
            "Failed stake table check for epoch {epoch}"
        );
    }
    fn assert_has_randomized_stake_table(&self, epoch: Option<TYPES::Epoch>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.drbs.contains(&epoch),
            "Failed drb check for epoch {epoch}"
        );
    }
}

impl<
        TYPES: NodeType,
        V: Versions,
        StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
    > Membership<TYPES> for StrictMembership<TYPES, V, StakeTable>
{
    type Error = anyhow::Error;

    fn new<I: NodeImplementation<TYPES>>(
        quorum_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        da_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        storage: <I as NodeImplementation<TYPES>>::Storage,
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
        public_key: TYPES::SignatureKey,
    ) -> Self {
        Self {
            inner: TestStakeTable::new(
                quorum_members.into_iter().map(Into::into).collect(),
                da_members.into_iter().map(Into::into).collect(),
            ),
            epochs: HashSet::new(),
            drbs: HashSet::new(),
            _phantom: PhantomData,
        }
    }

    fn stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        let peer_configs = self.inner.stake_table(epoch.map(|epoch| *epoch)).into_iter().map(Into::into).collect();
        HSStakeTable(
            peer_configs
        )
    }
}
