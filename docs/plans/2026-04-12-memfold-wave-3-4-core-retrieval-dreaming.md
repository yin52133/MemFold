# MemFold Wave 3 4 Core Retrieval Dreaming Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend MemFold from write/load into searchable, correctable, and self-consolidating memory with `search`, `feedback`, `dream run`, `qmd sync`, and `repair`.

**Architecture:** Keep QMD as a local sidecar index with deterministic document records and lexical scoring for this implementation slice. Use `feedback` and `dreaming` to update filesystem truth first, then SQLite projections and derived layers. Final CLI E2E will exercise `write-evidence -> dream run -> search -> feedback -> repair`.

**Tech Stack:** Rust 1.94, Cargo, rusqlite, serde, serde_json, clap, sha2, tempfile

---

## Wave 3 Modules

- `src/qmd_adapter.rs`
- `src/retrieval.rs`
- `tests/search_flow.rs`

## Wave 4 Modules

- `src/feedback.rs`
- `src/dreaming.rs`
- `src/repair.rs`
- `tests/feedback_dream_repair.rs`
- `tests/cli_e2e.rs`
