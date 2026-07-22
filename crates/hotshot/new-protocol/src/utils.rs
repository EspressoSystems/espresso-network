use anyhow::{anyhow, ensure};
use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    utils::epoch_from_block_number,
    vote::{Certificate, HasViewNumber},
};

use crate::message::Certificate2;

/// Verify that a leaf is finalized by a new-protocol Certificate2.
///
/// `cert2` directly commits the newest leaf in `leaf_chain`. By the indirect
/// commit rule, every ancestor of that leaf is finalized as well. This verifier
/// validates `cert2`, then walks backward through the certified leaf's parent
/// links until it finds `expected_height`.
///
/// View numbers are allowed to skip after timeouts, so the input may contain
/// leaves that are not on the certified ancestry path. Those leaves are ignored;
/// every accepted step must match the current leaf's justify QC, parent
/// commitment, and block height.
pub async fn verify_new_protocol_leaf_chain<T: NodeType>(
    mut leaf_chain: Vec<Leaf2<T>>,
    coordinator: &EpochMembershipCoordinator<T>,
    expected_height: u64,
    upgrade_lock: &UpgradeLock<T>,
    cert2: Certificate2<T>,
) -> anyhow::Result<Leaf2<T>> {
    leaf_chain.sort_by_key(|l| l.view_number());
    leaf_chain.reverse();

    ensure!(!leaf_chain.is_empty(), "empty leaf chain");
    let newest = &leaf_chain[0];

    ensure!(
        cert2.view_number() > ViewNumber::genesis(),
        "cert2 must not be the genesis view"
    );
    let epoch = EpochNumber::new(epoch_from_block_number(
        cert2.data.block_number,
        *coordinator.epoch_height(),
    ));
    ensure!(
        cert2.data.epoch == epoch,
        "cert2 epoch {} does not match epoch {epoch} derived from its block number {}",
        cert2.data.epoch,
        cert2.data.block_number
    );

    let membership = coordinator
        .stake_table_for_epoch(Some(epoch))
        .map_err(|err| anyhow!("no stake table available for epoch {epoch}: {err:?}"))?;
    let entries = StakeTableEntries::<T>::from_iter(membership.stake_table()).0;
    cert2.is_valid_cert(&entries, membership.success_threshold(), upgrade_lock)?;

    ensure!(
        cert2.data.leaf_commit == newest.commit(),
        "cert2 does not match the newest leaf in the chain"
    );
    ensure!(
        cert2.data.block_number == newest.height(),
        "cert2 block number does not match the newest leaf"
    );
    ensure!(
        cert2.view_number() == newest.view_number(),
        "cert2 view does not match the newest leaf"
    );

    if newest.height() == expected_height {
        return Ok(newest.clone());
    }

    let mut current = newest;
    for leaf in leaf_chain[1..].iter() {
        let justify_qc = current.justify_qc();
        if justify_qc.view_number() != leaf.view_number()
            || justify_qc.data().leaf_commit != leaf.commit()
        {
            tracing::warn!(
                view = ?leaf.view_number(),
                expected_view = ?justify_qc.view_number(),
                "leaf is off the leafchain path; expected only after a view timeout"
            );
            continue;
        }
        ensure!(
            current.parent_commitment() == leaf.commit(),
            "current leaf parent commitment does not match parent leaf"
        );
        ensure!(
            leaf.height().checked_add(1) == Some(current.height()),
            "leaf heights do not chain"
        );
        let qc_epoch = EpochNumber::new(epoch_from_block_number(
            leaf.height(),
            *coordinator.epoch_height(),
        ));
        ensure!(
            justify_qc.data().epoch == Some(qc_epoch),
            "justify QC claims epoch {:?} but certifies the leaf at height {} in epoch {qc_epoch}",
            justify_qc.data().epoch,
            leaf.height()
        );
        if let Some(block_number) = justify_qc.data().block_number {
            ensure!(
                block_number == leaf.height(),
                "justify QC claims block number {block_number} but certifies the leaf at height {}",
                leaf.height()
            );
        }
        let membership = coordinator
            .stake_table_for_epoch(Some(qc_epoch))
            .map_err(|err| anyhow!("no stake table available for epoch {qc_epoch}: {err:?}"))?;
        let entries = StakeTableEntries::<T>::from_iter(membership.stake_table()).0;
        justify_qc.is_valid_cert(&entries, membership.success_threshold(), upgrade_lock)?;
        if leaf.height() == expected_height {
            return Ok(leaf.clone());
        }
        current = leaf;
    }

    Err(anyhow!(
        "expected height was not found in the cert2-finalized chain"
    ))
}
