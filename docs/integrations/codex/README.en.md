# MemFold For Codex

[中文](./README.zh-CN.md) | [English](./README.en.md)

These docs explain how to integrate MemFold into Codex.

## Entry Points

- [Global Deployment Guide](./deployment.en.md)
- [AGENTS Example](./AGENTS.example.md)

## Repository Contract

- `hooks/` is the canonical hook source
- `skills/` is the canonical skill source
- `scripts/` provides install and verification flows

Other projects should integrate by:

1. referencing this repo’s `hooks/` and `skills/`
2. running the deployment script
3. validating with the verification script
