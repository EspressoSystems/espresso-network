use anyhow::Result;
use assert_cmd::Command;
use hotshot_types::signature_key::BLSPubKey;
use staking_cli::{deploy::TestSystem, DEV_MNEMONIC, DEV_PRIVATE_KEY};

// Signer variants are selectively used across different test binaries
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Signer {
    Ledger,
    Mnemonic,
    BrokeMnemonic,
    PrivateKey,
}

// MetadataCommand variants used in cli.rs parametrized tests, not node_signatures.rs
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum MetadataCommand {
    RegisterValidator,
    UpdateMetadataUri,
}

#[allow(dead_code)]
impl MetadataCommand {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RegisterValidator => "register-validator",
            Self::UpdateMetadataUri => "update-metadata-uri",
        }
    }
}

pub trait TestSystemExt {
    /// Create a base staking-cli command configured for this test system
    fn cmd(&self, signer: Signer) -> Command;

    // Used in cli.rs but not all test binaries
    #[allow(dead_code)]
    /// Create an export-calldata command with sender-address for validation
    fn export_calldata_cmd(&self) -> Command;

    // Used in node_signatures.rs but not all test binaries
    #[allow(dead_code)]
    /// Create an export-node-signatures command with system keys and address
    fn export_node_signatures_cmd(&self) -> Result<Command>;

    fn bls_private_key_str(&self) -> Result<String>;

    // Used in cli.rs but not all test binaries
    #[allow(dead_code)]
    fn bls_public_key_str(&self) -> String;

    fn state_private_key_str(&self) -> Result<String>;

    // Used in cli.rs parametrized tests, not all test binaries
    #[allow(dead_code)]
    /// Setup base command for metadata operations with prerequisite state and args.
    ///
    /// Returns a Command with subcommand and operation-specific args:
    /// - `register-validator`: consensus keys + commission (fixed 5.00)
    /// - `update-metadata-uri`: called after validator registration
    ///
    /// **Side effect**: For update operations, performs validator registration on-chain first.
    ///
    /// Callers must add metadata-related args (`--metadata-uri`, `--skip-metadata-validation`, etc.)
    async fn setup_metadata_cmd(&self, command: MetadataCommand, signer: Signer)
        -> Result<Command>;
}

impl TestSystemExt for TestSystem {
    fn cmd(&self, signer: Signer) -> Command {
        let mut cmd = base_cmd();
        cmd.arg("--rpc-url")
            .arg(self.rpc_url.to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.to_string())
            .arg("--account-index")
            .arg("0");

        match signer {
            Signer::Ledger => {
                cmd.arg("--ledger");
            },
            Signer::Mnemonic => {
                cmd.arg("--mnemonic").arg(DEV_MNEMONIC);
            },
            Signer::BrokeMnemonic => {
                cmd.arg("--mnemonic").arg(
                    "roast term reopen pave choose high rally trouble upon govern hollow stand",
                );
            },
            Signer::PrivateKey => {
                cmd.arg("--private-key").arg(DEV_PRIVATE_KEY);
            },
        };
        cmd
    }

    fn export_calldata_cmd(&self) -> Command {
        let mut cmd = base_cmd();
        cmd.arg("--rpc-url")
            .arg(self.rpc_url.to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.to_string())
            .arg("--export-calldata")
            .arg("--sender-address")
            .arg(self.deployer_address.to_string());
        cmd
    }

    fn export_node_signatures_cmd(&self) -> Result<Command> {
        let mut cmd = base_cmd();
        cmd.arg("export-node-signatures")
            .arg("--address")
            .arg(self.deployer_address.to_string())
            .arg("--consensus-private-key")
            .arg(self.bls_private_key_str()?)
            .arg("--state-private-key")
            .arg(self.state_private_key_str()?);
        Ok(cmd)
    }

    fn bls_private_key_str(&self) -> Result<String> {
        Ok(self
            .bls_key_pair
            .sign_key_ref()
            .to_tagged_base64()?
            .to_string())
    }

    fn bls_public_key_str(&self) -> String {
        BLSPubKey::from(self.bls_key_pair.ver_key()).to_string()
    }

    fn state_private_key_str(&self) -> Result<String> {
        Ok(self
            .state_key_pair
            .sign_key()
            .to_tagged_base64()?
            .to_string())
    }

    async fn setup_metadata_cmd(
        &self,
        command: MetadataCommand,
        signer: Signer,
    ) -> Result<Command> {
        // Side effect: For update-metadata-uri, register validator on-chain first
        if matches!(command, MetadataCommand::UpdateMetadataUri) {
            self.register_validator().await?;
        }

        let mut cmd = self.cmd(signer);
        cmd.arg(command.as_str());

        // For register-validator, add required node signature args
        if matches!(command, MetadataCommand::RegisterValidator) {
            cmd.arg("--consensus-private-key")
                .arg(self.bls_private_key_str()?)
                .arg("--state-private-key")
                .arg(self.state_private_key_str()?)
                .arg("--commission")
                .arg("5.00"); // Fixed commission for test setup
        }

        Ok(cmd)
    }
}

/// Creates a new command to run the staking-cli binary.
// https://github.com/rust-lang/rust/issues/148426
#[allow(deprecated)]
pub fn base_cmd() -> Command {
    Command::cargo_bin("staking-cli").unwrap()
}
