# /qmd

Initialize or sync MemFold QMD for the current scope.

## Arguments

- `scope_type`: `user` or `project` (default: `project`)
- `scope_id`: current project slug by default
- `mode`: `init-model` or `sync` (default: `sync`)
- `model`: embedding model name when `mode=init-model`

## Workflow

1. If `mode=init-model`, run:

```bash
memfold --root ~/.codex/memfold qmd init-model --model <model>
```

2. If `mode=sync`, run:

```bash
memfold --root ~/.codex/memfold qmd sync --scope-type <scope_type> --scope-id <scope_id>
```

3. Report:
- model status
- whether embeddings are enabled
- number of indexed records from stable memory, history summaries, and session logs
