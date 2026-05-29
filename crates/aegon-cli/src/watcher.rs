//! File watcher that tails Claude Code JSONL session files.
//!
//! Uses `notify` to detect writes to `~/.claude/projects/**/*.jsonl`, reads
//! new lines since the last known position, parses them with the Claude
//! adapter, and sends resulting [`LogEvent`]s down the channel.

use aegon_adapters::{Adapter, ClaudeAdapter};
use aegon_types::LogEvent;
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Watch all Claude Code session directories and forward parsed events to `tx`.
///
/// Blocks until the sender is dropped (i.e. the TUI exits).
pub fn watch(tx: Sender<LogEvent>) -> Result<()> {
    let claude_dir = dirs().context("could not locate ~/.claude directory")?;
    let adapter = ClaudeAdapter;

    // Track how many bytes we've already read per file so we only parse new lines.
    let mut read_positions: HashMap<PathBuf, u64> = HashMap::new();

    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(notify_tx)?;

    for subdir in ["projects", "sessions"] {
        let path = claude_dir.join(subdir);
        if path.exists() {
            watcher.watch(&path, RecursiveMode::Recursive)?;
        }
    }

    loop {
        match notify_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    continue;
                }
                for path in event.paths {
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }
                    if let Err(e) = tail_new_lines(&path, &adapter, &tx, &mut read_positions) {
                        eprintln!("parse error for {}: {e}", path.display());
                    }
                }
            }
            // Timeout is fine — just poll again.
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            // Sender dropped means TUI exited.
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(Err(e)) => eprintln!("notify error: {e}"),
        }

        // Exit if the downstream consumer (TUI) has gone away.
        if tx.send(sentinel_check()).is_err() {
            break;
        }
    }

    Ok(())
}

/// Read any lines added to `path` since the last recorded position.
fn tail_new_lines(
    path: &PathBuf,
    adapter: &ClaudeAdapter,
    tx: &Sender<LogEvent>,
    positions: &mut HashMap<PathBuf, u64>,
) -> Result<()> {
    let mut file = File::open(path)?;
    let pos = positions.entry(path.clone()).or_insert(0);
    file.seek(SeekFrom::Start(*pos))?;

    let mut reader = BufReader::new(&file);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match adapter.parse_line(trimmed) {
            Ok(events) => {
                for event in events {
                    // If the receiver is gone, stop reading.
                    if tx.send(event).is_err() {
                        return Ok(());
                    }
                }
            }
            Err(e) => eprintln!("skipping unparseable line: {e}"),
        }
    }

    *pos = file.stream_position()?;
    Ok(())
}

fn dirs() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

// A dummy event used only to probe whether the channel is still open.
// It is never rendered — the TUI discards Unknown events.
fn sentinel_check() -> LogEvent {
    LogEvent {
        id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::nil(),
        parent_id: None,
        timestamp: chrono::Utc::now(),
        kind: aegon_types::EventKind::Unknown,
    }
}
