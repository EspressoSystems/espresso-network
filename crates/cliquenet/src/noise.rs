use std::{collections::HashMap, sync::LazyLock};

use snow::params::NoiseParams;

static NOISE_PARAMS: LazyLock<HashMap<Protocol, NoiseParams>> = LazyLock::new(|| {
    HashMap::from_iter([
        (
            Protocol::IK_25519_AesGcm_Blake2s,
            "Noise_IK_25519_AESGCM_BLAKE2s"
                .parse()
                .expect("valid noise params"),
        ),
        (
            Protocol::IK_25519_ChaChaPoly_Blake2s,
            "Noise_IK_25519_ChaChaPoly_BLAKE2s"
                .parse()
                .expect("valid noise params"),
        ),
    ])
});

/// Supported noise protocol names.
///
/// A protocol name contains the handshake pattern, DH, cipher, and
/// hash function names. See https://noiseprotocol.org for details.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum Protocol {
    IK_25519_AesGcm_Blake2s,
    IK_25519_ChaChaPoly_Blake2s,
}

impl Protocol {
    pub(crate) fn noise_params(self) -> NoiseParams {
        NOISE_PARAMS
            .get(&self)
            .cloned()
            .expect("All protocol names are in NOISE_PARAMS.")
    }
}
