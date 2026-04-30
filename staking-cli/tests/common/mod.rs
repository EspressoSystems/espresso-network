use anyhow::Result;
use assert_cmd::Command;
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_types::signature_key::BLSPubKey;
use staking_cli::{DEV_MNEMONIC, DEV_PRIVATE_KEY, deploy::TestSystem};

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

/// Wraps `assert_cmd::Command` with a reference to `TestSystem` for convenience methods.
#[allow(dead_code)]
pub struct TestCommand<'a> {
    pub cmd: Command,
    system: &'a TestSystem,
}

#[allow(dead_code)]
impl<'a> TestCommand<'a> {
    pub fn arg(mut self, arg: impl AsRef<std::ffi::OsStr>) -> Self {
        self.cmd.arg(arg);
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> Self {
        self.cmd.args(args);
        self
    }

    /// Add consensus keys (BLS, Schnorr) and V3 network args (x25519, p2p) when applicable.
    pub fn with_keys(mut self) -> Self {
        self.cmd
            .arg("--consensus-private-key")
            .arg(
                self.system
                    .bls_private_key_str()
                    .expect("bls_private_key_str"),
            )
            .arg("--state-private-key")
            .arg(
                self.system
                    .state_private_key_str()
                    .expect("state_private_key_str"),
            );
        if matches!(self.system.version, StakeTableContractVersion::V3) {
            self.cmd
                .arg("--x25519-key")
                .arg(self.system.x25519_public_key_str())
                .arg("--p2p-addr")
                .arg("127.0.0.1:8080");
        }
        self
    }

    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.cmd.timeout(timeout);
        self
    }

    /// Extract the inner `Command`, e.g. for spawning into async tasks.
    pub fn into_inner(self) -> Command {
        self.cmd
    }

    pub fn assert(mut self) -> assert_cmd::assert::Assert {
        self.cmd.assert()
    }

    pub fn output(mut self) -> std::io::Result<std::process::Output> {
        self.cmd.output()
    }
}

pub trait TestSystemExt {
    /// Create a base staking-cli command configured for this test system
    fn cmd(&self, signer: Signer) -> TestCommand<'_>;

    // Used in cli.rs but not all test binaries
    #[allow(dead_code)]
    /// Create an export-calldata command with sender-address for validation
    fn export_calldata_cmd(&self) -> TestCommand<'_>;

    // Used in cli.rs but not all test binaries
    #[allow(dead_code)]
    /// Create an export-node-signatures command with system keys and address
    fn export_node_signatures_cmd(&self) -> Result<TestCommand<'_>>;

    #[allow(dead_code)]
    fn bls_private_key_str(&self) -> Result<String>;

    // Used in cli.rs but not all test binaries
    #[allow(dead_code)]
    fn bls_public_key_str(&self) -> String;

    #[allow(dead_code)]
    fn state_private_key_str(&self) -> Result<String>;

    #[allow(dead_code)]
    fn x25519_public_key_str(&self) -> String;

    // Used in cli.rs parametrized tests, not all test binaries
    #[allow(dead_code)]
    /// Setup base command for metadata operations with prerequisite state and args.
    ///
    /// Returns a TestCommand with subcommand and operation-specific args:
    /// - `register-validator`: consensus keys + commission (fixed 5.00)
    /// - `update-metadata-uri`: called after validator registration
    ///
    /// **Side effect**: For update operations, performs validator registration on-chain first.
    ///
    /// Callers must add metadata-related args (`--metadata-uri`, `--skip-metadata-validation`, etc.)
    async fn setup_metadata_cmd(
        &self,
        command: MetadataCommand,
        signer: Signer,
    ) -> Result<TestCommand<'_>>;
}

impl TestSystemExt for TestSystem {
    fn cmd(&self, signer: Signer) -> TestCommand<'_> {
        let mut cmd = base_cmd();
        cmd.arg("--rpc-url")
            .arg(self.rpc_url.to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.to_string());

        match signer {
            Signer::Ledger => {
                cmd.arg("--ledger").arg("--account-index").arg("0");
            },
            Signer::Mnemonic => {
                cmd.arg("--mnemonic")
                    .arg(DEV_MNEMONIC)
                    .arg("--account-index")
                    .arg("0");
            },
            Signer::BrokeMnemonic => {
                cmd.arg("--mnemonic")
                    .arg(
                        "roast term reopen pave choose high rally trouble upon govern hollow stand",
                    )
                    .arg("--account-index")
                    .arg("0");
            },
            Signer::PrivateKey => {
                cmd.arg("--private-key").arg(DEV_PRIVATE_KEY);
            },
        };
        TestCommand { cmd, system: self }
    }

    fn export_calldata_cmd(&self) -> TestCommand<'_> {
        let mut cmd = base_cmd();
        cmd.arg("--rpc-url")
            .arg(self.rpc_url.to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.to_string())
            .arg("--export-calldata")
            .arg("--sender-address")
            .arg(self.deployer_address.to_string());
        TestCommand { cmd, system: self }
    }

    fn export_node_signatures_cmd(&self) -> Result<TestCommand<'_>> {
        let mut cmd = base_cmd();
        cmd.arg("export-node-signatures")
            .arg("--address")
            .arg(self.deployer_address.to_string())
            .arg("--consensus-private-key")
            .arg(self.bls_private_key_str()?)
            .arg("--state-private-key")
            .arg(self.state_private_key_str()?);
        Ok(TestCommand { cmd, system: self })
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

    fn x25519_public_key_str(&self) -> String {
        self.x25519_keypair.public_key().to_string()
    }

    async fn setup_metadata_cmd(
        &self,
        command: MetadataCommand,
        signer: Signer,
    ) -> Result<TestCommand<'_>> {
        // Side effect: For update-metadata-uri, register validator on-chain first
        if matches!(command, MetadataCommand::UpdateMetadataUri) {
            self.register_validator().await?;
        }

        let cmd = self.cmd(signer).arg(command.as_str());

        // For register-validator, add required node signature args
        let cmd = if matches!(command, MetadataCommand::RegisterValidator) {
            cmd.with_keys().arg("--commission").arg("5.00")
        } else {
            cmd
        };

        Ok(cmd)
    }
}

/// Creates a new command to run the staking-cli binary.
// https://github.com/rust-lang/rust/issues/148426
#[allow(deprecated)]
pub fn base_cmd() -> Command {
    Command::cargo_bin("staking-cli").unwrap()
}
