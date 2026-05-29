# Journal

Running log of decisions, discoveries, achievements, and open questions.
Entries are dated `YYYY-MM-DD`, newest first.

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

## 2026-05-30 — JSONL format discovery

Ran `audit.py` across all `~/.claude/projects/**/*.jsonl` for the first time.

Key surprises:
- **8 record types** exist, not the 2–3 assumed during initial design (`assistant`, `user`, `queue-operation`). The others — `file-history-snapshot`, `ai-title`, `attachment`, `last-prompt`, `mode` — were completely unknown before the audit.
- **`attachment` records** carry the deferred-tools delta, which is how Claude Code communicates which tools are available at any point in a session. This is richer than expected.
- **`usage` has many more fields** than documented anywhere: `server_tool_use` (web call counts), `service_tier`, `speed`, `iterations`, `inference_geo`. These suggest Claude internally tracks much more about each inference than the public API exposes.
- **`tool_result` content is polymorphic**: sometimes a plain string, sometimes `[{"type":"text","text":"..."}]`, sometimes `null`. The adapter handles str and list correctly but `null` is not yet explicitly tested.
- **`isSidechain` is real and frequent** — 33 sidechain records in ~2 300 total. Sub-agent usage is not exotic; it happens in normal sessions.
