# Aegon

Observability for Claude Code agentic runs.

Watches the JSONL session files Claude Code writes during a run, parses them into typed Rust
structs, tracks live session state, and displays everything in a split terminal UI — a raw
event feed on the left and an aggregated dashboard on the right.

---

# Demo

https://github.com/user-attachments/assets/a34fd65b-3872-40b2-a0a4-ef012dcf0c0a

---

## What it looks like

```
┌─ Aegon   1 session   247 events ──────────────────────────────────────────────────────────┐
│                                                                                             │
│  Events (newest first)          │  Session                                                 │
│  ─────────────────────          │  Live session  ● LIVE  [plan]  PR#42  247 events        │
│  14:22:01 TOOL▶ Bash cargo…     │  2 retries  3 bg tasks                                  │
│  14:22:01 PLAN+ entering plan…  │  ─────────────────────────────────────────────────────  │
│  14:22:00 THINK I should…       │  Tokens  ████████████░░░░░░░░░░  42k / 200k             │
│  14:21:59 ASST  Let me run…     │  in=38420  out=3801  cache_r=71204                      │
│  14:21:58 ERR   connect (r=2/3) │  est. cost: $0.0321                                     │
│  14:21:55 TOOL◀ Finished `d…    │  ─────────────────────────────────────────────────────  │
│  14:21:54 TOOL▶ Read src/…      │  Steps                                                  │
│  14:21:54 TOOL▶ Bash ls -la…    │  ⠋ Bash          [1.2s]                                 │
│  14:21:53 ASST  I'll check…     │  ✓ Read          312ms                                  │
│  14:21:52 USER  lets start…     │  ✓ Write         89ms                                   │
│                                 │  ─────────────────────────────────────────────────────  │
│                                 │  Flow                                                    │
│                                 │  USER → THINK → [Bash ‖ Read] → ASST → Write → ASST    │
│ q quit                          │                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
```

Left panel — **raw event feed**: every tool call, result, thinking block, assistant message,
plan mode transition, hook output, todo update, and more — in arrival order, coloured by type,
newest first. All 20+ event kinds are labeled and coloured.

Right panel — **live dashboard**:
- **Session header**: title, live/idle status, `[plan]` badge when in plan mode, linked PR,
  retry count, background task count, total event count
- **Token gauge**: progress bar auto-filling as the context window fills; estimated cost
- **Steps**: spinner on active tool calls, ✓/✗ for completed/failed, with timing
- **Flow**: one-line causal chain showing order, parallel branches (`[Bash ‖ Read]`), and
  human turns

---

## Architecture

```
~/.claude/projects/**/*.jsonl
         │
         │  notify (OS file events: FSEvents / inotify)
         ▼
  aegon-cli / watcher.rs
  ┌─ background thread ──────────────────────────────────┐
  │  seek to last-read byte → read new lines              │
  │  ClaudeAdapter::parse_line() → Vec<LogEvent>          │
  │  tx.send(event)                                       │
  └──────────────────────────────────────────────────────┘
         │  mpsc channel
         ▼
  aegon-cli / main.rs  (main thread)
  App::push(event)
    ├─► events: Vec<LogEvent>        raw feed buffer (500 events)
    └─► SessionRegistry::ingest()
          └─► SessionState::ingest()
                ├─ pending: HashMap<id, Step>   running tool calls
                ├─ steps: Vec<Step>             completed with timing
                ├─ flow: Vec<FlowNode>          causal chain
                └─ token_totals: TokenTotals    cumulative usage

  aegon-ui / tui.rs  draws every 50ms:
    left  → raw event list
    right → dashboard::draw(SessionState)
              ├─ gauge::draw()   token progress bar + cost
              ├─ steps::draw()   spinners + completed steps
              └─ flow::draw()    USER → THINK → [T1 ‖ T2] → ASST
```

### Crate map

| Crate | Role |
|-------|------|
| `aegon-types` | All domain types — `LogEvent`, `EventKind` (20+ variants), `ToolCall`, `ToolResult`, `TokenUsage`, `Session`, `StreamId` |
| `aegon-adapters` | Claude JSONL → `Vec<LogEvent>`; lenient on unknown fields |
| `aegon-core` | Pure session logic — `SessionState`, `SessionRegistry`, `Step`, `FlowNode`, `TokenTotals` |
| `aegon-ui` | ratatui TUI — split event feed + dashboard |
| `aegon-cli` | Binary entry point — file watcher + channel wiring |

---

## Install

**Via pip** (no Rust toolchain needed):

```bash
pip install aegon-rs
```

**Via Cargo** (Rust toolchain required):

```bash
cargo install aegon-cli
```

---

## Running

```bash
aegon run              # TUI in the current terminal
aegon run --detached   # open TUI in a new terminal window
aegon                  # same as aegon run
```

Press `q` or `Esc` to quit.

The watcher automatically picks up `~/.claude/projects/**/*.jsonl` and
`~/.claude/sessions/**/*.jsonl`. Start a Claude Code session in another
terminal — events appear in real time.

### Proxy mode (mid-turn token streaming)

```bash
aegon run --proxy   # starts the JSONL watcher + HTTPS MitM proxy on port 8877
```

Set `HTTPS_PROXY=http://127.0.0.1:8877` (and optionally `NODE_EXTRA_CA_CERTS` pointing to
Aegon's local CA cert) before launching Claude Code. The proxy intercepts the Anthropic API
SSE stream and feeds live `TKN▸` / `THNK▸` token chunks into the event feed as they arrive,
giving real mid-turn visibility before a turn is written to the JSONL log.

**From source** (contributors):

```bash
just run          # build + launch in one step
just demo         # opens in a new Terminal window
just setup        # first-time: installs system deps and Cargo tools
```

---

## Development

```bash
just --list          # all available recipes
just ci              # mirrors what CI runs (fmt + clippy + check + test + docs)
just build           # cargo build --workspace
just test            # cargo test --workspace
just audit-jsonl     # scan ~/.claude/projects for JSONL structure insights
just demo            # run the tool in a separate terminal for live demo
```

See [CLAUDE.md](CLAUDE.md) for full design constraints, crate responsibilities,
and the development workflow.

See [CHANGELOG.md](CHANGELOG.md) for release history and [JOURNAL.md](JOURNAL.md)
for design decisions and session notes.

---

## Contributing

### Workflow

All changes go through a branch → PR → squash-merge cycle. The full
loop is documented in [CLAUDE.md](CLAUDE.md) and enforced by skills in
`.claude/skills/`. In short:

```bash
just branch feat/<name>     # cut a branch from latest main
# ... make changes ...
just ci                     # must pass locally before pushing
just push                   # push and set upstream
just pr                     # open PR (gh will prompt for title + body)
just merge                  # squash-merge once CI is green
just cleanup feat/<name>    # delete local branch + prune remote refs
```

### Rules

- **Never commit to `main` directly.** Every change goes through a PR.
- **`just ci` must pass** before pushing. It runs `fmt`, `clippy -D warnings`,
  `check`, `test`, and `cargo doc -D warnings` — exactly what CI runs.
- **Every public item needs a doc comment.** `cargo doc -D warnings` is an
  error, not a warning.
- **CHANGELOG.md and JOURNAL.md** must be updated on every PR that changes
  behaviour. `[Unreleased]` in CHANGELOG for unreleased changes; add a dated
  JOURNAL entry with achievements, caveats, and next steps.
- **No `unwrap()` or `expect()`** in library crates outside tests.
- **Adapters must not use `#[serde(deny_unknown_fields)]`** on top-level event
  types — the Claude JSONL format evolves without notice.

### Adding a new tool adapter

1. Create `crates/aegon-adapters/src/<tool>/mod.rs` and `raw.rs`
2. Implement the `Adapter` trait from `crates/aegon-adapters/src/lib.rs`
3. Add fixture JSONL samples to `aegon-tests/fixtures/<tool>/`
4. Wire into `aegon-cli` via config or format detection

### Codeowners

`@chinmaypandya` is the default owner for all files. All PRs require
approval before merge.
