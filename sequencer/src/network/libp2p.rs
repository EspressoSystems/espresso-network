use anyhow::Result;
use libp2p::{Multiaddr, PeerId, multiaddr::Protocol};

/// Split off the peer ID from a multiaddress, returning the shortened address and the peer ID.
///
/// # Errors
/// - If the last protocol in the address is not a peer ID.
pub fn split_off_peer_id(mut address: Multiaddr) -> Result<(PeerId, Multiaddr)> {
    let Some(Protocol::P2p(peer_id)) = address.pop() else {
        return Err(anyhow::anyhow!("Failed to parse peer ID from address"));
    };

    Ok((peer_id, address))
}
