# /dream

Run MemFold dreaming explicitly.

## Arguments

- `scope_type`: `user` or `project` (default: `project`)
- `scope_id`: current project slug by default
- `trigger`: `manual` or `scheduled` (default: `manual`)
- `mode`: `run` or `maybe-run` (default: `run`)

## Workflow

1. Resolve the current scope.
2. If `mode=maybe-run`, run:

```bash
memfold --root ~/.codex/memfold dream maybe-run --scope-type <scope_type> --scope-id <scope_id>
```

3. Otherwise run:

```bash
memfold --root ~/.codex/memfold dream run --scope-type <scope_type> --scope-id <scope_id> --trigger <trigger>
```

4. Summarize:
- whether dreaming ran
- promoted / discarded counts
- whether bundle and qmd were refreshed
