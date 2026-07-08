use alloy::primitives::U256;
use committable::{Commitment, Committable};
use espresso_types::{AuthenticatedValidatorMap, Leaf2, SeqTypes};
use hotshot_query_service_types::availability::LeafQueryData;
use hotshot_types::{
    PeerConfig,
    message::UpgradeLock,
    signature_key::BLSPubKey,
    stake_table::{HSStakeTable, StakeTableEntries},
    traits::signature_key::SignatureKey,
    vote::Certificate,
};
use vbs::{BinarySerializer, version::StaticVersion};
use versions::Upgrade;

type SerializerV3 = vbs::Serializer<StaticVersion<0, 3>>;

/// Decode a vbs-encoded `LeafQueryData` (v3 serialization format).
///
/// Deserialization already enforces that the QC references the contained leaf.
pub fn decode_leaf(bytes: &[u8]) -> anyhow::Result<LeafQueryData<SeqTypes>> {
    SerializerV3::deserialize(bytes)
}

/// Build a HotShot stake table from registered validators (e.g. the decaf fixture).
pub fn stake_table_from_validators(
    validators: &AuthenticatedValidatorMap,
) -> HSStakeTable<SeqTypes> {
    HSStakeTable(
        validators
            .values()
            .map(|v| PeerConfig {
                stake_table_entry: BLSPubKey::stake_table_entry(v.stake_table_key(), v.stake),
                state_ver_key: v.state_ver_key().clone(),
                connect_info: None,
            })
            .collect(),
    )
}

/// Recompute the leaf commitment, check it against the QC, and verify the QC
/// signatures against the stake table. Returns the leaf height and commitment.
pub fn verify_leaf(
    leaf_data: &LeafQueryData<SeqTypes>,
    stake_table: HSStakeTable<SeqTypes>,
    success_threshold: U256,
    upgrade: Upgrade,
) -> anyhow::Result<(u64, Commitment<Leaf2>)> {
    let leaf = leaf_data.leaf();
    let qc = leaf_data.qc();

    let commit = Committable::commit(leaf);
    anyhow::ensure!(
        qc.data.leaf_commit == commit,
        "QC leaf commitment does not match recomputed leaf commitment"
    );

    let entries = StakeTableEntries::<SeqTypes>::from(stake_table).0;
    let upgrade_lock = UpgradeLock::<SeqTypes>::new(upgrade);
    qc.is_valid_cert(&entries, success_threshold, &upgrade_lock)
        .map_err(|err| anyhow::anyhow!("invalid QC: {err:?}"))?;

    Ok((leaf.height(), commit))
}

#[cfg(test)]
mod tests {
    use espresso_types::MOCK_SEQUENCER_VERSIONS;

    use super::*;

    #[test]
    fn verify_reference_leaf_against_decaf_stake_table() {
        let leaf_bytes = include_bytes!("../../../data/v3/leaf_query_data.bin");
        let leaf_data = decode_leaf(leaf_bytes).unwrap();

        let validators: AuthenticatedValidatorMap =
            serde_json::from_str(include_str!("../../../data/v3/decaf_stake_table.json")).unwrap();
        let stake_table = stake_table_from_validators(&validators);
        let total_stake = stake_table.total_stakes();
        let success_threshold = total_stake * U256::from(2) / U256::from(3) + U256::from(1);

        let (height, commit) = verify_leaf(
            &leaf_data,
            stake_table,
            success_threshold,
            MOCK_SEQUENCER_VERSIONS,
        )
        .unwrap();

        // The reference leaf is the genesis leaf; its QC passes `is_valid_cert`
        // by the genesis-view short-circuit, not by checking signatures against
        // the decaf committee.
        assert_eq!(height, 0);
        assert_eq!(commit, leaf_data.hash());
    }
}
