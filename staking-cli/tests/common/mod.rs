use anyhow::Result;
use assert_cmd::Command;
use staking_cli::{deploy::TestSystem, DEV_MNEMONIC};

// rstest macro usage isn't detected
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Signer {
    Ledger,
    Mnemonic,
    BrokeMnemonic,
}

pub trait TestSystemExt {
    /// Create a base staking-cli command configured for this test system
    fn cmd(&self, signer: Signer) -> Command;

    // method is used, but somehow flagged as unused
    #[allow(dead_code)]
    /// Create an export-node-signatures command with system keys and address
    fn export_node_signatures_cmd(&self) -> Result<Command>;

    fn bls_private_key_str(&self) -> Result<String>;

    fn state_private_key_str(&self) -> Result<String>;
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
        };
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

    fn state_private_key_str(&self) -> Result<String> {
        Ok(self
            .state_key_pair
            .sign_key()
            .to_tagged_base64()?
            .to_string())
    }
}

/// Creates a new command to run the staking-cli binary.
pub fn base_cmd() -> Command {
    Command::cargo_bin("staking-cli").unwrap()
}
