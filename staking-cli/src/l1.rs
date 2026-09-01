//! Configuration of the L1 provider the CLI reads events from.

/// Blocks per request when the range has to be split. Matches the default of the environment
/// variable that overrides it, which is the range capped providers usually allow.
pub(crate) const DEFAULT_BLOCK_RANGE: u64 = 10_000;

/// The block range the user pinned for their provider, if any.
///
/// Set means "my provider caps the range at this", so the single request is skipped rather than
/// spent on a rejection.
pub(crate) fn configured_block_range() -> Option<u64> {
    let value = std::env::var_os(BLOCK_RANGE_VAR)?;
    match value.to_str().and_then(|value| value.parse().ok()) {
        Some(range) if range > 0 => Some(range),
        _ => {
            tracing::warn!("ignoring {BLOCK_RANGE_VAR}: expected a positive integer");
            None
        },
    }
}

pub(crate) const BLOCK_RANGE_VAR: &str = "ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE";
