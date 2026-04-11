# Skills

[中文](./README.zh-CN.md) | [English](./README.en.md)

This directory is the **canonical skill source** that MemFold exposes to Codex.

Principles:

- skill definitions must stay in sync with deployed behavior
- other projects should reference these files directly instead of copying from `~/.codex/skills`
- the global deployment script should link `~/.codex/skills/memfold-codex-memory` back here

## Current Skill

- `memfold-codex-memory/`

## Purpose

- define when Codex should call `load/search/write-evidence/feedback/dream/repair`
- enforce the boundary between hooks and skills/tools
