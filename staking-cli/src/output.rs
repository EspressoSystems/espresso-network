use alloy::primitives::{U256, utils::format_ether};
use anyhow::Result;
pub(crate) use espresso_safe_tx_builder::CalldataInfo;
use espresso_safe_tx_builder::output_safe_tx_builder;

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

pub(crate) fn output_calldata(
    info: &CalldataInfo,
    output: &OutputArgs,
    chain_id: u64,
) -> Result<()> {
    let fmt = output.format.unwrap_or(SerializationFormat::Safe);
    match fmt {
        SerializationFormat::Safe => {
            output_safe_tx_builder(info, output.output.as_deref(), chain_id)?;
        },
        SerializationFormat::Json | SerializationFormat::Toml => {
            // CalldataInfo derives Serialize with function_info skipped,
            // producing the legacy {to, data, value} format.
            let text = match fmt {
                SerializationFormat::Toml => toml::to_string_pretty(info)?,
                _ => serde_json::to_string_pretty(info)?,
            };
            if let Some(path) = &output.output {
                std::fs::write(path, &text)?;
                output_success(format!("Calldata written to {}", path.display()));
            } else {
                output_success(&text);
            }
        },
    }

    Ok(())
}
