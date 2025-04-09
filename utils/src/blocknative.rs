//! Copy from <https://github.com/gakonst/ethers-rs/blob/master/ethers-middleware/src/gas_oracle/blocknative.rs>
//! which is unmaintained and out-of-sync with the latest blocknative feed
//!
//! TODO: revisit this or remove this when switching to `alloy-rs`
use alloy::primitives::U256;
use async_trait::async_trait;
use reqwest::{header::AUTHORIZATION, Client};
use serde::Deserialize;
use url::Url;

const URL: &str = "https://api.blocknative.com/gasprices/blockprices";

/// A client over HTTP for the [BlockNative](https://www.blocknative.com/gas-estimator) gas tracker API
/// that implements the `GasOracle` trait.
#[derive(Clone, Debug)]
#[must_use]
pub struct BlockNative {
    client: Client,
    url: Url,
    api_key: Option<String>,
    gas_category: GasCategory,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub system: String,
    pub network: String,
    pub unit: String,
    pub max_price: f64,
    pub block_prices: Vec<BlockPrice>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockPrice {
    pub block_number: u64,
    pub estimated_transaction_count: u64,
    pub base_fee_per_gas: f64,
    pub blob_base_fee_per_gas: f64,
    pub estimated_prices: Vec<GasEstimate>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GasEstimate {
    pub confidence: u64,
    pub price: f64,
    pub max_priority_fee_per_gas: f64,
    pub max_fee_per_gas: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseFeeEstimate {
    pub confidence: u64,
    pub base_fee: f64,
}

impl Response {
    #[inline]
    pub fn estimate_from_category(&self, gas_category: &GasCategory) -> Result<GasEstimate> {
        let confidence = gas_category_to_confidence(gas_category);
        let price = self
            .block_prices
            .first()
            .ok_or(GasOracleError::InvalidResponse)?
            .estimated_prices
            .iter()
            .find(|p| p.confidence == confidence)
            .ok_or(GasOracleError::GasCategoryNotSupported)?;
        Ok(*price)
    }
}

impl Default for BlockNative {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GasOracle for BlockNative {
    async fn fetch(&self) -> Result<U256> {
        let estimate = self
            .query()
            .await?
            .estimate_from_category(&self.gas_category)?;
        Ok(from_gwei_f64(estimate.price))
    }

    async fn estimate_eip1559_fees(&self) -> Result<(U256, U256)> {
        let estimate = self
            .query()
            .await?
            .estimate_from_category(&self.gas_category)?;
        let max = from_gwei_f64(estimate.max_fee_per_gas);
        let prio = from_gwei_f64(estimate.max_priority_fee_per_gas);
        Ok((max, prio))
    }
}

impl BlockNative {
    /// Creates a new [BlockNative](https://www.blocknative.com/gas-estimator) gas oracle.
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_client(Client::new(), api_key)
    }

    /// Same as [`Self::new`] but with a custom [`Client`].
    pub fn with_client(client: Client, api_key: Option<String>) -> Self {
        let url = Url::parse(URL).unwrap();
        Self {
            client,
            api_key,
            url,
            gas_category: GasCategory::Standard,
        }
    }

    /// Sets the gas price category to be used when fetching the gas price.
    pub fn category(mut self, gas_category: GasCategory) -> Self {
        self.gas_category = gas_category;
        self
    }

    /// Perform a request to the gas price API and deserialize the response.
    pub async fn query(&self) -> Result<Response, GasOracleError> {
        let mut request = self.client.get(self.url.clone());
        if let Some(api_key) = self.api_key.as_ref() {
            request = request.header(AUTHORIZATION, api_key);
        }
        let response = request.send().await?.error_for_status()?.json().await?;
        Ok(response)
    }
}

#[inline]
fn gas_category_to_confidence(gas_category: &GasCategory) -> u64 {
    match gas_category {
        GasCategory::SafeLow => 80,
        GasCategory::Standard => 90,
        GasCategory::Fast => 95,
        GasCategory::Fastest => 99,
    }
}
