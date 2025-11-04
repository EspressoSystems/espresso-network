use std::{process::Child, time::Duration};

use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use committable::{Commitment, Committable};
use escargot::CargoBuild;
use espresso_types::{BlockMerkleTree, Header, NamespaceProofQueryData, SeqTypes, Transaction};
use futures::{StreamExt, TryStreamExt};
use hotshot_contract_adapter::sol_types::LightClientV2Mock;
use hotshot_query_service::{
    availability::{BlockQueryData, TransactionQueryData, VidCommonQueryData},
    explorer::TransactionDetailResponse,
};
use jf_merkle_tree_compat::MerkleTreeScheme;
use portpicker::pick_unused_port;
use rand::Rng;
use sequencer::SequencerApiVersion;
use serde::{Deserialize, Serialize};
use surf_disco::Client;
use tide_disco::error::ServerError;
use tokio::time::sleep;
use url::Url;

const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
const NUM_ALT_CHAIN_PROVIDERS: usize = 1;

pub struct BackgroundProcess(Child);

impl Drop for BackgroundProcess {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevInfo {
    pub builder_url: Url,
    pub sequencer_api_port: u16,
    pub l1_prover_port: u16,
    pub l1_url: Url,
    pub l1_light_client_address: Address,
    pub alt_chains: Vec<AltChainInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AltChainInfo {
    pub chain_id: u64,
    pub provider_url: Url,
    pub light_client_address: Address,
    pub prover_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetHotshotDownReqBody {
    pub chain_id: Option<u64>,
    pub height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetHotshotUpReqBody {
    pub chain_id: u64,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum DevNodeVersion {
    #[value(name = "0.3")]
    V0_3,
    #[value(name = "0.4")]
    V0_4,
}

impl std::fmt::Display for DevNodeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DevNodeVersion::V0_3 => write!(f, "0.3"),
            DevNodeVersion::V0_4 => write!(f, "0.4"),
        }
    }
}

// If this test failed and you are doing changes on the following stuff, please
// sync your changes to [`espresso-sequencer-go`](https://github.com/EspressoSystems/espresso-sequencer-go)
// and open a PR.
// - APIs update
// - Types (like `Header`) update
#[rstest::rstest]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_dev_node_test(
    #[values(DevNodeVersion::V0_3, DevNodeVersion::V0_4)] version: DevNodeVersion,
) {
    let builder_port = pick_unused_port().unwrap();
    let api_port = pick_unused_port().unwrap();
    let dev_node_port = pick_unused_port().unwrap();
    let instance = Anvil::new().spawn();
    let l1_url = instance.endpoint_url();

    let tmp_dir = tempfile::tempdir().unwrap();

    let process = CargoBuild::new()
        .bin("espresso-dev-node")
        .current_target()
        .run()
        .unwrap()
        .command()
        .env("ESPRESSO_SEQUENCER_L1_PROVIDER", l1_url.to_string())
        .env("ESPRESSO_BUILDER_PORT", builder_port.to_string())
        .env("ESPRESSO_SEQUENCER_API_PORT", api_port.to_string())
        .env("ESPRESSO_SEQUENCER_ETH_MNEMONIC", TEST_MNEMONIC)
        .env("ESPRESSO_DEPLOYER_ACCOUNT_INDEX", "0")
        .env("ESPRESSO_DEV_NODE_PORT", dev_node_port.to_string())
        .env(
            "ESPRESSO_SEQUENCER_STORAGE_PATH",
            tmp_dir.path().as_os_str(),
        )
        .env("ESPRESSO_SEQUENCER_DATABASE_MAX_CONNECTIONS", "25")
        .env("ESPRESSO_DEV_NODE_MAX_BLOCK_SIZE", "500000")
        .env("ESPRESSO_DEV_NODE_VERSION", version.to_string())
        .spawn()
        .unwrap();

    let process = BackgroundProcess(process);

    let api_client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    api_client.connect(None).await;

    tracing::info!("waiting for blocks");
    let _ = api_client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(5)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    let builder_api_client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{builder_port}").parse().unwrap());
    builder_api_client.connect(None).await;

    let tx = Transaction::new(100_u32.into(), vec![1, 2, 3]);

    let hash: Commitment<Transaction> = builder_api_client
        .post("txn_submit/submit")
        .body_json(&tx)
        .unwrap()
        .send()
        .await
        .unwrap();

    let tx_hash = tx.commit();
    assert_eq!(hash, tx_hash);

    let mut tx_result = api_client
        .get::<TransactionQueryData<SeqTypes>>(&format!(
            "availability/transaction/hash/{tx_hash}/noproof",
        ))
        .send()
        .await;
    while tx_result.is_err() {
        sleep(Duration::from_secs(1)).await;
        tracing::warn!("waiting for tx");

        tx_result = api_client
            .get::<TransactionQueryData<SeqTypes>>(&format!(
                "availability/transaction/hash/{tx_hash}/noproof"
            ))
            .send()
            .await;
    }

    let mut tx_result_from_explorer = api_client
        .get::<TransactionDetailResponse<SeqTypes>>(
            &format!("explorer/transaction/hash/{tx_hash}",),
        )
        .send()
        .await;

    while tx_result_from_explorer.is_err() {
        sleep(Duration::from_secs(1)).await;
        tracing::warn!("waiting for tx");

        tx_result_from_explorer = api_client
            .get::<TransactionDetailResponse<SeqTypes>>(&format!(
                "explorer/transaction/hash/{tx_hash}"
            ))
            .send()
            .await;
    }

    let large_tx = Transaction::new(100_u32.into(), vec![0; 20000]);
    let large_hash: Commitment<Transaction> = api_client
        .post("submit/submit")
        .body_json(&large_tx)
        .unwrap()
        .send()
        .await
        .unwrap();

    let tx_hash = large_tx.commit();
    assert_eq!(large_hash, tx_hash);

    let mut tx_result = api_client
        .get::<TransactionQueryData<SeqTypes>>(&format!(
            "availability/transaction/hash/{tx_hash}/noproof",
        ))
        .send()
        .await;
    while tx_result.is_err() {
        tracing::info!("waiting for large tx");
        sleep(Duration::from_secs(1)).await;

        tx_result = api_client
            .get::<TransactionQueryData<SeqTypes>>(&format!(
                "availability/transaction/hash/{tx_hash}/noproof"
            ))
            .send()
            .await;
    }

    {
        // transactions with size larger than max_block_size result in an error
        let extremely_large_tx = Transaction::new(100_u32.into(), vec![0; 7 * 1000 * 1000]);
        api_client
            .post::<Commitment<Transaction>>("submit/submit")
            .body_json(&extremely_large_tx)
            .unwrap()
            .send()
            .await
            .unwrap_err();

        // Now we send a small transaction to make sure this transaction can be included in a hotshot block.
        let tx = Transaction::new(100_u32.into(), vec![0; 3]);
        let tx_hash: Commitment<Transaction> = api_client
            .post("submit/submit")
            .body_json(&tx)
            .unwrap()
            .send()
            .await
            .unwrap();

        let mut result = api_client
            .get::<TransactionQueryData<SeqTypes>>(&format!(
                "availability/transaction/hash/{tx_hash}/noproof",
            ))
            .send()
            .await;
        while result.is_err() {
            sleep(Duration::from_secs(1)).await;

            result = api_client
                .get::<TransactionQueryData<SeqTypes>>(&format!(
                    "availability/transaction/hash/{tx_hash}/noproof"
                ))
                .send()
                .await;
        }
    }

    let tx_block_height = tx_result.unwrap().block_height();

    // Check the namespace proof
    let proof = api_client
        .get::<NamespaceProofQueryData>(&format!(
            "availability/block/{tx_block_height}/namespace/100"
        ))
        .send()
        .await
        .unwrap();
    assert!(proof.proof.is_some());

    // These endpoints are currently used in `espresso-sequencer-go`. These checks
    // serve as reminders of syncing the API updates to go client repo when they change.
    {
        api_client
            .get::<u64>("status/block-height")
            .send()
            .await
            .unwrap();

        api_client
            .get::<Header>("availability/header/3")
            .send()
            .await
            .unwrap();

        api_client
            .get::<VidCommonQueryData<SeqTypes>>(&format!(
                "availability/vid/common/{tx_block_height}"
            ))
            .send()
            .await
            .unwrap();

        while api_client
            .get::<<BlockMerkleTree as MerkleTreeScheme>::MembershipProof>("block-state/3/2")
            .send()
            .await
            .is_err()
        {
            sleep(Duration::from_secs(1)).await;
        }
    }

    let dev_node_client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{dev_node_port}").parse().unwrap());
    dev_node_client.connect(None).await;

    // Check the dev node api
    {
        tracing::info!("checking the dev node api");
        let dev_info = dev_node_client
            .get::<DevInfo>("api/dev-info")
            .send()
            .await
            .unwrap();

        let light_client_address = dev_info.l1_light_client_address;

        let signer = MnemonicBuilder::<English>::default()
            .phrase(TEST_MNEMONIC)
            .index(0)
            .unwrap()
            .build()
            .unwrap();
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer))
            .connect_http(l1_url.clone());

        let light_client = LightClientV2Mock::new(light_client_address, &provider);

        while light_client
            .getHotShotCommitment(U256::from(1))
            .call()
            .await
            .is_err()
        {
            tracing::info!("waiting for commitment");
            sleep(Duration::from_secs(3)).await;
        }

        let height = provider.get_block_number().await.unwrap();
        dev_node_client
            .post::<()>("api/set-hotshot-down")
            .body_json(&SetHotshotDownReqBody {
                chain_id: None,
                height: height - 1,
            })
            .unwrap()
            .send()
            .await
            .unwrap();

        while !light_client
            .lagOverEscapeHatchThreshold(U256::from(height), U256::from(0))
            .call()
            .await
            .unwrap_or(false)
        {
            tracing::info!("waiting for setting hotshot down");
            sleep(Duration::from_secs(3)).await;
        }

        dev_node_client
            .post::<()>("api/set-hotshot-up")
            .body_json(&())
            .unwrap()
            .send()
            .await
            .unwrap();

        while light_client
            .lagOverEscapeHatchThreshold(U256::from(height), U256::from(0))
            .call()
            .await
            .unwrap_or(true)
        {
            tracing::info!("waiting for setting hotshot up");
            sleep(Duration::from_secs(3)).await;
        }
    }

    drop(process);
}

async fn alt_chain_providers() -> (Vec<AnvilInstance>, Vec<Url>) {
    let mut providers = Vec::new();
    let mut urls = Vec::new();

    for _ in 0..NUM_ALT_CHAIN_PROVIDERS {
        let mut rng = rand::thread_rng();

        let anvil = Anvil::default()
            .chain_id(rng.gen_range(2..u32::MAX) as u64)
            .spawn();
        let url = anvil.endpoint_url();

        providers.push(anvil);
        urls.push(url);
    }

    (providers, urls)
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_dev_node_multiple_lc_providers_test() {
    let builder_port = pick_unused_port().unwrap();
    let api_port = pick_unused_port().unwrap();
    let dev_node_port = pick_unused_port().unwrap();

    let instance = Anvil::new().chain_id(1).spawn();
    let l1_url = instance.endpoint_url();

    let (alt_providers, alt_chain_urls) = alt_chain_providers().await;

    let alt_chains_env_value = alt_chain_urls
        .iter()
        .map(|url| url.as_str())
        .collect::<Vec<&str>>()
        .join(",");

    let tmp_dir = tempfile::tempdir().unwrap();

    let process = CargoBuild::new()
        .bin("espresso-dev-node")
        .current_target()
        .run()
        .unwrap()
        .command()
        .env("ESPRESSO_SEQUENCER_L1_PROVIDER", l1_url.to_string())
        .env("ESPRESSO_BUILDER_PORT", builder_port.to_string())
        .env("ESPRESSO_SEQUENCER_API_PORT", api_port.to_string())
        .env("ESPRESSO_SEQUENCER_ETH_MNEMONIC", TEST_MNEMONIC)
        .env("ESPRESSO_DEPLOYER_ACCOUNT_INDEX", "0")
        .env("ESPRESSO_DEV_NODE_PORT", dev_node_port.to_string())
        .env(
            "ESPRESSO_DEPLOYER_ALT_CHAIN_PROVIDERS",
            alt_chains_env_value,
        )
        .env(
            "ESPRESSO_SEQUENCER_STORAGE_PATH",
            tmp_dir.path().as_os_str(),
        )
        .env("ESPRESSO_SEQUENCER_DATABASE_MAX_CONNECTIONS", "25")
        .spawn()
        .unwrap();

    let process = BackgroundProcess(process);

    let api_client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    api_client.connect(None).await;

    tracing::info!("waiting for blocks");
    let _ = api_client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(5)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    let dev_node_client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{dev_node_port}").parse().unwrap());
    dev_node_client.connect(None).await;

    // Check the dev node api
    {
        tracing::info!("checking the dev node api");
        let dev_info = dev_node_client
            .get::<DevInfo>("api/dev-info")
            .send()
            .await
            .unwrap();

        let light_client_address = dev_info.l1_light_client_address;

        let signer = MnemonicBuilder::<English>::default()
            .phrase(TEST_MNEMONIC)
            .index(0)
            .unwrap()
            .build()
            .unwrap();
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer))
            .connect_http(l1_url.clone());

        let light_client = LightClientV2Mock::new(light_client_address, &provider);

        while light_client
            .getHotShotCommitment(U256::from(1))
            .call()
            .await
            .is_err()
        {
            tracing::info!("waiting for commitment");
            sleep(Duration::from_secs(3)).await;
        }

        for AltChainInfo {
            provider_url,
            light_client_address,
            chain_id,
            ..
        } in dev_info.alt_chains
        {
            tracing::info!("checking hotshot commitment for {chain_id}");

            let signer = MnemonicBuilder::<English>::default()
                .phrase(TEST_MNEMONIC)
                .index(0)
                .unwrap()
                .build()
                .unwrap();
            let provider = ProviderBuilder::new()
                .wallet(EthereumWallet::from(signer))
                .connect_http(provider_url.clone());

            let light_client = LightClientV2Mock::new(light_client_address, &provider);

            while light_client
                .getHotShotCommitment(U256::from(1))
                .call()
                .await
                .is_err()
            {
                tracing::info!("waiting for commitment");
                sleep(Duration::from_secs(3)).await;
            }

            let height = provider.get_block_number().await.unwrap();
            dev_node_client
                .post::<()>("api/set-hotshot-down")
                .body_json(&SetHotshotDownReqBody {
                    chain_id: Some(chain_id),
                    height: height - 1,
                })
                .unwrap()
                .send()
                .await
                .unwrap();

            while !light_client
                .lagOverEscapeHatchThreshold(U256::from(height), U256::from(0))
                .call()
                .await
                .unwrap_or(false)
            {
                tracing::info!("waiting for setting hotshot down");
                sleep(Duration::from_secs(3)).await;
            }

            dev_node_client
                .post::<()>("api/set-hotshot-up")
                .body_json(&SetHotshotUpReqBody { chain_id })
                .unwrap()
                .send()
                .await
                .unwrap();

            while light_client
                .lagOverEscapeHatchThreshold(U256::from(height), U256::from(0))
                .call()
                .await
                .unwrap_or(true)
            {
                tracing::info!("waiting for setting hotshot up");
                sleep(Duration::from_secs(3)).await;
            }
        }
    }

    drop(process);
    drop(alt_providers);
}
