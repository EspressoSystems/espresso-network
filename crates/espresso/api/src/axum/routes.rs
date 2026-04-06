//! Route constants and URL builders for Axum HTTP API

/// Route template for namespace proof endpoint
/// Axum parses {height} and {namespace} as path parameters
pub const NAMESPACE_PROOF_ROUTE: &str = "/namespace-proof/{height}/{namespace}";

/// Route template for reward claim input endpoint
/// Axum parses {block_height} and {address} as path parameters
pub const REWARD_CLAIM_INPUT_ROUTE: &str = "/reward-claim-input/{block_height}/{address}";

/// Build namespace proof URL for HTTP clients
///
/// This function constructs a complete URL by replacing the placeholders
/// in NAMESPACE_PROOF_ROUTE with actual values.
///
/// # Arguments
/// * `base` - Base URL (e.g., "http://localhost:24100")
/// * `height` - Block height
/// * `namespace` - Namespace ID
///
/// # Returns
/// Complete URL like "http://localhost:24100/namespace-proof/100/5"
pub fn namespace_proof_url(base: &str, height: u64, namespace: u64) -> String {
    let path = NAMESPACE_PROOF_ROUTE
        .replace("{height}", &height.to_string())
        .replace("{namespace}", &namespace.to_string());
    format!("{}{}", base, path)
}

/// Build reward claim input URL for HTTP clients
///
/// This function constructs a complete URL by replacing the placeholders
/// in REWARD_CLAIM_INPUT_ROUTE with actual values.
///
/// # Arguments
/// * `base` - Base URL (e.g., "http://localhost:24100")
/// * `block_height` - Block height (must match Light Client finalized height)
/// * `address` - Ethereum address (hex format, e.g., "0x1234...")
///
/// # Returns
/// Complete URL like "http://localhost:24100/reward-claim-input/100/0x1234..."
pub fn reward_claim_input_url(base: &str, block_height: u64, address: &str) -> String {
    let path = REWARD_CLAIM_INPUT_ROUTE
        .replace("{block_height}", &block_height.to_string())
        .replace("{address}", address);
    format!("{}{}", base, path)
}
