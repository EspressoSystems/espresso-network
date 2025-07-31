use std::{
    io::Read as _,
    path::{Path, PathBuf},
};

use alloy::{
    network::{Ethereum, EthereumWallet, NetworkWallet},
    primitives::Address,
};
use anyhow::bail;
use clap::Args;
use hotshot_contract_adapter::{
    sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol},
    stake_table::{self, StateSignatureSol},
};
use hotshot_types::{
    light_client::{StateKeyPair, StateSignature, StateVerKey},
    signature_key::BLSPubKey,
};
use jf_signature::bls_over_bn254;
use serde::{Deserialize, Serialize};

use crate::{parse, BLSKeyPair, BLSPrivKey, StateSignKey};

/// Node signatures containing pre-signed address signatures for validator operations
///
/// This is the native rust type.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeSignatures {
    /// The Ethereum address that was signed
    pub address: Address,
    /// BLS verification key
    pub bls_vk: BLSPubKey,
    /// BLS signature over the address
    pub bls_signature: bls_over_bn254::Signature,
    /// Schnorr verification key
    pub schnorr_vk: StateVerKey,
    /// Schnorr signature over the address
    pub schnorr_signature: StateSignature,
}

/// Only used for serialization to Solidity.
#[derive(Clone, Debug)]
pub struct NodeSignaturesSol {
    /// The Ethereum address that was signed
    pub address: Address,
    /// BLS verification key
    pub bls_vk: G2PointSol,
    /// BLS signature over the address
    pub bls_signature: G1PointSol,
    /// Schnorr verification key
    pub schnorr_vk: EdOnBN254PointSol,
    /// Schnorr signature over the address
    pub schnorr_signature: StateSignatureSol,
}

/// Represents either keys for signing or a pre-prepared node signature source
#[allow(clippy::large_enum_variant)]
pub enum NodeSignatureInput {
    /// Sign using private keys
    Keys {
        address: Address,
        bls_key_pair: BLSKeyPair,
        schnorr_key_pair: StateKeyPair,
    },
    /// Load from prepared node signature source
    PreparedPayload(NodeSignatureSource),
}

/// Serialization formats supported by the CLI
#[derive(Clone, Debug, Copy, Default, clap::ValueEnum, PartialEq, Eq)]
pub enum SerializationFormat {
    #[default]
    #[value(name = "json")]
    Json,
    #[value(name = "toml")]
    Toml,
}

/// Source for pre-prepared NodeSignatures
#[derive(Debug)]
pub enum NodeSignatureSource {
    /// Read from stdin with specified format
    Stdin(SerializationFormat),
    /// Read from file path with optional format override
    File {
        path: PathBuf,
        format: Option<SerializationFormat>,
    },
}

/// Destination for node signature output
#[derive(Debug)]
pub enum NodeSignatureDestination {
    /// Write to stdout with specified format
    Stdout(SerializationFormat),
    /// Write to file with specified format
    File {
        path: PathBuf,
        format: SerializationFormat,
    },
}

/// Clap arguments for node signature operations
#[derive(Args, Clone, Debug)]
pub struct NodeSignatureArgs {
    /// The consensus signing key. Used to sign a message to prove ownership of the key.
    #[clap(long, value_parser = parse::parse_bls_priv_key, env = "CONSENSUS_PRIVATE_KEY", required_unless_present = "node_signatures")]
    pub consensus_private_key: Option<BLSPrivKey>,

    /// The state signing key.
    #[clap(long, value_parser = parse::parse_state_priv_key, env = "STATE_PRIVATE_KEY", required_unless_present = "node_signatures")]
    pub state_private_key: Option<StateSignKey>,

    /// Path to file or "-" for stdin (format auto-detected)
    #[clap(long, required_unless_present_all = ["consensus_private_key", "state_private_key"])]
    pub node_signatures: Option<PathBuf>,

    /// Input format for stdin (auto-detected for files)
    #[clap(long, value_enum)]
    pub format: Option<SerializationFormat>,
}

/// Clap arguments for output operations
#[derive(Args, Clone, Debug)]
pub struct OutputArgs {
    /// Output file path. If not specified, outputs to stdout
    #[clap(long)]
    pub output: Option<PathBuf>,

    /// Output format
    #[clap(long, value_enum)]
    pub format: Option<SerializationFormat>,
}

impl TryFrom<&Path> for SerializationFormat {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> anyhow::Result<Self> {
        let extension = path.extension().and_then(|ext| ext.to_str());
        match extension {
            Some("json") => Ok(SerializationFormat::Json),
            Some("toml") => Ok(SerializationFormat::Toml),
            _ => anyhow::bail!(
                "Unsupported extension in path '{}'. Expected .json or .toml",
                path.display()
            ),
        }
    }
}

impl From<NodeSignatures> for NodeSignaturesSol {
    fn from(payload: NodeSignatures) -> Self {
        Self {
            address: payload.address,
            bls_vk: payload.bls_vk.to_affine().into(),
            bls_signature: payload.bls_signature.into(),
            schnorr_vk: payload.schnorr_vk.into(),
            schnorr_signature: StateSignatureSol::from(payload.schnorr_signature).into(),
        }
    }
}

impl NodeSignatures {
    /// Create NodeSignatures by signing an Ethereum address with BLS and Schnorr keys
    pub fn create(
        address: Address,
        bls_key_pair: &BLSKeyPair,
        schnorr_key_pair: &StateKeyPair,
    ) -> Self {
        let bls_signature = stake_table::sign_address_bls(bls_key_pair, address);
        let schnorr_signature = stake_table::sign_address_schnorr(schnorr_key_pair, address);

        Self {
            address,
            bls_vk: bls_key_pair.ver_key(),
            bls_signature,
            schnorr_vk: schnorr_key_pair.ver_key(),
            schnorr_signature,
        }
    }

    /// Verify that the BLS and Schnorr signatures are valid for the given address
    pub fn verify_signatures(&self, address: Address) -> anyhow::Result<()> {
        stake_table::authenticate_bls_sig(&self.bls_vk, address, &self.bls_signature)?;
        stake_table::authenticate_schnorr_sig(&self.schnorr_vk, address, &self.schnorr_signature)?;
        Ok(())
    }

    /// Handle output of the payload to the specified destination
    pub fn handle_output(&self, destination: NodeSignatureDestination) -> anyhow::Result<()> {
        match destination {
            NodeSignatureDestination::Stdout(format) => {
                let output = match format {
                    SerializationFormat::Json => serde_json::to_string_pretty(self)?,
                    SerializationFormat::Toml => toml::to_string_pretty(self)?,
                };
                println!("{output}");
            },
            NodeSignatureDestination::File { path, format } => {
                let output = match format {
                    SerializationFormat::Json => serde_json::to_string_pretty(self)?,
                    SerializationFormat::Toml => toml::to_string_pretty(self)?,
                };
                std::fs::write(&path, output)?;
                tracing::info!("{:?} signatures written to {}", format, path.display());
            },
        }
        Ok(())
    }
}

impl NodeSignatureSource {
    /// Parse NodeSignatureSource from a PathBuf and optional format, where "-" means stdin
    pub(crate) fn parse(
        path: PathBuf,
        format: Option<SerializationFormat>,
    ) -> anyhow::Result<Self> {
        if path.to_string_lossy() == "-" {
            Ok(Self::Stdin(format.unwrap_or_default()))
        } else {
            // Infer format from extension if not explicitly provided
            let format = if let Some(format) = format {
                Some(format)
            } else {
                Some(SerializationFormat::try_from(path.as_path())?)
            };
            Ok(Self::File { path, format })
        }
    }
}

impl TryFrom<OutputArgs> for NodeSignatureDestination {
    type Error = anyhow::Error;

    fn try_from(args: OutputArgs) -> anyhow::Result<Self> {
        match args.output {
            None => {
                let format = args.format.unwrap_or_default();
                Ok(Self::Stdout(format))
            },
            Some(path) => {
                // Format selection logic:
                // 1. If format is explicitly specified, use it (allows overrides)
                // 2. If no format specified, infer from extension
                let final_format = match args.format {
                    Some(explicit_format) => explicit_format,
                    None => SerializationFormat::try_from(path.as_path())?,
                };
                Ok(Self::File {
                    path,
                    format: final_format,
                })
            },
        }
    }
}

impl TryFrom<NodeSignatureSource> for NodeSignatures {
    type Error = anyhow::Error;

    fn try_from(source: NodeSignatureSource) -> anyhow::Result<Self> {
        match source {
            NodeSignatureSource::Stdin(format) => {
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;

                match format {
                    SerializationFormat::Json => serde_json::from_str::<Self>(&buffer)
                        .or_else(|e| bail!("Failed to parse JSON from stdin: {e}")),
                    SerializationFormat::Toml => toml::from_str::<Self>(&buffer)
                        .or_else(|e| bail!("Failed to parse TOML from stdin: {e}")),
                }
            },
            NodeSignatureSource::File { path, format } => {
                let content = std::fs::read_to_string(&path)?;

                let format = match format {
                    Some(f) => f,
                    None => SerializationFormat::try_from(path.as_path())?,
                };

                match format {
                    SerializationFormat::Json => serde_json::from_str::<Self>(&content)
                        .or_else(|e| bail!("Failed to parse JSON file {}: {e}", path.display())),
                    SerializationFormat::Toml => toml::from_str::<Self>(&content)
                        .or_else(|e| bail!("Failed to parse TOML file {}: {e}", path.display())),
                }
            },
        }
    }
}

impl TryFrom<NodeSignatureInput> for NodeSignatures {
    type Error = anyhow::Error;

    fn try_from(input: NodeSignatureInput) -> anyhow::Result<Self> {
        match input {
            NodeSignatureInput::Keys {
                address,
                bls_key_pair,
                schnorr_key_pair,
            } => Ok(Self::create(address, &bls_key_pair, &schnorr_key_pair)),
            NodeSignatureInput::PreparedPayload(source) => Self::try_from(source),
        }
    }
}

impl TryFrom<(NodeSignatureArgs, &EthereumWallet)> for NodeSignatureInput {
    type Error = anyhow::Error;

    fn try_from((args, wallet): (NodeSignatureArgs, &EthereumWallet)) -> anyhow::Result<Self> {
        let wallet_address =
            <EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(wallet);

        if let Some(sig_path) = args.node_signatures {
            let source = NodeSignatureSource::parse(sig_path, args.format)?;
            Ok(Self::PreparedPayload(source))
        } else {
            let Some(bls_key) = args.consensus_private_key else {
                bail!("consensus_private_key is required when not using node_signatures")
            };
            let Some(state_key) = args.state_private_key else {
                bail!("state_private_key is required when not using node_signatures")
            };

            Ok(Self::Keys {
                address: wallet_address,
                bls_key_pair: bls_key.into(),
                schnorr_key_pair: StateKeyPair::from_sign_key(state_key),
            })
        }
    }
}

/// Verifies the signatures sign the Ethereum wallet address
impl TryFrom<(NodeSignatureInput, &EthereumWallet)> for NodeSignatures {
    type Error = anyhow::Error;

    fn try_from((input, wallet): (NodeSignatureInput, &EthereumWallet)) -> anyhow::Result<Self> {
        match input {
            NodeSignatureInput::Keys {
                address,
                bls_key_pair,
                schnorr_key_pair,
            } => Ok(Self::create(address, &bls_key_pair, &schnorr_key_pair)),
            NodeSignatureInput::PreparedPayload(source) => {
                let payload = Self::try_from(source)?;
                let wallet_address =
                    <EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(wallet);

                // Verify the signatures match the expected keys using the wallet address
                payload.verify_signatures(wallet_address)?;

                // Should never fail unless serialized payload was tampered with
                if payload.address != wallet_address {
                    bail!(
                        "Address mismatch: payload contains {}, but wallet address is {}",
                        payload.address,
                        wallet_address
                    );
                }

                Ok(payload)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use alloy::primitives::Address;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use rstest::*;
    use tempfile::NamedTempFile;

    use super::*;
    use crate::BLSKeyPair;

    #[fixture]
    fn sample_address() -> Address {
        "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()
    }

    #[fixture]
    fn sample_node_signatures(sample_address: Address) -> NodeSignatures {
        let mut rng = StdRng::from_seed([42u8; 32]);
        let bls_key = BLSKeyPair::generate(&mut rng);
        let state_key = StateKeyPair::generate_from_seed(rng.gen());

        NodeSignatures::create(sample_address, &bls_key, &state_key)
    }

    #[fixture]
    fn json_content(sample_node_signatures: NodeSignatures) -> String {
        serde_json::to_string_pretty(&sample_node_signatures).unwrap()
    }

    #[fixture]
    fn toml_content(sample_node_signatures: NodeSignatures) -> String {
        toml::to_string_pretty(&sample_node_signatures).unwrap()
    }

    #[rstest]
    fn test_signature_source_parse_stdin() {
        let result = NodeSignatureSource::parse(PathBuf::from("-"), None).unwrap();
        matches!(
            result,
            NodeSignatureSource::Stdin(SerializationFormat::Json)
        );
    }

    #[rstest]
    fn test_signature_source_parse_stdin_with_format() {
        let result =
            NodeSignatureSource::parse(PathBuf::from("-"), Some(SerializationFormat::Toml))
                .unwrap();
        matches!(
            result,
            NodeSignatureSource::Stdin(SerializationFormat::Toml)
        );
    }

    #[rstest]
    #[case("test.json", true)]
    #[case("test.toml", true)]
    #[case("test.txt", false)]
    #[case("test", false)]
    #[case("test.yaml", false)]
    fn test_signature_source_parse_file(#[case] filename: &str, #[case] should_succeed: bool) {
        let path = PathBuf::from(filename);
        let result = NodeSignatureSource::parse(path.clone(), None);

        if should_succeed {
            let source = result.unwrap();
            matches!(source, NodeSignatureSource::File { path: p, .. } if p == path);
        } else {
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Unsupported extension"));
        }
    }

    #[rstest]
    fn test_signature_destination_stdout() {
        let args = OutputArgs {
            output: None,
            format: Some(SerializationFormat::Json),
        };
        let result = NodeSignatureDestination::try_from(args).unwrap();
        matches!(
            result,
            NodeSignatureDestination::Stdout(SerializationFormat::Json)
        );
    }

    #[rstest]
    #[case("test.json", SerializationFormat::Json)]
    #[case("test.toml", SerializationFormat::Toml)]
    fn test_signature_destination_file_valid(
        #[case] filename: &str,
        #[case] expected_format: SerializationFormat,
    ) {
        let path = PathBuf::from(filename);
        let args = OutputArgs {
            output: Some(path.clone()),
            format: None, // Let it infer from extension
        };
        let result = NodeSignatureDestination::try_from(args).unwrap();

        match result {
            NodeSignatureDestination::File { path: p, format } => {
                assert_eq!(p, path);
                assert_eq!(format, expected_format);
            },
            _ => panic!("Expected File variant"),
        }
    }

    #[rstest]
    fn test_parse_json_file(
        json_content: String,
        sample_node_signatures: NodeSignatures,
    ) -> anyhow::Result<()> {
        let temp_file = NamedTempFile::with_suffix(".json")?;
        std::fs::write(temp_file.path(), &json_content)?;

        let source = NodeSignatureSource::File {
            path: temp_file.path().to_path_buf(),
            format: Some(SerializationFormat::Json),
        };
        let parsed = NodeSignatures::try_from(source)?;

        assert_eq!(parsed.address, sample_node_signatures.address);
        assert_eq!(parsed.bls_vk, sample_node_signatures.bls_vk);
        assert_eq!(parsed.schnorr_vk, sample_node_signatures.schnorr_vk);
        Ok(())
    }

    #[rstest]
    fn test_parse_toml_file(
        toml_content: String,
        sample_node_signatures: NodeSignatures,
    ) -> anyhow::Result<()> {
        let temp_file = NamedTempFile::with_suffix(".toml")?;
        std::fs::write(temp_file.path(), &toml_content)?;

        let source = NodeSignatureSource::File {
            path: temp_file.path().to_path_buf(),
            format: Some(SerializationFormat::Toml),
        };
        let parsed = NodeSignatures::try_from(source)?;

        assert_eq!(parsed.address, sample_node_signatures.address);
        assert_eq!(parsed.bls_vk, sample_node_signatures.bls_vk);
        assert_eq!(parsed.schnorr_vk, sample_node_signatures.schnorr_vk);
        Ok(())
    }

    #[rstest]
    fn test_parse_invalid_json_file() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::with_suffix(".json")?;
        std::fs::write(temp_file.path(), "invalid json")?;

        let source = NodeSignatureSource::File {
            path: temp_file.path().to_path_buf(),
            format: Some(SerializationFormat::Json),
        };
        let result = NodeSignatures::try_from(source);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse JSON file"));
        Ok(())
    }

    #[rstest]
    fn test_parse_invalid_toml_file() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::with_suffix(".toml")?;
        std::fs::write(temp_file.path(), "invalid = toml = syntax")?;

        let source = NodeSignatureSource::File {
            path: temp_file.path().to_path_buf(),
            format: Some(SerializationFormat::Toml),
        };
        let result = NodeSignatures::try_from(source);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse TOML file"));
        Ok(())
    }

    #[rstest]
    #[case(SerializationFormat::Json, ".json")]
    #[case(SerializationFormat::Toml, ".toml")]
    #[case(SerializationFormat::Toml, ".not-toml")]
    fn test_handle_output_to_file(
        sample_node_signatures: NodeSignatures,
        #[case] format: SerializationFormat,
        #[case] suffix: &str,
    ) -> anyhow::Result<()> {
        let temp_file = NamedTempFile::with_suffix(suffix)?;
        let destination = NodeSignatureDestination::File {
            path: temp_file.path().to_path_buf(),
            format,
        };

        sample_node_signatures.handle_output(destination)?;

        let content = std::fs::read_to_string(temp_file.path())?;
        let parsed: NodeSignatures = match format {
            SerializationFormat::Json => serde_json::from_str(&content)?,
            SerializationFormat::Toml => toml::from_str(&content)?,
        };
        assert_eq!(parsed.address, sample_node_signatures.address);
        Ok(())
    }

    #[rstest]
    fn test_create_and_verify_signatures(sample_address: Address) {
        let bls_key = BLSKeyPair::generate(&mut rand::thread_rng());
        let state_key = hotshot_types::light_client::StateKeyPair::generate();

        let signatures = NodeSignatures::create(sample_address, &bls_key, &state_key);

        assert!(signatures.verify_signatures(sample_address).is_ok());

        let wrong_address: Address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        assert!(signatures.verify_signatures(wrong_address).is_err());
    }

    #[rstest]
    #[case("test.txt")]
    #[case("test")]
    #[case("test.yaml")]
    fn test_signature_destination_file_invalid(#[case] filename: &str) {
        let path = PathBuf::from(filename);
        let args = OutputArgs {
            output: Some(path),
            format: None,
        };
        let result = NodeSignatureDestination::try_from(args);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported extension"));
    }

    #[rstest]
    fn test_parse_unsupported_extension() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::with_suffix(".yaml")?;
        std::fs::write(temp_file.path(), "test: content")?;

        let result = NodeSignatureSource::parse(temp_file.path().to_path_buf(), None);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Unsupported extension"),
            "Got error: {}",
            error_msg
        );
        Ok(())
    }

    #[rstest]
    #[case("test.json", SerializationFormat::Json)]
    #[case("test.toml", SerializationFormat::Toml)]
    fn test_serialization_format_from_path_valid(
        #[case] filename: &str,
        #[case] expected_format: SerializationFormat,
    ) {
        let path = PathBuf::from(filename);
        let result = SerializationFormat::try_from(path.as_path()).unwrap();
        assert_eq!(result, expected_format);
    }

    #[rstest]
    #[case("test.txt")]
    #[case("test")]
    #[case("test.yaml")]
    fn test_serialization_format_from_path_invalid(#[case] filename: &str) {
        let path = PathBuf::from(filename);
        let result = SerializationFormat::try_from(path.as_path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported extension"));
    }
}
