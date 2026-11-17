//! Headers proven against the LC contract state, for the catchup seed and the stake table
//! backfill.

use std::{
    future::Future,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::Context;
use espresso_types::{
    BackoffParams, BlockMerkleCommitment, Header, LcContractFinalizedState, traits::StateCatchup,
};

/// Retries of one request after its first attempt.
pub(crate) const REQUEST_RETRIES: u32 = 4;

/// The block Merkle root of the header at `lc_contract_state.height`, which `verify_header`
/// accepted.
pub(crate) async fn fetch_anchor_root(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
) -> anyhow::Result<BlockMerkleCommitment> {
    retry_request(|retry| peers.try_fetch_lc_contract_block_merkle_root(retry, lc_contract_state))
        .await
        .with_context(|| {
            format!(
                "fetching the LC contract header at height {}",
                lc_contract_state.height
            )
        })
}

/// The header at `height`, proven against `anchor` from `fetch_anchor_root`.
pub(crate) async fn fetch_proven_header(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
    anchor: BlockMerkleCommitment,
    height: u64,
) -> anyhow::Result<Header> {
    retry_request(|retry| {
        peers.try_fetch_proven_header(retry, lc_contract_state.height, anchor, height)
    })
    .await
    .with_context(|| format!("fetching the header proof at height {height}"))
}

/// `request` with up to `REQUEST_RETRIES` retries; attempt `n` (0-based) is called with `n`.
async fn retry_request<T, Fut>(request: impl Fn(usize) -> Fut) -> anyhow::Result<T>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let attempts = AtomicUsize::new(0);
    // Not `peers.backoff()`: disabled on `ParallelStateCatchup`, unreachable on request-response.
    BackoffParams::default()
        .retry_if(
            REQUEST_RETRIES,
            |_| true,
            || request(attempts.fetch_add(1, Ordering::SeqCst)),
        )
        .await
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

#[cfg(test)]
mod tests {
    use super::{testing::mock_with_headers, *};

    /// A request is retried up to `REQUEST_RETRIES` times, then the error surfaces.
    #[rstest::rstest]
    #[case::retried_ok(2, 3, true)]
    #[case::bounded_fails(usize::MAX, REQUEST_RETRIES as usize + 1, false)]
    #[tokio::test(start_paused = true)]
    async fn fetch_proven_header_retries_failed_request(
        #[case] failing_calls: usize,
        #[case] expected_calls: usize,
        #[case] expect_ok: bool,
    ) {
        let (peers, lc_contract_state) = mock_with_headers(&[15]).await;
        let peers = peers.with_failing_header_calls(failing_calls);
        let anchor = fetch_anchor_root(&peers, &lc_contract_state).await.unwrap();

        let result = fetch_proven_header(&peers, &lc_contract_state, anchor, 15).await;

        assert_eq!(result.is_ok(), expect_ok, "{result:?}");
        if let Err(err) = result {
            assert!(
                format!("{err:#}").contains("the header proof at height 15"),
                "{err:#}"
            );
        }
        assert_eq!(peers.proven_header_calls(), expected_calls);
        let attempts: Vec<usize> = (0..expected_calls).collect();
        assert_eq!(peers.proven_header_retries(), attempts);
    }
}
