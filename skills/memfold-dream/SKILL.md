---
name: memfold-dream
description: Use when Codex needs to explicitly consolidate memory or check whether scheduled dreaming should run. Trigger when the user says dream, consolidate memory, summarize memory, or wants to force or inspect memory promotion.
---

# MemFold Dream

Use MemFold dreaming against the global deployment at:

```bash
~/.codex/memfold
```

## Commands

Explicit run:

```bash
memfold --root ~/.codex/memfold dream run --scope-type project --scope-id <slug> --trigger manual
```

Scheduled gate check:

```bash
memfold --root ~/.codex/memfold dream maybe-run --scope-type project --scope-id <slug>
```

## Notes

- `dream maybe-run` is silent-gated.
- The current gate requires at least 5 ended sessions and at least 24 hours since the last applied dream job.
- Tombstoned and `sterile` evidence should not promote.
