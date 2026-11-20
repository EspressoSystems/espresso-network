//! Headers proven against the LC contract state, for the catchup seed and the stake table
//! backfill.

use anyhow::Context;
use espresso_types::{
    BlockMerkleCommitment, Header, LcContractFinalizedState, traits::StateCatchup,
};

/// The block Merkle root at `lc_contract_state.height`, which `verify_block_merkle_root` accepted. One request; `retry` scales the peers' timeout.
pub(crate) async fn fetch_anchor_root(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
    retry: usize,
) -> anyhow::Result<BlockMerkleCommitment> {
    peers
        .try_fetch_lc_contract_block_merkle_root(retry, lc_contract_state)
        .await
        .with_context(|| {
            format!(
                "fetching the LC contract header at height {}",
                lc_contract_state.height
            )
        })
}

/// The header at `height`, proven against `anchor` from `fetch_anchor_root`. One request.
pub(crate) async fn fetch_proven_header(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
    anchor: BlockMerkleCommitment,
    height: u64,
    retry: usize,
) -> anyhow::Result<Header> {
    peers
        .try_fetch_proven_header(retry, lc_contract_state.height, anchor, height)
        .await
        .with_context(|| format!("fetching the header proof at height {height}"))
}

#[cfg(test)]
pub(crate) mod testing {
    use espresso_types::{LcContractFinalizedState, v0::mock::MockStateCatchup};
    use hotshot_types::{data::ViewNumber, traits::block_contents::BlockHeader};
    use light_client::testing::TestClient;

    pub(crate) const EPOCH_HEIGHT: u64 = 10;

    /// The mock serving the headers of `heights` and of `F = 62`, and the contract state at `F`.
    pub(crate) async fn mock_with_headers(
        heights: &[usize],
    ) -> (MockStateCatchup, LcContractFinalizedState) {
        mock_with_headers_at_lc_height(heights, 62).await
    }

    /// The mock serving the headers of `heights` and of `lc_contract_height`, and the contract
    /// state at `lc_contract_height`.
    pub(crate) async fn mock_with_headers_at_lc_height(
        heights: &[usize],
        lc_contract_height: usize,
    ) -> (MockStateCatchup, LcContractFinalizedState) {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let mut headers = Vec::new();
        for height in heights.iter().copied().chain([lc_contract_height]) {
            headers.push(client.leaf(height).await.leaf().block_header().clone());
        }
        let lc_contract_state = LcContractFinalizedState {
            height: lc_contract_height as u64,
            block_comm_root: headers
                .last()
                .unwrap()
                .get_light_client_state(ViewNumber::new(0))
                .unwrap()
                .block_comm_root,
        };
        (
            MockStateCatchup::default().with_headers(headers),
            lc_contract_state,
        )
    }
}
