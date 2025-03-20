use std::time::Duration;

use alloy::{
    contract::SolCallBuilder,
    network::{Ethereum, EthereumWallet, Network},
    primitives::U256,
    providers::{
        fillers::{FillProvider, JoinFill, WalletFiller},
        utils::JoinedRecommendedFillers,
        Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::TransactionReceipt,
    sol_types::{GenericContractError, SolCall},
    transports::Transport,
};
use anyhow::anyhow;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use committable::{Commitment, Committable};
use tokio::time::sleep;
use url::Url;

// FIXME: (alex) alloy doesn't have builtin external GasOracle support, do we still keep this?
// pub mod blocknative;
pub mod deployer;
pub mod logging;
pub mod ser;
// pub mod stake_table;
pub mod test_utils;

/// Type alias that connects to providers with recommended fillers and wallet
pub type HttpProviderWithWallet = FillProvider<
    JoinFill<JoinedRecommendedFillers, WalletFiller<EthereumWallet>>,
    RootProvider,
    Ethereum,
>;

pub async fn wait_for_http(
    url: &Url,
    interval: Duration,
    max_retries: usize,
) -> Result<usize, String> {
    for i in 0..(max_retries + 1) {
        let res = surf::get(url).await;
        if res.is_ok() {
            tracing::debug!("Connected to {url}");
            return Ok(i);
        }
        tracing::debug!("Waiting for {url}, retrying in {interval:?}");
        sleep(interval).await;
    }
    Err(format!("Url {url:?} not available."))
}

pub async fn wait_for_rpc(
    url: &Url,
    interval: Duration,
    max_retries: usize,
) -> Result<usize, String> {
    let retries = wait_for_http(url, interval, max_retries).await?;
    let client = ProviderBuilder::new().on_http(url.clone());
    for i in retries..(max_retries + 1) {
        if client.get_block_number().await.is_ok() {
            tracing::debug!("JSON-RPC ready at {url}");
            return Ok(i);
        }
        tracing::debug!("Waiting for JSON-RPC at {url}, retrying in {interval:?}");
        sleep(interval).await;
    }

    Err(format!("No JSON-RPC at {url}"))
}

/// converting a keccak256-based structured commitment (32 bytes) into type `U256`
pub fn commitment_to_u256<T: Committable>(comm: Commitment<T>) -> U256 {
    let mut buf = vec![];
    comm.serialize_uncompressed(&mut buf).unwrap();
    U256::from_le_slice(&buf)
}

/// converting a `U256` value into a keccak256-based structured commitment (32 bytes)
pub fn u256_to_commitment<T: Committable>(comm: U256) -> Result<Commitment<T>, SerializationError> {
    Commitment::deserialize_uncompressed_unchecked(&*comm.to_le_bytes_vec())
}

/// Implement `to_fixed_bytes` for wrapped types
#[macro_export]
macro_rules! impl_to_fixed_bytes {
    ($struct_name:ident, $type:ty) => {
        impl $struct_name {
            pub(crate) fn to_fixed_bytes(self) -> [u8; core::mem::size_of::<$type>()] {
                let bytes: [u8; core::mem::size_of::<$type>()] = self.0.to_le_bytes();
                bytes
            }
        }
    };
}

// NOTE: `wait_for_transaction_to_be_mined` is removed thanks to alloy's better builtin PendingTransaction await
/// send a transaction and wait for confirmation before returning the tx receipt and block included.
pub async fn contract_send<T, P, C, N>(
    call: &SolCallBuilder<T, P, C, N>,
) -> Result<(TransactionReceipt, u64), anyhow::Error>
where
    T: Transport + Clone,
    P: Provider<N>,
    C: SolCall,
    N: Network<ReceiptResponse = TransactionReceipt>,
{
    let pending = match call.send().await {
        Ok(pending) => pending,
        Err(err) => {
            if let Some(e) = err.as_decoded_interface_error::<GenericContractError>() {
                tracing::error!("contract err: {:?}", e);
            }
            return Err(anyhow!("error sending transaction: {:?}", err));
        },
    };

    let hash = pending.tx_hash().to_owned();
    tracing::debug!("submitted contract call {:x}", hash);

    let receipt = match pending.get_receipt().await {
        Ok(r) => r,
        Err(err) => {
            return Err(anyhow!(
                "contract call {hash:x}: error getting transaction receipt: {err}"
            ))
        },
    };

    // If a transaction is mined and we get a receipt for it, the block number should _always_ be
    // set. If it is not, something has gone horribly wrong with the RPC.
    let block_number = receipt
        .block_number
        .expect("transaction mined but block number not set");
    Ok((receipt, block_number))
}

#[cfg(test)]
mod test {
    use committable::RawCommitmentBuilder;

    use super::*;

    struct TestCommittable;

    impl Committable for TestCommittable {
        fn commit(&self) -> Commitment<Self> {
            RawCommitmentBuilder::new("TestCommittable").finalize()
        }
    }

    #[test]
    fn test_commitment_to_u256_round_trip() {
        assert_eq!(
            TestCommittable.commit(),
            u256_to_commitment(commitment_to_u256(TestCommittable.commit())).unwrap()
        );
    }
}
