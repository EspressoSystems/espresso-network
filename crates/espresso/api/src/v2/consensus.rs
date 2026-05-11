use async_trait::async_trait;
use serialization_api::ApiSerializations;

#[async_trait]
pub trait ConsensusApi: ApiSerializations {
    async fn get_state_certificate(&self, epoch: u64) -> anyhow::Result<Self::StateCertificate>;

    async fn get_stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable>;
}
