use futures::FutureExt;
use hotshot_types::traits::node_implementation::NodeType;
use tide_disco::{api::ApiError, method::ReadState, Api};

use super::{data_source::BuilderDataSource, Version};
use crate::api::load_api;
/// No changes to these types
pub use crate::v0_1::builder::{submit_api, BuildError, Error, Options};

pub fn define_api<State, Types: NodeType>(
    options: &Options,
) -> Result<Api<State, Error, Version>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + BuilderDataSource<Types>,
{
    let mut api = load_api::<State, Error, Version>(
        options.api_path.as_ref(),
        include_str!("../../api/v0_3/builder.toml"),
        options.extensions.clone(),
    )?;
    api.with_version("0.0.3".parse().unwrap())
        .get("bundle", |req, state| {
            async move {
                let parent_view = req.integer_param("parent_view")?;
                let parent_hash = req.blob_param("parent_hash")?;
                let view_number = req.integer_param("view_number")?;
                state
                    .bundle(parent_view, &parent_hash, view_number)
                    .await
                    .map_err(|source| Error::BlockClaim {
                        source,
                        resource: format!(
                            "Block for parent {parent_hash}@{parent_view} and view {view_number}"
                        ),
                    })
            }
            .boxed()
        })?
        .get("builder_address", |_req, state| {
            async move { state.builder_address().await.map_err(Error::BuilderAddress) }.boxed()
        })?;
    Ok(api)
}
