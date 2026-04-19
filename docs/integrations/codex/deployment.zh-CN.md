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
- verify 还会检查一次 raw-text memory 回路：`write-evidence -> dream run -> search`
- verify 会在最后清理自己的 smoke memory 和 verify session 目录，再做最终 repair
- deploy 会先跑一次 `memfold repair` 再进入 sync/verify；如果 verify 仍判断为“可修复漂移”，还会再补一次 repair

## 说明

- 对当前这套 launcher-based 接入来说，`plugin install` 本身不是完整部署
- launcher 的退出处理是基于 `EXIT / INT / TERM / HUP` 的 best-effort
- 启动阶段会补偿执行 `dream maybe-run + qmd sync`，降低退出时遗漏整理/索引刷新的概率
- 仓库里虽然发货了 `turn_end.sh`，但“每轮自动挂接”仍依赖宿主/plugin 进一步接入，不等同于只安装 plugin

## 回滚

```bash
rm -rf ~/.codex/memfold
rm -f ~/.local/bin/memfold
rm -rf ~/plugins/memfold
```
