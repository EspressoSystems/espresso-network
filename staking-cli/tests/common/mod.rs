use alloy::{
    primitives::{B256, U256},
    sol_types::SolValue as _,
};
use anyhow::Result;
use assert_cmd::Command;
use espresso_types::{
    v0::v0_4::{
        RewardAccountProofV2, RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleTreeV2,
    },
    v0_3::RewardAmount,
    v0_4::REWARD_MERKLE_TREE_V2_HEIGHT,
};
use hotshot_contract_adapter::sol_types::{LightClientV3Mock, StakeTableV2};
use jf_merkle_tree_compat::{MerkleCommitment, MerkleTreeScheme, UniversalMerkleTreeScheme};
use staking_cli::{deploy::TestSystem, DEV_MNEMONIC};
use url::Url;
use warp::Filter;

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

    // method is used, but somehow flagged as unused
    #[allow(dead_code)]
    async fn setup_reward_claim_mock(&self, balance: U256) -> Result<Url>;
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

    async fn setup_reward_claim_mock(&self, balance: U256) -> Result<Url> {
        let stake_table = StakeTableV2::new(self.stake_table, &self.provider);
        let light_client_addr = stake_table.lightClient().call().await?;
        let light_client = LightClientV3Mock::new(light_client_addr, &self.provider);

        let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let account = RewardAccountV2::from(self.deployer_address);
        let amount = RewardAmount::from(balance);

        tree.update(account, amount)?;

        let commitment = tree.commitment();
        let root = commitment.digest().into();

        let auth_root_fields: [B256; 8] = [
            root,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
        ];
        let auth_root = alloy::primitives::keccak256(auth_root_fields.abi_encode());

        light_client
            .setAuthRoot(auth_root.into())
            .send()
            .await?
            .get_receipt()
            .await?;

        let query_data: RewardAccountQueryDataV2 = RewardAccountProofV2::prove(&tree, account.0)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate proof"))?
            .into();
        let claim_input = query_data.to_reward_claim_input()?;
        let claim_input = std::sync::Arc::new(claim_input);

        let port = portpicker::pick_unused_port().expect("No ports available");

        let route = warp::path!("reward-state-v2" / "reward-claim-input" / u64 / String).map(
            move |_block_height: u64, _address: String| warp::reply::json(&*claim_input.clone()),
        );

        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));

        Ok(format!("http://localhost:{}/", port).parse()?)
    }
}

/// Creates a new command to run the staking-cli binary.
pub fn base_cmd() -> Command {
    Command::cargo_bin("staking-cli").unwrap()
}
