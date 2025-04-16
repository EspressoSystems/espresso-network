use alloy::{
    network::Ethereum,
    primitives::Log,
    providers::PendingTransactionBuilder,
    rpc::types::TransactionReceipt,
    sol_types::{SolEvent, SolInterface},
};

// TODO this function can be removed once we move to alloy 0.12
#[allow(dead_code)]
pub fn decode_log<E: SolEvent>(r: &TransactionReceipt) -> Option<Log<E>> {
    r.inner
        .logs()
        .iter()
        .find_map(|log| E::decode_log(&log.inner, false).ok())
}

pub trait DecodeRevertError {
    fn maybe_decode_revert<E: SolInterface + std::fmt::Debug>(
        self,
    ) -> anyhow::Result<PendingTransactionBuilder<Ethereum>>;
}

impl DecodeRevertError
    for alloy::contract::Result<PendingTransactionBuilder<Ethereum>, alloy::contract::Error>
{
    fn maybe_decode_revert<E: SolInterface + std::fmt::Debug>(
        self,
    ) -> anyhow::Result<PendingTransactionBuilder<Ethereum>> {
        match self {
            Ok(ret) => Ok(ret),
            Err(err) => {
                let decoded = err.as_decoded_interface_error::<E>();
                let msg = match decoded {
                    Some(e) => format!("{:?}", e),
                    None => format!("{:?}", err),
                };
                Err(anyhow::anyhow!(msg))
            },
        }
    }
}

#[cfg(test)]
mod test {
    use alloy::primitives::U256;
    use hotshot_contract_adapter::sol_types::StakeTable::{self, StakeTableErrors};

    use super::*;
    use crate::deploy::TestSystem;

    #[tokio::test]
    async fn test_decode_revert_error() -> anyhow::Result<()> {
        let system = TestSystem::deploy().await?;
        let st = StakeTable::new(system.stake_table, system.provider);
        let err = st
            .delegate(system.deployer_address, U256::from(123))
            .send()
            .await
            .maybe_decode_revert::<StakeTableErrors>()
            .unwrap_err();
        assert!(err.to_string().contains("ValidatorInactive"));

        Ok(())
    }
}
