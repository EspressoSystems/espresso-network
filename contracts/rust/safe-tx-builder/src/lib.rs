use std::{collections::BTreeMap, path::Path, time::SystemTime};

use alloy::{
    json_abi::Function,
    primitives::{Address, Bytes, U256},
};
use anyhow::Result;
use serde::Serialize;

/// Optional decoded function call info for Safe Transaction Builder output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionInfo {
    pub signature: String,
    pub args: Vec<String>,
}

#[derive(Serialize)]
pub struct CalldataInfo {
    pub to: Address,
    pub data: Bytes,
    /// Included because Safe UI requires this field, even when value is 0.
    pub value: U256,
    #[serde(skip)]
    pub function_info: Option<FunctionInfo>,
}

impl CalldataInfo {
    pub fn new(to: Address, data: Bytes) -> Self {
        Self {
            to,
            data,
            value: U256::ZERO,
            function_info: None,
        }
    }

    pub fn with_method(to: Address, data: Bytes, value: U256, function_info: FunctionInfo) -> Self {
        Self {
            to,
            data,
            value,
            function_info: Some(function_info),
        }
    }
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
struct SafeContractMethod {
    inputs: Vec<SafeMethodInput>,
    name: String,
    payable: bool,
}

#[derive(Serialize)]
struct SafeMethodInput {
    #[serde(rename = "internalType")]
    internal_type: String,
    name: String,
    #[serde(rename = "type")]
    solidity_type: String,
}

#[derive(Serialize)]
struct SafeTransaction {
    to: String,
    value: String,
    /// When `contractMethod` is present, `data` must be `null` — the Safe UI
    /// ignores `contractMethod` if `data` is truthy and falls back to
    /// "Custom hex data".
    data: Option<String>,
    #[serde(rename = "contractMethod")]
    contract_method: Option<SafeContractMethod>,
    #[serde(rename = "contractInputsValues")]
    contract_inputs_values: Option<BTreeMap<String, String>>,
}

fn build_safe_method(
    info: &FunctionInfo,
) -> Result<(SafeContractMethod, BTreeMap<String, String>)> {
    let func = Function::parse(&info.signature).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse function signature '{}': {e}",
            info.signature
        )
    })?;

    anyhow::ensure!(
        info.args.len() == func.inputs.len(),
        "function '{}' expects {} args but got {}",
        info.signature,
        func.inputs.len(),
        info.args.len(),
    );

    let inputs: Vec<SafeMethodInput> = func
        .inputs
        .iter()
        .enumerate()
        .map(|(i, param)| {
            let ty = param.ty.to_string();
            SafeMethodInput {
                internal_type: param
                    .internal_type
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| ty.clone()),
                name: if param.name.is_empty() {
                    format!("arg{i}")
                } else {
                    param.name.clone()
                },
                solidity_type: ty,
            }
        })
        .collect();

    let mut values = BTreeMap::new();
    for (i, arg) in info.args.iter().enumerate() {
        let name = func
            .inputs
            .get(i)
            .filter(|p| !p.name.is_empty())
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("arg{i}"));
        values.insert(name, arg.clone());
    }

    let method = SafeContractMethod {
        inputs,
        name: func.name.clone(),
        payable: func.state_mutability == alloy::json_abi::StateMutability::Payable,
    };
    Ok((method, values))
}

pub fn output_safe_tx_builder(
    info: &CalldataInfo,
    output_path: Option<&Path>,
    chain_id: u64,
) -> Result<()> {
    let created_at = u64::try_from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .expect("timestamp fits u64");

    let batch = SafeTransactionBuilderBatch {
        version: "1.0",
        chain_id: chain_id.to_string(),
        created_at,
        meta: SafeBatchMeta {
            name: "Espresso Multisig Transactions",
            description: "",
        },
        transactions: vec![{
            let (data, contract_method, contract_inputs_values) = match &info.function_info {
                Some(fi) => {
                    let (m, v) = build_safe_method(fi)?;
                    (None, Some(m), Some(v))
                },
                None => (Some(info.data.to_string()), None, None),
            };
            SafeTransaction {
                to: info.to.to_checksum(None),
                value: info.value.to_string(),
                data,
                contract_method,
                contract_inputs_values,
            }
        }],
    };
    let text = serde_json::to_string_pretty(&batch)?;

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
    use alloy::primitives::{Address, Bytes, U256};

    use super::*;

    fn test_addr() -> Address {
        "0x1234567890abcdef1234567890abcdef12345678"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_output_safe_tx_builder_to_file() {
        let addr = test_addr();
        let data = Bytes::from(vec![0xca, 0xfe]);
        let info = CalldataInfo::new(addr, data);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_output_safe_tx.json");
        output_safe_tx_builder(&info, Some(&path), 1).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["version"].as_str().unwrap(), "1.0");
        let txs = parsed["transactions"].as_array().unwrap();
        assert_eq!(txs.len(), 1);
        let to = txs[0]["to"].as_str().unwrap();
        assert_eq!(to, addr.to_checksum(None));
        assert!(txs[0]["data"].as_str().is_some());
    }

    #[test]
    fn test_output_safe_tx_builder_chain_id() {
        let info = CalldataInfo::new(test_addr(), Bytes::from(vec![0x01]));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_output_chain_id.json");
        output_safe_tx_builder(&info, Some(&path), 11155111).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["chainId"].as_str().unwrap(), "11155111");
    }

    #[test]
    fn test_output_safe_tx_builder_contract_method() {
        let addr = test_addr();
        let recipient: Address = "0x000000000000000000000000000000000000dead"
            .parse()
            .unwrap();
        let info = CalldataInfo::with_method(
            addr,
            Bytes::from(vec![0xca, 0xfe]),
            U256::ZERO,
            FunctionInfo {
                signature: "transfer(address recipient, uint256 amount)".to_string(),
                args: vec![format!("{recipient:#x}"), "1000".to_string()],
            },
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_contract_method.json");
        output_safe_tx_builder(&info, Some(&path), 1).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        let tx = &parsed["transactions"][0];

        // contractMethod is populated
        let method = &tx["contractMethod"];
        assert_eq!(method["name"].as_str().unwrap(), "transfer");
        assert!(!method["payable"].as_bool().unwrap());
        let inputs = method["inputs"].as_array().unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0]["name"].as_str().unwrap(), "recipient");
        assert_eq!(inputs[0]["type"].as_str().unwrap(), "address");
        assert_eq!(inputs[0]["internalType"].as_str().unwrap(), "address");
        assert_eq!(inputs[1]["name"].as_str().unwrap(), "amount");
        assert_eq!(inputs[1]["type"].as_str().unwrap(), "uint256");
        assert_eq!(inputs[1]["internalType"].as_str().unwrap(), "uint256");

        // data is null when contractMethod is present (Safe UI requirement)
        assert!(tx["data"].is_null());

        // contractInputsValues is populated
        let values = &tx["contractInputsValues"];
        assert_eq!(
            values["recipient"].as_str().unwrap(),
            format!("{recipient:#x}")
        );
        assert_eq!(values["amount"].as_str().unwrap(), "1000");
    }

    #[test]
    fn test_build_safe_method_arg_count_mismatch() {
        let addr = test_addr();
        let info = CalldataInfo::with_method(
            addr,
            Bytes::from(vec![0xca, 0xfe]),
            U256::ZERO,
            FunctionInfo {
                signature: "transfer(address,uint256)".to_string(),
                args: vec!["0xdead".to_string()], // only 1 arg, expects 2
            },
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_mismatch.json");
        let err = output_safe_tx_builder(&info, Some(&path), 1).unwrap_err();
        assert!(
            err.to_string().contains("expects 2 args but got 1"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_output_safe_tx_builder_no_method_has_raw_data() {
        let info = CalldataInfo::new(test_addr(), Bytes::from(vec![0x01]));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_no_method.json");
        output_safe_tx_builder(&info, Some(&path), 1).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        let tx = &parsed["transactions"][0];
        // Without function info, data is present and method fields are null
        assert!(tx["data"].is_string());
        assert!(tx["contractMethod"].is_null());
        assert!(tx["contractInputsValues"].is_null());
    }
}
