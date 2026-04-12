# MemFold 全局部署到 Codex

[中文](./deployment.zh-CN.md) | [English](./deployment.en.md)

## 目标

把 MemFold 以全局形态部署到：

```bash
~/.codex/memfold
```

并通过仓库内的 `hooks/` 与 `skills/` 维持部署同步。

## 标准部署入口

```bash
./scripts/deploy_codex_global.sh
```

这是标准安装 / 更新 / 重部署入口。

独立健康检查：

```bash
./scripts/verify_codex_global.sh
```

部署后可用：

```bash
cdx-memfold
```

## 部署结果

- `~/.codex/memfold/bin/memfold`
- `~/.codex/memfold/hooks/*` 指向仓库内 `hooks/codex-global/*`
- `~/plugins/memfold` 指向仓库内 `plugins/memfold`，运行态 skill 来自 plugin 内部 `skills/`
- 重部署时会刷新 plugin cache，确保更新后的 plugin command 和 skill 能被重新加载

## 验证

- deploy 会自动跑 verify
- verify 会检查 launcher 的 `session_start/session_end` smoke 行为
- verify 会检查全局 `turn_end` hook 是否能落盘
- verify 会检查 `qmd sync` 和一次真实 `search` 回路
- 只有 verify 判断为“可修复漂移”时，deploy 才会回退到 `memfold repair`

## 说明

- 对当前这套 launcher-based 接入来说，`plugin install` 本身不是完整部署
- 仓库里虽然发货了 `turn_end.sh`，但“每轮自动挂接”仍依赖宿主/plugin 进一步接入，不等同于只安装 plugin

## 回滚

```bash
rm -rf ~/.codex/memfold
rm -f ~/.local/bin/memfold
rm -rf ~/plugins/memfold
```
