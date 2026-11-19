//! Key encodings whose bytewise order is the numeric order.

/// `n` as a key.
pub fn u64(n: u64) -> [u8; 8] {
    n.to_be_bytes()
}

/// `(a, b)` as a key, ordered by `a`, then `b`.
pub fn u64_pair(a: u64, b: u64) -> [u8; 16] {
    let mut k = [0; 16];
    k[..8].copy_from_slice(&a.to_be_bytes());
    k[8..].copy_from_slice(&b.to_be_bytes());
    k
}

pub fn decode_u64(key: &[u8]) -> Option<u64> {
    Some(u64::from_be_bytes(key.try_into().ok()?))
}

pub fn decode_u64_pair(key: &[u8]) -> Option<(u64, u64)> {
    let key: &[u8; 16] = key.try_into().ok()?;
    let (a, b) = key.split_at(8);
    Some((decode_u64(a)?, decode_u64(b)?))
}

#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;

    quickcheck! {
        fn u64_order(a: u64, b: u64) -> bool {
            a.cmp(&b) == super::u64(a).cmp(&super::u64(b))
        }

        fn u64_pair_order(a: (u64, u64), b: (u64, u64)) -> bool {
            a.cmp(&b) == super::u64_pair(a.0, a.1).cmp(&super::u64_pair(b.0, b.1))
        }

        fn round_trip(a: u64, b: u64) -> bool {
            super::decode_u64(&super::u64(a)) == Some(a)
                && super::decode_u64_pair(&super::u64_pair(a, b)) == Some((a, b))
        }

        fn wrong_length(key: Vec<u8>) -> bool {
            (super::decode_u64(&key).is_some() == (key.len() == 8))
                && (super::decode_u64_pair(&key).is_some() == (key.len() == 16))
        }
    }
}
