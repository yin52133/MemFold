# MemFold Wave 2 Write Load Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first usable MemFold CLI loop for `init`, `load`, and `write-evidence`, backed by mutations, boot bundle compilation, and evidence persistence.

**Architecture:** Keep Wave 2 library-first. Add `mutations`, `boot`, and `evidence` modules under the Rust crate, then wrap them in a minimal CLI entrypoint. Use SQLite for projections and mutation state, filesystem for truth content, and deterministic JSON responses for CLI consumers.

**Tech Stack:** Rust 1.94, Cargo, rusqlite, serde, serde_json, clap, uuid, time, tempfile

---

## File Structure

- Create: `src/mutations.rs`
- Create: `src/boot.rs`
- Create: `src/evidence.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/error.rs`
- Modify: `src/domain.rs`
- Modify: `src/state/mod.rs`
- Modify: `src/memory_fs/mod.rs`
- Modify: `src/memory_fs/paths.rs`
- Create: `tests/boot_load.rs`
- Create: `tests/evidence_write.rs`
- Create: `tests/cli_wave2.rs`

## Task Groups

### Task Group A: `mutations`

Deliver:
- pending -> applied_to_content -> fully_applied lifecycle
- mutation record persistence helpers in SQLite
- scope-targeted pending mutation detection used by load

### Task Group B: `boot` + `load`

Deliver:
- parse stable markdown items
- compile scope bundle from stable markdown into `boot/bundle.md`
- refresh `boot_entries` projection
- load compiled items into JSON response with budget trimming

### Task Group C: `evidence` + `write-evidence`

Deliver:
- append JSONL evidence truth source
- append archive markdown log
- upsert `sessions` metadata
- insert `evidence_items` and `trace_archives` projections
- return structured evidence write response

### Task Group D: CLI integration

Deliver:
- `memfold init --root`
- `memfold load --mode --scope-type --scope-id --intent --budget`
- `memfold write-evidence --scope-type --scope-id --session-id --source-kind --summary --promotable --origin-mode [--claim-fingerprint]`
- structured JSON success / failure payloads

## Verification Targets

- `cargo test --test boot_load`
- `cargo test --test evidence_write`
- `cargo test --test cli_wave2`
- `cargo test --test config_defaults --test state_init --test memory_fs_io --test init_root --test boot_load --test evidence_write --test cli_wave2`
- `cargo test`
