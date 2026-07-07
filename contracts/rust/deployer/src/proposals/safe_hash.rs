//! EIP-712 Safe transaction hash computation (Safe v1.4.1).

use alloy::primitives::{Address, B256, Bytes, U256, keccak256};

const SAFE_TX_TYPEHASH_PREIMAGE: &[u8] = b"SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)";

#[derive(Debug, Clone)]
pub struct SafeTxHashes {
    pub domain: B256,
    pub message: B256,
    pub safe_tx: B256,
}

/// Compute EIP-712 Safe transaction hashes for a single-tx batch (operation=0).
///
/// Matches the domainSeparator and message hash produced by Safe v1.4.1
/// (`EIP712Domain(uint256 chainId,address verifyingContract)`).
pub fn safe_tx_hashes(
    safe: Address,
    chain_id: u64,
    to: Address,
    value: U256,
    data: &Bytes,
    operation: u8,
    nonce: u64,
) -> SafeTxHashes {
    let domain_typehash = keccak256(b"EIP712Domain(uint256 chainId,address verifyingContract)");
    let mut domain_encoded = [0u8; 96];
    domain_encoded[..32].copy_from_slice(domain_typehash.as_ref());
    domain_encoded[32..64]
        .copy_from_slice(&alloy::primitives::U256::from(chain_id).to_be_bytes::<32>());
    domain_encoded[64 + 12..].copy_from_slice(safe.as_slice());
    let domain_hash = keccak256(domain_encoded);

    let data_hash = keccak256(data.as_ref());
    let to_padded: [u8; 32] = {
        let mut b = [0u8; 32];
        b[12..].copy_from_slice(to.as_slice());
        b
    };
    let safe_tx_typehash = keccak256(SAFE_TX_TYPEHASH_PREIMAGE);

    let mut msg_encoded = [0u8; 32 * 11];
    msg_encoded[0..32].copy_from_slice(safe_tx_typehash.as_ref());
    msg_encoded[32..64].copy_from_slice(&to_padded);
    msg_encoded[64..96].copy_from_slice(&value.to_be_bytes::<32>());
    msg_encoded[96..128].copy_from_slice(data_hash.as_ref());
    msg_encoded[128..160]
        .copy_from_slice(&alloy::primitives::U256::from(operation).to_be_bytes::<32>());
    // safeTxGas, baseGas, gasPrice = 0; gasToken, refundReceiver = zero (already zero)
    msg_encoded[320..352]
        .copy_from_slice(&alloy::primitives::U256::from(nonce).to_be_bytes::<32>());
    let message_hash = keccak256(msg_encoded);

    let mut final_encoded = [0u8; 66];
    final_encoded[0] = 0x19;
    final_encoded[1] = 0x01;
    final_encoded[2..34].copy_from_slice(domain_hash.as_ref());
    final_encoded[34..66].copy_from_slice(message_hash.as_ref());

    SafeTxHashes {
        domain: domain_hash,
        message: message_hash,
        safe_tx: keccak256(final_encoded),
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, B256, Bytes, U256};

    use super::*;

    // ── TEST:safe-hash-known-vector ────────────────────────────────────────────
    //
    // Known-vector against on-chain domainSeparator for Safe v1.4.1
    // at 0xb76834e371b666feee48e5d7d9a97ca08b5a0620 on chain 11155111.
    #[test]
    fn test_safe_tx_hash_known_vector() {
        let safe: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        let chain_id: u64 = 11155111;
        let timelock: Address = "0x8e3b6563d683b87964104a2c3a4bf542bb70767f"
            .parse()
            .unwrap();

        // Pre-computed schedule calldata from decaf fixture (nonce=24).
        // Full calldata bytes omitted; we test domain hash which depends only on safe+chain_id.
        let data = Bytes::new();

        let hashes = safe_tx_hashes(safe, chain_id, timelock, U256::ZERO, &data, 0, 24);

        let expected_domain: B256 =
            "0x8f560c9d209e6d9320305560aee98fa1dea01510aa5451a9c0911401893835c6"
                .parse()
                .unwrap();
        assert_eq!(hashes.domain, expected_domain);
    }
}
