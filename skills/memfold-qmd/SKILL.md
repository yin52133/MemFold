---
name: memfold-qmd
description: Use when Codex needs to initialize or synchronize the QMD retrieval sidecar, especially for embedding-backed memory search. Trigger when the user says qmd, embeddings, retrieval index, initialize memory model, or sync the memory index.
---

# MemFold QMD

Use MemFold QMD against the global deployment at:

```bash
~/.codex/memfold
```

## First-time embedding setup

Recommended default:

```bash
memfold --root ~/.codex/memfold qmd init-model --model multilingual-e5-small
```

Other supported models:

- `bge-small-zh-v1.5`
- `bge-m3`
- `mock-test` (tests only)

This setup writes config to `~/.codex/memfold/qmd/config/model.json` and downloads the local model into `~/.codex/memfold/qmd/models`. It does not require Ollama or LM Studio.

## Sync

```bash
memfold --root ~/.codex/memfold qmd sync --scope-type project --scope-id <slug>
```
