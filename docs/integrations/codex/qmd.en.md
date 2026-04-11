# Codex QMD Guide

[English](./qmd.en.md) | [中文](./qmd.zh-CN.md)

## 1. Goal

Explain MemFold QMD deployment for Codex, first-time embedding model setup, and ongoing indexing behavior.

## 2. Current QMD Shape

QMD is a sidecar, not a content truth source.

It is responsible for:

- indexing `stable / evidence / archive`
- returning a `pointer` back to the truth source
- using vector similarity to enhance retrieval when embeddings are enabled

## 3. First-time Embedding Setup

Run this first:

```bash
memfold --root ~/.codex/memfold qmd init-model --model multilingual-e5-small
```

This will:

- create `~/.codex/memfold/qmd/config/model.json`
- download the embedding model into `~/.codex/memfold/qmd/models`
- not require a separate Ollama or LM Studio service

## 4. Recommended Models

- `multilingual-e5-small`
  - recommended default
  - good for mixed Chinese/English retrieval

Optional:

- `bge-small-zh-v1.5`
- `bge-m3`

Test-only:

- `mock-test`

## 5. Indexing

```bash
memfold --root ~/.codex/memfold qmd sync --scope-type project --scope-id my-project
```

It syncs:

- `stable`
- `evidence`
- `archive`

## 6. Retrieval

```bash
memfold --root ~/.codex/memfold search \
  --scope-type project \
  --scope-id my-project \
  --intent continue \
  --query "中文回答" \
  --budget 400
```

Ordering:

- `continue`: stable > evidence > archive
- `knowledge_lookup`: stable > archive > evidence

When embeddings are enabled:

- search uses embedding + lexical scoring

When embeddings are not enabled:

- search falls back to lexical retrieval

## 7. Slash Command

Plugin command:

```text
/qmd
```

Supported modes:

- `mode=init-model`
- `mode=sync`

## 8. Debugging

Inspect config:

```bash
cat ~/.codex/memfold/qmd/config/model.json
```

Inspect collections:

```bash
find ~/.codex/memfold/qmd/collections -type f | sort
```

## 9. FAQ

### Why is the first run slower?

Because the embedding model is downloaded on first initialization.

### Why is Ollama or LM Studio not required?

Because the current implementation uses an embedded local embedding provider, not an external inference service.
