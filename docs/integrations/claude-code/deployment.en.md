# MemFold Global Deployment For Claude Code

[中文](./deployment.zh-CN.md) | [English](./deployment.en.md)

## Goal

Deploy MemFold globally into:

```bash
~/.claude/memfold
```

while keeping deployed hooks and skills synchronized with the Git-managed repo sources.

## Standard Deployment

```bash
./scripts/deploy_claude_global.sh
```

This is the standard install / update / redeploy entrypoint.

Standalone health check:

```bash
./scripts/verify_claude_global.sh
```

## Deployment Result

- `~/.claude/memfold/bin/memfold` — binary
- `~/.claude/memfold/hooks/*` — linked to `hooks/claude-code/*`
- `~/.claude/skills/memfold-*` — linked to `skills/claude-code/*`
- `~/.claude/settings.json` — hooks configuration merged

## Hook Registration

The deploy script merges MemFold hooks into `~/.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [{"matcher": "startup", "hooks": [{"type": "command", "command": "~/.claude/memfold/hooks/session_start.sh"}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "~/.claude/memfold/hooks/stop.sh"}]}],
    "SessionEnd": [{"hooks": [{"type": "command", "command": "~/.claude/memfold/hooks/session_end.sh"}]}]
  }
}
```

## Skills

Six skills are deployed to `~/.claude/skills/`:

| Skill | Purpose |
|-------|---------|
| `memfold-memory` | Top-level router |
| `memfold-search` | Retrieve prior work and context |
| `memfold-remember` | Mark stable preferences as promotable |
| `memfold-forget` | Reject wrong memory claims |
| `memfold-dream` | Consolidate memory |
| `memfold-qmd` | Manage retrieval sidecar and embeddings |

## Validation

- deploy runs verify automatically
- verify checks binary, hooks, skills, and settings.json
- verify runs `load → write-evidence → search` smoke test
- deploy runs `memfold repair` and falls back to another repair if verify reports drift

## Rollback

```bash
rm -rf ~/.claude/memfold
rm -rf ~/.claude/skills/memfold-*
# Manually remove MemFold hooks from ~/.claude/settings.json
```
