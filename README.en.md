# MemFold

[English](./README.md) | [中文](./README.zh-CN.md)

MemFold is a **Codex / Claude Code memory framework** that combines `hooks + skills/tools + qmd + dreaming` into a local, auditable, replayable, and offline-testable memory system, instead of dumping raw history back into prompt context.

## Repository Focus

This is not a generic chat-memory demo. It is specifically built for **Codex and Claude Code memory orchestration**:

- Session startup, work-in-progress capture, and exit handling for both hosts
- Host-specific hooks, skills, and global deployment (separate storage per host)
- startup bundle injection, evidence capture, dreaming consolidation, and QMD retrieval

## Supported Hosts

| Host | Storage Root | Hook Protocol | Deploy Script |
|------|-------------|---------------|---------------|
| Codex | `~/.codex/memfold/` | Environment variables | `scripts/deploy_codex_global.sh` |
| Claude Code | `~/.claude/memfold/` | stdin JSON | `scripts/deploy_claude_global.sh` |

Memory is **fully isolated** between hosts — each has its own memory, state, QMD, and runtime directories.

## Why It Exists

Agent memory usually fails in two opposite ways:

- carry everything forward: token bloat and stale-path contamination
- remember nothing: stable preferences and project constraints keep getting lost

MemFold solves that with **layered filtering**, not by “storing more”.

## Highlights

- `hook + skill/tool`: hooks prevent misses, tools prevent noisy promotion
- `five-layer memory design`: boot / stable / evidence / wiki / archive
- `truth-source first`: Markdown/JSONL are the content truth source; SQLite stores state and projections
- `dreaming + tombstone`: promotion, rejection, quarantine, and anti-resurrection
- `QMD sidecar`: rebuildable retrieval index that can evolve independently
- `embedding-ready qmd`: local embedding model initialization and first-run model download
- `experiments / autoresearch`: fixture-driven offline validation instead of production trial-and-error
- `repo-canonical integration`: `hooks/`, `skills/`, and `plugins/` are all Git-managed sources

## Five Layers

```text
Layer 1  boot bundle
  minimal stable startup context

Layer 2  stable
  long-term memory truth source

Layer 3  evidence
  working-memory truth source

Layer 4  wiki
  background knowledge layer

Layer 5  archive
  dated human-readable trace layer
```

Default load order:

```text
boot -> stable -> evidence -> wiki -> archive
```

Knowledge lookup order:

```text
boot -> stable -> wiki -> archive -> evidence
```

## Core Workflow

```text
Session start (Codex or Claude Code)
  -> hook/session_start
  -> memfold init
  -> memfold load
  -> compensate with dream maybe-run
  -> qmd sync
  -> inject minimal bundle

During work
  -> hook/turn_end (Codex) or hook/Stop (Claude Code) captures filtered summaries
  -> skills/tools explicitly call write-evidence/search/feedback

Session end
  -> hook/session_end writes final session summary
  -> background dream maybe-run (best-effort)
  -> double gate: >=24h since last dream && >=5 ended sessions

Dreaming
  -> read promotable evidence
  -> filter tombstoned / sterile / analysis-draft candidates
  -> promote / hold / discard
  -> rebuild bundle
  -> qmd sync
```

## QMD and Embedding Models

QMD is an indexing sidecar, not a content truth source.

Two levels are supported:

1. base sidecar
- indexes `stable / session_log / history`
- falls back to lexical retrieval

2. embedding sidecar
- initialized with `memfold qmd init-model`
- first run downloads a local embedding model into:

```bash
# Codex
~/.codex/memfold/qmd/models
# Claude Code
~/.claude/memfold/qmd/models
```

- does **not** require a separate Ollama or LM Studio service
- writes config to:

```bash
# Codex
~/.codex/memfold/qmd/config/model.json
# Claude Code
~/.claude/memfold/qmd/config/model.json
```

Recommended default model:

- `multilingual-e5-small`

Other supported models:

- `bge-small-zh-v1.5`
- `bge-m3`

Example:

```bash
memfold --root ~/.codex/memfold qmd init-model --model multilingual-e5-small
memfold --root ~/.codex/memfold qmd sync --scope-type project --scope-id my-project
```

More:

- [Codex QMD Guide](./docs/integrations/codex/qmd.en.md)

## Repository Layout

```text
src/                         core engine
hooks/                       canonical hook source
  local/                     repo-local self-test hooks
  codex-global/              global Codex deployment hooks
  claude-code/               global Claude Code deployment hooks
skills/                      canonical skill source
  claude-code/               Claude Code skills (SKILL.md with frontmatter)
plugins/                     Codex plugin / slash command source
docs/integrations/codex/     Codex deployment and integration docs
docs/integrations/claude-code/  Claude Code deployment and integration docs
scripts/                     deployment / verification scripts
tests/                       regression and E2E coverage
```

## Available Commands

- `memfold init`
- `memfold load`
- `memfold write-evidence`
- `memfold search`
- `memfold qmd init-model`
- `memfold qmd sync`
- `memfold bundle compile`
- `memfold feedback`
- `memfold dream run`
- `memfold dream maybe-run`
- `memfold repair`
- `memfold hook capture`
- `memfold experiment run`

## /dream and Plugin Commands

Plugin command sources already exist in the repo:

- `plugins/memfold/commands/dream.md`
- `plugins/memfold/commands/qmd.md`
- `plugins/memfold/commands/memfold-search.md`
- `plugins/memfold/commands/memfold-remember.md`
- `plugins/memfold/commands/memfold-forget.md`

`/dream` supports:

- `scope_type`
- `scope_id`
- `trigger=manual|scheduled`
- `mode=run|maybe-run`

Where:

- `run` means explicit immediate consolidation
- `maybe-run` means silent gated consolidation

## Hook Policy

Only **state-changing summaries** should go into memory by default:

- never store raw prompts
- never store raw tool output
- never store reasoning drafts
- never store secrets
- hooks write filtered summaries with `promotable=0`
- skills/tools explicitly write stable candidates with `promotable=1`

## Global Deployment

### Codex

Global target:

```bash
~/.codex/memfold
```

Standard install / update / redeploy entrypoint:

```bash
./scripts/deploy_codex_global.sh
```

Standalone health check:

```bash
./scripts/verify_codex_global.sh
```

Launcher:

```bash
cdx-memfold
```

### Claude Code

Global target:

```bash
~/.claude/memfold
```

Standard install / update / redeploy entrypoint:

```bash
./scripts/deploy_claude_global.sh
```

Standalone health check:

```bash
./scripts/verify_claude_global.sh
```

### Deployment Behavior

- deploy rebuilds the binary, refreshes launcher / hooks / plugin source, clears plugin cache, runs QMD sync, and then runs verify
- deploy only runs `memfold repair` when verify reports repairable drift
- `cdx-memfold` runs `session_start` before Codex starts and best-effort runs `session_end` on `EXIT / ctrl+c / TERM / HUP`
- startup now compensates with `dream maybe-run` and `qmd sync` for work that may have been missed at exit time
- shipped `turn_end.sh` is refreshed by deploy, but current host-side per-turn attachment is still separate from plugin installation
- plugin installation alone is not the full deployment path

## Memory Migration

Migrate memory between Codex and Claude Code:

```bash
./scripts/migrate_memory.sh codex-to-claude              # Codex → Claude Code
./scripts/migrate_memory.sh claude-to-codex              # Claude Code → Codex
./scripts/migrate_memory.sh sync                          # Bidirectional
./scripts/migrate_memory.sh codex-to-claude --scope-id X  # Specific scope
./scripts/migrate_memory.sh codex-to-claude --dry-run     # Preview only
```

Migrates `stable`, `sessions`, `history`. Prompts on conflict. Runs `repair` + `bundle compile` + `qmd sync` after.

## Validation

```bash
cargo test
```

Current coverage includes:

- foundation
- write/load
- retrieval/index
- qmd embedding config
- feedback/dream/repair
- hook capture
- Codex CLI E2E
- experiments

## Related Docs

- [AGENTS.md](./AGENTS.md)
- [Codex Integration Docs](./docs/integrations/codex/README.en.md)
- [Claude Code Integration Docs](./docs/integrations/claude-code/README.en.md)
- [Codex QMD Guide](./docs/integrations/codex/qmd.en.md)
- [Hooks Overview](./hooks/README.en.md)
- [Skills Overview](./skills/README.en.md)
- [Architecture Spec (zh-CN)](./docs/specs/architecture.zh-CN.md)
- [Progress Board (zh-CN)](./docs/progress/memfold-v1/00-master-checklist.zh-CN.md)
