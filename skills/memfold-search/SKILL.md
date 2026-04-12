---
name: memfold-search
description: Use when Codex needs to retrieve prior work, prior preferences, or background knowledge from MemFold. Trigger when the user references previous work, says earlier, last time, remember, or asks for background context.
---

# MemFold Search

```bash
memfold --root ~/.codex/memfold search \
  --scope-type project \
  --scope-id <slug> \
  --intent continue|knowledge_lookup \
  --query "<query>" \
  --budget 400
```

## Intent guidance

- `continue`: prior work / prior decisions / prior constraints
- `knowledge_lookup`: background or reference lookup
