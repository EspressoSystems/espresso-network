use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    process::{Child, Command},
    time::Duration,
};

use anyhow::anyhow;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use committable::{Commitment, Committable};
use ethers::{
    abi::Detokenize,
    contract::builders::ContractCall,
    prelude::*,
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, Signer as _},
    types::U256,
};
use tempfile::TempDir;
use tokio::time::sleep;
use url::Url;

pub mod blocknative;
pub mod deployer;
pub mod logging;
pub mod ser;
pub mod stake_table;
pub mod test_utils;

pub type Signer = SignerMiddleware<Provider<Http>, LocalWallet>;
pub type NonceManager = NonceManagerMiddleware<Signer>;

#[derive(Clone, Debug, Default)]
pub struct AnvilOptions {
    block_time: Option<Duration>,
    port: Option<u16>,
    load_state: Option<PathBuf>,
    accounts: Option<usize>,
    chain_id: Option<u64>,
}

impl AnvilOptions {
    pub fn block_time(mut self, time: Duration) -> Self {
        self.block_time = Some(time);
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn load_state(mut self, path: PathBuf) -> Self {
        self.load_state = Some(path);
        self
    }

    pub fn accounts(mut self, accounts: usize) -> Self {
        self.accounts = Some(accounts);
        self
    }

    pub fn chain_id(mut self, id: u64) -> Self {
        self.chain_id = Some(id);
        self
    }

    pub async fn spawn(self) -> Anvil {
        let state_dir = TempDir::new().unwrap();
        let (child, url) = Anvil::spawn_server(&self, Some(state_dir.path())).await;
        let anvil: Anvil = Anvil {
            child,
            url,
            state_dir,
            opt: self,
        };

        // When we are running a local Anvil node, as in tests, some endpoints (e.g. eth_feeHistory)
        // do not work until at least one block has been mined.
        while let Err(err) = anvil
            .provider()
            .fee_history(1, BlockNumber::Latest, &[])
            .await
        {
            tracing::warn!("RPC is not ready: {err}");
            sleep(Duration::from_millis(200)).await;
        }

        anvil
    }
}

/// Convenient interfaces for using `anvil` command which runs a local blockchain
/// Similar to [`AnvilInstance`][https://docs.rs/ethers/latest/ethers/core/utils/struct.AnvilInstance.html], with more useful methods
#[derive(Debug)]
pub struct Anvil {
    child: Child,
    url: Url,
    state_dir: TempDir,
    opt: AnvilOptions,
}

impl Anvil {
    async fn spawn_server(opt: &AnvilOptions, state_dir: Option<&Path>) -> (Child, Url) {
        let port = opt
            .port
            .unwrap_or_else(|| portpicker::pick_unused_port().unwrap());

        let mut command = Command::new("anvil");
        command.args([
            "--silent",
            "--port",
            &port.to_string(),
            "--accounts",
            &opt.accounts.unwrap_or(20).to_string(),
        ]);

        if let Some(state_dir) = state_dir {
            command.args(["--dump-state", &state_dir.display().to_string()]);
        }
        if let Some(block_time) = opt.block_time {
            command.args(["-b", &block_time.as_secs().to_string()]);
        }
        if let Some(load_state) = &opt.load_state {
            command.args(["--load-state", &load_state.display().to_string()]);
        }
        if let Some(chain_id) = opt.chain_id {
            command.args(["--chain-id", &chain_id.to_string()]);
        }

        tracing::info!("Starting Anvil: {:?}", &command);

        let child = command.spawn().unwrap();

        let url = Url::parse(&format!("http://localhost:{port}")).unwrap();
        wait_for_rpc(&url, Duration::from_millis(200), 20)
            .await
            .unwrap();

        (child, url)
    }

    pub fn url(&self) -> Url {
        self.url.clone()
    }

    pub fn ws_url(&self) -> Url {
        let mut ws_url = self.url.clone();
        ws_url.set_scheme("ws").unwrap();
        ws_url
    }

    pub fn provider(&self) -> Provider<Http> {
        Provider::try_from(self.url().to_string()).unwrap()
    }

    fn shutdown_gracefully(&self) {
        Command::new("kill")
            .args(["-s", "INT", &self.child.id().to_string()])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    /// Restart the server, possibly with different options.
    pub async fn restart(&mut self, mut opt: AnvilOptions) {
        // Stop the server and wait for it to dump its state.
        self.shutdown_gracefully();

        // If `opt` does not explicitly override the URL, use the current one.
        if opt.port.is_none() {
            opt.port = self.url.port();
        }
        // If `opt` does not explicitly override the chain ID, use the current one.
        if opt.chain_id.is_none() {
            opt.chain_id = self.opt.chain_id;
        }

        // Load state from the file where we just dumped state.
        opt = opt.load_state(self.state_dir.path().join("state.json"));

        // Restart the server with the new options, loading state from disk.
        let (child, url) = Self::spawn_server(&opt, Some(self.state_dir.path())).await;
        self.child = child;
        self.url = url;
        self.opt = opt;
    }

    /// Force a reorg in the L1.
    ///
    /// This function will block until the reorg has completed; that is, a block has been added to
    /// the chain at the same height as a different, earlier block.
    ///
    /// Due to limitations in Anvil, the common ancestor of the reorg will always be the L1 genesis.
    /// However, after the reorg, the genesis state of the EVM will be the same as the state when
    /// this function was originally called. The recommended way to use this for testing reorg
    /// handling, then, is to perform initialization (like deploying contracts) and then start some
    /// service in the background and immediately call this function. This function will let the L1
    /// chain run until it reaches block height at least `min_depth` and then reset it to block
    /// height 0. Then it will let the L1 chain run again until it reaches block height `min_depth`.
    ///
    /// From the perspective of the service under test, this looks like an L1 chain with some
    /// initial state (e.g. contracts deployed) and a block height which happens to be 0 but, as far
    /// as the service cares, could be arbitrary. The block height grows somewhat before getting
    /// reorged back to an earlier height (0) and then growing again.
    ///
    /// The L1 node will restart several times during this process.
    pub async fn reorg(&mut self, min_depth: u64) {
        // Stop the server and wait for it to dump its state, to obtain a snapshot of the current,
        // pre-reorg state.
        self.shutdown_gracefully();

        // Ensure we restart on the same port and load the state snapshot.
        let opt = self
            .opt
            .clone()
            .port(self.url.port().unwrap())
            .load_state(self.state_dir.path().into());

        // Restart the server, loading state from disk but not dumping it on the next exit.
        let (child, url) = Self::spawn_server(&opt, None).await;
        self.child = child;
        self.url = url;

        // Wait for enough blocks to be produced.
        let client = Provider::try_from(self.url.to_string()).unwrap();
        while client.get_block_number().await.unwrap().as_u64() < min_depth {
            sleep(Duration::from_secs(1)).await;
        }

        // Restart again, loading from the previous state checkpoint.
        self.shutdown_gracefully();
        let (child, url) = Self::spawn_server(&opt, Some(self.state_dir.path())).await;
        self.child = child;
        self.url = url;

        // Wait for the chain to reach its former height again.
        let client = Provider::try_from(self.url.to_string()).unwrap();
        while client.get_block_number().await.unwrap().as_u64() < min_depth {
            sleep(Duration::from_secs(1)).await;
        }
    }
}

impl Drop for Anvil {
    fn drop(&mut self) {
        self.shutdown_gracefully()
    }
}

/// Prepare a `SignerMiddleware` by connecting to a provider and initiating a local wallet.
/// Returns a signer/client that can sign and send transactions to network.
pub async fn init_signer(provider: &Url, mnemonic: &str, index: u32) -> Option<Signer> {
    let provider = match Provider::try_from(provider.to_string()) {
        Ok(provider) => provider,
        Err(err) => {
            tracing::error!("error connecting to RPC {}: {}", provider, err);
            return None;
        },
    };
    let chain_id = match provider.get_chainid().await {
        Ok(id) => id.as_u64(),
        Err(err) => {
            tracing::error!("error getting chain ID: {}", err);
            return None;
        },
    };
    let mnemonic = match MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(index)
    {
        Ok(mnemonic) => mnemonic,
        Err(err) => {
            tracing::error!("error building wallet: {}", err);
            return None;
        },
    };
    let wallet = match mnemonic.build() {
        Ok(wallet) => wallet,
        Err(err) => {
            tracing::error!("error opening wallet: {}", err);
            return None;
        },
    };
    let wallet = wallet.with_chain_id(chain_id);
    Some(SignerMiddleware::new(provider, wallet))
}

pub async fn wait_for_http(
    url: &Url,
    interval: Duration,
    max_retries: usize,
) -> Result<usize, String> {
    for i in 0..(max_retries + 1) {
        let res = surf::get(url).await;
        if res.is_ok() {
            tracing::debug!("Connected to {url}");
            return Ok(i);
        }
        tracing::debug!("Waiting for {url}, retrying in {interval:?}");
        sleep(interval).await;
    }
    Err(format!("Url {url:?} not available."))
}

pub async fn wait_for_rpc(
    url: &Url,
    interval: Duration,
    max_retries: usize,
) -> Result<usize, String> {
    let retries = wait_for_http(url, interval, max_retries).await?;
    let client = Provider::new(Http::new(url.clone()));
    for i in retries..(max_retries + 1) {
        if client.get_block_number().await.is_ok() {
            tracing::debug!("JSON-RPC ready at {url}");
            return Ok(i);
        }
        tracing::debug!("Waiting for JSON-RPC at {url}, retrying in {interval:?}");
        sleep(interval).await;
    }

    Err(format!("No JSON-RPC at {url}"))
}

/// converting a keccak256-based structured commitment (32 bytes) into type `U256`
pub fn commitment_to_u256<T: Committable>(comm: Commitment<T>) -> U256 {
    let mut buf = vec![];
    comm.serialize_uncompressed(&mut buf).unwrap();
    let state_comm: [u8; 32] = buf.try_into().unwrap();
    U256::from_little_endian(&state_comm)
}

/// converting a `U256` value into a keccak256-based structured commitment (32 bytes)
pub fn u256_to_commitment<T: Committable>(comm: U256) -> Result<Commitment<T>, SerializationError> {
    let mut commit_bytes = [0; 32];
    comm.to_little_endian(&mut commit_bytes);
    Commitment::deserialize_uncompressed_unchecked(&*commit_bytes.to_vec())
}

/// Implement `to_fixed_bytes` for wrapped types
#[macro_export]
macro_rules! impl_to_fixed_bytes {
    ($struct_name:ident, $type:ty) => {
        impl $struct_name {
            pub(crate) fn to_fixed_bytes(self) -> [u8; core::mem::size_of::<$type>()] {
                let mut bytes = [0u8; core::mem::size_of::<$type>()];
                self.0.to_little_endian(&mut bytes);
                bytes
            }
        }
    };
}

/// send a transaction and wait for confirmation before returning the tx receipt and block included.
pub async fn contract_send<M: Middleware, T: Detokenize, E>(
    call: &ContractCall<M, T>,
) -> Result<(TransactionReceipt, u64), anyhow::Error>
where
    M::Provider: Clone,
    E: ContractRevert + Debug,
{
    let pending = match call.send().await {
        Ok(pending) => pending,
        Err(err) => {
            if let Some(e) = err.decode_contract_revert::<E>() {
                tracing::error!("contract revert: {:?}", e);
            }
            return Err(anyhow!("error sending transaction: {:?}", err));
        },
    };

    let hash = pending.tx_hash();
    let provider = pending.provider();
    tracing::debug!("submitted contract call {:x}", hash);

    if !wait_for_transaction_to_be_mined(&provider, hash).await {
        return Err(anyhow!("transaction not mined"));
    }

    let receipt = match provider.get_transaction_receipt(hash).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => {
            return Err(anyhow!("contract call {hash:x}: no receipt"));
        },
        Err(err) => {
            return Err(anyhow!(
                "contract call {hash:x}: error getting transaction receipt: {err}"
            ))
        },
    };
    if receipt.status != Some(1.into()) {
        return Err(anyhow!("contract call {hash:x}: transaction reverted"));
    }

    // If a transaction is mined and we get a receipt for it, the block number should _always_ be
    // set. If it is not, something has gone horribly wrong with the RPC.
    let block_number = receipt
        .block_number
        .expect("transaction mined but block number not set");
    Ok((receipt, block_number.as_u64()))
}

async fn wait_for_transaction_to_be_mined<P: JsonRpcClient>(
    provider: &Provider<P>,
    hash: H256,
) -> bool {
    let retries = 10;
    // It's common to have to try a few times before the transactions is mined. It is too noisy if
    // we log every retry. However, if it is not mined after several retries, something might be
    // wrong, and it would be useful to start logging.
    let log_retries = 5;

    let interval = provider.get_interval();
    for i in 0..retries {
        match provider.get_transaction(hash).await {
            Err(err) => {
                if i >= log_retries {
                    tracing::warn!("contract call {hash:?} (retry {i}/{retries}): error getting transaction status: {err}");
                }
            },
            Ok(None) => {
                if i >= log_retries {
                    tracing::warn!(
                        "contract call {hash:?} (retry {i}/{retries}): missing from mempool"
                    );
                }
            },
            Ok(Some(tx)) if tx.block_number.is_none() => {
                if i >= log_retries {
                    tracing::warn!("contract call {hash:?} (retry {i}/{retries}): pending");
                }
            },
            Ok(Some(_)) => return true,
        }

        sleep(interval).await;
    }

    tracing::error!("contract call {hash:?}: not mined after {retries} retries");
    false
}

#[cfg(test)]
mod test {
    use committable::RawCommitmentBuilder;

    use super::*;

    struct TestCommittable;

    impl Committable for TestCommittable {
        fn commit(&self) -> Commitment<Self> {
            RawCommitmentBuilder::new("TestCommittable").finalize()
        }
    }

    #[test]
    fn test_commitment_to_u256_round_trip() {
        assert_eq!(
            TestCommittable.commit(),
            u256_to_commitment(commitment_to_u256(TestCommittable.commit())).unwrap()
        );
    }
}
