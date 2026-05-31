//! Aegon CLI entry point.
//!
//! Usage:
//!   aegon run              — watch JSONL files, launch TUI in this terminal
//!   aegon run --proxy      — same, plus start the SSE proxy on port 8877
//!   aegon run --detached   — open the TUI in a new terminal window

mod watcher;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "aegon",
    about = "Observability for Claude Code agentic runs",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Watch live Claude Code session events in the terminal (default)
    Run {
        /// Open Aegon in a new terminal window instead of the current one
        #[arg(long, short)]
        detached: bool,

        /// Start the local HTTPS MitM proxy for mid-turn token streaming.
        ///
        /// Listens on 127.0.0.1:8877. Set HTTPS_PROXY=http://127.0.0.1:8877
        /// in the environment where Claude Code runs to enable token capture.
        #[arg(long, short)]
        proxy: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Run {
        detached: false,
        proxy: false,
    }) {
        Command::Run {
            detached: false,
            proxy,
        } => run_inline(proxy).await,
        Command::Run {
            detached: true,
            proxy: _,
        } => run_detached(),
    }
}

/// Launch the TUI in the current terminal, optionally starting the SSE proxy.
async fn run_inline(with_proxy: bool) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // JSONL file watcher — runs on its own OS thread.
    let watcher_tx = tx.clone();
    let watcher_handle = std::thread::spawn(move || {
        if let Err(e) = watcher::watch(watcher_tx) {
            eprintln!("watcher error: {e}");
        }
    });

    // SSE proxy — runs on the tokio runtime alongside the TUI, if requested.
    if with_proxy {
        let proxy_tx = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = aegon_proxy::serve(aegon_proxy::DEFAULT_PORT, proxy_tx).await {
                eprintln!("proxy error: {e}");
            }
        });
    }

    // TUI blocks until the user quits.
    aegon_ui::tui::run(rx)?;

    let _ = watcher_handle.join();
    Ok(())
}

/// Spawn Aegon in a new terminal window and return immediately.
fn run_detached() -> Result<()> {
    let bin = current_exe()?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .args([
                "-e",
                &format!("tell application \"Terminal\" to do script \"{}\"", bin),
            ])
            .spawn()
            .context("failed to open Terminal.app via osascript")?;
        println!("Aegon launched in a new Terminal window.");
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let terms = ["gnome-terminal", "xterm", "konsole", "alacritty", "kitty"];
        for term in terms {
            let result = match term {
                "gnome-terminal" => std::process::Command::new(term).args(["--", &bin]).spawn(),
                _ => std::process::Command::new(term).args(["-e", &bin]).spawn(),
            };
            if result.is_ok() {
                println!("Aegon launched in a new {term} window.");
                return Ok(());
            }
        }
        anyhow::bail!(
            "no supported terminal emulator found (tried: {}). \
             Run `aegon run` directly instead.",
            terms.join(", ")
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    anyhow::bail!("`--detached` is not supported on this platform yet. Run `aegon run` directly.");
}

/// Resolve the path to the current executable.
fn current_exe() -> Result<String> {
    std::env::current_exe()
        .context("could not determine aegon executable path")?
        .to_str()
        .context("executable path is not valid UTF-8")
        .map(str::to_owned)
}
