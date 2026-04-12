# MemFold

[English](./README.md) | [中文](./README.zh-CN.md)

面向 **Codex memory** 的分层记忆框架。它把 `hook + skill/tool + qmd + dreaming` 组合成一套本地、可审计、可回放、可离线验证的记忆系统，而不是把所有历史粗暴塞回上下文。

## 仓库定位

这个仓库不是通用“聊天记忆 demo”，而是专门围绕 **Codex 的记忆机制** 搭建：

- 针对 Codex session 启动、工作中、退出时的行为
- 针对 Codex 的 hook、skill、plugin command、全局部署
- 针对 Codex 场景下的 startup context、evidence capture、dreaming consolidation、QMD retrieval

## 为什么要做

Codex 长时间在多个项目里工作时，记忆系统通常会坏在两个方向：

- 什么都带着走：token 膨胀、旧错误路径污染新任务、旧项目判断侵入当前项目
- 什么都不敢记：稳定偏好总丢、项目约束总丢、每次 session 都像第一次进项目

MemFold 的目标不是“存更多”，而是 **只把真正稳定的东西带进去**，把临时工作轨迹保留在更深层，等到 dreaming 再做整理。

## 核心特色

- `hook + skill/tool`：hook 保证不漏，skill 保证不乱
- `五层分层记忆`：boot / stable / evidence / wiki / archive
- `truth-source first`：Markdown/JSONL 是正文真相源，SQLite 只存状态和投影
- `dreaming + tombstone`：支持晋升、隔离、拒绝和防复活
- `QMD sidecar`：索引可重建、检索可升级、存储层不被绑死
- `embedding-ready qmd`：支持本地 embedding model 配置与首次自动下载
- `experiments / autoresearch`：离线 fixture 驱动验证，不在生产里试错
- `repo canonical integration`：`hooks/`、`skills/`、`plugins/` 都在仓库内受 Git 管理

## 五层架构

```text
Layer 1  boot bundle
  启动时默认注入的最小稳定上下文

Layer 2  stable
  长期记忆真相源

Layer 3  evidence
  工作记录真相源

Layer 4  wiki
  背景知识层

Layer 5  archive
  追溯层 / 日期归档层
```

默认读取顺序：

```text
boot -> stable -> evidence -> wiki -> archive
```

知识检索时：

```text
boot -> stable -> wiki -> archive -> evidence
```

## 核心工作流

```text
Codex session start
  -> hook/session_start
  -> memfold init
  -> memfold load
  -> 启动补偿一次 dream maybe-run
  -> qmd sync
  -> 注入最小 bundle

工作进行中
  -> hook/turn_end 记录 promotable=0 的过滤摘要
  -> skill/tool 在需要时写 promotable=1 / search / feedback

session 结束
  -> hook/session_end 写 session 摘要
  -> 后台静默触发 dream maybe-run（best-effort）
  -> 双门控：距上次 >= 24h && 新结束 session >= 5

dreaming 运行
  -> 读取 promotable evidence
  -> tombstone / sterile / analysis-draft 先过滤
  -> promote / hold / discard
  -> 重编 bundle
  -> qmd sync
```

## QMD 与 embedding 模型

QMD 是索引侧边车，不是正文真相源。

当前支持两种层次：

1. 基础 sidecar
- `stable / session_log / history` 记录可同步到 `qmd/collections/`
- 搜索可退回词法匹配

2. embedding sidecar
- 通过 `memfold qmd init-model` 初始化 embedding model
- 首次初始化会自动下载本地模型到：

```bash
~/.codex/memfold/qmd/models
```

- 不依赖单独起 `Ollama` 或 `LM Studio`
- 配置写入：

```bash
~/.codex/memfold/qmd/config/model.json
```

推荐首选模型：

- `multilingual-e5-small`

可选模型：

- `bge-small-zh-v1.5`
- `bge-m3`

初始化示例：

```bash
memfold --root ~/.codex/memfold qmd init-model --model multilingual-e5-small
memfold --root ~/.codex/memfold qmd sync --scope-type project --scope-id my-project
```

更多见：

- [Codex QMD Guide](./docs/integrations/codex/qmd.zh-CN.md)

## 仓库结构

```text
src/                         核心引擎
hooks/                       canonical hook source
  local/                     仓库内自测版
  codex-global/              全局 Codex 部署态
skills/                      canonical skill source
plugins/                     Codex plugin / slash command source
docs/integrations/codex/     Codex 部署与接入文档
scripts/                     部署 / 验证脚本
tests/                       回归与 E2E
```

## 当前可用命令

- `memfold init`
- `memfold load`
- `memfold write-evidence`
- `memfold search`
- `memfold qmd init-model`
- `memfold qmd sync`
- `memfold bundle compile`
- `memfold feedback`
- `memfold dream run`
- `memfold dream maybe-run`
- `memfold repair`
- `memfold hook capture`
- `memfold experiment run`

## /dream 与 plugin command

仓库内已经提供 plugin command 源：

- `plugins/memfold/commands/dream.md`
- `plugins/memfold/commands/qmd.md`
- `plugins/memfold/commands/memfold-search.md`
- `plugins/memfold/commands/memfold-remember.md`
- `plugins/memfold/commands/memfold-forget.md`

`/dream` 支持的参数语义：

- `scope_type`
- `scope_id`
- `trigger=manual|scheduled`
- `mode=run|maybe-run`

其中：

- `run`：显式立即整理
- `maybe-run`：走静默双门控调度

## 本地 hook 策略

默认只把“有状态变化的摘要”写入 memory，而不是保存全部原始日志：

- 不保存：原始 prompt、原始 tool 输出、推理草稿、secret、重复噪声
- hook 自动保存：有状态变化的 turn/session 摘要，默认 `promotable=0`
- skill/tool 明确保存：用户明确要求记住的稳定偏好/约束，才允许 `promotable=1`

## 全局部署

全局部署目标：

```bash
~/.codex/memfold
```

标准安装 / 更新 / 重部署入口：

```bash
./scripts/deploy_codex_global.sh
```

独立健康检查：

```bash
./scripts/verify_codex_global.sh
```

启动器：

```bash
cdx-memfold
```

说明：

- deploy 会重建 binary、刷新 launcher / hooks / plugin source、清理 plugin cache、执行 QMD sync，然后跑 verify
- deploy 只会在 verify 判断为“可修复漂移”时才自动执行 `memfold repair`
- `cdx-memfold` 会在启动前跑 `session_start`，并在 `EXIT / ctrl+c / TERM / HUP` 时 best-effort 跑 `session_end`
- 当前启动阶段会补偿执行一次 `dream maybe-run + qmd sync`，用于兜底上一次退出时可能遗漏的整理与索引刷新
- 仓库内的 `turn_end.sh` 会随 deploy 刷新，但当前每轮自动挂接仍依赖宿主侧接入，不等同于只做 plugin install
- 只做 plugin 安装不是完整部署

## 验证

```bash
cargo test
```

当前已覆盖：

- foundation
- write/load
- retrieval/index
- qmd embedding config
- feedback/dream/repair
- hook capture
- Codex CLI E2E
- experiments

## 相关文档

- [AGENTS.md](./AGENTS.md)
- [Codex 集成文档入口](./docs/integrations/codex/README.zh-CN.md)
- [Codex QMD Guide](./docs/integrations/codex/qmd.zh-CN.md)
- [hooks/README.zh-CN.md](./hooks/README.zh-CN.md)
- [plugins/README.zh-CN.md](./plugins/README.zh-CN.md)
- [docs/specs/architecture.zh-CN.md](./docs/specs/architecture.zh-CN.md)
- [docs/progress/memfold-v1/00-master-checklist.zh-CN.md](./docs/progress/memfold-v1/00-master-checklist.zh-CN.md)
