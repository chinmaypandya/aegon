# Aegon

Observability for Claude Code agentic runs.

Watches the JSONL session files Claude Code writes during a run, parses them into typed Rust
structs, tracks live session state, and displays everything in a split terminal UI — a raw
event feed on the left and an aggregated dashboard on the right.

---

## What it looks like

```
┌─ Aegon   1 session   247 events ──────────────────────────────────────────────────────────┐
│                                                                                             │
│  Events (newest first)          │  Session                                                 │
│  ─────────────────────          │  Live session  ● LIVE   247 events                      │
│  14:22:01 TOOL▶ Bash cargo…     │  ─────────────────────────────────────────────────────  │
│  14:22:00 THINK I should…       │  Tokens  ████████████░░░░░░░░░░  42k / 200k             │
│  14:21:59 ASST  Let me run…     │  in=38420  out=3801  cache_r=71204                      │
│  14:21:55 TOOL◀ Finished `d…    │  est. cost: $0.0321                                     │
│  14:21:54 TOOL▶ Read src/…      │  ─────────────────────────────────────────────────────  │
│  14:21:54 TOOL▶ Bash ls -la…    │  Steps                                                  │
│  14:21:53 ASST  I'll check…     │  ⠋ Bash          [1.2s]                                 │
│  14:21:52 USER  lets start…     │  ✓ Read          312ms                                  │
│                                 │  ✓ Write         89ms                                   │
│                                 │  ─────────────────────────────────────────────────────  │
│                                 │  Flow                                                    │
│                                 │  USER → THINK → [Bash ‖ Read] → ASST → Write → ASST    │
│ q quit                          │                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
```

Left panel — **raw event feed**: every tool call, result, thinking block, and assistant message
in arrival order, coloured by type, newest first.

Right panel — **live dashboard**:
- **Session header**: title, live/idle status, event count
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
| `aegon-types` | All domain types — `LogEvent`, `EventKind`, `ToolCall`, `ToolResult`, `TokenUsage`, `Session`, `StreamId` |
| `aegon-adapters` | Claude JSONL → `Vec<LogEvent>`; lenient on unknown fields |
| `aegon-core` | Pure session logic — `SessionState`, `SessionRegistry`, `Step`, `FlowNode`, `TokenTotals` |
| `aegon-ui` | ratatui TUI — split event feed + dashboard |
| `aegon-cli` | Binary entry point — file watcher + channel wiring |

---

## Running

```bash
cargo run -p aegon-cli
# or build first:
cargo build --workspace
./target/debug/aegon
```

Press `q` or `Esc` to quit.

The watcher automatically picks up `~/.claude/projects/**/*.jsonl` and
`~/.claude/sessions/**/*.jsonl`. Start a Claude Code session in another
terminal — events appear in real time.

---

## Development

```bash
just --list          # all available recipes
just ci              # mirrors what CI runs (fmt + clippy + check + test + docs)
just build           # cargo build --workspace
just test            # cargo test --workspace
```

See [CLAUDE.md](CLAUDE.md) for full design constraints, crate responsibilities,
and the development workflow.

See [CHANGELOG.md](CHANGELOG.md) for release history and [JOURNAL.md](JOURNAL.md)
for design decisions and session notes.
