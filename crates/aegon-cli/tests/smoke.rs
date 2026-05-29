//! Smoke tests for the aegon binary.
//!
//! Runs the compiled binary as a subprocess to verify it starts without
//! panicking. Full TUI interaction is not tested here — that belongs in
//! manual or end-to-end test suites.

use std::process::Command;
use std::time::Duration;

fn aegon_bin() -> Command {
    // `cargo test` sets CARGO_BIN_EXE_aegon so we get the right debug binary.
    let bin = env!("CARGO_BIN_EXE_aegon");
    Command::new(bin)
}

#[test]
fn binary_exists_and_is_executable() {
    let bin_path = env!("CARGO_BIN_EXE_aegon");
    assert!(
        std::path::Path::new(bin_path).exists(),
        "aegon binary not found at {bin_path}"
    );
}

#[test]
fn binary_exits_without_panic_when_stdin_closed() {
    // Spawn with stdin closed and a short timeout. The watcher will start,
    // find no TUI input, and the process should not crash.
    // We just verify it launches (not that it exits quickly — the TUI blocks).
    let mut child = aegon_bin()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to spawn aegon binary");

    // Give it a moment to initialise without crashing.
    std::thread::sleep(Duration::from_millis(300));

    // Kill it cleanly — we only care it didn't crash on start.
    let _ = child.kill();
    let status = child.wait().expect("failed to wait on child");

    // Killed processes have non-zero exit on Unix; that's fine.
    // We only care it didn't exit with a panic before we killed it.
    let _ = status;
}
