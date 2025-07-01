use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

/// [ClientId] represents the unique identifier for a client that is connected
/// to the server.
///
/// Example:
/// ```rust
/// # use node_metrics::service::client_id::ClientId;
///
/// let client_id = ClientId::from_count(1);
///
/// # assert_eq!(ClientId::from_count(1), client_id);
/// let client_id_2 = client_id + 1;
///
/// # assert_ne!(client_id, client_id_2);
///
/// let mut client_id_3 = client_id;
/// client_id_3 += 1;
///
/// # assert_eq!(client_id_2, client_id_3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(u64);

impl ClientId {
    pub fn from_count(count: u64) -> Self {
        ClientId(count)
    }
}

/// [Add] implements basic addition for [ClientId], which allows [u64]s to be
/// added to the [ClientId] for convenience.
///
/// Example:
///
/// ```rust
///
/// # use node_metrics::service::client_id::ClientId;
///
/// let client_id = ClientId::from_count(1);
/// let new_client_id = client_id + 1;
///
/// # assert_eq!(ClientId::from_count(2), new_client_id);
/// # assert_ne!(client_id, new_client_id);
/// ```
impl Add<u64> for ClientId {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        ClientId(self.0 + rhs)
    }
}

/// [AddAssign] implements basic addition for [ClientId], which allows [u64]s to
/// be added to the mutable [ClientId] for convenience.
///
/// Example:
///
/// ```rust
/// # use node_metrics::service::client_id::ClientId;
///
/// let mut client_id = ClientId::from_count(1);
/// client_id += 1;
///
/// # assert_eq!(ClientId::from_count(2), client_id);
/// ```
impl AddAssign<u64> for ClientId {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::ClientId;

    #[test]
    fn test_client_id_debug() {
        let client_id = ClientId::from_count(1);
        assert_eq!(format!("{client_id:?}"), "ClientId(1)");
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_client_id_clone() {
        let client_id = ClientId::from_count(1);
        let cloned_client_id = client_id.clone();
        assert_eq!(client_id, cloned_client_id);
    }

    #[test]
    fn test_client_id_partial_eq() {
        let client_id_1 = ClientId::from_count(1);
        let client_id_2 = ClientId::from_count(2);
        let client_id_3 = ClientId::from_count(1);

        assert_ne!(client_id_1, client_id_2);
        assert_eq!(client_id_1, client_id_3);
    }

    #[test]
    fn test_client_id_eq() {
        let client_id_1 = ClientId::from_count(1);

        client_id_1.assert_receiver_is_total_eq();
    }

    #[test]
    fn test_hash() {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let hash_1 = {
            let client_id = ClientId::from_count(1);
            let mut hasher = DefaultHasher::new();
            client_id.hash(&mut hasher);
            hasher.finish()
        };

        let hash_2 = {
            let client_id = ClientId::from_count(2);
            let mut hasher = DefaultHasher::new();
            client_id.hash(&mut hasher);
            hasher.finish()
        };

        let hash_3 = {
            let client_id = ClientId::from_count(1);
            let mut hasher = DefaultHasher::new();
            client_id.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(hash_1, hash_3);
        assert_ne!(hash_1, hash_2);
        assert_ne!(hash_2, hash_3);
    }

    #[test]
    fn test_add() {
        let client_id = ClientId::from_count(1);
        let new_client_id = client_id + 1;
        assert_eq!(new_client_id, ClientId::from_count(2));
    }

    #[test]
    fn test_add_assign() {
        let mut client_id = ClientId::from_count(1);
        client_id += 1;
        assert_eq!(client_id, ClientId::from_count(2));
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_serialization() {
        use serde_json;
        let client_id = ClientId::from_count(1);
        let serialized = serde_json::to_string(&client_id).unwrap();
        let deserialized: ClientId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, client_id);
    }
}
