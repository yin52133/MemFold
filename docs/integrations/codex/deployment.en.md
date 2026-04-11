# MemFold Global Deployment For Codex

[中文](./deployment.zh-CN.md) | [English](./deployment.en.md)

## Goal

Deploy MemFold globally into:

```bash
~/.codex/memfold
```

while keeping deployed hooks and skills synchronized with the Git-managed repo sources.

## One-shot scripts

```bash
./scripts/deploy_codex_global.sh
./scripts/verify_codex_global.sh
```

## Deployment result

- `~/.codex/memfold/bin/memfold`
- `~/.codex/memfold/hooks/*` linked to `hooks/codex-global/*`
- `~/.codex/skills/memfold-codex-memory` linked to `skills/memfold-codex-memory`

## Validation

- CLI `load`
- global `turn_end` hook dry-run
- real `cdx exec` invocation of `memfold`

## Rollback

```bash
rm -rf ~/.codex/memfold
rm -f ~/.local/bin/memfold
rm -f ~/.codex/skills/memfold-codex-memory
```
