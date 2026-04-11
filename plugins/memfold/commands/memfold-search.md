# /memfold-search

Search prior Codex memory through MemFold.

## Arguments

- `intent`: `continue` or `knowledge_lookup`
- `query`: search query
- `scope_type`: `user` or `project` (default: `project`)
- `scope_id`: current project slug by default
- `budget`: token budget (default: `400`)

## Workflow

Run:

```bash
memfold --root ~/.codex/memfold search --scope-type <scope_type> --scope-id <scope_id> --intent <intent> --query "<query>" --budget <budget>
```

Return the result JSON and summarize the top matches.
