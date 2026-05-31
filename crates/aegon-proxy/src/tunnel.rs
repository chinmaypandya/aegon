//! CONNECT tunnel handler — one accepted TCP connection per call.
//!
//! Reads the HTTP CONNECT request, performs TLS MitM for `api.anthropic.com`,
//! proxies all traffic bidirectionally, and taps SSE response bodies to emit
//! `TokenChunk` events. Non-Anthropic CONNECT targets are tunnelled transparently.

use crate::{
    anthropic::{decode_sse_event, TurnState},
    ca::Ca,
    error::{ProxyError, Result},
    sse::SseParser,
};
use aegon_types::LogEvent;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::sync::{mpsc::Sender, Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use uuid::Uuid;

const ANTHROPIC_HOST: &str = "api.anthropic.com";

/// Handle one accepted client TCP connection.
///
/// Parses the CONNECT request, then either MitM-intercepts (Anthropic) or
/// transparently tunnels (everything else).
pub async fn handle(
    mut client: TcpStream,
    ca: Arc<Ca>,
    client_config: Arc<ClientConfig>,
    tx: Sender<LogEvent>,
) {
    if let Err(e) = try_handle(&mut client, ca, client_config, tx).await {
        // Log and move on — a single failed tunnel must not crash the server.
        eprintln!("aegon-proxy: tunnel error: {e}");
    }
}

async fn try_handle(
    client: &mut TcpStream,
    ca: Arc<Ca>,
    client_config: Arc<ClientConfig>,
    tx: Sender<LogEvent>,
) -> Result<()> {
    // ── Read the HTTP CONNECT request ────────────────────────────────────────
    let (host, port) = read_connect_target(client).await?;

    // Acknowledge: client can now start TLS (or raw TCP for non-TLS ports).
    client
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .await?;

    // ── Connect to the upstream server ───────────────────────────────────────
    let upstream_addr = format!("{host}:{port}");
    let upstream =
        TcpStream::connect(&upstream_addr)
            .await
            .map_err(|source| ProxyError::Upstream {
                host: host.clone(),
                source,
            })?;

    if host != ANTHROPIC_HOST {
        // Transparent tunnel — no TLS inspection.
        return bidirectional_copy(client, upstream).await;
    }

    // ── TLS MitM for api.anthropic.com ───────────────────────────────────────

    // Server side: accept TLS from Claude Code using our minted leaf cert.
    let leaf = ca.leaf_for(&host)?;
    let server_config = build_server_config(leaf.cert_der.clone(), leaf.key_der.clone_key())?;
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let mut tls_client = acceptor.accept(client).await?;

    // Client side: connect to real Anthropic with system trust.
    let connector = TlsConnector::from(client_config);
    let server_name =
        ServerName::try_from(ANTHROPIC_HOST.to_owned()).expect("static hostname is valid");
    let mut tls_upstream = connector.connect(server_name, upstream).await?;

    // ── Proxy: client → upstream (request) ───────────────────────────────────
    // Read the full HTTP request from Claude Code and forward to Anthropic.
    let mut req_buf = vec![0u8; 65536];
    let n = tls_client.read(&mut req_buf).await?;
    if n == 0 {
        return Ok(());
    }
    tls_upstream.write_all(&req_buf[..n]).await?;
    // Extract request-id hint from request headers (best-effort).
    let request_id = extract_request_id_from_request(&req_buf[..n]);
    let session_id = Uuid::new_v4(); // proxy-assigned; no JSONL session context available here

    // ── Proxy: upstream → client (response) with SSE tapping ─────────────────
    // Read response headers to detect SSE.
    let (header_bytes, is_sse, upstream_request_id) =
        read_response_headers(&mut tls_upstream).await?;
    let effective_request_id = upstream_request_id.unwrap_or(request_id);

    // Forward headers to client.
    tls_client.write_all(&header_bytes).await?;

    if is_sse {
        tap_sse_body(
            &mut tls_upstream,
            &mut tls_client,
            &effective_request_id,
            session_id,
            tx,
        )
        .await?;
    } else {
        // Non-SSE response (e.g. token count endpoints) — plain copy.
        copy_body(&mut tls_upstream, &mut tls_client).await?;
    }

    Ok(())
}

/// Read response headers into a buffer, returning:
/// - the raw header bytes (to forward to client),
/// - whether the response is `text/event-stream`,
/// - the `x-request-id` value if present.
async fn read_response_headers(
    upstream: &mut tokio_rustls::client::TlsStream<TcpStream>,
) -> Result<(Vec<u8>, bool, Option<String>)> {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 1];

    // Read byte-by-byte until \r\n\r\n.
    loop {
        upstream.read_exact(&mut tmp).await?;
        buf.push(tmp[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 65536 {
            break; // safety valve
        }
    }

    let header_text = String::from_utf8_lossy(&buf).to_lowercase();
    let is_sse = header_text.contains("content-type: text/event-stream");
    let request_id = extract_request_id_from_response(&header_text);

    Ok((buf, is_sse, request_id))
}

/// Copy an SSE response body from `upstream` to `client`, feeding each chunk
/// through `SseParser` and emitting `TokenChunk` events.
async fn tap_sse_body(
    upstream: &mut tokio_rustls::client::TlsStream<TcpStream>,
    client: &mut tokio_rustls::server::TlsStream<&mut TcpStream>,
    request_id: &str,
    session_id: Uuid,
    tx: Sender<LogEvent>,
) -> Result<()> {
    let mut parser = SseParser::default();
    let mut state = TurnState::new();
    let mut buf = vec![0u8; 8192];

    loop {
        let n = upstream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        // Forward bytes to client unchanged.
        client.write_all(&buf[..n]).await?;

        // Parse and emit token chunks.
        for event in parser.feed(&buf[..n]) {
            if let Some(log) = decode_sse_event(&event, &mut state, request_id, session_id) {
                // A disconnected receiver means the TUI has exited — stop quietly.
                if tx.send(log).is_err() {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

/// Copy a non-SSE response body verbatim.
async fn copy_body(
    upstream: &mut tokio_rustls::client::TlsStream<TcpStream>,
    client: &mut tokio_rustls::server::TlsStream<&mut TcpStream>,
) -> Result<()> {
    let mut buf = vec![0u8; 8192];
    loop {
        let n = upstream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        client.write_all(&buf[..n]).await?;
    }
    Ok(())
}

/// Bidirectional copy for non-Anthropic CONNECT targets.
async fn bidirectional_copy(client: &mut TcpStream, mut upstream: TcpStream) -> Result<()> {
    tokio::io::copy_bidirectional(client, &mut upstream).await?;
    Ok(())
}

/// Parse `HOST:PORT` from an HTTP CONNECT request line.
async fn read_connect_target(stream: &mut TcpStream) -> Result<(String, u16)> {
    let mut buf = vec![0u8; 2048];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // First line: "CONNECT host:port HTTP/1.1"
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| ProxyError::BadConnect("empty request".into()))?;

    let target = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| ProxyError::BadConnect(format!("malformed line: {first_line}")))?;

    let (host, port_str) = target
        .rsplit_once(':')
        .ok_or_else(|| ProxyError::BadConnect(format!("no port in CONNECT target: {target}")))?;

    let port = port_str
        .parse::<u16>()
        .map_err(|_| ProxyError::BadConnect(format!("invalid port: {port_str}")))?;

    Ok((host.to_owned(), port))
}

fn build_server_config(
    cert: rustls::pki_types::CertificateDer<'static>,
    key: rustls::pki_types::PrivateKeyDer<'static>,
) -> Result<ServerConfig> {
    Ok(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?)
}

/// Build a `ClientConfig` that trusts the system root certificate store.
///
/// Used for outbound TLS connections from the proxy to `api.anthropic.com`.
pub fn build_client_config() -> Result<ClientConfig> {
    let mut root_store = RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs();
    for cert in native_certs.certs {
        // Ignore individual cert errors — at least some roots must load.
        let _ = root_store.add(cert);
    }
    Ok(ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

fn extract_request_id_from_request(bytes: &[u8]) -> String {
    // Requests don't carry a request-id; use a fresh UUID as placeholder.
    // The response x-request-id will overwrite this when available.
    let _ = bytes;
    Uuid::new_v4().to_string()
}

fn extract_request_id_from_response(headers: &str) -> Option<String> {
    for line in headers.lines() {
        if let Some(value) = line.strip_prefix("x-request-id:") {
            return Some(value.trim().to_owned());
        }
    }
    None
}
