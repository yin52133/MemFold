# MemFold For Claude Code

[中文](./README.zh-CN.md) | [English](./README.en.md)

These docs explain how to integrate MemFold into Claude Code.

## Entry Points

- [Global Deployment Guide](./deployment.en.md)

## Repository Contract

- `hooks/claude-code/` is the canonical hook source
- `skills/claude-code/` is the canonical skill source
- `scripts/` provides install and verification flows

Other projects should integrate by:

1. referencing this repo's `hooks/claude-code/` and `skills/claude-code/`
2. running the deployment script
3. validating with the verification script

## Differences from Codex Integration

| Aspect | Codex | Claude Code |
|--------|-------|-------------|
| Storage root | `~/.codex/memfold/` | `~/.claude/memfold/` |
| Hook protocol | Environment variables | stdin JSON |
| Hook registration | Global hook directory | `~/.claude/settings.json` |
| Skill format | Plugin system (plugin.json) | `.claude/skills/` with YAML frontmatter |
| Deploy script | `scripts/deploy_codex_global.sh` | `scripts/deploy_claude_global.sh` |
| Verify script | `scripts/verify_codex_global.sh` | `scripts/verify_claude_global.sh` |

Memory is **fully isolated** between the two hosts.

## Memory Migration

Migrate memory between Codex and Claude Code manually:

```bash
# Codex → Claude Code
./scripts/migrate_memory.sh codex-to-claude

# Claude Code → Codex
./scripts/migrate_memory.sh claude-to-codex

# Bidirectional sync
./scripts/migrate_memory.sh sync

# Migrate specific scope only
./scripts/migrate_memory.sh codex-to-claude --scope-id my-project

# Preview without copying
./scripts/migrate_memory.sh codex-to-claude --dry-run
```

Migrated layers: `stable`, `sessions`, `history`. On conflict, the script shows a diff and asks: skip / overwrite / keep-both. After migration, `repair` + `bundle compile` + `qmd sync` run automatically.
