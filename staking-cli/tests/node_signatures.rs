mod common;

use anyhow::Result;
use assert_cmd::assert::OutputAssertExt;
use common::{base_cmd, Signer, TestSystemExt};
use hotshot_contract_adapter::{stake_table, stake_table::StakeTableContractVersion};
use predicates::str;
use rand::SeedableRng;
use staking_cli::{deploy::TestSystem, signature::NodeSignatures};

#[rstest_reuse::template]
#[rstest::rstest]
#[case(Extension::Json, Output::Stdout)]
#[case(Extension::Json, Output::File)]
#[case(Extension::Toml, Output::Stdout)]
#[case(Extension::Toml, Output::File)]
pub fn format_combinations(#[case] format: Extension, #[case] output: Output) {}

#[derive(Debug, Clone, Copy)]
enum Extension {
    Json,
    Toml,
}

impl std::fmt::Display for Extension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Toml => write!(f, "toml"),
        }
    }
}

impl Extension {
    fn parse_node_signatures(&self, content: &str) -> Result<NodeSignatures> {
        match self {
            Self::Json => Ok(serde_json::from_str(content)?),
            Self::Toml => Ok(toml::from_str(content)?),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Output {
    Stdout,
    File,
}

#[test_log::test(rstest_reuse::apply(format_combinations))]
#[tokio::test]
async fn test_export_format_combinations(
    #[case] format: Extension,
    #[case] output: Output,
) -> Result<()> {
    let system = TestSystem::deploy().await?;

    let mut cmd = system.export_node_signatures_cmd()?;

    let content = match output {
        Output::Stdout => {
            cmd.arg("--format").arg(format.to_string());
            let output = cmd.assert().success().get_output().to_owned();
            String::from_utf8(output.stdout)?
        },
        Output::File => {
            let tmpdir = tempfile::tempdir()?;
            let output_file = tmpdir.path().join(format!("payload.{format}"));
            cmd.arg("--output").arg(&output_file);
            cmd.assert().success();
            std::fs::read_to_string(&output_file)?
        },
    };

    let parsed = format.parse_node_signatures(&content)?;
    assert_eq!(parsed.address, system.deployer_address);

    Ok(())
}

#[test_log::test(rstest::rstest)]
#[case(Extension::Json, Extension::Json)]
#[case(Extension::Json, Extension::Toml)]
#[case(Extension::Toml, Extension::Json)]
#[case(Extension::Toml, Extension::Toml)]
#[tokio::test]
async fn test_explicit_format_override(
    #[case] extension: Extension,
    #[case] format: Extension,
) -> Result<()> {
    let system = TestSystem::deploy().await?;

    let tmpdir = tempfile::tempdir()?;
    let output_file = tmpdir.path().join(format!("payload.{extension}"));

    let mut cmd = system.export_node_signatures_cmd()?;
    cmd.arg("--output")
        .arg(&output_file)
        .arg("--format")
        .arg(format.to_string());

    cmd.assert().success();

    let content = std::fs::read_to_string(&output_file)?;
    let parsed = format.parse_node_signatures(&content)?;
    assert_eq!(parsed.address, system.deployer_address);

    Ok(())
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn test_file_extension_inference(
    #[values(Extension::Json, Extension::Toml)] extension: Extension,
) -> Result<()> {
    let system = TestSystem::deploy().await?;

    let tmpdir = tempfile::tempdir()?;
    let payload_file = tmpdir.path().join(format!("payload.{extension}"));

    let mut cmd = system.export_node_signatures_cmd()?;
    cmd.arg("--output").arg(&payload_file);

    cmd.assert().success();

    let content = std::fs::read_to_string(&payload_file)?;
    let parsed = extension.parse_node_signatures(&content)?;
    assert_eq!(parsed.address, system.deployer_address);

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum BadExtension {
    Yaml,
    None,
}

impl BadExtension {
    fn as_filename(&self) -> &'static str {
        match self {
            Self::Yaml => "payload.yaml",
            Self::None => "payload",
        }
    }
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn test_unsupported_file_extensions(
    #[values(BadExtension::Yaml, BadExtension::None)] extension: BadExtension,
) -> Result<()> {
    let system = TestSystem::deploy().await?;

    let tmpdir = tempfile::tempdir()?;
    let payload_file = tmpdir.path().join(extension.as_filename());

    let mut cmd = system.export_node_signatures_cmd()?;
    cmd.arg("--output").arg(&payload_file);

    cmd.assert()
        .failure()
        .stderr(str::contains("Unsupported extension"));

    Ok(())
}

#[test_log::test(rstest_reuse::apply(format_combinations))]
#[tokio::test]
async fn test_export_node_signatures_command(
    #[case] format: Extension,
    #[case] output: Output,
) -> Result<()> {
    let system = TestSystem::deploy().await?;

    let mut cmd = system.export_node_signatures_cmd()?;

    match output {
        Output::Stdout => {
            cmd.arg("--format").arg(format.to_string());
            let output = cmd.output()?;

            assert!(output.status.success(), "Command failed");
            let result = String::from_utf8(output.stdout)?;

            let parsed = format.parse_node_signatures(&result)?;
            assert_eq!(parsed.address, system.deployer_address);
        },
        Output::File => {
            let tmpdir = tempfile::tempdir()?;
            let output_file = tmpdir.path().join(format!("payload.{format}"));
            cmd.arg("--output").arg(&output_file);
            cmd.assert().success();

            assert!(output_file.exists());
            let content = std::fs::read_to_string(&output_file)?;

            let parsed = format.parse_node_signatures(&content)?;
            assert_eq!(parsed.address, system.deployer_address);
        },
    }

    Ok(())
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn test_register_validator_with_pre_signed_payload(
    #[values(StakeTableContractVersion::V1, StakeTableContractVersion::V2)]
    version: StakeTableContractVersion,
    #[values(Extension::Json, Extension::Toml)] format: Extension,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    let tmpdir = tempfile::tempdir()?;
    let payload_path = tmpdir.path().join(format!("payload.{format}"));

    let mut sign_cmd = system.export_node_signatures_cmd()?;
    sign_cmd.arg("--output").arg(&payload_path);

    sign_cmd.assert().success();

    let mut reg_cmd = system.cmd(Signer::Mnemonic);
    reg_cmd
        .arg("register-validator")
        .arg("--commission")
        .arg("12.34")
        .arg("--node-signatures")
        .arg(&payload_path);

    if let Extension::Toml = format {
        reg_cmd.arg("--format").arg("toml");
    }

    let output = reg_cmd.output()?;
    output.assert().success();

    Ok(())
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn test_update_consensus_keys_with_pre_signed_payload(
    #[values(StakeTableContractVersion::V1, StakeTableContractVersion::V2)]
    version: StakeTableContractVersion,
    #[values(Extension::Json, Extension::Toml)] format: Extension,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    system.register_validator().await?;

    let mut rng = rand::rngs::StdRng::from_seed([43u8; 32]);
    let (_, new_bls, new_state) = TestSystem::gen_keys(&mut rng);

    let tmpdir = tempfile::tempdir()?;
    let payload_path = tmpdir.path().join(format!("payload.{format}"));

    let mut sign_cmd = base_cmd();
    sign_cmd
        .arg("export-node-signatures")
        .arg("--address")
        .arg(system.deployer_address.to_string())
        .arg("--consensus-private-key")
        .arg(new_bls.sign_key_ref().to_tagged_base64()?.to_string())
        .arg("--state-private-key")
        .arg(new_state.sign_key().to_tagged_base64()?.to_string());

    sign_cmd.arg("--output").arg(&payload_path);

    sign_cmd.assert().success();

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("update-consensus-keys")
        .arg("--node-signatures")
        .arg(&payload_path);

    if let Extension::Toml = format {
        cmd.arg("--format").arg("toml");
    }

    let output = cmd.output()?;
    output.assert().success();

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_address_validation_mismatch_error() -> Result<()> {
    let system = TestSystem::deploy().await?;

    let tmpdir = tempfile::tempdir()?;
    let payload_file = tmpdir.path().join("payload.json");

    let mut sign_cmd = system.export_node_signatures_cmd()?;
    sign_cmd.arg("--output").arg(&payload_file);

    sign_cmd.assert().success();

    let payload_content = std::fs::read_to_string(&payload_file)?;
    let mut payload: serde_json::Value = serde_json::from_str(&payload_content)?;

    let different_address = "0x1111111111111111111111111111111111111111";
    payload["address"] = serde_json::Value::String(different_address.to_string());

    std::fs::write(&payload_file, serde_json::to_string_pretty(&payload)?)?;

    let mut cmd = system.cmd(Signer::Mnemonic);

    cmd.arg("register-validator")
        .arg("--commission")
        .arg("12.34")
        .arg("--node-signatures")
        .arg(&payload_file)
        .assert()
        .failure()
        .stderr(str::contains("Address mismatch"));
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum BadPayloadScenario {
    Address,
    Bls,
    Schnorr,
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn test_signature_verification_failure(
    #[values(
        BadPayloadScenario::Address,
        BadPayloadScenario::Bls,
        BadPayloadScenario::Schnorr
    )]
    scenario: BadPayloadScenario,
) -> Result<()> {
    let system = TestSystem::deploy().await?;

    let mut rng = rand::rngs::StdRng::from_seed([99u8; 32]);
    let (_, bad_bls, bad_schnorr) = TestSystem::gen_keys(&mut rng);

    let tmpdir = tempfile::tempdir()?;
    let payload_file = tmpdir.path().join("payload.json");

    let mut sign_cmd = system.export_node_signatures_cmd()?;
    sign_cmd.arg("--output").arg(&payload_file);

    let result = sign_cmd.output()?;
    result.assert().success();

    let mut payload: NodeSignatures = {
        let content = std::fs::read_to_string(&payload_file)?;
        serde_json::from_str(&content)?
    };

    match scenario {
        BadPayloadScenario::Address => {
            payload.address = "0x1111111111111111111111111111111111111111".parse()?;
        },
        BadPayloadScenario::Bls => {
            payload.bls_signature = stake_table::sign_address_bls(&bad_bls, payload.address);
        },
        BadPayloadScenario::Schnorr => {
            payload.schnorr_signature =
                stake_table::sign_address_schnorr(&bad_schnorr, payload.address);
        },
    };

    let tampered_content = serde_json::to_string_pretty(&payload)?;
    std::fs::write(&payload_file, tampered_content)?;

    let mut cmd = system.cmd(Signer::Mnemonic);

    let cmd_result = cmd
        .arg("register-validator")
        .arg("--commission")
        .arg("12.34")
        .arg("--node-signatures")
        .arg(&payload_file);

    let out = cmd_result.assert().failure();
    match scenario {
        BadPayloadScenario::Address => {
            out.stderr(str::contains("Address mismatch"));
        },
        BadPayloadScenario::Bls => {
            out.stderr(str::contains("BLS"));
        },
        BadPayloadScenario::Schnorr => {
            out.stderr(str::contains("Schnorr"));
        },
    }

    Ok(())
}
