# MemFold Claude Code 集成

[中文](./README.zh-CN.md) | [English](./README.en.md)

本文档说明如何将 MemFold 集成到 Claude Code。

## 入口

- [全局部署指南](./deployment.zh-CN.md)

## 仓库契约

- `hooks/claude-code/` 是 hook 的 canonical 源
- `skills/claude-code/` 是 skill 的 canonical 源
- `scripts/` 提供安装和验证流程

## 与 Codex 集成的差异

| 方面 | Codex | Claude Code |
|------|-------|-------------|
| 存储根目录 | `~/.codex/memfold/` | `~/.claude/memfold/` |
| Hook 协议 | 环境变量 | stdin JSON |
| Hook 注册 | 全局 hook 目录 | `~/.claude/settings.json` |
| Skill 格式 | Plugin 系统 (plugin.json) | `.claude/skills/` + YAML frontmatter |
| 部署脚本 | `scripts/deploy_codex_global.sh` | `scripts/deploy_claude_global.sh` |
| 验证脚本 | `scripts/verify_codex_global.sh` | `scripts/verify_claude_global.sh` |

两个宿主的记忆**完全隔离**，互不影响。

## 记忆迁移

在 Codex 和 Claude Code 之间手动迁移记忆：

```bash
# Codex → Claude Code
./scripts/migrate_memory.sh codex-to-claude

# Claude Code → Codex
./scripts/migrate_memory.sh claude-to-codex

# 双向同步
./scripts/migrate_memory.sh sync

# 只迁移指定 scope
./scripts/migrate_memory.sh codex-to-claude --scope-id my-project

# 预览不复制
./scripts/migrate_memory.sh codex-to-claude --dry-run
```

迁移层：`stable`、`sessions`、`history`。遇到冲突时脚本会显示 diff 并询问：skip / overwrite / keep-both。迁移后自动执行 `repair` + `bundle compile` + `qmd sync`。
