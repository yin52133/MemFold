# /memfold-forget

Reject an existing memory claim through MemFold feedback.

## Arguments

- `scope_type`: `user` or `project`
- `scope_id`: scope id
- `claim_fingerprint`: claim fingerprint to reject
- `reason`: why it is wrong
- `session_id`: optional current session id

## Workflow

Run:

```bash
memfold --root ~/.codex/memfold feedback --scope-type <scope_type> --scope-id <scope_id> --claim-fingerprint <claim_fingerprint> --verdict rejected --reason "<reason>" [--session-id <session_id>]
```
