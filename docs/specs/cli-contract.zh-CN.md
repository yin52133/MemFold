# CLI 契约

> 状态：目标态设计 Draft。当前已发货的宿主集成仍以 `load / write-evidence / search / feedback / dream / repair` 和 `--scope-type user|project` 为准；在迁移完成前，以 `AGENTS.md` 和 `docs/integrations/codex/*` 为当前实现真相源。

## 1. Problem and Goal

宿主必须用稳定、明确的 CLI 契约调用 MemFold，不能靠猜测参数、返回值和错误码。这份文档同时给出当前已发货接口和目标迁移接口，避免设计先于实现却没有边界说明。

## 2. Scope and Non-Goals

In scope:
- current shipped CLI commands
- target migration command names
- 参数、成功返回、命名错误码
- 兼容别名说明

Out of scope:
- 交互式 TUI
- 图形界面

## 3. System Boundaries

```text
host / hook / skill
  └── calls ──► memfold CLI
                  └── returns JSON or named error
```

## 4. Source of Truth Declaration

| Contract | Owner |
|----------|-------|
| command names and parameters | this spec |
| storage field names | `sqlite-schema.zh-CN.md` + `memory-item-schema.zh-CN.md` |

## 5. Core Decisions

1. **What:** 当前已发货接口继续保留 `write-evidence` 与 `scope-type user|project`。 **Why:** 它们已经出现在 AGENTS、README、Codex 集成文档和已发货 skills 中。 **Reversal condition:** 如果 repo-wide 文档、工具和实现已经同时完成迁移。
2. **What:** `write-session-log`、`summarize-history`、`trace find` 是目标迁移接口，不是当前已发货接口。 **Why:** 设计需要表达目标状态，但不能伪装成当前可直接复制执行的命令。 **Reversal condition:** 如果这些命令已经在当前 repo 中实现并接入宿主。
3. **What:** 当前阶段的 canonical CLI 文档必须显式区分“已发货”和“迁移目标”。 **Why:** 否则使用者会复制一个当前 host setup 无法运行的命令。 **Reversal condition:** 如果系统已经完成单一接口收口。

## 6. Runtime Flows

### 6.1 Current Shipped Write Path

```text
host / hook / skill
  └──► memfold write-evidence
          ├── append JSONL
          ├── update SQLite
          ├── rebuild derived layers only if needed
          └── return evidence id
```

### 6.2 Target History Summarization

```text
session end / manual operator
  └──► memfold summarize-history
          ├── read session_log + runtime logs
          ├── write history/daily/<date>.md
          └── return summary result
```

## 7. Data and Interface Contracts

### 7.1 `memfold load`

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `--mode` | enum | yes | `normal / fresh / sterile` |
| `--scope-type` | enum | yes | current shipped: `user / project`; target migration: `user / repo / family` |
| `--scope-id` | string | yes | scope identifier |
| `--intent` | enum | yes | `startup / continue / knowledge_lookup / reset` |
| `--budget` | integer | optional | token budget |

success:

```json
{
  "mode": "normal",
  "scope": {"type": "project", "id": "MemFold"},
  "items": [{"item_key":"project.rule.truthful","text":"..."}],
  "total_tokens_estimate": 128,
  "degraded": false
}
```

named errors:
- `INVALID_SCOPE`
- `REPO_SCOPE_RESOLUTION_FAILED`
- `UNSTABLE_MUTATION`

### 7.2 `memfold write-evidence`（current shipped）

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `--scope-type` | enum | yes | `user / project` |
| `--scope-id` | string | yes | target scope |
| `--session-id` | string | yes | session id |
| `--source-kind` | enum | yes | `user / tool / code / test / decision / feedback` |
| `--summary` | string | yes | sanitized structured summary |
| `--promotable` | `0|1` | yes | promotion candidate flag |
| `--origin-mode` | enum | yes | `normal / fresh / sterile` |
| `--claim-fingerprint` | string | optional | semantic key |

success:

```json
{
  "evidence_id": "ev_...",
  "stored": true
}
```

named errors:
- `SANITIZE_FAILED`
- `JSONL_WRITE_FAILED`
- `EVIDENCE_INDEX_FAILED`

### 7.3 `memfold write-session-log`（target migration）

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `--scope-type` | enum | yes | target: `user / repo / family` |
| `--scope-id` | string | yes | target scope |
| `--session-id` | string | yes | session id |
| `--source-kind` | enum | yes | `user / decision / feedback / code / test / tool / runtime` |
| `--raw-text` | string | optional | required for user-origin entries |
| `--summary` | string | optional | normalized summary |
| `--promotable` | `0|1` | yes | promotion candidate flag |
| `--origin-mode` | enum | yes | `normal / fresh / sterile` |

success:

```json
{
  "entry_id": "sl_...",
  "stored": true
}
```

### 7.4 `memfold summarize-history`（target migration）

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `--scope-type` | enum | yes | target: `user / repo / family` |
| `--scope-id` | string | yes | target scope |
| `--session-id` | string | optional | summarize one session |
| `--date` | string | optional | `YYYY-MM-DD` |
| `--trigger` | enum | yes | `session_end / manual / rebuild` |

success:

```json
{
  "summary_id": "hs_...",
  "updated": true
}
```

named errors:
- `HISTORY_SUMMARY_FAILED`
- `SESSION_LOG_NOT_FOUND`
- `RUNTIME_LOG_NOT_FOUND`

### 7.5 `memfold trace find`（target migration）

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `--query` | string | yes | lookup text |
| `--scope-type` | enum | optional | constrain scope |
| `--scope-id` | string | optional | constrain scope id |

success:

```json
{
  "entry_id": "sl_...",
  "session_id": "sess_...",
  "line_no": 14,
  "raw_text": "以后不要奉承我，要判断我说得对不对。",
  "summary": "用户要求实事求是，不要奉承。"
}
```

named errors:
- `TRACE_NOT_FOUND`
- `SESSION_LOG_CORRUPTED`

## 8. Failure Cases and Acceptance

Capability: canonical CLI names are consistent  
Failure example: target design命令被当成当前已发货接口直接复制使用  
Expected: docs clearly distinguish current shipped commands from migration targets  
Completion signal: current-vs-target ambiguity in CLI contract = 0

## 9. Implementation Phases

Phase 1: define shipped-vs-target command boundary  
Done when: current host users can follow this doc without invoking nonexistent commands

Phase 2: migrate hooks and host integration  
Done when: repo-wide docs and tools can switch from `write-evidence` to `write-session-log`
