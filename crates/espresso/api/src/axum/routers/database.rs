use super::*;

async fn get_table_sizes<S: v1::DatabaseApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::TableSizes>, ApiError> {
    state
        .get_table_sizes()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn get_migration_status<S: v1::DatabaseApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::MigrationStatus>, ApiError> {
    state
        .get_migration_status()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

pub(crate) fn router_database<S>(state: S) -> ApiRouter
where
    S: v1::DatabaseApi + Clone + Send + Sync + 'static,
{
    let database = ApiRouter::new()
        .api_route(
            "/table-sizes",
            get_with(get_table_sizes::<S>, |op| {
                op.summary("Get database table sizes")
                    .description("Get the sizes of all database tables: row counts and disk usage.")
            }),
        )
        .api_route(
            "/migration-status",
            get_with(get_migration_status::<S>, |op| {
                op.summary("Get migration status").description(
                    "Get the status of all deferred background migrations: start/completion time \
                     and last processed offset.",
                )
            }),
        );

    ApiRouter::new()
        .nest("/database", database)
        .with_state(state)
}
