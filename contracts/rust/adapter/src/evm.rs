use alloy::{network::Ethereum, providers::PendingTransactionBuilder, sol_types::SolInterface};

pub trait DecodeRevert {
    fn maybe_decode_revert<E: SolInterface + std::fmt::Debug>(
        self,
    ) -> anyhow::Result<PendingTransactionBuilder<Ethereum>>;
}

impl DecodeRevert
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
    use alloy::{
        primitives::{Address, U256},
        providers::ProviderBuilder,
    };

    use super::*;
    use crate::sol_types::EspToken::{self, EspTokenErrors};

    #[tokio::test]
    async fn test_decode_revert_error() -> anyhow::Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();

        let token = EspToken::deploy(&provider).await?;
        let err = token
            .transfer(Address::random(), U256::MAX)
            .send()
            .await
            .maybe_decode_revert::<EspTokenErrors>()
            .unwrap_err();
        assert!(err.to_string().contains("ERC20InsufficientBalance"));

        Ok(())
    }
}
