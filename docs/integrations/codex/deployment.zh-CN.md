# MemFold 全局部署到 Codex

[中文](./deployment.zh-CN.md) | [English](./deployment.en.md)

## 目标

把 MemFold 以全局形态部署到：

```bash
~/.codex/memfold
```

并通过仓库内的 `hooks/` 与 `skills/` 维持部署同步。

## 一键脚本

```bash
./scripts/deploy_codex_global.sh
./scripts/verify_codex_global.sh
```

部署后可用：

```bash
cdx-memfold
```

## 部署结果

- `~/.codex/memfold/bin/memfold`
- `~/.codex/memfold/hooks/*` 指向仓库内 `hooks/codex-global/*`
- `~/.codex/skills/memfold-codex-memory` 指向仓库内 `skills/memfold-codex-memory`

## 验证

- CLI `load`
- 全局 `turn_end` hook dry-run
- `cdx exec` 真实调用 `memfold`
- `cdx-memfold` 启动器存在且可执行

## 回滚

```bash
rm -rf ~/.codex/memfold
rm -f ~/.local/bin/memfold
rm -f ~/.codex/skills/memfold-codex-memory
```
