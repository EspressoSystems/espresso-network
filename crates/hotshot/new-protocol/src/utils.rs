use alloy::primitives::U256;
use anyhow::{anyhow, ensure};
use committable::Committable;
use hotshot_types::{
    PeerConfig,
    data::Leaf2,
    message::UpgradeLock,
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
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
pub async fn verify_leaf_chain_with_cert2<T: NodeType>(
    mut leaf_chain: Vec<Leaf2<T>>,
    stake_table: &[PeerConfig<T>],
    success_threshold: U256,
    expected_height: u64,
    upgrade_lock: &UpgradeLock<T>,
    cert2: Certificate2<T>,
) -> anyhow::Result<Leaf2<T>> {
    leaf_chain.sort_by_key(|l| l.view_number());
    leaf_chain.reverse();

    ensure!(!leaf_chain.is_empty(), "empty leaf chain");

    let stake_table_entries = StakeTableEntries::<T>::from(stake_table.to_vec()).0;

    cert2.is_valid_cert(&stake_table_entries, success_threshold, upgrade_lock)?;

    ensure!(
        cert2.data.leaf_commit == leaf_chain[0].commit(),
        "cert2 does not match the newest leaf in the chain"
    );
    ensure!(
        cert2.data.block_number == leaf_chain[0].height(),
        "cert2 block number does not match the newest leaf"
    );
    ensure!(
        cert2.view_number() == leaf_chain[0].view_number(),
        "cert2 view does not match the newest leaf"
    );

    if leaf_chain[0].height() == expected_height {
        return Ok(leaf_chain[0].clone());
    }

    let mut current = &leaf_chain[0];
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
        justify_qc.is_valid_cert(&stake_table_entries, success_threshold, upgrade_lock)?;
        if leaf.height() == expected_height {
            return Ok(leaf.clone());
        }
        current = leaf;
    }

    Err(anyhow!(
        "expected height was not found in the cert2-finalized chain"
    ))
}
