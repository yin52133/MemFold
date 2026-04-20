# MemFold Claude Code 全局部署

[中文](./deployment.zh-CN.md) | [English](./deployment.en.md)

## 目标

将 MemFold 全局部署到：

```bash
~/.claude/memfold
```

同时保持已部署的 hook 和 skill 与 Git 仓库源同步。

## 标准部署

```bash
./scripts/deploy_claude_global.sh
```

这是标准的安装 / 更新 / 重新部署入口。

独立健康检查：

```bash
./scripts/verify_claude_global.sh
```

## 部署产物

- `~/.claude/memfold/bin/memfold` — 二进制
- `~/.claude/memfold/hooks/*` — 链接到 `hooks/claude-code/*`
- `~/.claude/skills/memfold-*` — 链接到 `skills/claude-code/*`
- `~/.claude/settings.json` — 合并 hooks 配置

## Hook 注册

部署脚本将 MemFold hooks 合并到 `~/.claude/settings.json`：

```json
{
  "hooks": {
    "SessionStart": [{"matcher": "startup", "hooks": [{"type": "command", "command": "~/.claude/memfold/hooks/session_start.sh"}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "~/.claude/memfold/hooks/stop.sh"}]}],
    "SessionEnd": [{"hooks": [{"type": "command", "command": "~/.claude/memfold/hooks/session_end.sh"}]}]
  }
}
```

## Skills

六个 skill 部署到 `~/.claude/skills/`：

| Skill | 用途 |
|-------|------|
| `memfold-memory` | 顶层路由 |
| `memfold-search` | 检索先前工作和上下文 |
| `memfold-remember` | 标记稳定偏好为可提升 |
| `memfold-forget` | 拒绝错误记忆 |
| `memfold-dream` | 整合记忆 |
| `memfold-qmd` | 管理检索 sidecar 和 embedding |

## 验证

- 部署自动运行验证
- 验证检查二进制、hook、skill 和 settings.json
- 验证运行 `load → write-evidence → search` 冒烟测试
- 部署运行 `memfold repair`，若验证报告可修复偏差则再次修复

## 回滚

```bash
rm -rf ~/.claude/memfold
rm -rf ~/.claude/skills/memfold-*
# 手动从 ~/.claude/settings.json 移除 MemFold hooks
```
