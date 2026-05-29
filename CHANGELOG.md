# Changelog

All notable changes to Aegon are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions are dated `YYYY-MM-DD`. Unreleased work sits under `[Unreleased]`.

---

## [Unreleased]

---

## [0.3.2] — 2026-05-30

### Fixed
- Broken intra-doc links in `aegon-core` (`FlowNode::Tool` → `FlowNode::ToolGroup`, bare method names → `Self::method`, cross-crate types → `aegon_types::` prefix)
- `just docs-check` now runs with `RUSTDOCFLAGS="-D warnings"` to mirror CI — broken links are now errors locally, not just in CI

---

## [0.3.1] — 2026-05-30

### Added
- `just setup` — installs all system (tmux, gh, just) and Cargo (cargo-outdated, cargo-audit) dependencies in one command
- `just run` / `just watch` — build and launch the TUI watcher in the current terminal
- `just demo` — launch in a new macOS Terminal window via osascript
- `just audit-jsonl` — run `examples/jsonl/audit.py` against real session files

---

## [0.3.0] — 2026-05-30

### Added

**`aegon-core`** — new crate, pure session logic with no I/O
- `SessionState` — ingests `LogEvent`s and maintains: pending tool calls, completed step history with timing, cumulative token totals, ordered causal flow nodes
- `SessionRegistry` — routes events to per-session state; creates sessions on first sight; tracks insertion order
- `Step` / `StepStatus` — one tool invocation through `Running → Done/Failed`; records wall-clock duration
- `FlowNode` — causal chain unit: `Human`, `Thinking`, `ToolGroup(Vec<String>)`, `Assistant`; parallel dispatch detected via shared `parent_id`
- `TokenTotals` — accumulates `input`, `output`, `cache_read`, `cache_created` across all turns; `estimated_cost_usd()` for rough cost display

**`aegon-ui`** — dashboard panel (right half of the split TUI)
- `dashboard::draw` — splits right panel into session header, token gauge, steps list, and flow string
- `dashboard::gauge` — ratatui `Gauge` showing token usage against 200k context limit; colour shifts green → yellow → red as context fills; estimated cost display
- `dashboard::steps` — braille spinner on active tool calls with elapsed time; ✓/✗ icons on completed/failed steps with duration
- `dashboard::flow` — one-line causal chain: `USER → THINK → [Bash ‖ Read] → ASST` with parallel groups rendered inline

**`aegon-ui`** — split TUI layout
- Left 50%: raw event feed (width-aware truncation — no more hard-coded 80 chars)
- Right 50%: live dashboard for the latest active session

**`README.md`** — first version: overview, ASCII screenshot of the split TUI, architecture diagram, crate map, and quick-start instructions

### Changed
- `App` now holds `SessionRegistry` alongside the raw event buffer; `push` feeds both
- `App::tick()` advances a frame counter used for spinner animation
- `format_event` now takes `max_detail: usize` (derived from actual terminal width) instead of hard-coding 80

---

## [0.2.0] — 2026-05-30

### Added

**`aegon-types`**
- `EventKind::Thinking { text, signature }` — captures extended model reasoning blocks
- `StreamId` enum (`Main` / `Sidechain`) — identifies whether an event belongs to the main session chain or a sub-agent run
- `LogEvent.stream: StreamId` field — defaults to `Main`; set to `Sidechain` when `isSidechain: true` in the source record
- `ToolMetadata` struct — supplementary execution metadata from the `toolUseResult` field: `stdout`, `stderr`, `interrupted`, `is_image`, `file_path`
- `ToolResult.metadata: Option<ToolMetadata>` — attaches `ToolMetadata` when present

**`aegon-adapters` (ClaudeAdapter)**
- Parses `thinking` content blocks into `EventKind::Thinking`
- Reads `isSidechain` and sets `LogEvent.stream` accordingly
- Reads `toolUseResult` and populates `ToolResult.metadata`
- Handles alternate `cache_creation` key in `usage` (falls back from `cache_creation_input_tokens`)

**`aegon-ui`**
- Renders `EventKind::Thinking` rows with `THINK` label in yellow
- Prefixes sidechain events with `[S]` in the event list

### Changed
- `ToolResult` now has an additional `metadata: Option<ToolMetadata>` field (non-breaking: defaults to `None` when deserialised from older data)

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
