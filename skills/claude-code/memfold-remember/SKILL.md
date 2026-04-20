---
name: memfold-remember
description: Use when Claude Code should mark a memory candidate as promotable. Trigger when the user explicitly says remember this, this is important, keep this preference, or keep this rule.
---

# MemFold Remember

```bash
memfold --root ~/.claude/memfold write-evidence \
  --scope-type <user|project> \
  --scope-id <scope-id> \
  --session-id <session-id> \
  --source-kind user|decision \
  --summary "<safe-summary>" \
  --promotable 1 \
  --origin-mode normal
```
