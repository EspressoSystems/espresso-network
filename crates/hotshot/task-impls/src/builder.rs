// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use hotshot_builder_api::v0_1::{
    block_info::AvailableBlockInfo,
    builder::{BuildError, Error as BuilderApiError},
};
use hotshot_types::{
    constants::LEGACY_BUILDER_MODULE,
    data::VidCommitment,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use simple_moving_average::{SumTreeSMA, SMA};
use surf_disco::{client::HealthStatus, Client, Url};
use tagged_base64::TaggedBase64;
use thiserror::Error;
use tokio::{spawn, time::sleep};
use tokio_util::task::AbortOnDropHandle;
use tracing::warn;
use vbs::version::StaticVersionType;

#[derive(Debug, Error, Serialize, Deserialize)]
/// Represents errors that can occur while interacting with the builder
pub enum BuilderClientError {
    /// The requested block was not found
    #[error("Requested block not found")]
    BlockNotFound,

    /// The requested block was missing
    #[error("Requested block was missing")]
    BlockMissing,

    /// Generic error while accessing the API
    #[error("Builder API error: {0}")]
    Api(String),
}

impl From<BuilderApiError> for BuilderClientError {
    fn from(value: BuilderApiError) -> Self {
        match value {
            BuilderApiError::Request(source) | BuilderApiError::TxnUnpack(source) => {
                Self::Api(source.to_string())
            },
            BuilderApiError::TxnSubmit(source) | BuilderApiError::BuilderAddress(source) => {
                Self::Api(source.to_string())
            },
            BuilderApiError::Custom { message, .. } => Self::Api(message),
            BuilderApiError::BlockAvailable { source, .. }
            | BuilderApiError::BlockClaim { source, .. } => match source {
                BuildError::NotFound => Self::BlockNotFound,
                BuildError::Missing => Self::BlockMissing,
                BuildError::Error(message) => Self::Api(message),
            },
            BuilderApiError::TxnStat(source) => Self::Api(source.to_string()),
        }
    }
}

/// Client for builder API
#[derive(Clone)]
pub struct BuilderClient<TYPES: NodeType, Ver: StaticVersionType + 'static> {
    /// Underlying surf_disco::Client for the legacy builder api
    client: Client<BuilderApiError, Ver>,

    /// A simple moving average of the latency to this builder
    latency_sma: Arc<RwLock<SumTreeSMA<f64, f64, 10>>>,

    /// A handle to the task that continuously updates the latency SMA
    latency_eval_task: Option<Arc<AbortOnDropHandle<()>>>,

    /// Marker for [`NodeType`] used here
    _marker: std::marker::PhantomData<TYPES>,
}

impl<TYPES: NodeType, Ver: StaticVersionType> BuilderClient<TYPES, Ver> {
    /// Construct a new client from base url
    ///
    /// # Panics
    ///
    /// If the URL is malformed.
    pub fn new(base_url: impl Into<Url>, request_timeout: Duration) -> Self {
        let url = base_url.into();

        // Initialize the latency moving average
        let latency_sma = Arc::new(RwLock::new(SumTreeSMA::new()));

        // Initialize the client
        let mut self_ = Self {
            client: Client::builder(url.clone())
                .set_timeout(Some(request_timeout))
                .build(),
            latency_sma,
            latency_eval_task: None,
            _marker: std::marker::PhantomData,
        };

        // Create a handle to the task that continuously updates the latency SMA
        let self_clone = self_.clone();
        let latency_eval_task = AbortOnDropHandle::new(spawn(async move {
            self_clone.run_latency_monitor().await;
        }));

        // Set the task handle
        self_.latency_eval_task = Some(Arc::new(latency_eval_task));

        self_
    }

    /// Wait for server to become available
    /// Returns `false` if server doesn't respond
    /// with OK healthcheck before `timeout`
    pub async fn connect(&self, timeout: Duration) -> bool {
        let timeout = Instant::now() + timeout;
        let mut backoff = Duration::from_millis(50);
        while Instant::now() < timeout {
            if matches!(
                self.client.healthcheck::<HealthStatus>().await,
                Ok(HealthStatus::Available)
            ) {
                return true;
            }
            sleep(backoff).await;
            backoff *= 2;
        }
        false
    }

    /// Query builder for available blocks
    ///
    /// # Errors
    /// - [`BuilderClientError::BlockNotFound`] if blocks aren't available for this parent
    /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
    pub async fn available_blocks(
        &self,
        parent: VidCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<TYPES>>, BuilderClientError> {
        let encoded_signature: TaggedBase64 = signature.clone().into();
        self.client
            .get(&format!(
                "{LEGACY_BUILDER_MODULE}/availableblocks/{parent}/{view_number}/{sender}/\
                 {encoded_signature}"
            ))
            .send()
            .await
            .map_err(Into::into)
    }

    /// The task that periodically pings the builder to measure latency
    async fn run_latency_monitor(&self) {
        // Create a 5 minute interval
        let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));

        loop {
            // Wait for the interval to elapse
            interval.tick().await;

            // Start the timer
            let now = Instant::now();

            // Ping the builder, continue if it fails
            if let Err(err) = self.ping().await {
                warn!("Failed to ping builder: {err:?}");
                continue;
            }

            // Calculate the latency and update the SMA
            let latency = now.elapsed().as_secs_f64();
            self.latency_sma.write().add_sample(latency);
        }
    }

    /// Returns the calculated latency to this builder (in seconds)
    pub fn latency(&self) -> f64 {
        self.latency_sma.read().get_average()
    }

    /// Ping the builder. We use this to measure latency to the builders so we can
    /// select the closest/least latent one.
    pub async fn ping(&self) -> anyhow::Result<()> {
        // Join the request URL with the base URL
        let url = self
            .client
            .base_url()
            .join("v0/status/ping")
            .with_context(|| "failed to join request URL")?;

        // Perform the request
        let response = reqwest::get(url).await.with_context(|| "request failed")?;

        // Make sure the response is OK
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "request failed with status code {}",
                response.status()
            ));
        }

        Ok(())
    }
}

/// Version 0.1
pub mod v0_1 {
    use hotshot_builder_api::v0_1::block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV2, AvailableBlockHeaderInputV2Either,
        AvailableBlockHeaderInputV2Legacy,
    };
    pub use hotshot_builder_api::v0_1::Version;
    use hotshot_types::{
        constants::LEGACY_BUILDER_MODULE,
        traits::{node_implementation::NodeType, signature_key::SignatureKey},
        utils::BuilderCommitment,
    };
    use tagged_base64::TaggedBase64;
    use vbs::BinarySerializer;

    use super::BuilderClientError;

    /// Client for builder API
    pub type BuilderClient<TYPES> = super::BuilderClient<TYPES, Version>;

    impl<TYPES: NodeType> BuilderClient<TYPES> {
        /// Claim block header input
        ///
        /// # Errors
        /// - [`BuilderClientError::BlockNotFound`] if block isn't available
        /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
        pub async fn claim_block_header_input(
            &self,
            block_hash: BuilderCommitment,
            view_number: u64,
            sender: TYPES::SignatureKey,
            signature: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        ) -> Result<AvailableBlockHeaderInputV2<TYPES>, BuilderClientError> {
            let encoded_signature: TaggedBase64 = signature.clone().into();
            self.client
                .get(&format!(
                    "{LEGACY_BUILDER_MODULE}/claimheaderinput/v2/{block_hash}/{view_number}/\
                     {sender}/{encoded_signature}"
                ))
                .send()
                .await
                .map_err(Into::into)
        }

        /// Claim block header input, using the legacy `AvailableBlockHeaderInputV2Legacy` type
        ///
        /// # Errors
        /// - [`BuilderClientError::BlockNotFound`] if block isn't available
        /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
        pub async fn claim_legacy_block_header_input(
            &self,
            block_hash: BuilderCommitment,
            view_number: u64,
            sender: TYPES::SignatureKey,
            signature: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        ) -> Result<AvailableBlockHeaderInputV2Legacy<TYPES>, BuilderClientError> {
            let encoded_signature: TaggedBase64 = signature.clone().into();
            self.client
                .get(&format!(
                    "{LEGACY_BUILDER_MODULE}/claimheaderinput/v2/{block_hash}/{view_number}/\
                     {sender}/{encoded_signature}"
                ))
                .send()
                .await
                .map_err(Into::into)
        }

        /// Claim block header input, preferring the current `AvailableBlockHeaderInputV2` type but falling back to
        /// the `AvailableBlockHeaderInputV2Legacy` type
        ///
        /// # Errors
        /// - [`BuilderClientError::BlockNotFound`] if block isn't available
        /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
        pub async fn claim_either_block_header_input(
            &self,
            block_hash: BuilderCommitment,
            view_number: u64,
            sender: TYPES::SignatureKey,
            signature: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        ) -> Result<AvailableBlockHeaderInputV2Either<TYPES>, BuilderClientError> {
            let encoded_signature: TaggedBase64 = signature.clone().into();
            let result = self
                .client
                .get::<Vec<u8>>(&format!(
                    "{LEGACY_BUILDER_MODULE}/claimheaderinput/v2/{block_hash}/{view_number}/\
                     {sender}/{encoded_signature}"
                ))
                .bytes()
                .await
                .map_err(Into::<BuilderClientError>::into)?;

            // Manually deserialize the result as one of the enum types. Bincode doesn't support deserialize_any,
            // so we can't go directly into our target type.

            if let Ok(available_block_header_input_v2) = vbs::Serializer::<Version>::deserialize::<
                AvailableBlockHeaderInputV2<TYPES>,
            >(&result)
            {
                Ok(AvailableBlockHeaderInputV2Either::Current(
                    available_block_header_input_v2,
                ))
            } else {
                vbs::Serializer::<Version>::deserialize::<AvailableBlockHeaderInputV2Legacy<TYPES>>(
                    &result,
                )
                .map_err(|e| BuilderClientError::Api(format!("Failed to deserialize: {e:?}")))
                .map(AvailableBlockHeaderInputV2Either::Legacy)
            }
        }

        /// Claim block
        ///
        /// # Errors
        /// - [`BuilderClientError::BlockNotFound`] if block isn't available
        /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
        pub async fn claim_block(
            &self,
            block_hash: BuilderCommitment,
            view_number: u64,
            sender: TYPES::SignatureKey,
            signature: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        ) -> Result<AvailableBlockData<TYPES>, BuilderClientError> {
            let encoded_signature: TaggedBase64 = signature.clone().into();
            self.client
                .get(&format!(
                    "{LEGACY_BUILDER_MODULE}/claimblock/{block_hash}/{view_number}/{sender}/\
                     {encoded_signature}"
                ))
                .send()
                .await
                .map_err(Into::into)
        }

        /// Claim block and provide the number of nodes information to the builder for VID
        /// computation.
        ///
        /// # Errors
        /// - [`BuilderClientError::BlockNotFound`] if block isn't available
        /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
        pub async fn claim_block_with_num_nodes(
            &self,
            block_hash: BuilderCommitment,
            view_number: u64,
            sender: TYPES::SignatureKey,
            signature: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
            num_nodes: usize,
        ) -> Result<AvailableBlockData<TYPES>, BuilderClientError> {
            let encoded_signature: TaggedBase64 = signature.clone().into();
            self.client
                .get(&format!(
                    "{LEGACY_BUILDER_MODULE}/claimblockwithnumnodes/{block_hash}/{view_number}/\
                     {sender}/{encoded_signature}/{num_nodes}"
                ))
                .send()
                .await
                .map_err(Into::into)
        }
    }
}

/// Version 0.2. No changes in API
pub mod v0_2 {
    use vbs::version::StaticVersion;

    pub use super::v0_1::*;

    /// Builder API version
    pub type Version = StaticVersion<0, 2>;
}
