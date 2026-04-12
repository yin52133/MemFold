# Codex Global Deploy Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make MemFold global deployment idempotent and self-verifying, while cleaning current user-facing docs to match the shipped `stable / session_log / history` QMD shape.

**Architecture:** Keep the current launcher-based host integration model and upgrade the existing deployment scripts rather than introducing a second installation path. `deploy_codex_global.sh` becomes the only install/redeploy entrypoint, `verify_codex_global.sh` becomes the health gate, and deploy only runs `repair` when verify reports repairable drift.

**Tech Stack:** Bash, Cargo, Rust integration tests, SQLite, existing MemFold CLI

---

### Task 1: Lock deployment script expectations with tests

**Files:**
- Create: `tests/deploy_scripts.rs`
- Test: `tests/deploy_scripts.rs`

- [ ] Add script-content regression tests for deploy and verify behavior.
- [ ] Run `cargo test --test deploy_scripts` and confirm the new assertions fail against the current scripts.

### Task 2: Upgrade one-shot deploy/redeploy flow

**Files:**
- Modify: `scripts/deploy_codex_global.sh`
- Modify: `scripts/verify_codex_global.sh`

- [ ] Make deploy refresh binary, launcher, hooks, plugin source link, marketplace entry, and plugin cache.
- [ ] Make deploy run verify and only trigger `repair` when verify reports repairable drift.
- [ ] Make deploy resync QMD after install or repair, then rerun verify.
- [ ] Keep the script idempotent for repeated redeploys from the same or newer repo checkout.

### Task 3: Clean current user-facing documentation

**Files:**
- Modify: `README.md`
- Modify: `README.en.md`
- Modify: `README.zh-CN.md`
- Modify: `docs/integrations/codex/deployment.en.md`
- Modify: `docs/integrations/codex/deployment.zh-CN.md`
- Modify: `docs/integrations/codex/qmd.en.md`
- Modify: `docs/integrations/codex/qmd.zh-CN.md`

- [ ] Replace outdated QMD source wording with `stable / session_log / history` where describing the current shipped behavior.
- [ ] Document `./scripts/deploy_codex_global.sh` as the standard install/update/redeploy entrypoint.
- [ ] Document that plugin installation alone is not the full deployment path.

### Task 4: Re-run targeted verification

**Files:**
- Test: `tests/deploy_scripts.rs`
- Test: `tests/launcher_script.rs`

- [ ] Run `cargo test --test deploy_scripts --test launcher_script`.
- [ ] Run `./scripts/verify_codex_global.sh` and inspect the result.
- [ ] If deploy behavior changed materially, run `./scripts/deploy_codex_global.sh` once end-to-end and confirm verify succeeds without falling back to repair on a healthy install.
