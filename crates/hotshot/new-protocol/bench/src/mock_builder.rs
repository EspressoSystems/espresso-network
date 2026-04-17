use std::collections::BTreeMap;

use committable::Committable;
use hotshot::traits::BlockPayload;
use hotshot_example_types::{
    block_types::{TestBlockPayload, TestMetadata, TestTransaction},
    node_types::TestTypes,
};
use hotshot_new_protocol::{
    block::{BlockAndHeaderRequest, BlockBuilder, BlockBuilderOutput, BlockError},
    message::DedupManifest,
};
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{VidCommitment, VidDisperse2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        EncodeBytes, block_contents::BuilderFee, node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::error;

/// Mock block builder for benchmarks.
pub struct MockBuilder {
    block_size: usize,
    membership: EpochMembershipCoordinator<TestTypes>,
    tasks: JoinSet<Result<BlockBuilderOutput<TestTypes>, BlockError>>,
    calculations: BTreeMap<ViewNumber, AbortHandle>,
}

impl MockBuilder {
    pub fn new(block_size: usize, membership: EpochMembershipCoordinator<TestTypes>) -> Self {
        Self {
            block_size,
            membership,
            tasks: JoinSet::new(),
            calculations: BTreeMap::new(),
        }
    }
}

impl BlockBuilder<TestTypes> for MockBuilder {
    fn request_block(&mut self, req: BlockAndHeaderRequest<TestTypes>) {
        let view = req.view;
        if self.calculations.contains_key(&view) {
            return;
        }
        let block_size = self.block_size;
        let membership = self.membership.clone();

        let handle = self.tasks.spawn(async move {
            let tx = TestTransaction::new(vec![0u8; block_size]);
            let block = TestBlockPayload {
                transactions: vec![tx],
            };
            let metadata = TestMetadata {
                num_transactions: 1,
            };

            let (vid_disperse, _duration) = VidDisperse2::calculate_vid_disperse(
                &block,
                &membership,
                req.view,
                Some(req.epoch),
                Some(req.epoch),
                &metadata,
            )
            .await
            .map_err(|_| BlockError::StakeTableUnavailable)?;

            let payload_commitment = VidCommitment::V2(vid_disperse.payload_commitment);
            let builder_commitment =
                <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
                    &block, &metadata,
                );

            let (builder_key, builder_private_key) =
                <<TestTypes as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::generated_from_seed_indexed([0u8; 32], 0);
            let payload_bytes = block.encode();
            let block_size_bytes = payload_bytes.len() as u64;
            let builder_fee = BuilderFee {
                fee_amount: block_size_bytes,
                fee_account: builder_key,
                fee_signature:
                    <<TestTypes as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::sign_fee(
                        &builder_private_key,
                        block_size_bytes,
                        &metadata,
                    )
                    .map_err(|_| BlockError::BuilderSignature)?,
            };

            // Create empty manifest (no mempool in benchmarks).
            let hashes = block
                .transactions
                .iter()
                .map(|tx| tx.commit())
                .collect();
            let manifest = DedupManifest {
                view: req.view,
                epoch: req.epoch,
                hashes,
            };

            let payload = PayloadWithMetadata {
                payload: block,
                metadata,
            };

            Ok(BlockBuilderOutput {
                view: req.view,
                epoch: req.epoch,
                payload,
                parent_proposal: req.parent_proposal,
                builder_commitment,
                builder_fee,
                payload_commitment,
                vid_disperse,
                manifest,
            })
        });
        self.calculations.insert(view, handle);
    }

    async fn next(&mut self) -> Option<Result<BlockBuilderOutput<TestTypes>, BlockError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(err)) => {
                    if err.is_panic() {
                        error!(%err, "mock block builder task panicked");
                    }
                },
                None => return None,
            }
        }
    }

    fn gc(&mut self, view: ViewNumber) {
        let keep = self.calculations.split_off(&view);
        for handle in self.calculations.values() {
            handle.abort();
        }
        self.calculations = keep;
    }
}
