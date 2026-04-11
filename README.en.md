# MemFold

[中文](./README.md) | [English](./README.en.md)

A layered **Codex memory** framework that turns the `hook + skill/tool` pattern into a local, auditable, replayable, and offline-testable memory system.

## What It Is

MemFold addresses two opposite failure modes in long-running Codex work:

- carrying everything forward: token bloat and stale-path pollution
- remembering nothing: stable preferences and project constraints keep getting lost

Its core strategy is **layered filtering**, not “store more”.

## Key Layers

- `boot bundle`: minimal startup context
- `stable`: long-term memory truth source
- `evidence`: working-memory truth source
- `archive`: human-readable dated trace layer
- `qmd sidecar`: rebuildable retrieval index

## Highlights

- `hook + skill/tool`: hooks prevent misses, tools prevent noisy promotion
- `Rust + SQLite + local sidecar`
- Markdown/JSONL as truth source, SQLite as state/projection store
- `dreaming + tombstone` for promotion, rejection, and anti-resurrection
- `experiments` for offline rule validation
- repo-local hook rollout before global Codex deployment

## Architecture

```text
Codex Hooks / Tools
        |
        v
     memfold CLI
        |
        +-- boot / load
        +-- evidence / mutations
        +-- retrieval / qmd
        +-- feedback / dreaming / repair
        +-- experiments
        |
        +-- SQLite (state/projections/locks)
        +-- Markdown / JSONL (truth source)
        +-- QMD sidecar (discardable index)
```

## Available Commands

- `memfold init`
- `memfold load`
- `memfold write-evidence`
- `memfold search`
- `memfold qmd sync`
- `memfold bundle compile`
- `memfold feedback`
- `memfold dream run`
- `memfold repair`
- `memfold hook capture`
- `memfold experiment run`

## Basic Usage

```bash
cargo run -- init --root ./.memfold-local
```

```bash
cargo run -- load \
  --root ./.memfold-local \
  --mode normal \
  --scope-type project \
  --scope-id MemFold \
  --intent startup \
  --budget 400
```

```bash
cargo run -- experiment run --fixture tests/fixtures/experiments/passing.json
```

## Local Hook Policy

The local hook path stores only **meaningful state-change summaries**, not full raw logs:

- never store raw prompts, raw tool output, reasoning drafts, or secrets
- hooks write filtered summaries with `promotable=0`
- explicit skills/tools may write stable candidates with `promotable=1`

## Validation

```bash
cargo test
```

## Related Docs

- [AGENTS.md](./AGENTS.md)
- [Codex Integration Docs](./docs/integrations/codex/README.en.md)
- [Architecture Spec (zh-CN)](./docs/specs/2026-04-11-memfold-architecture-design.zh-CN.md)
- [Local Hook Readme (zh-CN)](./hooks/local/README.zh-CN.md)
- [Hooks Overview](./hooks/README.en.md)
- [Skills Overview](./skills/README.en.md)
- [Progress Board (zh-CN)](./docs/progress/memfold-v1/00-master-checklist.zh-CN.md)
