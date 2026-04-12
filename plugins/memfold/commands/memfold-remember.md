# /memfold-remember

Write a promotable MemFold memory candidate record for a stable preference, rule, or constraint.

## Arguments

- `summary`: safe summary to remember
- `scope_type`: `user` or `project` (default depends on content)
- `scope_id`: current scope id
- `session_id`: current session id
- `source_kind`: usually `user` or `decision`
- `claim_fingerprint`: optional explicit fingerprint

## Workflow

Run:

```bash
memfold --root ~/.codex/memfold write-evidence --scope-type <scope_type> --scope-id <scope_id> --session-id <session_id> --source-kind <source_kind> --summary "<summary>" --promotable 1 --origin-mode normal [--claim-fingerprint <claim_fingerprint>]
```

Note:
- Current shipped host surface still uses `write-evidence`.
- Under the hood, this record is part of the session logging flow and may later promote into stable memory.
