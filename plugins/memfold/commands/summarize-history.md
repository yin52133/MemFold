# /summarize-history

Summarize one session log into the daily history file.

## Arguments

- `scope_type`: current shipped values are `user` or `project`
- `scope_id`: target scope id
- `session_id`: session to summarize
- `trigger`: `manual` or `session_end`

## Workflow

Run:

```bash
memfold --root ~/.codex/memfold summarize-history --scope-type <scope_type> --scope-id <scope_id> --session-id <session_id> --trigger <trigger>
```

Report:
- whether the daily history file was updated
- the summary id
- the daily history path
