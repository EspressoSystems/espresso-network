//! Validation of the p2p address a validator registers in the stake table.

use hotshot_types::addr::NetAddr;
use tokio::net::lookup_host;

use crate::output::{output_error, output_warn};

/// The longest address the stake table contract accepts.
const MAX_ADDR_LEN: usize = 512;

/// Parse a p2p address to register in the stake table.
///
/// [`NetAddr::validate`] holds the rules for the address itself; what is added here is
/// what the stake table asks of one: it fits the contract's length limit, and it names a
/// port a peer can connect to, which a missing port does not. An address that prints
/// differently to how it was written is registered in the form it prints in, and the
/// caller is told so, since that is the string that lands on chain.
pub fn parse_p2p_addr(s: &str) -> Result<NetAddr, String> {
    let addr: NetAddr = s.parse().map_err(|e| format!("`{s}`: {e}"))?;
    addr.validate().map_err(|e| format!("`{s}`: {e}"))?;

    if addr.port() == 0 {
        return Err(format!("`{s}`: port 0 is not a port a peer can connect to"));
    }

    let registered = addr.to_string();
    if registered.len() > MAX_ADDR_LEN {
        return Err(format!(
            "`{s}` is {} bytes long, the stake table contract accepts at most {MAX_ADDR_LEN}",
            registered.len()
        ));
    }

    if registered != s {
        output_warn(format!("`{s}` will be registered as `{registered}`"));
    }

    Ok(addr)
}

pub(crate) async fn check_if_reachable(addr: &NetAddr) {
    if !addr.is_probably_global() {
        output_error(format!(
            "`{addr}` is not publicly routable, other validators will not be able to connect to it"
        ));
    }

    if addr.is_ip() {
        return;
    }

    let resolved = match lookup_host(addr.to_string()).await {
        Ok(addrs) => addrs.collect::<Vec<_>>(),
        Err(err) => {
            output_error(format!("`{addr}` does not resolve: {err}"));
        },
    };

    if resolved.is_empty() {
        output_error(format!("`{addr}` does not resolve to any address"));
    }

    if !resolved
        .iter()
        .any(|a| NetAddr::from((a.ip().to_canonical(), a.port())).is_probably_global())
    {
        output_error(format!(
            "`{addr}` resolves to addresses that are not publicly routable, other validators will \
             not be able to connect to it"
        ))
    }
}
