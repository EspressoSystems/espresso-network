use std::str::FromStr;

use anyhow::Context;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use hotshot_types::{
    data::ViewNumber,
    traits::{
        auction_results_provider::AuctionResultsProvider,
        node_implementation::{ConsensusTime, HasUrls, NodeType},
        signature_key::BuilderSignatureKey,
    },
};
use thiserror::Error;
use tide_disco::error::ServerError;
use url::Url;

use super::{state::ValidatedState, MarketplaceVersion};
use crate::{
    eth_signature_key::{EthKeyPair, SigningError},
    v0_99::{BidTx, BidTxBody, FullNetworkTx, SolverAuctionResults},
    FeeAccount, FeeAmount, FeeError, FeeInfo, NamespaceId,
};

impl FullNetworkTx {
    /// Proxy for `execute` method of each transaction variant.
    pub fn execute(&self, state: &mut ValidatedState) -> Result<(), ExecutionError> {
        match self {
            Self::Bid(bid) => bid.execute(state),
        }
    }
}

impl Committable for BidTx {
    fn tag() -> String {
        "BID_TX".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .field("body", self.body.commit())
            .fixed_size_field("signature", &self.signature.as_bytes());
        comm.finalize()
    }
}

impl Committable for BidTxBody {
    fn tag() -> String {
        "BID_TX_BODY".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("account", &self.account.to_fixed_bytes())
            .fixed_size_field("gas_price", &self.gas_price.to_fixed_bytes())
            .fixed_size_field("bid_amount", &self.bid_amount.to_fixed_bytes())
            .var_size_field("url", self.url.as_str().as_ref())
            .u64_field("view", self.view.u64())
            .array_field(
                "namespaces",
                &self
                    .namespaces
                    .iter()
                    .map(|e| {
                        committable::RawCommitmentBuilder::<BidTxBody>::new("namespace")
                            .u64(e.0)
                            .finalize()
                    })
                    .collect::<Vec<_>>(),
            );
        comm.finalize()
    }
}

impl BidTxBody {
    /// Construct a new `BidTxBody`.
    pub fn new(
        account: FeeAccount,
        bid: FeeAmount,
        view: ViewNumber,
        namespaces: Vec<NamespaceId>,
        url: Url,
        gas_price: FeeAmount,
    ) -> Self {
        Self {
            account,
            bid_amount: bid,
            view,
            namespaces,
            url,
            gas_price,
        }
    }

    /// Sign Body and return a `BidTx`. This is the expected way to obtain a `BidTx`.
    /// ```
    /// # use espresso_types::FeeAccount;
    /// # use espresso_types::v0_99::BidTxBody;
    ///
    /// BidTxBody::default().signed(&FeeAccount::test_key_pair()).unwrap();
    /// ```
    pub fn signed(self, key: &EthKeyPair) -> Result<BidTx, SigningError> {
        let signature = FeeAccount::sign_builder_message(key, self.commit().as_ref())?;
        let bid = BidTx {
            body: self,
            signature,
        };
        Ok(bid)
    }

    /// Get account submitting the bid
    pub fn account(&self) -> FeeAccount {
        self.account
    }
    /// Get amount of bid
    pub fn amount(&self) -> FeeAmount {
        self.bid_amount
    }
    /// get the view number
    pub fn view(&self) -> ViewNumber {
        self.view
    }
    /// Instantiate a `BidTxBody` containing the values of `self`
    /// with a new `url` field.
    pub fn with_url(self, url: Url) -> Self {
        Self { url, ..self }
    }

    /// Get the cloned `url` field.
    fn url(&self) -> Url {
        self.url.clone()
    }
}

impl Default for BidTxBody {
    fn default() -> Self {
        let key = FeeAccount::test_key_pair();
        let nsid = NamespaceId::from(999u64);
        Self {
            url: Url::from_str("https://sequencer:3939").unwrap(),
            account: key.fee_account(),
            gas_price: FeeAmount::default(),
            bid_amount: FeeAmount::default(),
            view: ViewNumber::genesis(),
            namespaces: vec![nsid],
        }
    }
}
impl Default for BidTx {
    fn default() -> Self {
        BidTxBody::default()
            .signed(&FeeAccount::test_key_pair())
            .unwrap()
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
/// Failure cases of transaction execution
pub enum ExecutionError {
    #[error("Invalid Signature")]
    /// Transaction Signature could not be verified.
    InvalidSignature,
    #[error("Invalid Phase")]
    /// Transaction submitted during incorrect Marketplace Phase
    InvalidPhase,
    #[error("FeeError: {0}")]
    /// Insufficient funds or MerkleTree error.
    FeeError(FeeError),
    #[error("Could not resolve `ChainConfig`")]
    /// Could not resolve `ChainConfig`.
    UnresolvableChainConfig,
    #[error("Bid recipient not set on `ChainConfig`")]
    /// Bid Recipient is not set on `ChainConfig`
    BidRecipientNotFound,
}

impl From<FeeError> for ExecutionError {
    fn from(e: FeeError) -> Self {
        Self::FeeError(e)
    }
}

impl BidTx {
    /// Execute `BidTx`.
    ///   * verify signature
    ///   * charge bid amount
    ///   * charge gas
    pub fn execute(&self, state: &mut ValidatedState) -> Result<(), ExecutionError> {
        self.verify()?;

        // In JIT sequencer only receives winning bids. In AOT all
        // bids are charged as received (losing bids are refunded). In
        // any case we can charge the bids and gas during execution.
        self.charge(state)?;

        Ok(())
    }
    /// Charge Bid. Only winning bids are charged in JIT.
    fn charge(&self, state: &mut ValidatedState) -> Result<(), ExecutionError> {
        // As the code is currently organized, I think chain_config
        // will always be resolved here. But let's guard against the
        // error in case code is shifted around in the future.
        let Some(chain_config) = state.chain_config.resolve() else {
            return Err(ExecutionError::UnresolvableChainConfig);
        };

        let Some(recipient) = chain_config.bid_recipient else {
            return Err(ExecutionError::BidRecipientNotFound);
        };
        // Charge the bid amount
        state
            .charge_fee(FeeInfo::new(self.account(), self.amount()), recipient)
            .map_err(ExecutionError::from)?;

        // Charge the gas amount
        state
            .charge_fee(FeeInfo::new(self.account(), self.gas_price()), recipient)
            .map_err(ExecutionError::from)?;

        Ok(())
    }
    /// Cryptographic signature verification
    fn verify(&self) -> Result<(), ExecutionError> {
        self.body
            .account
            .validate_builder_signature(&self.signature, self.body.commit().as_ref())
            .then_some(())
            .ok_or(ExecutionError::InvalidSignature)
    }
    /// Return the body of the transaction
    pub fn body(self) -> BidTxBody {
        self.body
    }
    /// get gas price
    pub fn gas_price(&self) -> FeeAmount {
        self.body.gas_price
    }
    /// get bid amount
    pub fn amount(&self) -> FeeAmount {
        self.body.bid_amount
    }
    /// get bid account
    pub fn account(&self) -> FeeAccount {
        self.body.account
    }
    /// get the view number
    pub fn view(&self) -> ViewNumber {
        self.body.view
    }
    /// Get the `url` field from the body.
    pub fn url(&self) -> Url {
        self.body.url()
    }
}

impl Committable for SolverAuctionResults {
    fn tag() -> String {
        "SOLVER_AUCTION_RESULTS".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("view_number", &self.view_number.commit().into())
            .array_field(
                "winning_bids",
                &self
                    .winning_bids
                    .iter()
                    .map(Committable::commit)
                    .collect::<Vec<_>>(),
            )
            .array_field(
                "reserve_bids",
                &self
                    .reserve_bids
                    .iter()
                    .map(|(nsid, url)| {
                        // Set a phantom type to make the compiler happy
                        committable::RawCommitmentBuilder::<SolverAuctionResults>::new(
                            "RESERVE_BID",
                        )
                        .u64(nsid.0)
                        .constant_str(url.as_str())
                        .finalize()
                    })
                    .collect::<Vec<_>>(),
            );
        comm.finalize()
    }
}

impl SolverAuctionResults {
    /// Construct a `SolverAuctionResults`
    pub fn new(
        view_number: ViewNumber,
        winning_bids: Vec<BidTx>,
        reserve_bids: Vec<(NamespaceId, Url)>,
    ) -> Self {
        Self {
            view_number,
            winning_bids,
            reserve_bids,
        }
    }
    /// Get the view number for these auction results
    pub fn view(&self) -> ViewNumber {
        self.view_number
    }
    /// Get the winning bids of the auction
    pub fn winning_bids(&self) -> &[BidTx] {
        &self.winning_bids
    }
    /// Get the reserve bids of the auction
    pub fn reserve_bids(&self) -> &[(NamespaceId, Url)] {
        &self.reserve_bids
    }
    /// Empty results for the genesis view.
    pub fn genesis() -> Self {
        Self {
            view_number: ViewNumber::genesis(),
            winning_bids: vec![],
            reserve_bids: vec![],
        }
    }
}

impl Default for SolverAuctionResults {
    fn default() -> Self {
        Self::genesis()
    }
}

impl HasUrls for SolverAuctionResults {
    /// Get the urls to fetch bids from builders.
    fn urls(&self) -> Vec<Url> {
        self.winning_bids()
            .iter()
            .map(|bid| bid.url())
            .chain(self.reserve_bids().iter().map(|bid| bid.1.clone()))
            .collect()
    }
}

type SurfClient = surf_disco::Client<ServerError, MarketplaceVersion>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// Auction Results provider holding the Url of the solver in order to fetch auction results.
pub struct SolverAuctionResultsProvider {
    pub url: Url,
    pub marketplace_path: String,
    pub results_path: String,
}

impl Default for SolverAuctionResultsProvider {
    fn default() -> Self {
        Self {
            url: Url::from_str("http://localhost:25000").unwrap(),
            marketplace_path: "marketplace-solver/".into(),
            results_path: "auction_results/".into(),
        }
    }
}

#[async_trait]
impl<TYPES: NodeType> AuctionResultsProvider<TYPES> for SolverAuctionResultsProvider {
    /// Fetch the auction results from the solver.
    async fn fetch_auction_result(
        &self,
        view_number: TYPES::View,
    ) -> anyhow::Result<TYPES::AuctionResult> {
        let resp = SurfClient::new(
            self.url
                .join(&self.marketplace_path)
                .context("Malformed solver URL")?,
        )
        .get::<TYPES::AuctionResult>(&format!("{}{}", self.results_path, *view_number))
        .send()
        .await?;
        Ok(resp)
    }
}

mod test {
    use super::*;

    impl BidTx {
        pub fn mock(key: EthKeyPair) -> Self {
            BidTxBody::default().signed(&key).unwrap()
        }
    }

    #[test]
    fn test_mock_bid_tx_sign_and_verify() {
        let key = FeeAccount::test_key_pair();
        let bidtx = BidTx::mock(key);
        bidtx.verify().unwrap();
    }

    #[test]
    #[ignore] // TODO enable after upgrade to v3
    fn test_mock_bid_tx_charge() {
        let mut state = ValidatedState::default();
        let key = FeeAccount::test_key_pair();
        let bidtx = BidTx::mock(key);
        bidtx.charge(&mut state).unwrap();
    }

    #[test]
    fn test_bid_tx_construct() {
        let key_pair = EthKeyPair::random();
        BidTxBody::new(
            key_pair.fee_account(),
            FeeAmount::from(1),
            ViewNumber::genesis(),
            vec![NamespaceId::from(999u64)],
            Url::from_str("https://my.builder:3131").unwrap(),
            FeeAmount::default(),
        )
        .signed(&key_pair)
        .unwrap()
        .verify()
        .unwrap();
    }
}
