---
name: memfold-codex-memory
description: Use for Codex memory operations backed by the globally deployed MemFold framework. Trigger when the task involves startup memory, remembering stable preferences, retrieving prior work, rejecting wrong memory, or consolidating Codex memory.
---

# MemFold Codex Memory

Use the globally deployed MemFold instance at:

```bash
~/.codex/memfold
```

## Core commands

- `memfold --root ~/.codex/memfold load --mode normal --scope-type project --scope-id <slug> --intent startup --budget 400`
- `memfold --root ~/.codex/memfold write-evidence --scope-type project --scope-id <slug> --session-id <sid> --source-kind <kind> --summary "<safe-summary>" --promotable 0|1 --origin-mode normal|fresh|sterile`
- `memfold --root ~/.codex/memfold search --scope-type project --scope-id <slug> --intent continue|knowledge_lookup --query "<query>" --budget 400`
- `memfold --root ~/.codex/memfold feedback --scope-type project --scope-id <slug> --claim-fingerprint <cfp> --verdict confirmed|disputed|rejected --reason "<reason>" [--session-id <sid>]`
- `memfold --root ~/.codex/memfold dream run --scope-type project --scope-id <slug> --trigger manual`
- `memfold --root ~/.codex/memfold dream maybe-run --scope-type project --scope-id <slug>`
- `memfold --root ~/.codex/memfold repair --scope-type project --scope-id <slug>`

## Rules

- Hooks are audit/default capture only; default to `promotable=0`
- Only mark `promotable=1` when the user explicitly asks to remember something or when a stable preference/constraint is clearly established
- Use `feedback` when the user says an existing claim is wrong
- Do not auto-search in `sterile` mode
