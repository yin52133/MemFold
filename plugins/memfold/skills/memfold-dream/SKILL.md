---
name: memfold-dream
description: Use when Codex needs to explicitly consolidate memory or inspect whether scheduled dreaming should run. Trigger when the user says dream, consolidate memory, summarize memory, or wants to force or inspect memory promotion.
---

# MemFold Dream

Run MemFold dreaming against the global deployment:

```bash
memfold --root ~/.codex/memfold dream run --scope-type project --scope-id <slug> --trigger manual
```

For silent-gated background eligibility:

```bash
memfold --root ~/.codex/memfold dream maybe-run --scope-type project --scope-id <slug>
```

## Notes

- `dream run` is the explicit operator path.
- `dream maybe-run` uses the built-in gate:
  - at least 5 ended sessions since the last applied dream job
  - at least 24 hours since the last applied dream job
- Tombstoned, sterile, and analysis-draft style candidates should not promote.
