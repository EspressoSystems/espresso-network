use std::time::Duration;

use alloy::{
    contract::SolCallBuilder,
    network::{Ethereum, EthereumWallet},
    primitives::U256,
    providers::{
        fillers::{FillProvider, JoinFill, WalletFiller},
        utils::JoinedRecommendedFillers,
        Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::TransactionReceipt,
    sol_types::{GenericContractError, SolCall},
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
pub mod stake_table;
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

/// send a transaction and wait for confirmation before returning the tx receipt and block included.
///
/// # NOTE:
/// - `wait_for_transaction_to_be_mined` is removed thanks to alloy's better builtin PendingTransaction await
/// - DON'T use this if you want parse the exact revert reason/type, since this func will only give err msg like: "custom error 0x23b0db14",
///   instead, follow <https://docs.rs/alloy/0.12.5/alloy/contract/enum.Error.html#method.as_decoded_interface_error> to pattern-match err type
pub async fn contract_send<T, P, C>(
    call: &SolCallBuilder<T, P, C>,
) -> Result<(TransactionReceipt, u64), anyhow::Error>
where
    P: Provider,
    C: SolCall,
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
    tracing::info!("submitted contract call 0x{:x}", hash);

    let receipt = match pending.get_receipt().await {
        Ok(r) => r,
        Err(err) => {
            return Err(anyhow!(
                "contract call 0x{hash:x}: error getting transaction receipt: {err}"
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
    use alloy::{primitives::I256, sol};
    use anyhow::Result;
    use committable::RawCommitmentBuilder;
    use test_utils::setup_test;

    use super::*;

    // contract for tests, credit: <https://alloy.rs/examples/sol-macro/events_errors.html>
    sol! {
        #[allow(missing_docs)]
        #[sol(rpc, bytecode = "608060405260008055348015601357600080fd5b506103e9806100236000396000f3fe608060405234801561001057600080fd5b50600436106100575760003560e01c80632baeceb71461005c5780632ccbdbca1461006657806361bc221a14610070578063c3e8b5ca1461008e578063d09de08a14610098575b600080fd5b6100646100a2565b005b61006e610103565b005b61007861013e565b60405161008591906101f9565b60405180910390f35b610096610144565b005b6100a061017f565b005b60016000808282546100b49190610243565b925050819055506000543373ffffffffffffffffffffffffffffffffffffffff167fdc69c403b972fc566a14058b3b18e1513da476de6ac475716e489fae0cbe4a2660405160405180910390a3565b6040517f23b0db14000000000000000000000000000000000000000000000000000000008152600401610135906102e3565b60405180910390fd5b60005481565b6040517fa5f9ec670000000000000000000000000000000000000000000000000000000081526004016101769061034f565b60405180910390fd5b6001600080828254610191919061036f565b925050819055506000543373ffffffffffffffffffffffffffffffffffffffff167ff6d1d8d205b41f9fb9549900a8dba5d669d68117a3a2b88c1ebc61163e8117ba60405160405180910390a3565b6000819050919050565b6101f3816101e0565b82525050565b600060208201905061020e60008301846101ea565b92915050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b600061024e826101e0565b9150610259836101e0565b92508282039050818112600084121682821360008512151617156102805761027f610214565b5b92915050565b600082825260208201905092915050565b7f4572726f72204100000000000000000000000000000000000000000000000000600082015250565b60006102cd600783610286565b91506102d882610297565b602082019050919050565b600060208201905081810360008301526102fc816102c0565b9050919050565b7f4572726f72204200000000000000000000000000000000000000000000000000600082015250565b6000610339600783610286565b915061034482610303565b602082019050919050565b600060208201905081810360008301526103688161032c565b9050919050565b600061037a826101e0565b9150610385836101e0565b9250828201905082811215600083121683821260008412151617156103ad576103ac610214565b5b9291505056fea2646970667358221220a878a3c1da1a1170e4496cdbc63bd5ed1587374bcd6cf6d4f1d5b88fa981795d64736f6c63430008190033")]
        contract CounterWithError {
            int256 public counter = 0;

            // Events - using `Debug` to print the events
            #[derive(Debug)]
            event Increment(address indexed by, int256 indexed value);
            #[derive(Debug)]
            event Decrement(address indexed by, int256 indexed value);

            // Custom Error
            #[derive(Debug)]
            error ErrorA(string message);
            #[derive(Debug)]
            error ErrorB(string message);

            // Functions
            function increment() public {
                counter += 1;
                emit Increment(msg.sender, counter);
            }

            function decrement() public {
                counter -= 1;
                emit Decrement(msg.sender, counter);
            }

            function revertA() public pure {
                revert ErrorA("Error A");
            }

            function revertB() public pure {
                revert ErrorB("Error B");
            }
        }
    }

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

    #[tokio::test]
    async fn test_contract_send() -> Result<()> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let contract = CounterWithError::deploy(provider.clone()).await?;

        // test normal contract sending should success
        let inc_call = contract.increment();
        let (receipt, block_num) = contract_send(&inc_call).await?;
        assert_eq!(block_num, 2); // 1 for deployment, 1 for this send
        assert!(receipt.inner.is_success());
        assert_eq!(contract.counter().call().await?.counter, I256::ONE);

        // test contract revert will return useful error message
        let revert_call = contract.revertA();
        assert!(contract_send(&revert_call).await.is_err());

        Ok(())
    }
}
