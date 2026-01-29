use alloy::primitives::{utils::format_ether, Address, Bytes, U256};
use anyhow::Result;
use serde::Serialize;

use crate::signature::{OutputArgs, SerializationFormat};

pub(crate) fn format_esp(value: U256) -> String {
    let formatted = format_ether(value);
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    format!("{} ESP", trimmed)
}

pub fn output_success(msg: impl AsRef<str>) {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::info!("{}", msg.as_ref());
    } else {
        println!("{}", msg.as_ref());
    }
}

pub(crate) fn output_warn(msg: impl AsRef<str>) {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::warn!("{}", msg.as_ref());
    } else {
        eprintln!("{}", msg.as_ref());
    }
}

pub fn output_error(msg: impl AsRef<str>) -> ! {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::error!("{}", msg.as_ref());
    } else {
        eprintln!("{}", msg.as_ref());
    }
    std::process::exit(1);
}

#[derive(Serialize)]
pub(crate) struct CalldataInfo {
    to: Address,
    data: Bytes,
    /// Included because Safe UI requires this field, even when value is 0.
    value: U256,
}

impl CalldataInfo {
    pub(crate) fn new(to: Address, data: Bytes) -> Self {
        Self {
            to,
            data,
            value: U256::ZERO,
        }
    }
}

pub(crate) fn output_calldata(info: &CalldataInfo, output: &OutputArgs) -> Result<()> {
    let text = match output.format {
        Some(SerializationFormat::Json) => serde_json::to_string_pretty(info)?,
        Some(SerializationFormat::Toml) => toml::to_string_pretty(info)?,
        None => format!(
            "Target: {}\nCalldata: {}\nValue: {}",
            info.to, info.data, info.value
        ),
    };

    if let Some(path) = &output.output {
        std::fs::write(path, &text)?;
        output_success(format!("Calldata written to {}", path.display()));
    } else {
        output_success(&text);
    }

    Ok(())
}
