//! Fixtures for tests that need signed leaf chains.

use hotshot_types::{
    data::Leaf2,
    message::UpgradeLock,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};

use crate::utils::CertifiedLeafChain;

/// `leaves` re-signed by `signers`: every justify QC and a closing cert2, so
/// `CertifiedLeafChain::verify` accepts it.
pub fn sign_leaf_chain<T: NodeType>(
    leaves: Vec<Leaf2<T>>,
    signers: &[(
        T::SignatureKey,
        <T::SignatureKey as SignatureKey>::PrivateKey,
    )],
    upgrade_lock: &UpgradeLock<T>,
) -> CertifiedLeafChain<T> {
    let _ = (leaves, signers, upgrade_lock);
    todo!()
}
