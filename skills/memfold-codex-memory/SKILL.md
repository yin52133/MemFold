---
name: memfold-codex-memory
description: Use as the top-level router for Codex memory workflows backed by MemFold. Trigger when the task involves startup memory, retrieving prior work, remembering stable constraints, rejecting wrong memory, or consolidating memory.
---

# MemFold Codex Memory

This is the **routing skill** for MemFold, not the detailed command reference.

Use MemFold at:

```bash
~/.codex/memfold
```

## Choose the right sub-skill

- `memfold-search`
  Use when the user asks about previous work, past context, or background knowledge.

- `memfold-remember`
  Use when the user explicitly says to remember something, or a stable preference/constraint is clearly established.

- `memfold-forget`
  Use when the user says an existing memory is wrong and should be rejected.

- `memfold-dream`
  Use when memory should be consolidated now, or when checking whether scheduled dreaming should run.

- `memfold-qmd`
  Use when the retrieval sidecar or embedding model needs initialization or synchronization.

## Minimal rules

- Hooks are the default audit/safety net path and should usually write `promotable=0`.
- Only explicit stable memory candidates should be written with `promotable=1`.
- Do not auto-search in `sterile` mode.
- Prefer the focused MemFold sub-skills over dumping full CLI help.
