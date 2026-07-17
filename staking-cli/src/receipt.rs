use alloy::{
    network::Ethereum, providers::PendingTransactionBuilder, rpc::types::TransactionReceipt,
};
use anyhow::{Result, bail};

pub(crate) trait ReceiptExt {
    async fn assert_success(self) -> Result<TransactionReceipt>;
}

impl ReceiptExt for PendingTransactionBuilder<Ethereum> {
    async fn assert_success(self) -> Result<TransactionReceipt> {
        let receipt = self.get_receipt().await?;
        if !receipt.status() {
            bail!("transaction failed: hash={:?}", receipt.transaction_hash);
        }
        Ok(receipt)
    }
}
