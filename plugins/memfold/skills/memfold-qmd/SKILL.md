---
name: memfold-qmd
description: Use when Codex needs to initialize or synchronize the MemFold retrieval sidecar, especially for embedding-backed search. Trigger when the user says qmd, embeddings, retrieval index, memory index, or sync the memory search layer.
---

# MemFold QMD

Use the global MemFold deployment:

```bash
~/.codex/memfold
```

## First-time embedding model setup

Recommended default:

```bash
memfold --root ~/.codex/memfold qmd init-model --model multilingual-e5-small
```

Other supported models:

- `bge-small-zh-v1.5`
- `bge-m3`
- `mock-test` (tests only)

This setup writes:

- `~/.codex/memfold/qmd/config/model.json`
- `~/.codex/memfold/qmd/models`

No separate Ollama or LM Studio service is required.

## Sync

```bash
memfold --root ~/.codex/memfold qmd sync --scope-type project --scope-id <slug>
```
