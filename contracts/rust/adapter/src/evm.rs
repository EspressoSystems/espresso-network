use alloy::{
    sol_types::SolInterface,
    transports::{RpcError, TransportErrorKind},
};

pub trait DecodeRevert<T> {
    fn maybe_decode_revert<E: SolInterface + std::fmt::Debug>(self) -> anyhow::Result<T>;
}

impl<T> DecodeRevert<T> for Result<T, alloy::contract::Error> {
    fn maybe_decode_revert<E: SolInterface + std::fmt::Debug>(self) -> anyhow::Result<T> {
        match self {
            Ok(ret) => Ok(ret),
            Err(err) => {
                let msg = match err.as_decoded_interface_error::<E>() {
                    Some(e) => format!("{e:?}"),
                    None => format!("{err:?}"),
                };
                Err(anyhow::anyhow!(msg))
            },
        }
    }
}

impl<T> DecodeRevert<T> for Result<T, RpcError<TransportErrorKind>> {
    fn maybe_decode_revert<E: SolInterface + std::fmt::Debug>(self) -> anyhow::Result<T> {
        match self {
            Ok(ret) => Ok(ret),
            Err(RpcError::ErrorResp(payload)) => match payload.as_decoded_interface_error::<E>() {
                Some(e) => Err(anyhow::anyhow!("{e:?}")),
                None => Err(anyhow::anyhow!("{payload}")),
            },
            Err(err) => Err(anyhow::anyhow!("{err:?}")),
        }
    }
}

#[cfg(test)]
mod test {
    use alloy::{
        primitives::{Address, U256},
        providers::{Provider, ProviderBuilder},
        rpc::types::{TransactionInput, TransactionRequest},
        sol_types::SolCall,
    };

    use super::*;
    use crate::sol_types::EspToken::{self, transferCall, EspTokenErrors};

    #[tokio::test]
    async fn test_decode_revert_contract_error() -> anyhow::Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

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

    #[tokio::test]
    async fn test_decode_revert_rpc_error() -> anyhow::Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let token = EspToken::deploy(&provider).await?;
        let call = transferCall {
            to: Address::random(),
            value: U256::MAX,
        };
        let tx = TransactionRequest::default()
            .to(*token.address())
            .input(TransactionInput::new(call.abi_encode().into()));

        let err = provider
            .send_transaction(tx)
            .await
            .maybe_decode_revert::<EspTokenErrors>()
            .unwrap_err();
        assert!(err.to_string().contains("ERC20InsufficientBalance"));

        Ok(())
    }
}
