use std::{collections::BTreeSet, sync::Arc};

use bytes::Bytes;
use parking_lot::RwLock;
use rcgen::{CertificateParams, PKCS_ED25519, PublicKeyData};
use tokio_rustls::rustls::{
    self, ClientConfig, PeerIncompatible, ServerConfig, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::WebPkiSupportedAlgorithms,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime},
    server::danger::{ClientCertVerified, ClientCertVerifier},
};
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::InvalidKeypair;

static SUPPORTED_SIG_ALGS: WebPkiSupportedAlgorithms = WebPkiSupportedAlgorithms {
    all: &[webpki::aws_lc_rs::ED25519],
    mapping: &[(SignatureScheme::ED25519, &[webpki::aws_lc_rs::ED25519])],
};

pub struct Keypair {
    pair: rcgen::KeyPair,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey {
    key: Bytes,
}

pub struct SecretKey {
    key: PrivateKeyDer<'static>,
}

pub struct Certificate {
    crt: rcgen::Certificate,
}

#[derive(Debug)]
pub struct Verifier {
    sig_algs: WebPkiSupportedAlgorithms,
    whitelist: RwLock<BTreeSet<Bytes>>,
}

impl Keypair {
    pub fn generate() -> Result<Self, InvalidKeypair> {
        let kp = rcgen::KeyPair::generate_for(&PKCS_ED25519).map_err(|_| InvalidKeypair(()))?;
        Ok(Self { pair: kp })
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            key: self.pair.der_bytes().to_vec().into(),
        }
    }

    pub fn cert(&self) -> Certificate {
        let crt = CertificateParams::new([])
            .expect("empty alt names yield cert")
            .self_signed(&self.pair)
            .expect("valid keypair signs cert");
        Certificate { crt }
    }

    pub fn der(&self) -> &[u8] {
        self.pair.serialized_der()
    }
}

impl From<Keypair> for SecretKey {
    fn from(val: Keypair) -> Self {
        Self {
            key: val.pair.into(),
        }
    }
}

impl SecretKey {
    pub fn server_config(
        &self,
        vrf: Arc<dyn ClientCertVerifier>,
        crt: CertificateDer<'static>,
    ) -> Result<ServerConfig, InvalidCert> {
        ServerConfig::builder()
            .with_client_cert_verifier(vrf)
            .with_single_cert(vec![crt], self.key.clone_key())
            .map_err(|_| InvalidCert(()))
    }

    pub fn client_config(
        &self,
        vrf: Arc<dyn ServerCertVerifier>,
        crt: CertificateDer<'static>,
    ) -> Result<ClientConfig, InvalidCert> {
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(vrf)
            .with_client_auth_cert(vec![crt], self.key.clone_key())
            .map_err(|_| InvalidCert(()))
    }
}

impl PublicKey {
    pub fn der(&self) -> &[u8] {
        &self.key
    }
}

impl Certificate {
    pub fn der(&self) -> &CertificateDer<'static> {
        self.crt.der()
    }
}

impl Verifier {
    pub fn new() -> Self {
        Self {
            sig_algs: SUPPORTED_SIG_ALGS,
            whitelist: RwLock::new(BTreeSet::new()),
        }
    }

    pub fn add(&self, k: PublicKey) {
        self.whitelist.write().insert(k.key);
    }

    pub fn remove(&self, k: &PublicKey) {
        self.whitelist.write().remove(&k.key);
    }

    fn verify(&self, c: &CertificateDer<'_>) -> Result<(), rustls::Error> {
        let (_, crt) = X509Certificate::from_der(&*c).map_err(|_| {
            rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding)
        })?;
        if self.whitelist.read().contains(crt.public_key().subject_public_key.as_ref()) {
            Ok(())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            ))
        }
    }
}

impl ClientCertVerifier for Verifier {
    fn verify_client_cert(
        &self,
        c: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.verify(c).map(|()| ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::PeerIncompatible(
            PeerIncompatible::Tls12NotOffered,
        ))
    }

    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        s: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, s, &self.sig_algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }

    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }
}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        c: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        self.verify(c).map(|()| ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::PeerIncompatible(
            PeerIncompatible::Tls12NotOffered,
        ))
    }

    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        s: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, s, &self.sig_algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid certificate")]
pub struct InvalidCert(());
