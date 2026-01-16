use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
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
use tokio::sync::Semaphore;

const MAX_RETRIES: u32 = 10;
pub const DEFAULT_PARALLELISM: usize = 20;

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
pub struct TxInput {
    pub phase: TxPhase,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegator_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<TxHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInputLog {
    pub transactions: Vec<TxInput>,
}

impl TxInputLog {
    pub fn new(transactions: Vec<TxInput>) -> Self {
        Self { transactions }
    }

    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(path)?;
        let log: TxInputLog = serde_json::from_str(&contents)?;
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

    pub fn transactions_for_phase(&self, phase: TxPhase) -> Vec<&TxInput> {
        self.transactions
            .iter()
            .filter(|tx| tx.phase == phase)
            .collect()
    }

    pub fn pending_for_phase(&self, phase: TxPhase) -> Vec<&TxInput> {
        self.transactions
            .iter()
            .filter(|tx| tx.phase == phase && tx.tx_hash.is_none())
            .collect()
    }

    pub fn mark_confirmed(&mut self, from: Address, to: Address, phase: TxPhase, tx_hash: TxHash) {
        for tx in &mut self.transactions {
            if tx.from == from && tx.to == to && tx.phase == phase && tx.tx_hash.is_none() {
                tx.tx_hash = Some(tx_hash);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx_hash: TxHash,
    pub nonce: u64,
    pub signed_tx: Bytes,
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub phase: TxPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLog {
    pub transactions: Vec<TxRecord>,
}

impl TxLog {
    pub fn new(transactions: Vec<TxRecord>) -> Self {
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
}

pub fn max_retries() -> u32 {
    MAX_RETRIES
}

pub async fn sign_transaction(
    provider: &impl Provider,
    wallet: &EthereumWallet,
    tx: TransactionRequest,
    nonce: u64,
) -> Result<(TxHash, Bytes)> {
    let chain_id = provider.get_chain_id().await?;
    let fees = provider.estimate_eip1559_fees().await?;
    let gas = provider.estimate_gas(tx.clone()).await?;

    let tx = tx
        .with_nonce(nonce)
        .with_chain_id(chain_id)
        .with_gas_limit(gas)
        .with_max_fee_per_gas(fees.max_fee_per_gas)
        .with_max_priority_fee_per_gas(fees.max_priority_fee_per_gas);

    let signed: TxEnvelope = tx.build(wallet).await?;
    let tx_hash = *signed.tx_hash();
    let raw = signed.encoded_2718();
    Ok((tx_hash, Bytes::from(raw)))
}

fn is_already_known(err: &str) -> bool {
    let err_lower = err.to_lowercase();
    err_lower.contains("already known")
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

pub async fn execute_with_log<P: Provider + Clone + 'static>(
    provider: P,
    log_path: &Path,
    txs: Vec<TxRecord>,
    parallelism: usize,
) -> Result<()> {
    let log = match TxLog::load(log_path)? {
        Some(existing) => {
            tracing::info!(
                "resuming from existing log with {} txs",
                existing.transactions.len()
            );
            existing
        },
        None => {
            tracing::info!("creating new log with {} txs", txs.len());
            let log = TxLog::new(txs);
            log.save(log_path)?;
            log
        },
    };

    let total = log.transactions.len();
    let semaphore = Arc::new(Semaphore::new(parallelism));

    let mut phases: Vec<TxPhase> = log.transactions.iter().map(|tx| tx.phase).collect();
    phases.sort();
    phases.dedup();

    tracing::info!(
        "executing {} txs across {} phases (parallelism={})",
        total,
        phases.len(),
        parallelism
    );

    for phase in phases {
        let phase_txs: Vec<_> = log
            .transactions
            .iter()
            .filter(|tx| tx.phase == phase)
            .collect();

        if phase_txs.is_empty() {
            continue;
        }

        tracing::info!("phase {}: {} txs", phase, phase_txs.len());

        let confirmed_count = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for tx in phase_txs.iter() {
            let provider = provider.clone();
            let sem = semaphore.clone();
            let confirmed = confirmed_count.clone();
            let tx = (*tx).clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let receipt = get_receipt_with_retry(&provider, tx.tx_hash).await?;
                if receipt.is_some() {
                    confirmed.fetch_add(1, Ordering::Relaxed);
                }
                Ok::<_, anyhow::Error>((tx, receipt.is_some()))
            });
            handles.push(handle);
        }

        let mut pending = Vec::new();
        for handle in handles {
            let (tx, is_confirmed) = handle.await??;
            if !is_confirmed {
                pending.push(tx);
            }
        }

        let already_confirmed = confirmed_count.load(Ordering::Relaxed);
        if already_confirmed > 0 {
            tracing::info!(
                "phase {}: {} already confirmed, {} pending",
                phase,
                already_confirmed,
                pending.len()
            );
        }

        if !pending.is_empty() {
            let pending_count = pending.len();
            for (i, tx) in pending.iter().enumerate() {
                submit_with_retry(&provider, &tx.signed_tx, tx.tx_hash).await?;
                let count = i + 1;
                if count % 100 == 0 || count == pending_count {
                    tracing::info!("phase {}: submitted {}/{}", phase, count, pending_count);
                }
            }
        }

        loop {
            let confirmed = Arc::new(AtomicUsize::new(0));
            let mut handles = Vec::new();

            for tx in phase_txs.iter() {
                let provider = provider.clone();
                let sem = semaphore.clone();
                let confirmed = confirmed.clone();
                let tx_hash = tx.tx_hash;

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let receipt = get_receipt_with_retry(&provider, tx_hash).await?;
                    if receipt.is_some() {
                        confirmed.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok::<_, anyhow::Error>(receipt.is_some())
                });
                handles.push(handle);
            }

            let mut all_confirmed = true;
            for handle in handles {
                if !handle.await?? {
                    all_confirmed = false;
                }
            }

            let count = confirmed.load(Ordering::Relaxed);
            if all_confirmed {
                tracing::info!("phase {}: all {} txs confirmed", phase, phase_txs.len());
                break;
            }

            tracing::debug!(
                "phase {}: {}/{} confirmed, waiting...",
                phase,
                count,
                phase_txs.len()
            );
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    let archived = log.archive(log_path)?;
    tracing::info!("archived log to {}", archived.display());

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
        let log = TxLog::new(vec![TxRecord {
            tx_hash,
            nonce: 42,
            signed_tx: Bytes::from(vec![0x01, 0x02, 0x03]),
            metadata: serde_json::json!({"action": "delegate", "validator": "0x123"}),
            phase: TxPhase::Delegate,
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
}
