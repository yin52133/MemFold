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

After deployment:

```bash
cdx-memfold
```

## Deployment result

- `~/.codex/memfold/bin/memfold`
- `~/.codex/memfold/hooks/*` linked to `hooks/codex-global/*`
- `~/plugins/memfold` linked to the repo plugin source, with runtime skills provided from plugin-internal `skills/`

## Validation

- CLI `load`
- global `turn_end` hook dry-run
- real `cdx exec` invocation of `memfold`
- `cdx-memfold` launcher exists and is executable

## Rollback

```bash
rm -rf ~/.codex/memfold
rm -f ~/.local/bin/memfold
rm -rf ~/plugins/memfold
```
