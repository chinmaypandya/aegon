#!/usr/bin/env python3
"""
Audit all Claude Code JSONL session files and report:
- Every record type and the keys it carries
- Every message.content[].type value
- Every tool name seen in tool_use blocks
- All usage field keys observed
- Shapes of tool_result content (str vs list)
- Sidechain and sub-agent record counts

Run from the repo root:
    python3 examples/jsonl/audit.py
"""

import json
import collections
import os
import glob

record_types: collections.Counter = collections.Counter()
content_types: collections.Counter = collections.Counter()
top_level_keys: dict = collections.defaultdict(set)
tool_names: collections.Counter = collections.Counter()
usage_fields: set = set()
tool_result_content_shapes: set = set()
sidechain_count = 0
total = 0

pattern = os.path.expanduser("~/.claude/projects/**/*.jsonl")

for path in glob.glob(pattern, recursive=True):
    for raw in open(path, errors="ignore"):
        raw = raw.strip()
        if not raw:
            continue
        try:
            d = json.loads(raw)
        except json.JSONDecodeError:
            continue

        total += 1
        rtype = d.get("type", "?")
        record_types[rtype] += 1

        for k in d:
            top_level_keys[rtype].add(k)

        if d.get("isSidechain"):
            sidechain_count += 1

        msg = d.get("message") or {}
        for item in msg.get("content") or []:
            if not isinstance(item, dict):
                continue
            ct = item.get("type", "?")
            content_types[ct] += 1
            if ct == "tool_use":
                tool_names[item.get("name", "?")] += 1
            if ct == "tool_result":
                tool_result_content_shapes.add(type(item.get("content")).__name__)

        if "usage" in msg:
            usage_fields.update(msg["usage"].keys())

print(f"Total records scanned : {total}")
print(f"Sidechain records     : {sidechain_count}")

print("\n── Record types ────────────────────────────────")
for k, v in record_types.most_common():
    print(f"  {v:>6}  {k}")

print("\n── Top-level keys per record type ──────────────")
for rtype, keys in sorted(top_level_keys.items()):
    print(f"  {rtype}")
    for k in sorted(keys):
        print(f"    {k}")

print("\n── message.content[].type ──────────────────────")
for k, v in content_types.most_common():
    print(f"  {v:>6}  {k}")

print("\n── Tool names (tool_use) ───────────────────────")
for k, v in tool_names.most_common():
    print(f"  {v:>6}  {k}")

print("\n── usage object keys observed ──────────────────")
for k in sorted(usage_fields):
    print(f"  {k}")

print("\n── tool_result content shapes ──────────────────")
for s in sorted(tool_result_content_shapes):
    print(f"  {s}")
