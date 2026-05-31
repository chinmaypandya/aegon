# Aegon

An observability tool for Claude Code agentic runs. Watches the JSONL files Claude writes
during a session, parses them into typed Rust structures, builds DAGs of each run, stores
them in SQLite, and displays them — live or historical — in a terminal UI and an external
web UI (similar to what LangSmith does for LangChain).

---

## Purpose

Claude Code writes raw JSONL to `~/.claude/projects/` and `~/.claude/sessions/` as it runs.
These files are the ground truth of what happened: every tool call, tool output, assistant
message, token count, and context event. Aegon makes that data:

- **Parseable** — raw JSONL → typed native Rust structs, available workspace-wide
- **Structured** — each run becomes a DAG (nodes = steps, edges = dependencies/causality)
- **Queryable** — stored in SQLite as a local index for fast retrieval and replay
- **Backed up** — periodic snapshots to disk as permanent, portable storage
- **Observable** — live view of an ongoing run; full history replay of past runs
- **Visual** — terminal UI for local use; web UI for a richer LangSmith-style view

Target use case: you trigger a Claude Code agentic run, Aegon runs alongside it, and you can
watch every tool call, token spent, and decision in real time — or replay any past run later.

---

## Crate Map

```
aegon/
├── aegon-types/       Core domain types — the single source of truth for all data shapes
├── aegon-core/        JSONL parsing, DAG construction, run lifecycle logic
├── aegon-adapters/    Per-tool adapters: Claude, Gemini, OpenAI — normalise their JSONL to aegon-types
├── aegon-db/          Storage engine: SQLite index + disk backup
├── aegon-cli/         CLI entry point — file watcher, live mode, query commands
├── aegon-ui/          Terminal UI (ratatui) + web UI bridge
├── aegon-proxy/       Local HTTPS MitM proxy — intercepts Anthropic SSE streams for mid-turn token observability
├── aegon-mcp/         Optional MCP server — exposes run data to other tools
└── aegon-tests/       Integration tests across the full stack
```

### Crate responsibilities

**`aegon-types`**
- All domain types: `Event`, `ToolCall`, `ToolResult`, `Message`, `TokenUsage`, `Run`, `Step`, `DagNode`, `DagEdge`
- No logic, no I/O — pure data shapes and their `serde` impls
- Every other crate depends on this; nothing else may define domain types
- Error types for each domain concept live here too (`thiserror`)

**`aegon-core`**
- DAG builder: takes a `Vec<Event>` for a run → `Dag<Step>`
- Run lifecycle: detects run start/end, session boundaries, context resets
- No storage, no UI, no knowledge of any specific tool format
- This is the brain; keep it free of infrastructure and adapter specifics

**`aegon-adapters`**
- One sub-module per agentic tool: `claude`, `gemini`, `openai`
- Each adapter knows the JSONL schema that tool emits and normalises it into `Vec<Event>` (aegon-types)
- New tool support = new adapter module, nothing else changes
- Adapters must be lenient on unknown fields — tool formats evolve without notice
- Exposes a common `Adapter` trait so `aegon-cli` can select the right one at runtime

**`aegon-db`**
- `SqliteStore`: persists `Run`, `Dag`, `Event` to SQLite; queries by run ID, time range, tool name
- `DiskBackup`: periodic snapshots of the SQLite DB + raw JSONL to a configured backup path
- Exposes a `Store` trait (defined in `aegon-types`) so the CLI can swap backends in tests
- Migrations live here; schema is an implementation detail, not part of the public API

**`aegon-cli`**
- Binary entry point
- File watcher: uses `notify` to tail `~/.claude/projects/**/*.jsonl` and `~/.claude/sessions/**/*.jsonl`
- Selects the right `aegon-adapters` adapter based on config or detected format
- Live mode: raw lines → adapter → `aegon-core` → `aegon-db` in real time
- Query commands: `aegon history`, `aegon replay <run-id>`, `aegon status`
- Passes data to `aegon-ui` for display; does not own rendering

**`aegon-ui`**
- Terminal UI built with `ratatui`: live dashboard + history browser
- Web UI: serves a local HTTP endpoint that a browser-based frontend can poll or subscribe to
  (the external "LangSmith-style" view)
- Consumes `aegon-types` only — no direct DB or parser access

**`aegon-proxy`**
- Local HTTPS MitM proxy that intercepts Anthropic API SSE streams before Claude Code buffers them
- `ca.rs` — generates a local CA and signs per-hostname leaf certs on the fly (rcgen)
- `sse.rs` — stateful byte-level SSE parser: raw bytes → `SseEvent`
- `anthropic.rs` — Anthropic SSE decoder: `SseEvent` → `Option<LogEvent>` (emits `EventKind::TokenChunk`)
- `tunnel.rs` — CONNECT handler: TLS MitM for `api.anthropic.com`, transparent tunnel for all other hosts
- `lib.rs` — public `serve(port, tx)` entry point consumed by `aegon-cli --proxy`
- Emits `EventKind::TokenChunk { request_id, text, is_thinking }` into the shared event channel

**`aegon-mcp`**
- Optional MCP server that exposes run data and queries as MCP tools
- Allows other Claude Code sessions to introspect Aegon data
- Thin adapter over `aegon-db`

**`aegon-tests`**
- Integration tests that wire the full stack: real JSONL fixtures → adapter → core → db → query
- No unit test duplication — those live in each crate's `#[cfg(test)]` module
- Fixture JSONL files live in `aegon-tests/fixtures/<tool>/` (e.g. `fixtures/claude/`, `fixtures/gemini/`)

---

## Key Data Flow

```
~/.claude/projects/**/*.jsonl   (or gemini / openai equivalent paths)
         │
         │  (file watcher — aegon-cli)
         ▼
    raw JSONL lines
         │
         │  (tool adapter — aegon-adapters)
         │  Claude adapter / Gemini adapter / OpenAI adapter
         ▼
    Vec<Event>  (aegon-types)
         │
         │  (DAG builder — aegon-core)
         ▼
    Dag<Step>   (aegon-types)
         │
         ├──► SqliteStore  (aegon-db) ──► local index
         │
         ├──► DiskBackup   (aegon-db) ──► periodic snapshot
         │
         └──► UI layer     (aegon-ui)
                  ├──► ratatui terminal dashboard
                  └──► HTTP / web UI (LangSmith-style)
```

---

## What Aegon Tracks

From the raw JSONL, Aegon captures:

| Signal | Source |
|--------|--------|
| Tool calls (name, input, timing) | `tool_use` events |
| Tool results (output, success/error) | `tool_result` events |
| Assistant messages | `assistant` turns |
| Token usage (input, output, cache) | `usage` fields |
| Context window state | Summarised in session metadata |
| Run boundaries (start, end, abort) | Session lifecycle events |
| Step causality (what triggered what) | Event ordering + parent IDs |
| Plan mode transitions | `attachment` / `PlanModeInfo` records |
| PR links | `attachment` / `PrLink` records |
| Registered tools | `attachment` / `RegisteredTools` records |
| Todo list state | `attachment` / `TodoList` records |
| Hook output | `attachment` / `HookOutputData` records |
| Transient errors + retry counts | `system` records with `retry_attempt` field |
| Background task completions | `BackgroundTaskResult` events |
| File snapshot / edit history | `FileHistorySnapshot` / `FileEdited` events |

---

## Directory Structure

```
aegon/
├── .claude/
│   ├── agents/
│   │   ├── ci-debug.md         Sub-agent: full CI failure reproduction logic
│   │   ├── unit-testing.md     Sub-agent: diff-aware batch test writing and fix loop
│   │   ├── code-reviewer.md    Sub-agent: correctness + simplification review
│   │   └── documentation.md    Sub-agent: CHANGELOG, JOURNAL, README, CLAUDE.md updates
│   └── skills/
│       ├── clean-code/     Rust SOLID principles + feature-driven design + docs rules
│       ├── unit-testing/   Trigger/playbook — delegates implementation to agents/unit-testing.md
│       ├── lint-check/     fmt + clippy + check — mirrors CI
│       ├── ci-debug/       Trigger/playbook — delegates implementation to agents/ci-debug.md
│       └── git-worflows/   Orchestrator: branch → code → test → lint → commit → PR → merge
├── .github/
│   └── workflows/
│       ├── ci.yml          On push/PR: fmt-check, clippy, check, test, docs
│       └── release.yml     On version tag: validate, cross-compile, GitHub release
├── aegon-types/
├── aegon-core/
├── aegon-adapters/
│   ├── src/claude/         Claude Code JSONL adapter
│   ├── src/gemini/         Gemini adapter (future)
│   └── src/openai/         OpenAI adapter (future)
├── aegon-db/
├── aegon-cli/
├── aegon-ui/
├── aegon-proxy/
│   └── src/
│       ├── ca.rs           Local CA + per-hostname leaf cert generation (rcgen)
│       ├── sse.rs          Stateful SSE parser (raw bytes → SseEvent)
│       ├── anthropic.rs    Anthropic SSE decoder (SseEvent → Option<LogEvent>)
│       ├── tunnel.rs       CONNECT handler: TLS MitM for api.anthropic.com
│       └── lib.rs          Public serve(port, tx) entry point
├── aegon-mcp/
├── aegon-tests/
│   └── fixtures/
│       ├── claude/         Real Claude JSONL samples
│       ├── gemini/         Real Gemini JSONL samples (future)
│       └── openai/         Real OpenAI JSONL samples (future)
├── Cargo.toml              Workspace root
├── Justfile                Task runner — use `just --list` to see all targets
└── CLAUDE.md               This file
```

---

## Development Workflow

Follow the `/git-workflows` skill. In short:

```
just branch feat/<name>
    → apply /clean-code before writing
    → write features
    → /unit-testing   (batch test the diff)
    → /lint-check     (just ci-lint)
    → /documentation  (update CHANGELOG.md, JOURNAL.md, README.md, CLAUDE.md)
    → just commit "feat(<crate>): ..."
    → just push
    → just pr
    → CI runs automatically
    → just merge
    → just cleanup <branch>
```

If CI fails: `/ci-debug` — or run `just ci` locally to reproduce exactly.

---

## Skills

| Skill | When to use |
|-------|------------|
| `/clean-code` | Before writing: design types, modules, traits |
| `/unit-testing` | After writing a batch: diff-aware test loop (delegates to `agents/unit-testing.md`) |
| `/lint-check` | After tests pass: fmt + clippy + check |
| `/ci-debug` | CI fails and you can't reproduce locally (delegates to `agents/ci-debug.md`) |
| `/documentation` | Before committing: update CHANGELOG, JOURNAL, README, CLAUDE.md (delegates to `agents/documentation.md`) |
| `/git-workflows` | All branching, commit, PR, merge operations |

---

## Design Constraints

- `aegon-types` has zero dependencies except `serde`, `thiserror`, and `uuid`/`chrono` for IDs
  and timestamps. Never add logic or I/O here.
- `aegon-core` has no knowledge of any specific tool format. It only receives `Vec<Event>`.
- `aegon-adapters` is the only layer that knows what Claude/Gemini/OpenAI JSONL looks like.
  Never parse tool-specific formats in `aegon-core`, `aegon-db`, or `aegon-cli`.
- Use `anyhow` only in `aegon-cli` and `aegon-ui` (binary crates). All library crates use
  typed errors via `thiserror`.
- No `unwrap()` or `expect()` outside of tests and truly unreachable paths.
- Every `pub` item must have a doc comment explaining its purpose — see `/clean-code`.
- Adapters must never use `#[serde(deny_unknown_fields)]` on top-level event types — tool
  JSONL formats evolve without notice and the parser must not break on new fields.

---

## Changelog and Journal

**`CHANGELOG.md`** and **`JOURNAL.md`** must be kept up to date as work lands on `main`.

### CHANGELOG.md
- Follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.
- Every PR that adds, changes, fixes, or removes user-facing behaviour must include a
  `CHANGELOG.md` entry under `[Unreleased]`.
- On release, `[Unreleased]` is renamed to `[x.y.z] — YYYY-MM-DD`.
- Entries are grouped: `Added`, `Changed`, `Fixed`, `Removed`.

### JOURNAL.md
- Free-form running log of decisions, discoveries, and context that won't fit in a commit
  message or changelog entry.
- Each entry is dated `YYYY-MM-DD` and has a short title.
- Must include at minimum: **achievements** (what shipped), **caveats** (known gaps or
  limitations), and **next steps** (priority-ordered follow-up work).
- New entries go at the top (newest first).
