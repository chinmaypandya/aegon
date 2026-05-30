# Journal

Running log of decisions, discoveries, achievements, and open questions.
Entries are dated `YYYY-MM-DD`, newest first.

---

## 2026-05-30 — v0.4.0: published to crates.io and PyPI

### Achievements

- All five crates published to crates.io in dependency order: `aegon-types` → `aegon-core` + `aegon-adapters` → `aegon-ui` → `aegon-cli`
- `aegon-rs 0.1.0` published to PyPI via maturin — ships the native ARM64/x86 binary in a platform wheel, no Rust toolchain needed by end users
- `aegon run --detached` added — opens the TUI in a new terminal window; cross-platform (macOS + Linux terminal emulator detection)
- Install is now `pip install aegon-rs` or `cargo install aegon-cli`; verified end-to-end by installing from PyPI and running

### Caveats

- **PyPI name collision**: `aegon` was taken by an unrelated package; shipped as `aegon-rs` instead — the standard convention for Rust-backed Python packages
- **Platform wheels only**: maturin currently builds for the host platform only (macOS arm64 in this case). Linux/Windows users need to build from source or wait for CI-based multi-platform wheel builds
- **Token auth**: maturin publish requires `MATURIN_PYPI_TOKEN` env var; username/password rejected by PyPI
- **`--detached` on Linux**: terminal emulator detection is best-effort; no fallback if none of the five tried are installed

### Next steps

- Set up GitHub Actions to build and publish multi-platform wheels (Linux x86_64, macOS arm64/x86_64, Windows) on release tag
- Consider a Homebrew tap as a third install path
- Continue working down the adapter gap list (items 4–8 from the original JSONL audit)

---

## 2026-05-30 — v0.1.0 shipped: live TUI watcher

### What was built

Four crates scaffolded and merged to `main` in one session:

| Crate | Role |
|-------|------|
| `aegon-types` | Pure domain types — the single source of truth for all data shapes |
| `aegon-adapters` | Claude JSONL → `Vec<LogEvent>`; lenient on unknown fields |
| `aegon-ui` | ratatui TUI dashboard with colour-coded live event list |
| `aegon-cli` | `notify`-based file watcher + binary entry point |

The live watcher works: run `cargo run -p aegon-cli` alongside a Claude Code session and events appear in real time.

### Achievements

- **End-to-end pipeline** compiles and runs: JSONL file → adapter → `LogEvent` → TUI render
- **32 tests** across unit (`#[cfg(test)]`) and integration (`tests/`) directories; all green
- **CI fully passing**: fmt, clippy (`-D warnings`), check, test, docs — including `cargo doc -D warnings` which enforces intra-doc link correctness
- **Proper workspace structure**: `crates/` layout, shared `[workspace.dependencies]`, each crate self-contained with its own `Cargo.toml`
- **JSONL format fully audited**: `audit.py` scanned 2 348 records across 6 real projects and documented every record type, content type, tool name, and usage field

### Caveats

These are known limitations as of v0.1.0, grounded in the JSONL audit:

1. **`thinking` blocks invisible** — extended model reasoning (136 occurrences in audit) is silently dropped; the TUI never shows what the model was thinking
2. **`isSidechain` not separated** — sub-agent events (33 in audit) are mixed into the main event stream; the DAG planned in CLAUDE.md cannot be built correctly until this is handled
3. **`agentId` / `attributionSkill` ignored** — every event knows which skill or sub-agent produced it; that attribution is thrown away
4. **`usage` fields incomplete** — `server_tool_use` (web call counts), `service_tier`, `speed`, and the `cache_creation` alternate key are all dropped; cost tracking will undercount
5. **`toolUseResult` alternate path unread** — some tool results land in a top-level `toolUseResult` field on `user` records, not in `message.content`; those results are silently lost
6. **`attachment` records ignored** — deferred-tools delta (which tools became available mid-session) not tracked
7. **Session metadata not captured** — `cwd`, `gitBranch`, `version`, `ai-title` present on every record but discarded; history browser will have no context
8. **No persistence** — `aegon-db` (SQLite store) is not yet built; events exist only for the lifetime of the TUI process
9. **No history or replay** — `aegon history` and `aegon replay` commands planned in CLAUDE.md do not exist yet

### Next steps (priority order)

1. **Fix `toolUseResult` alternate path** — silent data loss; easy and high-impact
2. **Capture `thinking` blocks** — add `EventKind::Thinking` variant and surface it in TUI
3. **Handle `isSidechain`** — add `stream: StreamId` to `LogEvent`; separate main-chain from sub-agent events in TUI
4. **Capture `agentId` / `attributionSkill`** — add to `LogEvent` metadata
5. **Extend `TokenUsage`** — add `server_tool_use`, `service_tier`, `speed`; handle `cache_creation` alternate key
6. **Session metadata** — capture `cwd`, `gitBranch`, `version` into `Session`; surface `ai-title` in TUI header
7. **Build `aegon-db`** — SQLite store so events survive process restarts; enables history and replay
8. **`aegon history` / `aegon replay` commands** — query and replay past sessions

---

## 2026-05-30 — docs-check gap and recurring doc link pattern

CI failed on `cargo doc` again after the `aegon-core` PR because `just docs-check` was running without `RUSTDOCFLAGS="-D warnings"`. Locally, broken intra-doc links are *warnings*; CI promotes them to *errors*. This is now fixed — `docs-check` passes the flag explicitly so the gap between local and CI is closed.

**Pattern to remember:** any new crate that references types from a sibling crate in doc comments needs the full `crate_name::Type` path, not the bare type name. The compiler knows about `use` imports but rustdoc resolves links in the item's own scope, not the import scope. Methods on `self` need `[Self::method]`; struct fields cannot be linked with `[brackets]` at all — use backticks.

---

## 2026-05-30 — Justfile entry points and dependency setup

Added `just setup`, `just run`, `just watch`, `just demo`, and `just audit-jsonl`. Previously getting started required knowing to run `brew install tmux gh` manually and then find the binary. Now `just setup && just run` is the complete onboarding path.

`just demo` uses `osascript` to open a new Terminal window — macOS-only. If Linux support is added later, this recipe will need a branch for `xterm` or similar.

---

## 2026-05-30 — Live demo: parser crash discovered during demo run

During the first live demo run, the watcher was printing `skipping unparseable line: invalid type: map, expected u64` for almost every assistant record. Root cause: `cache_creation` and `server_tool_use` in the Claude API `usage` object are nested JSON objects, not numbers. `RawUsage` had `cache_creation: u64` which caused serde to hard-fail on the entire record — dropping all events from that line silently from the TUI's perspective.

Fix: removed `cache_creation` from `RawUsage`. Since serde ignores unknown fields by default (no `deny_unknown_fields`), both fields are now skipped cleanly. This was only caught by running the actual binary against real session files — the unit tests used handcrafted JSON that didn't include these fields.

**Lesson:** always run the binary against real `~/.claude/projects/` data before shipping an adapter change. The real JSONL has object-valued usage fields that handcrafted test JSON never will.

---

## 2026-05-30 — v0.3.0: aegon-core + split dashboard TUI

### What was built

`aegon-core` — the pure computation layer described in CLAUDE.md from day one, now implemented:

- `SessionState::ingest(event)` is the single mutation point. It classifies each event, updates pending tool calls, detects when parallel branches complete, accumulates token totals, and extends the causal flow chain — no I/O, no side effects.
- `SessionRegistry` routes events to the right session and creates new ones on first sight.
- `FlowNode::ToolGroup(Vec<String>)` groups parallel tool calls by checking whether a new `ToolCall` shares a `parent_id` with already-pending calls — if so, it merges into the same group.

The TUI now splits 50/50: left shows the raw event feed (width-adaptive truncation), right shows the live dashboard for the most recent session — token gauge auto-filling, braille spinners on active tool calls, ✓/✗ on completed steps, and the causal flow string.

### Achievements

- `aegon-core` compiles with zero dependencies beyond `aegon-types` and `chrono`/`uuid`
- Split TUI renders correctly and `cargo doc` passes clean
- First `README.md` written — overview, architecture diagram, crate map
- `SessionState` correctly tracks parallel branches via `parent_id` heuristic

### Caveats

- `SessionRegistry.order` has a subtle bug: `entry().or_insert_with()` mutates `self.sessions` but `self.order.push(id)` inside the closure runs before the entry is confirmed — works correctly in practice because the closure only runs on insertion, but it's a borrow-checker workaround worth revisiting
- Token gauge denominator is hard-coded at 200k; the actual context limit varies by model and isn't in the JSONL
- `estimated_cost_usd()` uses approximate pricing that will drift as Anthropic adjusts rates
- Dashboard shows only the *latest* session; multi-session view (one column per session) is not yet built
- No persistence — `SessionState` is rebuilt from scratch every time `aegon` is launched

### Next steps (revised priority)

1. **Multi-session columns** in the dashboard — already have the data, just need the layout
2. **Session metadata** (`cwd`, `gitBranch`, `ai-title`) captured into `Session` and shown in dashboard header
3. **`aegon-db`** — SQLite persistence so sessions survive process restarts
4. **`aegon history`** / **`aegon replay`** — query and replay past sessions
5. **agentId / attributionSkill** on `LogEvent` (gap #4 from original list)

---

## 2026-05-30 — v0.2.0: adapter gap fixes (thinking, sidechain, tool metadata)

### What was built

- `EventKind::Thinking` — extended model reasoning now visible in the TUI (yellow `THINK` rows)
- `StreamId` on every `LogEvent` — main-chain vs sidechain events are now separated; TUI prefixes sidechain rows with `[S]`
- `ToolMetadata` on `ToolResult` — `stdout`, `stderr`, `interrupted`, `file_path` from `toolUseResult` are now captured
- `cache_creation` alternate key handled — cache creation tokens no longer undercounted on responses that use the shorter key

### Caveats remaining

Gaps 4–8 from the original list are still open:
4. `agentId` / `attributionSkill` not yet captured on `LogEvent`
5. `usage.server_tool_use`, `service_tier`, `speed` not yet in `TokenUsage`
6. Session metadata (`cwd`, `gitBranch`, `version`, `ai-title`) not yet captured
7. `aegon-db` not yet built — no persistence between runs
8. `aegon history` / `aegon replay` commands not yet implemented

### Next steps

Continue down the journal priority list: gaps 4 (`agentId`/`attributionSkill`) and 5 (extended `TokenUsage` fields) are small and belong in the same PR. Gap 6 (session metadata) follows. Gaps 7–8 are larger and will each need their own branch.

---

## 2026-05-30 — toolUseResult is supplementary, not an alternate path

**Correction to gap #1 in the next-steps list.**

Investigated 518 real `toolUseResult` records. In every case, `message.content` also contains the corresponding `tool_result` block — they are never mutually exclusive. `toolUseResult` is **supplementary metadata** about how the tool ran:

- Bash: `{ stdout, stderr, interrupted, isImage, noOutputExpected }`
- Read/Write: `{ type, file: { filePath, content } }`

This means there is no silent data loss from the tool_result path. The fix is to **enrich `ToolResult`** with this structured metadata rather than treating it as an alternate parse path.

---

## 2026-05-30 — JSONL format discovery

Ran `audit.py` across all `~/.claude/projects/**/*.jsonl` for the first time.

Key surprises:
- **8 record types** exist, not the 2–3 assumed during initial design (`assistant`, `user`, `queue-operation`). The others — `file-history-snapshot`, `ai-title`, `attachment`, `last-prompt`, `mode` — were completely unknown before the audit.
- **`attachment` records** carry the deferred-tools delta, which is how Claude Code communicates which tools are available at any point in a session. This is richer than expected.
- **`usage` has many more fields** than documented anywhere: `server_tool_use` (web call counts), `service_tier`, `speed`, `iterations`, `inference_geo`. These suggest Claude internally tracks much more about each inference than the public API exposes.
- **`tool_result` content is polymorphic**: sometimes a plain string, sometimes `[{"type":"text","text":"..."}]`, sometimes `null`. The adapter handles str and list correctly but `null` is not yet explicitly tested.
- **`isSidechain` is real and frequent** — 33 sidechain records in ~2 300 total. Sub-agent usage is not exotic; it happens in normal sessions.
