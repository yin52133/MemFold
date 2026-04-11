# Hooks

[中文](./README.zh-CN.md) | [English](./README.en.md)

This directory is the **canonical hook source** for MemFold.

Principles:

- all Codex hook integrations originate here
- both repo-local and global deployment variants are Git-managed
- deployed hook files under `~/.codex/.../hooks` should not be manually edited
- deployment should use symlinks back to this directory

## Subdirectories

- `local/`
  - repo-local self-test hooks
- `codex-global/`
  - canonical hooks for global Codex deployment

## Design Rules

- hooks provide the safety net and filtered default capture
- skills/tools handle higher-precision memory actions
- any hook behavior change should be made here first, then redeployed
