//! Error type for `aegon-proxy`.

/// All errors that can occur inside the proxy.
///
/// Kept as a single enum so callers can match on specific failure modes
/// without importing internal implementation types.
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("certificate generation failed: {0}")]
    Cert(#[from] rcgen::Error),

    #[error("malformed CONNECT request: {0}")]
    BadConnect(String),

    #[error("upstream connection to {host} failed: {source}")]
    Upstream {
        host: String,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, ProxyError>;
