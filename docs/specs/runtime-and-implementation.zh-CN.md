# 运行时与实现

> 状态：目标态设计 Draft。当前已发货命令集不变；这里补的是运行可观测性与迁移后的实现边界。

## 1. Problem and Goal

宿主调用、核心决策、运行日志和长耗时进度现在混在一起，导致系统看起来像“卡住了”。这份文档定义 MemFold 在宿主、核心、日志、阶段可观测性上的明确边界。

## 2. Scope and Non-Goals

In scope:
- 宿主与核心分工
- canonical CLI 命令集合
- 阶段进度输出
- `runtime/logs` 落盘日志

Out of scope:
- 常驻服务
- 图形化监控页面
- 宿主内部复制记忆决策

## 3. System Boundaries

```text
host
  └── triggers ──► memfold CLI
                    ├── memory/ content
                    ├── SQLite state
                    ├── QMD indexes
                    └── runtime/logs
```

宿主只负责触发，不负责：
- 决定加载哪些层
- 决定是否提升长期记忆
- 决定 scope 路由

## 4. Source of Truth Declaration

| Layer | Owns | Disposable |
|-------|------|------------|
| `runtime/logs/*.jsonl` | phase timing and execution logs | Yes |
| CLI stdout progress | live operator feedback | Yes |
| `memfold.db` | state / trace / jobs | No |

## 5. Core Decisions

1. **What:** CLI 仍是核心入口。 **Why:** 简单、稳定、易于被不同宿主调用。 **Reversal condition:** 如果后续宿主数量和并发模型要求常驻服务。
2. **What:** 每个明显耗时阶段必须输出进度。 **Why:** 没有进度时，用户会把系统当成卡死。 **Reversal condition:** 如果关键命令都足够快，不再需要可观测性。
3. **What:** 详细阶段日志落盘到 `runtime/logs`，由 dreaming 只提炼关键结论。 **Why:** 排障需要完整日志，但长期记忆只需要关键信息。 **Reversal condition:** 如果 runtime logs 被证明不再需要归档。

## 6. Runtime Flows

### 6.1 Standard Command Execution

```text
host invokes memfold command
  └──► emit stage_started
          ├── run command stage
          ├── emit progress heartbeat if slow
          ├── emit stage_finished on success
          ├── emit stage_failed on error
          └── append all events to runtime/logs/<session-id>.jsonl
```

### 6.2 Long-Running Stage Progress

```text
command enters slow stage
  └──► stdout: [stage] started
          ├── stdout: [stage] elapsed 5s
          ├── stdout: [stage] elapsed 10s
          ├── ...
          └── stdout: [stage] done in 14.2s
```

Required slow stages:
- `load`
- `search`
- `dream run`
- `repair`
- `qmd sync`
- `summarize-history`

## 7. Data and Interface Contracts

### 7.1 Runtime Log Event

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `session_id` | string | yes | current session |
| `command` | string | yes | CLI command |
| `stage` | string | yes | logical phase name |
| `event` | enum | yes | `started / heartbeat / finished / failed` |
| `elapsed_ms` | integer | yes | elapsed time |
| `message` | string | optional | human-readable note |
| `created_at` | string | yes | RFC3339 |

### 7.2 Progress Output Contract

stdout progress line:

```text
[dream.gather] elapsed=5.2s status=running
```

stderr error line:

```text
[dream.gather] status=failed error=TRACE_VALIDATION_FAILED
```

## 8. Failure Cases and Acceptance

Capability: slow stages are visible  
Failure example: `dream run` executes for 30 seconds with no output  
Expected: stage name and elapsed time are visible during long-running operations  
Completion signal: silent period > 5s during marked slow stages = 0

Capability: logs are durable  
Failure example: command fails and no runtime log remains  
Expected: `runtime/logs/<session-id>.jsonl` contains stage failure event  
Completion signal: runtime log loss rate for failed commands = 0

## 9. Implementation Phases

Phase 1: canonical log event format  
Done when: runtime log schema is fixed and shared by all slow commands

Phase 2: stdout progress integration  
Done when: all marked slow stages emit start / heartbeat / finish lines

Phase 3: history extraction from runtime logs  
Done when: session-end summarization can pull key failures and timing from runtime logs
