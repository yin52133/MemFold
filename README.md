# MemFold

[中文](./README.md) | [English](./README.en.md)

面向 **Codex memory** 的分层记忆框架。它把 `hook + skill/tool` 组合落成一个本地、可审计、可回放、可离线验证的记忆系统，而不是把所有历史粗暴塞回上下文。

## 这是什么

MemFold 用于解决 Codex 在多项目、多 session 工作时的两个相反问题：

- 什么都带着走：token 膨胀、旧错误路径污染当前任务
- 什么都不敢记：稳定偏好、项目约束、已确认规则总是丢

核心策略不是“存更多”，而是 **分层筛选**：

- `boot bundle`：启动时默认注入的最小稳定上下文
- `stable`：长期记忆真相源
- `evidence`：工作记录真相源
- `archive`：按日期归档的人类可读追溯层
- `qmd sidecar`：可重建的检索索引层

## 特色

- `hook + skill/tool` 组合：hook 保证不漏，skill 保证不乱
- `Rust + SQLite + local sidecar`：本地运行、易审计、易重建
- `truth-source first`：正文在 Markdown/JSONL，SQLite 只存状态与投影
- `dreaming + tombstone`：支持晋升、隔离、拒绝和防复活
- `autoresearch / experiments`：离线 fixture 驱动的规则验证
- `repo-local hook rollout`：可先在当前仓库自测，再迁全局 Codex

## 核心架构

```text
Codex Hooks / Tools
        |
        v
     memfold CLI
        |
        +-- boot / load
        +-- evidence / mutations
        +-- retrieval / qmd
        +-- feedback / dreaming / repair
        +-- experiments
        |
        +-- SQLite (state/projections/locks)
        +-- Markdown / JSONL (truth source)
        +-- QMD sidecar (discardable index)
```

## 当前可用能力

- `memfold init`
- `memfold load`
- `memfold write-evidence`
- `memfold search`
- `memfold qmd sync`
- `memfold bundle compile`
- `memfold feedback`
- `memfold dream run`
- `memfold repair`
- `memfold hook capture`
- `memfold experiment run`

## 用法

### 1. 初始化本地 memory 根目录

```bash
cargo run -- init --root ./.memfold-local
```

### 2. 启动时加载最小上下文

```bash
cargo run -- load \
  --root ./.memfold-local \
  --mode normal \
  --scope-type project \
  --scope-id MemFold \
  --intent startup \
  --budget 400
```

### 3. 写入工作记录

```bash
cargo run -- write-evidence \
  --root ./.memfold-local \
  --scope-type project \
  --scope-id MemFold \
  --session-id sess_demo \
  --source-kind decision \
  --summary "完成本地 hook 自测接线" \
  --promotable 0 \
  --origin-mode normal
```

### 4. 本地 hook 自测

```bash
MEMFOLD_ROOT="$PWD/.memfold-local" \
MEMFOLD_SCOPE_TYPE=project \
MEMFOLD_SCOPE_ID=MemFold \
MEMFOLD_SESSION_ID=sess_demo \
MEMFOLD_SOURCE_KIND=decision \
MEMFOLD_TURN_SUMMARY="完成本地 hook 自测接线" \
MEMFOLD_STATE_CHANGED=1 \
bash hooks/local/turn_end.sh
```

### 5. 运行离线 experiments

```bash
cargo run -- experiment run --fixture tests/fixtures/experiments/passing.json
```

## 本地 hook 策略

默认只把“有状态变化的摘要”写入本地 memory，而不是保存全部原始日志：

- 不保存：原始 prompt、原始 tool 输出、推理草稿、secret、重复噪声
- hook 自动保存：有状态变化的 turn/session 摘要，默认 `promotable=0`
- skill/tool 明确保存：用户明确要求记住的稳定偏好/约束，才允许 `promotable=1`

## 验证

```bash
cargo test
```

当前仓库已经覆盖：

- foundation
- write/load
- retrieval/index
- feedback/dream/repair
- Codex CLI E2E
- experiments

## 相关文档

- [AGENTS.md](./AGENTS.md)
- [Codex 集成文档入口](./docs/integrations/codex/README.zh-CN.md)
- [docs/specs/2026-04-11-memfold-architecture-design.zh-CN.md](./docs/specs/2026-04-11-memfold-architecture-design.zh-CN.md)
- [hooks/local/README.zh-CN.md](./hooks/local/README.zh-CN.md)
- [hooks/README.zh-CN.md](./hooks/README.zh-CN.md)
- [skills/README.zh-CN.md](./skills/README.zh-CN.md)
- [docs/progress/memfold-v1/00-master-checklist.zh-CN.md](./docs/progress/memfold-v1/00-master-checklist.zh-CN.md)
