# MemFold Global Deployment For Codex

[中文](./deployment.zh-CN.md) | [English](./deployment.en.md)

## Goal

Deploy MemFold globally into:

```bash
~/.codex/memfold
```

while keeping deployed hooks and skills synchronized with the Git-managed repo sources.

## Standard Deployment

```bash
./scripts/deploy_codex_global.sh
```

This is the standard install / update / redeploy entrypoint.

Standalone health check:

```bash
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
- plugin cache refreshed so updated plugin commands and skills are picked up on redeploy

## Validation

- deploy runs verify automatically
- verify checks launcher `session_start/session_end` smoke behavior
- verify checks global `turn_end` hook persistence
- verify checks `qmd sync` plus a real `search` round-trip
- deploy only falls back to `memfold repair` when verify reports repairable drift

## Notes

- `plugin install` alone is not the full deployment path for the current launcher-based integration
- launcher exit handling is best-effort on `EXIT / INT / TERM / HUP`
- startup compensates with `dream maybe-run` and `qmd sync` to reduce missed exit work
- the repo ships `turn_end.sh`, but per-turn host attachment still depends on host/plugin integration beyond the launcher

## Rollback

```bash
rm -rf ~/.codex/memfold
rm -f ~/.local/bin/memfold
rm -rf ~/plugins/memfold
```
