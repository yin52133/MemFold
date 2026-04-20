---
name: memfold-forget
description: Use when Claude Code should reject a wrong memory claim. Trigger when the user says this is wrong, ignore that memory, do not remember this, or reject this claim.
---

# MemFold Forget

```bash
memfold --root ~/.claude/memfold feedback \
  --scope-type <user|project> \
  --scope-id <scope-id> \
  --claim-fingerprint <cfp> \
  --verdict rejected \
  --reason "<reason>" \
  [--session-id <session-id>]
```
