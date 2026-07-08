use alloy::primitives::U256;
use committable::{Commitment, Committable};
use espresso_types::{Leaf2, SeqTypes};
use hotshot_query_service_types::availability::LeafQueryData;
use hotshot_types::{
    PeerConfig,
    data::ViewNumber,
    stake_table::{HSStakeTable, supermajority_threshold},
};
use light_client::consensus::quorum::StakeTable;
use vbs::version::{StaticVersion, StaticVersionType, Version};

/// Protocol version of the committed decaf fixture.
///
/// A fixture sanity check: re-fetching the fixture under a different protocol
/// version fails loudly in [`verify_leaf`]. The QC2 signature check itself does
/// not depend on this version here (`VersionedVoteData::commit` ignores it in
/// this tree).
pub type FixtureVersion = StaticVersion<0, 5>;

/// Outcome of a successful leaf verification, for the proof journal.
#[derive(Debug)]
pub struct VerifiedLeaf {
    pub height: u64,
    pub epoch: u64,
    pub commitment: Commitment<Leaf2>,
    pub threshold: U256,
}

pub fn parse_leaf(bytes: &[u8]) -> anyhow::Result<LeafQueryData<SeqTypes>> {
    Ok(serde_json::from_slice(bytes)?)
}

pub fn parse_stake_table(bytes: &[u8]) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
    Ok(serde_json::from_slice(bytes)?)
}

/// Verify a leaf and its QC against the given stake table, using the
/// light-client crate's quorum verification (the path espresso-stack's
/// `LocalClient` runs).
///
/// Checks, in order: the header protocol version matches [`FixtureVersion`],
/// the QC view is past the genesis view, the recomputed leaf commitment
/// matches the QC, and the QC carries a valid supermajority threshold
/// signature of the stake table.
pub fn verify_leaf(
    leaf_data: &LeafQueryData<SeqTypes>,
    peers: Vec<PeerConfig<SeqTypes>>,
) -> anyhow::Result<VerifiedLeaf> {
    let leaf = leaf_data.leaf();
    let qc = leaf_data.qc();

    let header_version = leaf.block_header().version();
    let pinned = Version {
        major: FixtureVersion::MAJOR,
        minor: FixtureVersion::MINOR,
    };
    anyhow::ensure!(
        header_version == pinned,
        "header version {header_version} does not match pinned version {pinned}"
    );

    // `is_valid_cert` short-circuits Ok at the genesis view, so a genesis-view
    // QC would pass without any signature check.
    anyhow::ensure!(
        qc.view_number > ViewNumber::genesis(),
        "QC view {:?} is not past the genesis view",
        qc.view_number
    );

    let commitment = Committable::commit(leaf);
    anyhow::ensure!(
        qc.data.leaf_commit == commitment,
        "QC leaf commitment {} does not match recomputed leaf commitment {commitment}",
        qc.data.leaf_commit
    );

    let height = leaf.height();
    anyhow::ensure!(
        qc.data.block_number == Some(height),
        "QC block number {:?} does not match leaf height {height}",
        qc.data.block_number
    );
    let epoch = qc
        .data
        .epoch
        .map(|epoch| *epoch)
        .ok_or_else(|| anyhow::anyhow!("QC has no epoch"))?;

    // `StakeTable` computes the supermajority threshold internally but does not
    // expose it; recompute it the same way for the journal.
    let total_stake: U256 = peers
        .iter()
        .map(|peer| peer.stake_table_entry.stake_amount)
        .sum();
    let threshold = supermajority_threshold(total_stake);

    let stake_table = StakeTable::from(HSStakeTable(peers));
    // `verify_cert` is async but never awaits; polling it once completes.
    futures::executor::block_on(stake_table.verify_cert::<FixtureVersion, _>(qc))?;

    Ok(VerifiedLeaf {
        height,
        epoch,
        commitment,
        threshold,
    })
}

#[cfg(test)]
mod tests {
    use tagged_base64::TaggedBase64;

    use super::*;

    const LEAF_JSON: &[u8] = include_bytes!("../fixtures/leaf_query_data.json");
    const STAKE_TABLE_JSON: &[u8] = include_bytes!("../fixtures/stake_table.json");

    fn fixture() -> (LeafQueryData<SeqTypes>, Vec<PeerConfig<SeqTypes>>) {
        (
            parse_leaf(LEAF_JSON).unwrap(),
            parse_stake_table(STAKE_TABLE_JSON).unwrap(),
        )
    }

    fn assert_err_contains<T: std::fmt::Debug>(result: anyhow::Result<T>, needle: &str) {
        let message = format!("{:#}", result.unwrap_err());
        assert!(
            message.contains(needle),
            "error {message:?} does not contain {needle:?}"
        );
    }

    #[test]
    fn verify_decaf_leaf() {
        let (leaf_data, peers) = fixture();

        // The genesis-view short-circuit in `is_valid_cert` must not apply.
        assert!(
            leaf_data.qc().view_number > ViewNumber::genesis(),
            "fixture QC is at the genesis view; verification would be vacuous"
        );

        let verified = verify_leaf(&leaf_data, peers).unwrap();
        assert_eq!(verified.height, 10541613);
        assert_eq!(verified.epoch, 3514);
        assert_eq!(
            verified.commitment,
            "COMMIT~wlx_EbkGEGdgZO4_t6VYDXTT3RtMcPvNAfnlX4Jri98X"
                .parse()
                .unwrap()
        );
        assert!(verified.threshold > U256::ZERO);
    }

    /// A crafted genesis-view QC without signatures hits the `is_valid_cert`
    /// genesis short-circuit; only the view guard can reject it.
    #[test]
    fn reject_genesis_view_qc() {
        let (mut leaf_data, peers) = fixture();

        leaf_data.qc.view_number = ViewNumber::genesis();
        leaf_data.qc.signatures = None;
        // The leaf is unchanged, so `qc.data.leaf_commit` still matches it.
        assert_eq!(leaf_data.qc.data.leaf_commit, leaf_data.leaf.commit());

        assert_err_contains(
            verify_leaf(&leaf_data, peers),
            "is not past the genesis view",
        );
    }

    /// Flipping one byte of the aggregate signature must fail, either at parse
    /// time (the corrupted bytes are no longer a valid curve point) or at
    /// signature verification.
    #[test]
    fn reject_corrupted_signature() {
        let (_, peers) = fixture();

        let mut json: serde_json::Value = serde_json::from_slice(LEAF_JSON).unwrap();
        let sig = json["qc"]["signatures"][0].as_str().unwrap();
        let tb64: TaggedBase64 = sig.parse().unwrap();
        let mut bytes = tb64.value();
        bytes[0] ^= 0x01;
        json["qc"]["signatures"][0] = TaggedBase64::new(&tb64.tag(), &bytes)
            .unwrap()
            .to_string()
            .into();

        let result = serde_json::to_vec(&json)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| parse_leaf(&bytes))
            .and_then(|leaf_data| verify_leaf(&leaf_data, peers).map(|_| ()));
        assert!(result.is_err());
    }

    /// Replacing the aggregate signature with a well-formed signature over
    /// different data (the leaf's own justify QC) must fail the threshold
    /// signature check.
    #[test]
    fn reject_swapped_signature() {
        let (mut leaf_data, peers) = fixture();

        leaf_data.qc.signatures = leaf_data.leaf.justify_qc().signatures;
        assert_err_contains(
            verify_leaf(&leaf_data, peers),
            "invalid threshold signature",
        );
    }

    /// Zeroing out all stake makes the supermajority threshold unreachable.
    #[test]
    fn reject_zeroed_stake_table() {
        let (leaf_data, mut peers) = fixture();

        for peer in &mut peers {
            peer.stake_table_entry.stake_amount = U256::ZERO;
        }
        assert_err_contains(
            verify_leaf(&leaf_data, peers),
            "invalid threshold signature",
        );
    }

    /// Dropping stake table entries must fail (the signer set no longer
    /// matches the committee).
    #[test]
    fn reject_truncated_stake_table() {
        let (leaf_data, mut peers) = fixture();

        peers.truncate(peers.len() / 2);
        assert_err_contains(
            verify_leaf(&leaf_data, peers),
            "invalid threshold signature",
        );
    }

    /// A QC referencing a different leaf commitment must fail the recomputed
    /// commitment check.
    #[test]
    fn reject_wrong_leaf_commit() {
        let (mut leaf_data, peers) = fixture();

        leaf_data.qc.data.leaf_commit = leaf_data.leaf.parent_commitment();
        assert_err_contains(
            verify_leaf(&leaf_data, peers),
            "does not match recomputed leaf commitment",
        );
    }

    /// Tampering with a header field changes `leaf.commit()`; the
    /// `LeafQueryData` deserialization invariant must already reject it.
    #[test]
    fn reject_tampered_header() {
        let mut json: serde_json::Value = serde_json::from_slice(LEAF_JSON).unwrap();
        let fields = &mut json["leaf"]["block_header"]["fields"];
        let timestamp = fields["timestamp"].as_u64().unwrap();
        fields["timestamp"] = (timestamp + 1).into();

        let bytes = serde_json::to_vec(&json).unwrap();
        assert_err_contains(parse_leaf(&bytes), "QC references leaf");
    }
}
