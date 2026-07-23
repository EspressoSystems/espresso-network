use super::*;

async fn total_minted_supply<S: v1::TokenApi>(
    State(state): State<S>,
) -> Result<ApiJson<String>, ApiError> {
    state
        .total_minted_supply()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn circulating_supply<S: v1::TokenApi>(
    State(state): State<S>,
) -> Result<ApiJson<String>, ApiError> {
    state
        .circulating_supply()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn circulating_supply_ethereum<S: v1::TokenApi>(
    State(state): State<S>,
) -> Result<ApiJson<String>, ApiError> {
    state
        .circulating_supply_ethereum()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn total_issued_supply<S: v1::TokenApi>(
    State(state): State<S>,
) -> Result<ApiJson<String>, ApiError> {
    state
        .total_issued_supply()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn total_reward_distributed<S: v1::TokenApi>(
    State(state): State<S>,
) -> Result<ApiJson<String>, ApiError> {
    state
        .total_reward_distributed()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_token<S>(state: S) -> ApiRouter
where
    S: v1::TokenApi + Clone + Send + Sync + 'static,
{
    let token = ApiRouter::new()
        .api_route(
            "/total-minted-supply",
            get_with(total_minted_supply::<S>, |op| {
                op.summary("Get total minted supply").description(
                    "Total supply of the ESP token minted on Ethereum; excludes unclaimed \
                     rewards. Cached for an hour.",
                )
            }),
        )
        .api_route(
            "/circulating-supply",
            get_with(circulating_supply::<S>, |op| {
                op.summary("Get circulating supply").description(
                    "Circulating supply: initial_supply + reward_distributed - locked, following \
                     the mainnet unlock schedule.",
                )
            }),
        )
        .api_route(
            "/circulating-supply-ethereum",
            get_with(circulating_supply_ethereum::<S>, |op| {
                op.summary("Get circulating supply (Ethereum L1)")
                    .description(
                        "Circulating supply of ESP tokens on Ethereum L1: total_supply_l1 - \
                         locked.",
                    )
            }),
        )
        .api_route(
            "/total-issued-supply",
            get_with(total_issued_supply::<S>, |op| {
                op.summary("Get total issued supply").description(
                    "Total issued supply: initial_supply + total_reward_distributed, including \
                     rewards not yet claimed on Ethereum.",
                )
            }),
        )
        .api_route(
            "/total-reward-distributed",
            get_with(total_reward_distributed::<S>, |op| {
                op.summary("Get total reward distributed").description(
                    "Total rewards distributed by consensus, including rewards not yet claimed on \
                     Ethereum.",
                )
            }),
        );

    ApiRouter::new().nest("/token", token).with_state(state)
}
