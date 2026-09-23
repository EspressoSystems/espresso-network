//! `DatabaseService`.

use super::*;

#[tonic::async_trait]
impl<D> proto::database_service_server::DatabaseService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: DatabaseMetadataSource + Send + Sync,
{
    async fn get_table_sizes(
        &self,
        _request: tonic::Request<proto::GetTableSizesRequest>,
    ) -> Result<tonic::Response<proto::TableSizesResponse>, tonic::Status> {
        let tables = <Self as v1::DatabaseApi>::get_table_sizes(self)
            .await
            .map_err(to_status)?
            .into_iter()
            .map(|table| proto::TableSize {
                table_name: table.table_name,
                row_count: table.row_count,
                total_size_bytes: table.total_size_bytes,
            })
            .collect();
        Ok(tonic::Response::new(proto::TableSizesResponse { tables }))
    }

    async fn get_migration_status(
        &self,
        _request: tonic::Request<proto::GetMigrationStatusRequest>,
    ) -> Result<tonic::Response<proto::MigrationStatusResponse>, tonic::Status> {
        let migrations = <Self as v1::DatabaseApi>::get_migration_status(self)
            .await
            .map_err(to_status)?
            .into_iter()
            .map(|migration| proto::MigrationStatus {
                name: migration.name,
                // v1 serializes these through chrono's serde impl, which ends in `Z`, where plain
                // `to_rfc3339` would write `+00:00` and disagree with it and with protoJSON.
                started_at: migration
                    .started_at
                    .to_rfc3339_opts(SecondsFormat::AutoSi, true),
                completed_at: migration
                    .completed_at
                    .map(|time| time.to_rfc3339_opts(SecondsFormat::AutoSi, true)),
                last_offset: migration.last_offset,
            })
            .collect();
        Ok(tonic::Response::new(proto::MigrationStatusResponse {
            migrations,
        }))
    }
}
