use alloy::{
    network::{Ethereum, TransactionBuilder as _},
    primitives::{Address, U256},
    providers::{PendingTransactionBuilder, Provider},
    rpc::types::TransactionRequest,
};
use anyhow::Result;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::EspToken::{self, EspTokenErrors},
};

pub async fn send_eth(
    provider: impl Provider,
    to: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    tracing::info!("fund address {to} with {amount} ETH");
    let tx = TransactionRequest::default().with_to(to).with_value(amount);
    Ok(provider.send_transaction(tx).await?)
}

pub async fn send_esp(
    provider: impl Provider,
    token_address: Address,
    to: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    tracing::info!("transfer {amount} ESP to {to}");
    let token = EspToken::new(token_address, provider);
    token
        .transfer(to, amount)
        .send()
        .await
        .maybe_decode_revert::<EspTokenErrors>()
}
