//! Local HTTPS MitM proxy for mid-turn Anthropic API token observability.
//!
//! Run alongside `aegon-cli` to intercept the SSE token stream that Claude Code
//! receives from `api.anthropic.com`. Every `content_block_delta` event becomes
//! an `EventKind::TokenChunk` sent down the same channel as JSONL events, giving
//! the TUI real-time character-by-character visibility into what the model is
//! generating — including thinking blocks — before the turn is committed to JSONL.
//!
//! # Setup
//!
//! 1. Start the proxy (done automatically by `aegon run --proxy`).
//! 2. Set `HTTPS_PROXY=http://127.0.0.1:8877` in the environment where Claude Code
//!    runs, **or** trust the proxy CA and set `NODE_EXTRA_CA_CERTS` to the exported
//!    CA cert path.
//! 3. Watch `TokenChunk` events appear in the TUI as the model streams.

mod anthropic;
mod ca;
mod error;
mod sse;
mod tunnel;

pub use error::ProxyError;

use aegon_types::LogEvent;
use ca::Ca;
use std::sync::{mpsc::Sender, Arc};
use tokio::net::TcpListener;
use tunnel::build_client_config;

/// Default port the proxy listens on.
pub const DEFAULT_PORT: u16 = 8877;

/// Start the proxy on `port`, forwarding parsed `TokenChunk` events to `tx`.
///
/// Blocks until the listener errors or the process exits. Designed to be
/// spawned on a background `tokio` task alongside the JSONL file watcher.
///
/// # Errors
///
/// Returns an error if the port is already in use or if the CA or root cert
/// store cannot be initialised.
pub async fn serve(port: u16, tx: Sender<LogEvent>) -> Result<(), ProxyError> {
    let ca = Arc::new(Ca::generate()?);
    let client_config = Arc::new(build_client_config()?);

    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!(
        "aegon-proxy: listening on 127.0.0.1:{port} — set HTTPS_PROXY=http://127.0.0.1:{port}"
    );

    loop {
        let (socket, _addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("aegon-proxy: accept error: {e}");
                continue;
            }
        };

        let ca = Arc::clone(&ca);
        let client_config = Arc::clone(&client_config);
        let tx = tx.clone();

        tokio::spawn(async move {
            tunnel::handle(socket, ca, client_config, tx).await;
        });
    }
}
