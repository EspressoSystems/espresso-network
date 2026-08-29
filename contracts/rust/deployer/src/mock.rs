//! Deployment of the `LightClient*Mock` contracts.
//!
//! Without the `mocks` feature the bindings are not compiled and every entry point here fails at
//! runtime, so callers keep their `mock: bool` argument either way.

#[cfg(feature = "mocks")]
use alloy::{hex::ToHexExt, network::TransactionBuilder};
use alloy::{primitives::Address, providers::Provider};
use anyhow::Result;
#[cfg(feature = "mocks")]
use hotshot_contract_adapter::sol_types::{LightClientMock, LightClientV2Mock, LightClientV3Mock};

#[cfg(feature = "mocks")]
pub(crate) async fn deploy_light_client(
    provider: impl Provider,
    plonk_verifier: Address,
) -> Result<Address> {
    let bytecode = crate::link_library(LightClientMock::BYTECODE.encode_hex(), plonk_verifier)?;
    let addr = LightClientMock::deploy_builder(&provider)
        .map(|req| req.with_deploy_code(bytecode))
        .deploy()
        .await?;
    tracing::info!("deployed LightClientMock at {addr:#x}");
    Ok(addr)
}

#[cfg(feature = "mocks")]
pub(crate) async fn deploy_light_client_v2(
    provider: impl Provider,
    plonk_verifier: Address,
) -> Result<Address> {
    let bytecode = crate::link_library(LightClientV2Mock::BYTECODE.encode_hex(), plonk_verifier)?;
    let addr = LightClientV2Mock::deploy_builder(&provider)
        .map(|req| req.with_deploy_code(bytecode))
        .deploy()
        .await?;
    tracing::info!("deployed LightClientV2Mock at {addr:#x}");
    Ok(addr)
}

#[cfg(feature = "mocks")]
pub(crate) async fn deploy_light_client_v3(
    provider: impl Provider,
    plonk_verifier: Address,
) -> Result<Address> {
    let bytecode = crate::link_library(LightClientV3Mock::BYTECODE.encode_hex(), plonk_verifier)?;
    let addr = LightClientV3Mock::deploy_builder(&provider)
        .map(|req| req.with_deploy_code(bytecode))
        .deploy()
        .await?;
    tracing::info!("deployed LightClientV3Mock at {addr:#x}");
    Ok(addr)
}

#[cfg(not(feature = "mocks"))]
fn disabled(contract: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "can't deploy {contract}: espresso-contract-deployer was built without the `mocks` feature"
    )
}

#[cfg(not(feature = "mocks"))]
pub(crate) async fn deploy_light_client(
    _provider: impl Provider,
    _plonk_verifier: Address,
) -> Result<Address> {
    Err(disabled("LightClientMock"))
}

#[cfg(not(feature = "mocks"))]
pub(crate) async fn deploy_light_client_v2(
    _provider: impl Provider,
    _plonk_verifier: Address,
) -> Result<Address> {
    Err(disabled("LightClientV2Mock"))
}

#[cfg(not(feature = "mocks"))]
pub(crate) async fn deploy_light_client_v3(
    _provider: impl Provider,
    _plonk_verifier: Address,
) -> Result<Address> {
    Err(disabled("LightClientV3Mock"))
}
