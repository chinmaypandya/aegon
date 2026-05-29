# Changelog

All notable changes to Aegon are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions are dated `YYYY-MM-DD`. Unreleased work sits under `[Unreleased]`.

---

## [Unreleased]

---

## [0.1.0] — 2026-05-30

### Added

**`aegon-types`** — core domain types (no logic, no I/O)
- `LogEvent` — central event record: `id`, `session_id`, `parent_id`, `timestamp`, `kind`
- `EventKind` — enum of observable actions: `ToolCall`, `ToolResult`, `AssistantMessage`, `UserMessage`, `TokenUsage`, `Unknown`
- `ToolCall` — tool invocation with raw JSON `input` preserved for downstream consumers
- `ToolResult` — tool output with `is_error` flag; `content` can be a string or flattened text-block list
- `TokenUsage` — per-turn token counts including `cache_read` and `cache_creation` fields
- `Session` — top-level container tying events to a source JSONL file path
- `Error` / `Result` — typed errors via `thiserror`

**`aegon-adapters`** — JSONL parsing layer
- `Adapter` trait — `parse_line(&str) -> Result<Vec<LogEvent>>`
- `ClaudeAdapter` — parses Claude Code JSONL format; one raw line can expand to multiple events (e.g. assistant turn with tool calls → `ToolCall` + `AssistantMessage`); uses `#[serde(other)]` so unknown fields never cause parse failures

**`aegon-ui`** — terminal UI
- `App` — rolling event buffer (capped at 500); `should_quit` flag
- `tui::run` — ratatui dashboard with header, colour-coded live event list (newest first), and footer with event count; press `q` / `Esc` to quit

**`aegon-cli`** — runnable binary (`cargo run -p aegon-cli`)
- `watcher` — uses `notify` to tail `~/.claude/projects/**/*.jsonl`; tracks per-file read positions so only new lines are parsed
- `main` — spawns watcher on a background thread, runs TUI on the main thread via `aegon_ui::tui::run`

**Workspace**
- Four-crate workspace under `crates/` with shared `[workspace.dependencies]`
- Each crate has `src/` and `tests/` — 32 tests total (unit + integration)
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`, `cargo doc -D warnings`

**`examples/`**
- `examples/jsonl/audit.py` — scans all `~/.claude/projects/**/*.jsonl` and reports every record type, content type, tool name, usage key, and `tool_result` shape
- `examples/jsonl/findings.md` — annotated findings from the audit with full adapter gap analysis
- `examples/usage/README.md` — placeholder for CLI usage examples

### Fixed
- Broken rustdoc intra-doc links in `aegon-types` sub-modules (`[LogEvent]` → `[crate::LogEvent]` etc.) and private `MAX_EVENTS` link in `aegon-ui`

---

[Unreleased]: https://github.com/chinmaypandya/aegon/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/chinmaypandya/aegon/releases/tag/v0.1.0
