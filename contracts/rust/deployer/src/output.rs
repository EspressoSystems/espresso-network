use std::{path::PathBuf, time::SystemTime};

use alloy::primitives::{Address, Bytes, U256};
use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

#[derive(Serialize)]
pub struct CalldataInfo {
    pub to: Address,
    pub data: Bytes,
    /// Included because Safe UI requires this field, even when value is 0.
    pub value: U256,
}

impl CalldataInfo {
    pub fn new(to: Address, data: Bytes) -> Self {
        Self {
            to,
            data,
            value: U256::ZERO,
        }
    }

    pub fn with_value(to: Address, data: Bytes, value: U256) -> Self {
        Self { to, data, value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Json,
    SafeTransactionBuilder,
}

/// Safe Transaction Builder batch format
/// See: https://help.safe.global/en/articles/40795-transaction-builder
#[derive(Serialize)]
struct SafeTransactionBuilderBatch {
    version: &'static str,
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "createdAt")]
    created_at: u64,
    meta: SafeBatchMeta,
    transactions: Vec<SafeTransaction>,
}

#[derive(Serialize)]
struct SafeBatchMeta {
    name: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct SafeTransaction {
    to: String,
    value: String,
    data: String,
    #[serde(rename = "contractMethod")]
    contract_method: Option<()>,
    #[serde(rename = "contractInputsValues")]
    contract_inputs_values: Option<()>,
}

pub fn output_calldata(
    info: &CalldataInfo,
    format: OutputFormat,
    output_path: Option<&PathBuf>,
) -> Result<()> {
    let text = match format {
        OutputFormat::Json => serde_json::to_string_pretty(info)?,
        OutputFormat::SafeTransactionBuilder => {
            let created_at = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let batch = SafeTransactionBuilderBatch {
                version: "1.0",
                chain_id: String::new(),
                created_at,
                meta: SafeBatchMeta {
                    name: "Espresso Multisig Transactions",
                    description: "",
                },
                transactions: vec![SafeTransaction {
                    to: format!("{:#x}", info.to),
                    value: info.value.to_string(),
                    data: info.data.to_string(),
                    contract_method: None,
                    contract_inputs_values: None,
                }],
            };
            serde_json::to_string_pretty(&batch)?
        },
    };

    if let Some(path) = output_path {
        std::fs::write(path, &text)?;
        tracing::info!("Calldata written to {}", path.display());
    } else {
        println!("{text}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use alloy::primitives::{Address, Bytes, U256};

    use super::*;

    fn test_addr() -> Address {
        "0x1234567890abcdef1234567890abcdef12345678"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_output_json_stdout() {
        let info = CalldataInfo::new(test_addr(), Bytes::from(vec![1, 2, 3]));
        assert!(output_calldata(&info, OutputFormat::Json, None).is_ok());
    }

    #[test]
    fn test_output_json_to_file() {
        let addr = test_addr();
        let data = Bytes::from(vec![0xde, 0xad, 0xbe, 0xef]);
        let info = CalldataInfo::new(addr, data);

        let path = PathBuf::from("./tmp/test_output_json.json");
        std::fs::create_dir_all("./tmp").unwrap();
        output_calldata(&info, OutputFormat::Json, Some(&path)).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        let to_str = parsed["to"].as_str().unwrap().to_lowercase();
        assert!(to_str.contains("1234567890abcdef1234567890abcdef12345678"));
    }

    #[test]
    fn test_output_safe_tx_builder_to_file() {
        let addr = test_addr();
        let data = Bytes::from(vec![0xca, 0xfe]);
        let info = CalldataInfo::new(addr, data);

        let path = PathBuf::from("./tmp/test_output_safe_tx.json");
        std::fs::create_dir_all("./tmp").unwrap();
        output_calldata(&info, OutputFormat::SafeTransactionBuilder, Some(&path)).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["version"].as_str().unwrap(), "1.0");
        let txs = parsed["transactions"].as_array().unwrap();
        assert_eq!(txs.len(), 1);
        assert!(txs[0]["to"].as_str().is_some());
        assert!(txs[0]["data"].as_str().is_some());
    }

    #[test]
    fn test_calldata_info_with_value() {
        let addr = test_addr();
        let data = Bytes::from(vec![0x01]);
        let value = U256::from(42u64);
        let info = CalldataInfo::with_value(addr, data, value);
        assert_eq!(info.value, value);
        assert_eq!(info.to, addr);
    }

    #[test]
    fn test_output_empty_init_data() {
        let addr = test_addr();
        let info = CalldataInfo::new(addr, Bytes::new());

        let path = PathBuf::from("./tmp/test_output_empty.json");
        std::fs::create_dir_all("./tmp").unwrap();
        output_calldata(&info, OutputFormat::Json, Some(&path)).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("data"));
    }
}
