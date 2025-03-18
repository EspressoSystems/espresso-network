use alloy::{primitives::Log, rpc::types::TransactionReceipt, sol_types::SolEvent};

// TODO this function can be removed once we move to alloy 0.12
#[allow(dead_code)]
pub fn decode_log<E: SolEvent>(r: &TransactionReceipt) -> Option<Log<E>> {
    r.inner
        .logs()
        .iter()
        .find_map(|log| E::decode_log(&log.inner, false).ok())
}
