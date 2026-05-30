//! Aegon CLI entry point.
//!
//! Usage:
//!   aegon run             — launch the TUI in this terminal
//!   aegon run --detached  — open the TUI in a new terminal window

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
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Run { detached: false }) {
        Command::Run { detached: false } => run_inline().await,
        Command::Run { detached: true } => run_detached(),
    }
}

/// Launch the TUI in the current terminal.
async fn run_inline() -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let watcher_handle = std::thread::spawn(move || {
        if let Err(e) = watcher::watch(tx) {
            eprintln!("watcher error: {e}");
        }
    });

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
        // Try common terminal emulators in preference order.
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
