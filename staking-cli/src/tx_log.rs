use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alloy::{
    consensus::TxEnvelope,
    eips::eip2718::Encodable2718,
    network::{EthereumWallet, TransactionBuilder as _},
    primitives::{Address, Bytes, TxHash, U256},
    providers::Provider,
    rpc::types::{TransactionReceipt, TransactionRequest},
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinSet,
};

use crate::concurrent::map_concurrent;

const MAX_RETRIES: u32 = 10;
pub const DEFAULT_CONCURRENCY: usize = 20;
const DEFAULT_GAS_LIMIT: u64 = 1_000_000;

/// Geth's default txpool pending limit per account.
/// Beyond this, transactions go to the "queued" pool where they cannot
/// be reliably tracked via eth_getTransactionByHash.
const GETH_PENDING_LIMIT: usize = 64;

/// Maximum unconfirmed transactions in flight (outer semaphore).
/// Provides backpressure: we don't submit faster than confirmations.
/// Set high enough that blocks are mostly full (~1000 txs/sec throughput).
const MAX_UNCONFIRMED: usize = 1000;

/// Timeout for waiting on a single transaction confirmation.
/// Set high to tolerate temporary geth dev mode deadlocks.
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(180);

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

pub struct TxParams {
    pub chain_id: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
}

impl TxParams {
    pub async fn fetch(provider: &impl Provider) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        let fees = provider.estimate_eip1559_fees().await?;
        // Use high gas price floor (100 gwei) for pre-signed txs since base fee can rise
        // significantly over thousands of blocks in dev mode
        let min_gas_price = 100_000_000_000u128; // 100 gwei
        let estimated_with_buffer = fees.max_fee_per_gas * 100;
        Ok(Self {
            chain_id,
            max_fee_per_gas: estimated_with_buffer.max(min_gas_price),
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

fn is_timeout_error(err: &str) -> bool {
    let err_lower = err.to_lowercase();
    err_lower.contains("timed out") || err_lower.contains("timeout")
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

                // On timeout, tx might have been accepted - check if already confirmed
                if is_timeout_error(&err_str) {
                    if let Ok(Some(_receipt)) = provider.get_transaction_receipt(tx_hash).await {
                        tracing::info!("tx {} already confirmed despite timeout", tx_hash);
                        return Ok(tx_hash);
                    }
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

/// Wait for a transaction to be confirmed, holding the permit until confirmation.
/// Returns the receipt on success, releases permit on drop. Times out after CONFIRMATION_TIMEOUT.
async fn wait_for_confirmation<P: Provider>(
    provider: P,
    tx: SignedTx,
    _permit: OwnedSemaphorePermit,
) -> Result<TransactionReceipt> {
    let start = std::time::Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        if let Some(receipt) = get_receipt_with_retry(&provider, tx.tx_hash).await? {
            if !receipt.status() {
                bail!("tx {} reverted", tx.tx_hash);
            }
            return Ok(receipt);
        }
        if start.elapsed() > CONFIRMATION_TIMEOUT {
            bail!(
                "tx {} not confirmed after {:?}",
                tx.tx_hash,
                CONFIRMATION_TIMEOUT
            );
        }
    }
}

/// Execute transactions from a single sender with backpressure.
/// Uses semaphore to limit unconfirmed txs, batches of 64 to avoid geth txpool drops.
async fn execute_single_sender_batched<P: Provider + Clone + 'static>(
    provider: P,
    phase: TxPhase,
    mut txs: Vec<SignedTx>,
) -> Result<()> {
    let total = txs.len();
    let unconfirmed_sem = Arc::new(Semaphore::new(MAX_UNCONFIRMED));

    // Sort by nonce
    txs.sort_by_key(|tx| tx.nonce);

    let mut confirm_tasks: JoinSet<Result<TransactionReceipt>> = JoinSet::new();
    let mut submitted = 0;

    for batch in txs.chunks(GETH_PENDING_LIMIT) {
        // Acquire permits for batch (blocks if would exceed MAX_UNCONFIRMED)
        let mut permits = vec![];
        for _ in 0..batch.len() {
            permits.push(unconfirmed_sem.clone().acquire_owned().await?);
        }

        // Submit batch
        for tx in batch {
            submit_with_retry(&provider, &tx.signed_bytes, tx.tx_hash).await?;
        }

        submitted += batch.len();
        tracing::info!("phase {}: submitted {}/{}", phase, submitted, total);

        // Give geth time to process the batch before submitting more
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Spawn confirm tasks (release permit on confirmation)
        for (tx, permit) in batch.iter().cloned().zip(permits) {
            let provider = provider.clone();
            confirm_tasks.spawn(async move { wait_for_confirmation(provider, tx, permit).await });
        }
    }

    // Wait for all confirmations (aborts all on first failure)
    let mut confirmed = 0;
    while let Some(result) = confirm_tasks.join_next().await {
        result??;
        confirmed += 1;
        if confirmed % 100 == 0 || confirmed == total {
            tracing::info!("phase {}: confirmed {}/{}", phase, confirmed, total);
        }
    }

    Ok(())
}

/// Execute pre-signed transactions with two-level flow control:
///
/// 1. **Outer semaphore** (`MAX_UNCONFIRMED`): Limits total unconfirmed txs in flight.
///    Provides backpressure - submission pauses when confirmations lag behind.
///    Set high enough to keep blocks full (~2000 txs for ~1000 tx/sec throughput).
///
/// 2. **Inner semaphore** (`parallelism`): Limits concurrent RPC requests to the node.
///    Prevents overwhelming the RPC endpoint with too many simultaneous calls.
///
/// When `geth_mode` is true, single-sender phases (FundEth, FundEsp) use batched
/// submission (64 txs at a time) to avoid geth txpool drops. When false, all phases
/// use the simpler multi-sender flow which works well with reth and other clients.
pub async fn execute_signed_tx_log<P: Provider + Clone + 'static>(
    provider: P,
    log: &TxLog,
    parallelism: usize,
    geth_mode: bool,
) -> Result<()> {
    let total = log.transactions.len();
    let phases = log.phases();

    tracing::info!(
        "executing {} txs across {} phases (parallelism={}, geth_mode={})",
        total,
        phases.len(),
        parallelism,
        geth_mode
    );

    // Background task to periodically poke geth's miner (only in geth_mode).
    // Works around a known deadlock bug in geth dev mode where the miner
    // stops producing blocks under load. Periodic RPC calls can unstick it.
    let poker_handle = if geth_mode {
        let poker_provider = provider.clone();
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let _ = poker_provider.get_block_number().await;
            }
        }))
    } else {
        None
    };

    let result = execute_signed_tx_log_inner(&provider, log, parallelism, phases, geth_mode).await;
    if let Some(h) = poker_handle {
        h.abort();
    }
    result
}

async fn execute_signed_tx_log_inner<P: Provider + Clone + 'static>(
    provider: &P,
    log: &TxLog,
    parallelism: usize,
    phases: Vec<TxPhase>,
    geth_mode: bool,
) -> Result<()> {
    for phase in phases {
        let phase_txs = log.transactions_for_phase(phase);
        let phase_total = phase_txs.len();

        if phase_txs.is_empty() {
            continue;
        }

        tracing::info!("phase {}: {} txs", phase, phase_total);

        // Step 1: Check which txs are already confirmed
        let results = map_concurrent(
            &format!("{} checking", phase),
            phase_txs.into_iter().cloned(),
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
        let senders: HashSet<_> = to_submit.iter().map(|tx| tx.from).collect();
        let is_single_sender = senders.len() == 1 && to_submit.len() > GETH_PENDING_LIMIT;

        if is_single_sender && geth_mode {
            // Use batching for single-sender phases in geth mode (avoids geth txpool drops)
            execute_single_sender_batched(provider.clone(), phase, to_submit).await?;
        } else {
            // Multi-sender flow: semaphore for unconfirmed + semaphore for concurrency
            let total = to_submit.len();
            let unconfirmed_sem = Arc::new(Semaphore::new(MAX_UNCONFIRMED));
            let concurrency_sem = Arc::new(Semaphore::new(parallelism));
            let submitted = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            let mut tasks: JoinSet<Result<TransactionReceipt>> = JoinSet::new();
            for tx in to_submit {
                let provider = provider.clone();
                let unconfirmed_sem = unconfirmed_sem.clone();
                let concurrency_sem = concurrency_sem.clone();
                let submitted = submitted.clone();
                tasks.spawn(async move {
                    // Acquire unconfirmed permit (limits total in-flight)
                    let unconfirmed_permit = unconfirmed_sem.acquire_owned().await?;

                    // Acquire concurrency permit (limits concurrent RPC calls)
                    let concurrency_permit = concurrency_sem.acquire().await?;
                    submit_with_retry(&provider, &tx.signed_bytes, tx.tx_hash).await?;
                    drop(concurrency_permit);

                    let count = submitted.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if count % 100 == 0 || count == total {
                        tracing::info!("phase {}: submitted {}/{}", phase, count, total);
                    }

                    // Wait for confirmation (holds unconfirmed_permit)
                    wait_for_confirmation(provider, tx, unconfirmed_permit).await
                });
            }

            // Wait for all confirmations (aborts all on first failure)
            let mut confirmed = 0;
            while let Some(result) = tasks.join_next().await {
                result??;
                confirmed += 1;
                if confirmed % 100 == 0 || confirmed == total {
                    tracing::info!("phase {}: confirmed {}/{}", phase, confirmed, total);
                }
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

    #[test]
    fn test_is_timeout_error() {
        assert!(is_timeout_error("request timed out"));
        assert!(is_timeout_error("-32002: request timed out"));
        assert!(is_timeout_error("Request Timed Out"));
        assert!(is_timeout_error("connection timeout"));
        assert!(is_timeout_error("Timeout waiting for response"));
        assert!(!is_timeout_error("nonce too low"));
        assert!(!is_timeout_error("connection reset"));
        assert!(!is_timeout_error("rate limited"));
    }
}
