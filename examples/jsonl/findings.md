# Claude Code JSONL — Discovery Findings

Produced by running `audit.py` across all `~/.claude/projects/**/*.jsonl` files
(2 348 records across 6 projects, May 2026).

---

## Record types

| type | count | what it is |
|------|------:|-----------|
| `assistant` | 908 | Model turn — text, tool calls, thinking, usage |
| `user` | 589 | Human turn — text input or tool results |
| `file-history-snapshot` | 239 | Backup of file state before an edit |
| `ai-title` | 175 | Auto-generated session title |
| `queue-operation` | 164 | Session queue bookkeeping (enqueue/dequeue) |
| `attachment` | 133 | Deferred-tools delta — which tools became available |
| `last-prompt` | 127 | Pointer to the last user message in the session |
| `mode` | 13 | Session mode change (`normal` / `auto`) |

---

## Top-level keys per record type

### `assistant`
```
parentUuid, uuid, isSidechain, type, timestamp, sessionId,
message, agentId, attributionAgent, attributionSkill,
cwd, entrypoint, gitBranch, requestId, slug, userType, version
```

### `user`
```
parentUuid, uuid, isSidechain, type, timestamp, sessionId, promptId,
message, toolUseResult, sourceToolAssistantUUID, sourceToolUseID,
agentId, cwd, entrypoint, gitBranch, isMeta, permissionMode,
slug, userType, version
```

### `queue-operation`
```
type, operation, timestamp, sessionId, content
```

### `file-history-snapshot`
```
type, messageId, snapshot, isSnapshotUpdate
```

### `attachment`
```
type, parentUuid, uuid, isSidechain, timestamp, sessionId,
attachment (object with type, addedNames, addedLines, removedNames, removedLines),
agentId, cwd, entrypoint, gitBranch, slug, userType, version
```

### `ai-title` / `last-prompt` / `mode`
```
type, sessionId
+ aiTitle  (ai-title)
+ lastPrompt, leafUuid  (last-prompt)
+ mode  (mode)
```

---

## message.content[] types

| type | count | what it is |
|------|------:|-----------|
| `tool_use` | 500 | Tool invocation (id, name, input JSON) |
| `tool_result` | 499 | Tool output (tool_use_id, content, is_error) |
| `text` | 425 | Plain text from model or user |
| `thinking` | 136 | Extended thinking block (thinking, signature) |

---

## Tool names seen in `tool_use`

| name | count |
|------|------:|
| Bash | 134 |
| Write | 132 |
| Read | 102 |
| Edit | 78 |
| TodoWrite | 26 |
| WebFetch | 12 |
| Skill | 6 |
| ToolSearch | 5 |
| ExitPlanMode | 2 |
| AskUserQuestion | 2 |
| Agent | 1 |

---

## `usage` object — all keys observed

```
input_tokens
output_tokens
cache_read_input_tokens
cache_creation_input_tokens
cache_creation              ← alternate key, same concept
server_tool_use             ← nested: {web_search_requests, web_fetch_requests}
service_tier                ← "standard" / "priority" etc.
speed                       ← "fast" / "normal"
iterations                  ← loop iteration count (auto-mode)
inference_geo               ← geography of inference
```

---

## `tool_result` content shapes

The `content` field on a tool_result block is **not always a string**:

| shape | example |
|-------|---------|
| `str` | `"file.txt\ndir/"` |
| `list` | `[{"type": "text", "text": "..."}]` |
| `null` | empty result (tool returned nothing) |

---

## What the current adapter handles vs. gaps

### ✅ Handled

| Signal | Maps to |
|--------|---------|
| `assistant` + `text` content | `EventKind::AssistantMessage` |
| `assistant` + `tool_use` content | `EventKind::ToolCall` |
| `assistant` + `usage.input/output_tokens` | `TokenUsage` on `AssistantMessage` |
| `user` + `text` content | `EventKind::UserMessage` |
| `user` + `tool_result` content (str or list) | `EventKind::ToolResult` |
| `queue-operation` | silently skipped — correct |

### ❌ Gaps

| Signal | Current behaviour | Impact |
|--------|------------------|--------|
| `thinking` content blocks | silently skipped | Extended thinking invisible in TUI |
| `isSidechain: true` records | treated same as main chain | Sub-agent runs mixed in with main run |
| `agentId` / `attributionSkill` | not captured | Can't attribute events to a skill or sub-agent |
| `usage.cache_creation` (alternate key) | ignored | Cache creation tokens undercounted |
| `usage.server_tool_use` | ignored | Web search / fetch call counts lost |
| `usage.service_tier` / `speed` | ignored | Tier and speed metadata lost |
| `attachment` (deferred-tools delta) | silently skipped | Tool availability changes not tracked |
| `file-history-snapshot` | silently skipped | File change history not linked to events |
| `ai-title` | silently skipped | Session title not surfaced in TUI |
| `toolUseResult` on `user` records | not read | Alternate tool-result path not parsed |
| `sourceToolUseID` / `sourceToolAssistantUUID` | not read | Sub-agent call linkage lost |
| `cwd`, `gitBranch`, `version` | not read | Execution context not captured |
