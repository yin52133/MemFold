# /memfold-remember

Write a promotable MemFold evidence record for a stable preference, rule, or constraint.

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
