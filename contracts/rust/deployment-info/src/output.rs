use std::{
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

use alloy::primitives::Address;
use anyhow::{Context, Result, bail};

use crate::{
    contracts::{
        AccessControlDeployment, CollectedDeployment, DeploymentInfo, OwnableDeployment,
        TimelockDeployment,
    },
    get_crate_dir,
};

impl CollectedDeployment {
    pub(crate) fn to_toml_string(&self) -> Result<String> {
        let header = format_header_comment(self.block_number, self.block_timestamp);
        let toml_data = toml::to_string_pretty(&self.info)?;
        Ok(format!("{header}{toml_data}"))
    }

    pub(crate) fn write_toml(&self, output_path: &Path, force: bool) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create output directory")?;
        }

        if !force && output_path.exists() {
            let existing =
                std::fs::read_to_string(output_path).context("Failed to read existing file")?;
            let existing_info: DeploymentInfo =
                toml::from_str(&existing).context("Failed to parse existing deployment file")?;
            if existing_info == self.info {
                tracing::info!(
                    "{:?}: deployment info unchanged, skipping write",
                    output_path.file_name().unwrap_or_default()
                );
                return Ok(());
            }
        }

        let content = self.to_toml_string()?;
        std::fs::write(output_path, content).context("Failed to write deployment info")?;
        tracing::info!("Wrote: {:?}", output_path);

        Ok(())
    }
}

impl DeploymentInfo {
    pub(crate) fn to_markdown_table(&self) -> String {
        let etherscan = self.network.etherscan_base_url();
        let mut out = format!("### {}\n\n", self.network);

        out.push_str("| Contract | Address | Version | Owner | Pauser |\n");
        out.push_str("|----------|---------|---------|-------|--------|\n");

        for (name, deployment) in [
            ("EspToken", &self.esp_token),
            ("FeeContract", &self.fee_contract),
            ("LightClient", &self.light_client),
        ] {
            match deployment {
                OwnableDeployment::Deployed {
                    address,
                    owner_name,
                    version,
                    ..
                } => out.push_str(&contract_row(
                    name, *address, version, owner_name, "-", etherscan,
                )),
                OwnableDeployment::NotYetDeployed => {
                    out.push_str(&format!("| {name} | Not deployed | | | |\n"))
                },
            }
        }
        for (name, deployment) in [
            ("RewardClaim", &self.reward_claim),
            ("StakeTable", &self.stake_table),
        ] {
            match deployment {
                AccessControlDeployment::Deployed {
                    address,
                    default_admin_name,
                    version,
                    pauser_name,
                    ..
                } => out.push_str(&contract_row(
                    name,
                    *address,
                    version,
                    default_admin_name,
                    pauser_name,
                    etherscan,
                )),
                AccessControlDeployment::NotYetDeployed => {
                    out.push_str(&format!("| {name} | Not deployed | | | |\n"))
                },
            }
        }

        if !self.multisigs.is_empty() {
            out.push('\n');
            out.push_str("| Multisig | Address | Version | Threshold |\n");
            out.push_str("|----------|---------|---------|----------|\n");
            for (name, ms) in &self.multisigs {
                out.push_str(&format!(
                    "| {name} | {} | {} | {} |\n",
                    address_link(ms.address, etherscan),
                    ms.version,
                    ms.threshold,
                ));
            }
        }

        let timelocks: &[(&str, &TimelockDeployment)] = &[
            ("ops_timelock", &self.ops_timelock),
            ("safe_exit_timelock", &self.safe_exit_timelock),
        ];

        let has_timelocks = timelocks
            .iter()
            .any(|(_, tl)| matches!(tl, TimelockDeployment::Deployed { .. }));

        if has_timelocks {
            out.push('\n');
            out.push_str("| Timelock | Address | Min Delay |\n");
            out.push_str("|---------|---------|----------|\n");
            for (name, tl) in timelocks {
                match tl {
                    TimelockDeployment::Deployed {
                        address, min_delay, ..
                    } => {
                        out.push_str(&format!(
                            "| {name} | {} | {} |\n",
                            address_link(*address, etherscan),
                            humantime::format_duration(*min_delay),
                        ));
                    },
                    TimelockDeployment::NotYetDeployed => {
                        out.push_str(&format!("| {name} | Not deployed | |\n"));
                    },
                }
            }
        }

        out
    }
}

fn format_header_comment(block_number: u64, block_timestamp: u64) -> String {
    let system_time = UNIX_EPOCH + Duration::from_secs(block_timestamp);
    let formatted = humantime::format_rfc3339_seconds(system_time);
    format!("# fetched at block {block_number} ({formatted})\n")
}

fn address_link(addr: Address, etherscan_url: &str) -> String {
    format!("[`{addr}`]({etherscan_url}/address/{addr})")
}

fn contract_row(
    name: &str,
    address: Address,
    version: &str,
    owner: &str,
    pauser: &str,
    etherscan: &str,
) -> String {
    format!(
        "| {name} | {} | {version} | {owner} | {pauser} |\n",
        address_link(address, etherscan),
    )
}

fn replace_between_markers(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    replacement: &str,
) -> Result<String> {
    let start = content.find(start_marker).context("Missing start marker")?;
    let end = content.find(end_marker).context("Missing end marker")?;
    if end < start + start_marker.len() {
        bail!("End marker appears before start marker");
    }
    Ok(format!(
        "{}{start_marker}\n<!-- prettier-ignore-start -->\n{replacement}<!-- prettier-ignore-end \
         -->\n{end_marker}{}",
        &content[..start],
        &content[end + end_marker.len()..],
    ))
}

pub(crate) fn generate_updated_readme(
    deployments: &[DeploymentInfo],
    readme: &str,
) -> Result<String> {
    let sections: Vec<_> = deployments
        .iter()
        .map(DeploymentInfo::to_markdown_table)
        .collect();
    let combined = sections.join("\n");

    replace_between_markers(
        readme,
        "<!-- DEPLOYMENT_TABLE_START -->",
        "<!-- DEPLOYMENT_TABLE_END -->",
        &combined,
    )
    .context("README.md marker error")
}

pub(crate) fn update_readme_from_deployment_files() -> Result<()> {
    let crate_dir = get_crate_dir();
    let deployments_dir = crate_dir.join("deployments");
    let readme_path = crate_dir.join("README.md");

    let mut deployments = Vec::new();
    for entry in std::fs::read_dir(&deployments_dir)
        .context("Failed to read deployments directory")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
    {
        let path = entry.path();
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;
        let info: DeploymentInfo =
            toml::from_str(&content).with_context(|| format!("Failed to parse {:?}", path))?;
        deployments.push(info);
    }
    deployments.sort_by_key(|info| info.network.display_order());

    let readme = std::fs::read_to_string(&readme_path).context("Failed to read README.md")?;
    let new_readme = generate_updated_readme(&deployments, &readme)?;

    if readme == new_readme {
        tracing::info!("README.md unchanged, skipping write");
        return Ok(());
    }

    std::fs::write(&readme_path, new_readme).context("Failed to write README.md")?;
    tracing::info!("Updated README.md with deployment tables");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_toml_string_contains_header_and_valid_toml() {
        let collected = CollectedDeployment {
            info: DeploymentInfo::for_test(),
            block_number: 12345,
            block_timestamp: 1705312235,
        };
        let output = collected.to_toml_string().unwrap();
        assert!(output.starts_with("# fetched at block 12345"));
        // Strip the header comment line and parse the rest as valid TOML
        let toml_body = output.lines().skip(1).collect::<Vec<_>>().join("\n");
        let parsed: DeploymentInfo = toml::from_str(&toml_body).unwrap();
        assert_eq!(parsed, collected.info);
    }

    #[test]
    fn test_generate_updated_readme_inserts_tables() {
        let deployments = vec![DeploymentInfo::for_test()];
        let readme = "# README\n<!-- DEPLOYMENT_TABLE_START -->\nstale content\n<!-- \
                      DEPLOYMENT_TABLE_END -->\nfooter\n";
        let result = generate_updated_readme(&deployments, readme).unwrap();
        assert!(result.contains("### mainnet"));
        assert!(result.contains("<!-- DEPLOYMENT_TABLE_START -->"));
        assert!(result.contains("<!-- DEPLOYMENT_TABLE_END -->"));
        assert!(result.contains("footer"));
        assert!(!result.contains("stale content"));
    }

    #[test]
    fn test_generate_updated_readme_empty_deployments() {
        let readme = "# README\n<!-- DEPLOYMENT_TABLE_START -->\nstale content\n<!-- \
                      DEPLOYMENT_TABLE_END -->\nfooter\n";
        let result = generate_updated_readme(&[], readme).unwrap();
        assert!(result.contains("<!-- DEPLOYMENT_TABLE_START -->"));
        assert!(result.contains("<!-- DEPLOYMENT_TABLE_END -->"));
        assert!(!result.contains("stale content"));
        assert!(result.contains("footer"));
    }

    #[test]
    fn test_format_header_comment() {
        let comment = format_header_comment(12345678, 1705312235);
        assert!(comment.starts_with("# fetched at block 12345678 ("));
        assert!(comment.ends_with(")\n"));
        assert!(comment.contains("2024-01-15"));
    }

    #[test]
    fn test_address_link() {
        let addr: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let link = address_link(addr, "https://etherscan.io");
        assert_eq!(
            link,
            "[`0x1111111111111111111111111111111111111111`](https://etherscan.io/address/0x1111111111111111111111111111111111111111)"
        );
    }

    #[test]
    fn test_replace_between_markers() {
        let content = "before\n<!-- START -->\nold content\n<!-- END -->\nafter\n";
        let result =
            replace_between_markers(content, "<!-- START -->", "<!-- END -->", "new\n").unwrap();
        assert_eq!(
            result,
            "before\n<!-- START -->\n<!-- prettier-ignore-start -->\nnew\n<!-- \
             prettier-ignore-end -->\n<!-- END -->\nafter\n"
        );
    }

    #[test]
    fn test_replace_between_markers_missing_start() {
        let content = "no markers here";
        let result = replace_between_markers(content, "<!-- START -->", "<!-- END -->", "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_between_markers_reversed() {
        let content = "<!-- END -->\n<!-- START -->";
        let result = replace_between_markers(content, "<!-- START -->", "<!-- END -->", "x");
        assert!(result.is_err());
    }
}
