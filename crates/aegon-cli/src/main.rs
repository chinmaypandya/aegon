//! Aegon CLI entry point.
//!
//! Starts the file watcher and the ratatui TUI, wiring them together via a
//! channel so the watcher thread can push new events into the UI loop.

mod app;
mod tui;
mod watcher;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // Spawn the file watcher on a background thread.
    let watcher_handle = std::thread::spawn(move || {
        if let Err(e) = watcher::watch(tx) {
            eprintln!("watcher error: {e}");
        }
    });

    // Run the TUI on the main thread (crossterm requires it).
    tui::run(rx)?;

    // Watcher thread exits when the channel sender is dropped (TUI closed).
    let _ = watcher_handle.join();

    Ok(())
}
