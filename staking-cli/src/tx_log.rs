use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alloy::{
    consensus::TxEnvelope,
    eips::eip2718::Encodable2718,
    network::{EthereumWallet, TransactionBuilder as _},
    primitives::{Address, Bytes, TxHash, U256},
    providers::Provider,
    rpc::types::TransactionRequest,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::concurrent::map_concurrent;

const MAX_RETRIES: u32 = 10;
pub const DEFAULT_CONCURRENCY: usize = 20;
pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;

/// Geth's default txpool pending limit per account.
/// Beyond this, transactions go to the "queued" pool where they cannot
/// be reliably tracked via eth_getTransactionByHash.
const GETH_PENDING_LIMIT: usize = 64;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TxPhase {
    #[default]
    FundEth,
    FundEsp,
    Approve,
    Delegate,
    Undelegate,
}

impl std::fmt::Display for TxPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxPhase::FundEth => write!(f, "fund_eth"),
            TxPhase::FundEsp => write!(f, "fund_esp"),
            TxPhase::Approve => write!(f, "approve"),
            TxPhase::Delegate => write!(f, "delegate"),
            TxPhase::Undelegate => write!(f, "undelegate"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTx {
    pub phase: TxPhase,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegator_index: Option<u64>,
    pub tx_hash: TxHash,
    pub signed_bytes: Bytes,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLog {
    pub transactions: Vec<SignedTx>,
}

impl TxLog {
    pub fn new(transactions: Vec<SignedTx>) -> Self {
        Self { transactions }
    }

    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(path)?;
        let log: TxLog = serde_json::from_str(&contents)?;
        Ok(Some(log))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn archive(&self, path: &Path) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let archived_name = format!("{}.completed.{}.json", stem, timestamp);
        let archived_path = path.with_file_name(archived_name);
        fs::rename(path, &archived_path)?;
        Ok(archived_path)
    }

    pub fn phases(&self) -> Vec<TxPhase> {
        let mut phases: Vec<TxPhase> = self.transactions.iter().map(|tx| tx.phase).collect();
        phases.sort();
        phases.dedup();
        phases
    }

    pub fn transactions_for_phase(&self, phase: TxPhase) -> Vec<&SignedTx> {
        self.transactions
            .iter()
            .filter(|tx| tx.phase == phase)
            .collect()
    }
}

pub fn max_retries() -> u32 {
    MAX_RETRIES
}

pub struct TxParams {
    pub chain_id: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
}

impl TxParams {
    pub async fn fetch(provider: &impl Provider) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        let fees = provider.estimate_eip1559_fees().await?;
        Ok(Self {
            chain_id,
            // 10 x to add some buffer because we're pre-signing many txns
            max_fee_per_gas: fees.max_fee_per_gas * 10,
            max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
        })
    }
}

pub async fn sign_transaction(
    wallet: &EthereumWallet,
    tx: TransactionRequest,
    nonce: u64,
    params: &TxParams,
) -> Result<(TxHash, Bytes)> {
    let tx = tx
        .with_nonce(nonce)
        .with_chain_id(params.chain_id)
        .with_gas_limit(DEFAULT_GAS_LIMIT)
        .with_max_fee_per_gas(params.max_fee_per_gas)
        .with_max_priority_fee_per_gas(params.max_priority_fee_per_gas);

    let signed: TxEnvelope = tx.build(wallet).await?;
    let tx_hash = *signed.tx_hash();
    let raw = signed.encoded_2718();
    Ok((tx_hash, Bytes::from(raw)))
}

pub struct TxInput {
    pub phase: TxPhase,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub delegator_index: Option<u64>,
}

pub async fn sign_all_transactions<P: Provider + Clone + 'static>(
    provider: &P,
    wallets: &HashMap<Address, EthereumWallet>,
    inputs: Vec<TxInput>,
    concurrency: usize,
    build_tx: impl Fn(&TxInput) -> TransactionRequest,
) -> Result<Vec<SignedTx>> {
    let params = TxParams::fetch(provider).await?;

    let addresses: Vec<Address> = inputs
        .iter()
        .map(|i| i.from)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let nonces: HashMap<Address, u64> =
        map_concurrent("fetching nonces", addresses, concurrency, {
            let provider = provider.clone();
            move |addr| {
                let provider = provider.clone();
                async move {
                    let nonce = provider.get_transaction_count(addr).await?;
                    Ok((addr, nonce))
                }
            }
        })
        .await?
        .into_iter()
        .collect();
    let mut nonces = nonces;

    let total = inputs.len();
    let mut signed_txs = Vec::with_capacity(total);

    for (i, input) in inputs.into_iter().enumerate() {
        let wallet = wallets
            .get(&input.from)
            .ok_or_else(|| anyhow::anyhow!("wallet not found for {}", input.from))?;
        let nonce = nonces.get_mut(&input.from).unwrap();
        let tx_request = build_tx(&input);

        let (tx_hash, signed_bytes) = sign_transaction(wallet, tx_request, *nonce, &params).await?;

        signed_txs.push(SignedTx {
            phase: input.phase,
            from: input.from,
            to: input.to,
            amount: input.amount,
            delegator_index: input.delegator_index,
            tx_hash,
            signed_bytes,
            nonce: *nonce,
        });

        *nonce += 1;

        let count = i + 1;
        if count % 1000 == 0 || count == total {
            tracing::info!("signed {}/{} transactions", count, total);
        }
    }

    Ok(signed_txs)
}

fn is_already_known(err: &str) -> bool {
    let err_lower = err.to_lowercase();
    err_lower.contains("already known") || err_lower.contains("already imported")
}

fn is_retriable_error(err: &str) -> bool {
    let non_retriable = [
        "nonce too low",
        "nonce too high",
        "replacement transaction underpriced",
        "insufficient funds",
        "exceeds block gas limit",
        "invalid sender",
        "invalid signature",
    ];
    let err_lower = err.to_lowercase();
    !non_retriable.iter().any(|s| err_lower.contains(s))
}

pub async fn submit_with_retry(
    provider: &impl Provider,
    signed_tx: &Bytes,
    tx_hash: TxHash,
) -> Result<TxHash> {
    let mut attempts = 0;
    loop {
        match provider.send_raw_transaction(signed_tx).await {
            Ok(pending) => return Ok(*pending.tx_hash()),
            Err(e) => {
                let err_str = e.to_string();

                if is_already_known(&err_str) {
                    tracing::debug!("tx {} already in mempool", tx_hash);
                    return Ok(tx_hash);
                }

                if !is_retriable_error(&err_str) {
                    bail!("non-retriable error: {}", err_str);
                }

                attempts += 1;
                if attempts >= MAX_RETRIES {
                    bail!("failed after {} attempts: {}", MAX_RETRIES, err_str);
                }
                let delay = Duration::from_millis(500 * attempts as u64);
                tracing::warn!(
                    "tx submission failed (attempt {}): {}, retrying in {:?}...",
                    attempts,
                    err_str,
                    delay
                );
                tokio::time::sleep(delay).await;
            },
        }
    }
}

pub async fn get_receipt_with_retry(
    provider: &impl Provider,
    tx_hash: TxHash,
) -> Result<Option<alloy::rpc::types::TransactionReceipt>> {
    let mut attempts = 0;
    loop {
        match provider.get_transaction_receipt(tx_hash).await {
            Ok(receipt) => return Ok(receipt),
            Err(e) => {
                attempts += 1;
                if attempts >= MAX_RETRIES {
                    bail!(
                        "failed to get receipt for {} after {} attempts: {}",
                        tx_hash,
                        MAX_RETRIES,
                        e
                    );
                }
                let delay = Duration::from_millis(500 * attempts as u64);
                tracing::warn!(
                    "get receipt failed (attempt {}): {}, retrying in {:?}...",
                    attempts,
                    e,
                    delay
                );
                tokio::time::sleep(delay).await;
            },
        }
    }
}

pub async fn get_confirmed_receipt(
    provider: &impl Provider,
    tx_hash: TxHash,
) -> Result<alloy::rpc::types::TransactionReceipt> {
    let receipt = get_receipt_with_retry(provider, tx_hash).await?;
    let receipt = receipt.ok_or_else(|| anyhow::anyhow!("no receipt for tx {}", tx_hash))?;
    if !receipt.status() {
        bail!("tx {} failed (reverted)", tx_hash);
    }
    Ok(receipt)
}

pub async fn execute_signed_tx_log<P: Provider + Clone + 'static>(
    provider: P,
    log: &TxLog,
    parallelism: usize,
) -> Result<()> {
    let total = log.transactions.len();
    let phases = log.phases();

    tracing::info!(
        "executing {} txs across {} phases (parallelism={})",
        total,
        phases.len(),
        parallelism
    );

    for phase in phases {
        let phase_txs = log.transactions_for_phase(phase);

        if phase_txs.is_empty() {
            continue;
        }

        tracing::info!("phase {}: {} txs", phase, phase_txs.len());

        // Step 1: Check which txs are already confirmed
        let results = map_concurrent(
            &format!("{} checking", phase),
            phase_txs.iter().cloned().cloned(),
            parallelism,
            {
                let provider = provider.clone();
                move |tx: SignedTx| {
                    let provider = provider.clone();
                    async move {
                        let receipt = get_receipt_with_retry(&provider, tx.tx_hash).await?;
                        if let Some(r) = &receipt {
                            if !r.status() {
                                bail!("tx {} failed (reverted)", tx.tx_hash);
                            }
                        }
                        Ok((tx, receipt.is_some()))
                    }
                }
            },
        )
        .await?;

        let mut to_submit: Vec<SignedTx> = Vec::new();
        let mut already_confirmed = 0;
        for (tx, is_confirmed) in results {
            if is_confirmed {
                already_confirmed += 1;
            } else {
                to_submit.push(tx);
            }
        }

        if already_confirmed > 0 {
            tracing::info!(
                "phase {}: {} already confirmed, {} to submit",
                phase,
                already_confirmed,
                to_submit.len()
            );
        }

        if to_submit.is_empty() {
            continue;
        }

        // Detect single-sender phase (e.g., FundEth/FundEsp from funder account)
        // For single-sender with many txs, batch to avoid geth queue limits
        let senders: HashSet<_> = to_submit.iter().map(|tx| tx.from).collect();
        let use_batching = senders.len() == 1 && to_submit.len() > GETH_PENDING_LIMIT;

        let batches: Vec<Vec<SignedTx>> = if use_batching {
            to_submit
                .chunks(GETH_PENDING_LIMIT)
                .map(|c| c.to_vec())
                .collect()
        } else {
            vec![to_submit]
        };

        let num_batches = batches.len();
        for (batch_idx, batch) in batches.into_iter().enumerate() {
            if num_batches > 1 {
                tracing::info!(
                    "phase {}: batch {}/{} ({} txs)",
                    phase,
                    batch_idx + 1,
                    num_batches,
                    batch.len()
                );
            }

            // Step 2: Submit batch
            map_concurrent(
                &format!("{} submitting", phase),
                batch.clone(),
                parallelism,
                {
                    let provider = provider.clone();
                    move |tx: SignedTx| {
                        let provider = provider.clone();
                        async move {
                            submit_with_retry(&provider, &tx.signed_bytes, tx.tx_hash).await?;
                            Ok(())
                        }
                    }
                },
            )
            .await?;

            // Step 3: Confirm loop for this batch
            let mut unconfirmed = batch;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;

                let results = map_concurrent(
                    &format!("{} confirming", phase),
                    unconfirmed.iter().cloned(),
                    parallelism,
                    {
                        let provider = provider.clone();
                        move |tx: SignedTx| {
                            let provider = provider.clone();
                            async move {
                                let receipt = get_receipt_with_retry(&provider, tx.tx_hash).await?;
                                if let Some(r) = &receipt {
                                    if !r.status() {
                                        bail!("tx {} failed (reverted)", tx.tx_hash);
                                    }
                                    return Ok((tx, true));
                                }
                                Ok((tx, false))
                            }
                        }
                    },
                )
                .await?;

                unconfirmed = results
                    .into_iter()
                    .filter_map(|(tx, confirmed)| if confirmed { None } else { Some(tx) })
                    .collect();

                if unconfirmed.is_empty() {
                    if num_batches > 1 {
                        tracing::info!(
                            "phase {}: batch {}/{} confirmed",
                            phase,
                            batch_idx + 1,
                            num_batches
                        );
                    } else {
                        tracing::info!("phase {}: all {} txs confirmed", phase, phase_txs.len());
                    }
                    break;
                }

                // Resubmit unconfirmed (they may have been dropped)
                tracing::debug!(
                    "phase {}: {} unconfirmed, resubmitting",
                    phase,
                    unconfirmed.len()
                );

                map_concurrent(
                    &format!("{} resubmitting", phase),
                    unconfirmed.iter().cloned(),
                    parallelism,
                    {
                        let provider = provider.clone();
                        move |tx: SignedTx| {
                            let provider = provider.clone();
                            async move {
                                submit_with_retry(&provider, &tx.signed_bytes, tx.tx_hash).await?;
                                Ok(())
                            }
                        }
                    },
                )
                .await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_returns_none() {
        let result = TxLog::load(Path::new("/nonexistent/path/tx_log.json")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("tx_log.json");

        let tx_hash: TxHash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            .parse()
            .unwrap();
        let log = TxLog::new(vec![SignedTx {
            phase: TxPhase::Delegate,
            from: Address::ZERO,
            to: Address::ZERO,
            amount: U256::ZERO,
            delegator_index: Some(0),
            tx_hash,
            signed_bytes: Bytes::from(vec![0x01, 0x02, 0x03]),
            nonce: 42,
        }]);

        log.save(&path).unwrap();
        let loaded = TxLog::load(&path).unwrap().unwrap();

        assert_eq!(loaded.transactions.len(), 1);
        assert_eq!(loaded.transactions[0].tx_hash, tx_hash);
        assert_eq!(loaded.transactions[0].nonce, 42);
        assert_eq!(loaded.transactions[0].phase, TxPhase::Delegate);
    }

    #[test]
    fn test_archive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("tx_log.json");

        let log = TxLog::new(vec![]);
        log.save(&path).unwrap();

        assert!(path.exists());
        let archived = log.archive(&path).unwrap();
        assert!(!path.exists());
        assert!(archived.exists());
        assert!(archived
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains(".completed."));
    }

    #[test]
    fn test_is_already_known() {
        assert!(is_already_known("transaction already known"));
        assert!(is_already_known("Transaction Already Known"));
        assert!(is_already_known("already imported"));
        assert!(is_already_known("transaction already imported"));
        assert!(!is_already_known("nonce too low"));
        assert!(!is_already_known("insufficient funds"));
    }

    #[test]
    fn test_is_retriable_error() {
        assert!(!is_retriable_error("nonce too low"));
        assert!(!is_retriable_error("nonce too high"));
        assert!(!is_retriable_error("insufficient funds"));
        assert!(!is_retriable_error("invalid signature"));
        assert!(is_retriable_error("connection reset"));
        assert!(is_retriable_error("timeout"));
        assert!(is_retriable_error("rate limited"));
    }
}
