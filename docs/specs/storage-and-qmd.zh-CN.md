# 存储与 QMD

> 状态：目标态设计 Draft。当前 canonical 方向是减少重复层与字段，但现有实现仍保留旧命名和旧投影，迁移前不能把这里的对象名直接当成当前实现名。

## 1. Problem and Goal

MemFold 同时使用文件系统、SQLite 和 QMD。这个设计只有在“谁拥有什么、谁可重建、谁只是索引”被明确写死时才不会走向重复和互相覆盖。

## 2. Scope and Non-Goals

In scope:
- 文件层、SQLite、QMD 的所有权边界
- 会话日志、历史总结、长期记忆的重建关系
- QMD 的索引职责

Out of scope:
- 远端检索服务
- 云端对象存储
- 让 QMD 成为正文真相源

## 3. System Boundaries

```text
memfold core
  ├── reads/writes ──► filesystem content
  ├── reads/writes ──► SQLite state
  └── rebuilds      ──► QMD indexes
```

## 4. Source of Truth Declaration

| Layer | Owns | Disposable |
|-------|------|------------|
| `stable/*.md` | 长期记忆正文 | No |
| `wiki/*.md` | 背景知识正文 | No |
| `sessions/*/session_log.jsonl` | 会话原始记录 | No |
| `history/daily/*.md` | 历史总结正文 | Yes |
| `state/memfold.db` | 状态、索引、路由、锁 | No |
| `boot/bundle.md` | compiled startup output only | Yes |
| `qmd/` | 搜索索引 | Yes |

## 5. Core Decisions

1. **What:** `session_log.jsonl` 是会话级原始内容真相源。 **Why:** 需要能验证“是不是用户真说过”。 **Reversal condition:** 如果未来有更低成本、更高可信的原始日志层。
2. **What:** `history/daily/*.md` 是整理后的历史总结，不再是会话日志镜像。 **Why:** 避免内容在 JSONL、Markdown、SQLite 三层重复。 **Reversal condition:** 如果历史总结被证明必须逐条镜像才能支撑 dreaming。
3. **What:** SQLite 负责追溯索引，不负责承载人读正文。 **Why:** 文件层和 DB 层职责必须分开。 **Reversal condition:** 如果 repair/rebuild 无法仅靠文件层恢复内容。
4. **What:** QMD 只做检索 sidecar。 **Why:** 它必须可以丢弃重建。 **Reversal condition:** 如果系统需要不可重建的向量写入状态。

## 6. Runtime Flows

### 6.1 Cross-Storage Write Order

```text
1. write SQLite mutation record (pending)
2. write content layer
   - session_log.jsonl or stable/*.md or history/*.md
3. update SQLite projection
4. rebuild derived layers
   - boot/bundle.md
   - qmd/
5. mark mutation fully_applied
```

Failure branch:
- 步骤 2 失败：mutation 保持 `pending`
- 步骤 3 失败：允许 repair 根据内容层重建 SQLite
- 步骤 4 失败：内容层和 SQLite 保留，派生层标记可重建

### 6.2 Repair Order

```text
content layer
  ├── stable/*.md
  ├── wiki/*.md
  ├── session_log.jsonl
  └── history/daily/*.md
        │
        ▼
rebuild SQLite projections
        │
        ▼
rebuild boot bundle + qmd
```

## 7. Data and Interface Contracts

### 7.1 Persistent Objects

| Object | Owner layer | Notes |
|--------|-------------|-------|
| `memory_item` | `stable/*.md` + `memory_items` projection | 人读正文 + 状态索引 |
| `session_log_entry` | `session_log.jsonl` + `session_log_entries` projection | 原始记录 + trace 索引 |
| `history_summary_block` | `history/daily/*.md` | 人读总结 |
| `boot_entry` | `boot/bundle.md` + `boot_entries` projection | 派生产物 |

### 7.2 QMD Collections

| Collection | Source |
|------------|--------|
| `stable` | `stable/*.md` |
| `wiki` | `wiki/*.md` |
| `history` | `history/daily/*.md` |
| `session_log` | `session_log.jsonl` 的可检索摘要投影（含可选 raw_text） |

## 8. Failure Cases and Acceptance

Capability: QMD is disposable  
Failure example: deleting `qmd/` destroys memory content  
Expected: deleting `qmd/` only removes indexes  
Completion signal: `qmd/` 删除后可从内容层与 SQLite 完整重建

Capability: history is rebuildable  
Failure example: deleting `history/*.md` loses unique facts unavailable elsewhere  
Expected: history can be reconstructed from session_log and runtime logs  
Completion signal: sampled history rebuild mismatch rate = 0

Capability: SQLite trace index is consistent  
Failure example: DB points to a missing `session_log` entry  
Expected: every `session_log_entries` row resolves to one JSONL line  
Completion signal: broken trace pointer rate = 0

Capability: derived layers respect rejection state
Failure example: a tombstoned claim is removed from SQLite state but still survives in `stable` markdown, boot bundle, or `qmd/`
Expected: repair / rebuild removes tombstoned stable blocks and QMD skips tombstoned claims during sidecar rebuild
Completion signal: tombstoned-claim resurrection rate in derived layers = 0

## 9. Implementation Phases

Phase 1: rename and ownership cleanup  
Done when: `session_log`, `history`, `stable`, `boot` ownership is documented without contradictions

Phase 2: repair and projection redesign  
Done when: SQLite projections can be rebuilt from content layer only

Phase 3: QMD and boot derivation update  
Done when: deleting `qmd/` and `boot/bundle.md` remains safe and reversible
