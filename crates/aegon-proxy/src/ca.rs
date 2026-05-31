//! Local certificate authority for TLS MitM.
//!
//! Generates a self-signed CA at startup and mints leaf certificates on demand,
//! one per target hostname. Leaf certs are cached so each hostname only pays the
//! generation cost once per proxy process lifetime.

use crate::error::Result;
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A signed leaf certificate and its private key, ready for use in a TLS server config.
pub struct LeafCert {
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
}

/// The local certificate authority.
///
/// Holds the CA certificate and key used to sign leaf certificates for each
/// intercepted hostname. Leaf certs are cached after first generation.
pub struct Ca {
    ca_cert: rcgen::Certificate,
    ca_key: KeyPair,
    cache: Mutex<HashMap<String, Arc<LeafCert>>>,
}

impl Ca {
    /// Generate a fresh in-memory CA. Called once at proxy startup.
    pub fn generate() -> Result<Self> {
        let ca_key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![])?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "Aegon Local CA");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Aegon");
        let ca_cert = params.self_signed(&ca_key)?;
        Ok(Self {
            ca_cert,
            ca_key,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Mint (or return cached) a leaf certificate for `hostname`.
    pub fn leaf_for(&self, hostname: &str) -> Result<Arc<LeafCert>> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(hostname) {
                return Ok(Arc::clone(cached));
            }
        }

        let leaf_key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![hostname.to_string()])?;
        params.distinguished_name.push(DnType::CommonName, hostname);
        let leaf_cert = params.signed_by(&leaf_key, &self.ca_cert, &self.ca_key)?;

        let cert_der = CertificateDer::from(leaf_cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));

        let leaf = Arc::new(LeafCert { cert_der, key_der });

        let mut cache = self.cache.lock().unwrap();
        cache.insert(hostname.to_string(), Arc::clone(&leaf));

        Ok(leaf)
    }
}
