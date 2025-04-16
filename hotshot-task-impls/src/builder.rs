// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::{Duration, Instant};

use hotshot_builder_api::v0_1::{
    block_info::AvailableBlockInfo,
    builder::{BuildError, Error as BuilderApiError},
};
use hotshot_types::{
    constants::LEGACY_BUILDER_MODULE,
    data::VidCommitment,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};
use serde::{Deserialize, Serialize};
use surf_disco::{client::HealthStatus, Client, Url};
use tagged_base64::TaggedBase64;
use thiserror::Error;
use tokio::time::sleep;
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
pub struct BuilderClient<TYPES: NodeType, Ver: StaticVersionType> {
    /// Underlying surf_disco::Client for the legacy builder api
    client: Client<BuilderApiError, Ver>,
    /// Marker for [`NodeType`] used here
    _marker: std::marker::PhantomData<TYPES>,
}

impl<TYPES: NodeType, Ver: StaticVersionType> BuilderClient<TYPES, Ver> {
    /// Construct a new client from base url
    ///
    /// # Panics
    ///
    /// If the URL is malformed.
    pub fn new(base_url: impl Into<Url>) -> Self {
        let url = base_url.into();

        Self {
            client: Client::builder(url.clone())
                .set_timeout(Some(Duration::from_secs(2)))
                .build(),
            _marker: std::marker::PhantomData,
        }
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
                "{LEGACY_BUILDER_MODULE}/availableblocks/{parent}/{view_number}/{sender}/{encoded_signature}"
            ))
            .send()
            .await
            .map_err(Into::into)
    }
}

/// Version 0.1
pub mod v0_1 {
    use hotshot_builder_api::v0_1::block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV2, AvailableBlockHeaderInputV2Legacy,
    };
    pub use hotshot_builder_api::v0_1::Version;
    use hotshot_types::{
        constants::LEGACY_BUILDER_MODULE,
        traits::{node_implementation::NodeType, signature_key::SignatureKey},
        utils::BuilderCommitment,
    };
    use tagged_base64::TaggedBase64;

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
                    "{LEGACY_BUILDER_MODULE}/claimheaderinput/v2/{block_hash}/{view_number}/{sender}/{encoded_signature}"
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
                    "{LEGACY_BUILDER_MODULE}/claimheaderinput/v2/{block_hash}/{view_number}/{sender}/{encoded_signature}"
                ))
                .send()
                .await
                .map_err(Into::into)
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
                    "{LEGACY_BUILDER_MODULE}/claimblock/{block_hash}/{view_number}/{sender}/{encoded_signature}"
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
                    "{LEGACY_BUILDER_MODULE}/claimblockwithnumnodes/{block_hash}/{view_number}/{sender}/{encoded_signature}/{num_nodes}"
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

/// Version 0.3: marketplace. Bundles.
pub mod v0_99 {
    pub use hotshot_builder_api::v0_99::Version;
    use hotshot_types::{
        bundle::Bundle, constants::MARKETPLACE_BUILDER_MODULE, data::VidCommitment,
        traits::node_implementation::NodeType,
    };
    use vbs::version::StaticVersion;

    pub use super::BuilderClientError;

    /// Client for builder API
    pub type BuilderClient<TYPES> = super::BuilderClient<TYPES, StaticVersion<0, 99>>;

    impl<TYPES: NodeType> BuilderClient<TYPES> {
        /// Claim block
        ///
        /// # Errors
        /// - [`BuilderClientError::BlockNotFound`] if block isn't available
        /// - [`BuilderClientError::Api`] if API isn't responding or responds incorrectly
        pub async fn bundle(
            &self,
            parent_view: u64,
            parent_hash: VidCommitment,
            view_number: u64,
        ) -> Result<Bundle<TYPES>, BuilderClientError> {
            self.client
                .get(&format!(
                    "{MARKETPLACE_BUILDER_MODULE}/bundle/{parent_view}/{parent_hash}/{view_number}"
                ))
                .send()
                .await
                .map_err(Into::into)
        }
    }
}
