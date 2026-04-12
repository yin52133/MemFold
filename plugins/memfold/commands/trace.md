# /trace

Trace a memory claim back to the stored session log entry.

## Arguments

- `query`: required lookup text
- `scope_type`: optional, current shipped values are `user` or `project`
- `scope_id`: optional scope id

## Workflow

Run:

```bash
memfold --root ~/.codex/memfold trace find --query "<query>" [--scope-type <scope_type> --scope-id <scope_id>]
```

Report:
- whether a trace match was found
- the session id and line number
- the stored raw user text and summary
