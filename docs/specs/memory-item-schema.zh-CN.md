# 记忆对象 Schema

> 状态：目标态设计 Draft。当前实现中的人读文件和投影字段仍未完全收口；这里定义的是迁移完成后的对象契约。

## 1. Problem and Goal

MemFold 需要一组最小、明确、可重建的数据对象，避免同一条信息在人读层和机器层重复出现但语义不清。

## 2. Scope and Non-Goals

In scope:
- 长期记忆条目
- 会话日志条目
- 历史总结块
- 墓碑
- 启动包条目

Out of scope:
- 完整 tool 原始输出对象
- 把运行日志直接提升为长期记忆

## 3. System Boundaries

```text
session_log.jsonl
  └── raw session record
history/daily/*.md
  └── human summary block
stable/*.md
  └── long-term memory item
boot/bundle.md
  └── compiled startup item
```

## 4. Source of Truth Declaration

| Object | Truth layer |
|--------|-------------|
| `memory_item` | `stable/*.md` |
| `session_log_entry` | `session_log.jsonl` |
| `history_summary_block` | `history/daily/*.md` |
| `tombstone` | SQLite |
| `boot_entry` | `boot/bundle.md` |

## 5. Core Decisions

1. **What:** `memory_item` 不再要求 `title` 出现在人读文件中。 **Why:** 当前 `title` 与正文重复，没有额外信息。 **Reversal condition:** 如果后续出现明确的标题导航需求。
2. **What:** `session_log_entry` 对 user 记录保存 `raw_text`。 **Why:** 需要验证“是不是用户真说过”。 **Reversal condition:** 如果原话存储被判定为不可接受。
3. **What:** `history_summary_block` 只保留简洁人读字段。 **Why:** 它不是 trace index。 **Reversal condition:** 如果人工排障必须依赖 history 中的内部追溯字段。

## 6. Data and Interface Contracts

### 6.1 长期记忆条目 `memory_item`

存放于 `stable/*.md`，每个 block 一条。

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `item_key` | string | yes | format: `scope.category.name` |
| `status` | enum | yes | `stable / disputed / rejected / quarantined` |
| `autoload` | enum | yes | `boot_user / boot_repo / manual_only` |
| `claim_fingerprint_short` | string | optional | short human-readable suffix |
| `revision` | integer | yes | monotonic |
| `body` | string | yes | human-readable memory text |

Markdown example:

```markdown
## item_key: user.preference.truthfulness
autoload: boot_user
claim_fingerprint_short: cfp_187b967ca75c
revision: 1

实事求是，不要奉承；如果用户前提有误，要直接指出并解释原因。
```

### 6.2 会话日志条目 `session_log_entry`

存放于 `sessions/<session-id>/session_log.jsonl`，append-only。

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `entry_id` | string | yes | `sl_...` |
| `session_id` | string | yes | owning session |
| `scope_type` | enum | yes | `user / repo / family` |
| `scope_id` | string | yes | target scope |
| `source_kind` | enum | yes | `user / decision / feedback / code / test / tool / runtime` |
| `raw_text` | string | conditional | required for `source_kind=user` |
| `summary` | string | optional | normalized retrieval summary |
| `promotable` | integer | yes | `0 / 1` |
| `claim_fingerprint` | string | optional | semantic key |
| `created_at` | string | yes | RFC3339 |

JSONL example:

```json
{"entry_id":"sl_001","session_id":"sess_abc","scope_type":"repo","scope_id":"repo_openai_memfold","source_kind":"user","raw_text":"以后不要奉承我，要判断我说得对不对。","summary":"用户要求实事求是，不要奉承。","promotable":1,"claim_fingerprint":"cfp_187b967ca75c9074","created_at":"2026-04-13T10:00:00Z"}
```

### 6.3 历史总结块 `history_summary_block`

存放于 `history/daily/<YYYY-MM-DD>.md`。

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `time` | string | yes | RFC3339 or local timestamp |
| `source_kind` | enum | yes | same enum as session log |
| `summary` | string | yes | human-readable short summary |

Markdown example:

```markdown
## 2026-04-13T10:30:00Z [decision]
修复 scheduled dream 时间戳兼容问题，并重新部署全局 memfold。
```

### 6.4 墓碑 `tombstone`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `claim_fingerprint` | string | yes | semantic key |
| `scope_type` | enum | yes | `user / repo / family` |
| `scope_id` | string | yes | target scope |
| `reason` | string | yes | reject reason |
| `created_at` | string | yes | RFC3339 |

### 6.5 启动包条目 `boot_entry`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `item_key` | string | yes | source memory item |
| `text` | string | yes | injected startup text |
| `source_item_id` | string | yes | source projection id |
| `token_estimate` | integer | yes | token budget value |

## 7. Failure Cases and Acceptance

Capability: stable item body is not duplicated  
Failure example: title and body repeat the same sentence  
Expected: human-readable files contain one正文版本  
Completion signal: duplicate title/body block count = 0

Capability: user quote can be verified  
Failure example: promoted user preference lacks raw user text in session log  
Expected: user-origin promoted claims trace to one `raw_text` field  
Completion signal: raw quote trace miss rate = 0

## 8. Implementation Phases

Phase 1: slim human-readable objects  
Done when: `stable` and `history` examples no longer contain long machine metadata

Phase 2: session log traceability  
Done when: user-origin entries keep raw text and can be promoted through trace validation
