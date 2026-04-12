# SQLite Schema

> 状态：目标态设计 Draft。当前实现仍存在旧表名与旧投影；这里定义的是迁移完成后的 canonical schema。

## 1. Problem and Goal

SQLite 负责状态、索引和追溯，不负责承载人读正文。Schema 必须支持三件事：稳定加载、精确追溯、跨层 repair。

## 2. Scope and Non-Goals

In scope:
- scope 路由
- `session_log` 索引
- `stable` 投影
- boot projection
- tombstone、feedback、dreaming、retrieval、mutations

Out of scope:
- 把 Markdown/JSONL 正文完整搬进 SQLite
- 让 SQLite 成为唯一记忆正文来源

## 3. System Boundaries

```text
session_log.jsonl / stable/*.md / history/*.md
  └── project into ──► memfold.db
                          ├── state
                          ├── trace index
                          ├── routing
                          └── jobs / locks
```

## 4. Source of Truth Declaration

| Table group | Owns | Disposable |
|-------------|------|------------|
| `repo_scopes`, `family_scopes`, `repo_family_links` | scope routing state | No |
| `session_log_entries` | trace index for session logs | Rebuildable |
| `memory_items` | stable projection | Rebuildable |
| `boot_entries` | bundle projection | Rebuildable |
| `sessions` | session metadata | Rebuildable from logs + hooks |
| `tombstones`, `feedback_events`, `dream_jobs`, `retrieval_events`, `mutations`, `lock_leases` | state and jobs | No |

## 5. Core Decisions

1. **What:** 把 `evidence_items` 更名为 `session_log_entries`。 **Why:** 与 `session_log.jsonl` 保持同名，避免“文件叫 A、表叫 B”。 **Reversal condition:** 如果兼容层要求永久保留旧表名。
2. **What:** 删除 `trace_archives` 的 canonical 地位。 **Why:** history 不再是原始镜像，单独的 trace archive 表没有必要。 **Reversal condition:** 如果 history 重新承担不可重建的独特内容。
3. **What:** 新增 repo/family 路由表。 **Why:** basename 不能稳定标识工程。 **Reversal condition:** 如果宿主层提供更稳定的统一 scope registry。

## 6. Runtime Flows

### 6.1 Session Log Write Projection

```text
append session_log.jsonl
  └──► upsert sessions
          └──► insert session_log_entries
                  └──► mutation fully_applied
```

### 6.2 Stable Rebuild

```text
parse stable/*.md
  └──► rebuild memory_items
          └──► rebuild boot_entries
```

### 6.3 Trace Query

```text
query session_log_entries
  └──► resolve repo_id + session_id + line_no
          └──► open session_log.jsonl line
```

## 7. Data and Interface Contracts

### 7.1 Tables

| Table | Purpose |
|-------|---------|
| `repo_scopes` | repo 身份与 git 规范化信息 |
| `family_scopes` | family 路由目标 |
| `repo_family_links` | repo 到 family 的自动归属 |
| `memory_items` | 长期记忆投影 |
| `session_log_entries` | 会话日志索引 |
| `boot_entries` | 启动包投影 |
| `sessions` | session 元数据 |
| `tombstones` | 被拒绝的语义断言 |
| `feedback_events` | 用户反馈事件 |
| `dream_jobs` | dreaming 作业 |
| `retrieval_events` | 检索记录 |
| `mutations` | 跨存储事务状态 |
| `lock_leases` | 锁和 lease |

### 7.2 `session_log_entries`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | TEXT | yes | `sl_...` |
| `session_id` | TEXT | yes | owning session |
| `scope_type` | TEXT | yes | `user / repo / family` |
| `scope_id` | TEXT | yes | scope identifier |
| `source_kind` | TEXT | yes | `user / decision / feedback / code / test / tool / runtime` |
| `line_no` | INTEGER | yes | line in `session_log.jsonl` |
| `file_path` | TEXT | yes | relative path to JSONL |
| `summary` | TEXT | optional | normalized retrieval summary |
| `raw_text` | TEXT | optional | required for user entries |
| `promotable` | INTEGER | yes | `0 / 1` |
| `claim_fingerprint` | TEXT | optional | semantic key |
| `created_at` | TEXT | yes | RFC3339 |

### 7.3 `memory_items`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | TEXT | yes | `mem_...` |
| `scope_type` | TEXT | yes | `user / repo / family` |
| `scope_id` | TEXT | yes | target scope |
| `item_key` | TEXT | yes | stable key |
| `file_path` | TEXT | yes | relative markdown path |
| `status` | TEXT | yes | `stable / disputed / rejected / quarantined` |
| `autoload` | TEXT | yes | `boot_user / boot_repo / manual_only` |
| `claim_fingerprint` | TEXT | yes | semantic key |
| `content_hash` | TEXT | yes | computed from content |
| `revision` | INTEGER | yes | monotonic |
| `deleted_at` | TEXT | optional | soft delete |

### 7.4 `repo_scopes`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `repo_id` | TEXT | yes | canonical repo identity |
| `git_remote_norm` | TEXT | yes | normalized remote |
| `repo_root_hint` | TEXT | yes | stable root hint |
| `created_at` | TEXT | yes | RFC3339 |

### 7.5 Status Transitions

```text
stable     -> disputed      (feedback or conflict detection)
stable     -> rejected      (explicit user rejection)
disputed   -> quarantined   (dreaming isolates)
disputed   -> stable        (feedback confirms)
any        -> rejected      (user explicitly rejects)
```

## 8. Failure Cases and Acceptance

Capability: table and file naming stays aligned  
Failure example: file叫 `session_log`，表仍叫 `evidence_items`  
Expected: one concept uses one canonical name  
Completion signal: canonical docs contain zero live references to `evidence_items`

Capability: trace index resolves to raw log lines  
Failure example: DB row exists but referenced file/line does not  
Expected: every `session_log_entries` row resolves to one JSONL line  
Completion signal: dangling trace row count = 0

## 9. Implementation Phases

Phase 1: rename schema entities  
Done when: `session_log_entries` replaces `evidence_items` in canonical schema

Phase 2: add routing tables  
Done when: repo/family relationships can be reconstructed without basename

Phase 3: remove obsolete projections  
Done when: `trace_archives` is no longer required for canonical flows
