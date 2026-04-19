# Dreaming 与记忆保留

> 状态：目标态设计 Draft。当前 dreaming 实现仍基于现有 `write-evidence` / `project` surface；这里定义的是在增量迁移完成后的职责与采集顺序。

## 1. Problem and Goal

工作中的会话日志不能直接变成长期记忆，否则系统会把临时路径、错误结论和未验证判断带入未来 session。dreaming 的职责是从 `session_log` 和 `history` 中提炼稳定信号，把真正该保留的内容写入 `stable`。

## 2. Scope and Non-Goals

In scope:
- manual 与 scheduled dreaming
- `keep / hold / quarantine / discard` 四选一
- `stable` 写入和 `boot` 重编
- tombstone 防复活

Out of scope:
- 自由生成新事实
- 把所有会话记录都自动升成长期记忆
- 把 history 当作原始证据层

## 3. System Boundaries

```text
host / operator
  └── calls ──► memfold dream run
                  ├── reads stable/*
                  ├── reads history/daily/*.md
                  ├── targeted reads session_log.jsonl
                  ├── writes stable/*
                  ├── rebuilds boot + qmd
                  └── updates SQLite state
```

## 4. Source of Truth Declaration

| Layer | Dreaming reads | Dreaming writes |
|-------|----------------|-----------------|
| `stable/*.md` | Yes | Yes |
| `history/daily/*.md` | Yes | No |
| `sessions/*/session_log.jsonl` | targeted only | No |
| `boot/bundle.md` | No | rebuild only |
| `memfold.db` | Yes | Yes |

## 5. Core Decisions

1. **What:** Dreaming 先读 `stable`，再读 `history`，最后按需窄读 `session_log`。 **Why:** `history` 是低成本信号层，`session_log` 是高成本证据层。 **Reversal condition:** 如果 `history` 无法提供足够信号。
2. **What:** `history` 不再作为逐条镜像输入。 **Why:** 否则 dreaming 只是重新扫描另一份会话日志副本。 **Reversal condition:** 如果丢失逐条镜像会明显降低准确率。
3. **What:** 提升到 `stable` 的用户偏好必须带显式 `raw_text` 证据，而不是由 summary 反推。 **Why:** 否则无法验证“是不是用户真说过”。 **Reversal condition:** 如果系统接受 summary-only 的不可验证提升。
4. **What:** `claim_fingerprint` 继续作为 tombstone 的语义键，但不再要求出现在人读 Markdown 中。 **Why:** 它是决策约束，不是人读正文。 **Reversal condition:** 如果 repair 必须依赖 Markdown 里的完整指纹。

## 6. Runtime Flows

### 6.1 Manual Dreaming

```text
operator triggers manual dream
  └──► memfold dream run --trigger manual
          ├── read stable state
          ├── gather history signals
          ├── open targeted session_log evidence when needed
          ├── decide keep / hold / quarantine / discard
          ├── write stable
          ├── rebuild boot + qmd
          └── return counts
```

### 6.2 Scheduled Dreaming

```text
session end / scheduler
  └──► check gate
          ├── >= 24h since last applied dream
          ├── >= 5 ended sessions since last applied dream
          ├── if gate fails ──► return not eligible
          └── if gate passes ──► run same flow as manual
```

### 6.3 Promotion Validation

```text
candidate from session_log/history
  └──► check tombstone
          ├── hit ──► discard
          └── miss ──► check stability rules
                          ├── user claim with no explicit raw_text ──► discard
                          ├── stable preference / repeated constraint ──► keep
                          ├── weak single-session signal ──► hold
                          ├── conflict with long-term memory ──► quarantine
                          └── noise / wrong / irrelevant ──► discard
```

## 7. Data and Interface Contracts

### 7.1 Dream Candidate Minimum Fields

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `scope_type` | enum | yes | `user / repo / family` |
| `scope_id` | string | yes | target scope |
| `source_layer` | enum | yes | `history / session_log` |
| `source_ref` | string | yes | pointer to summary block or JSONL line |
| `source_kind` | enum | yes | `user / decision / feedback / code / test / tool / runtime` |
| `summary` | string | yes | normalized candidate summary |
| `claim_fingerprint` | string | optional | required for tombstone-protected claims |

### 7.2 Command Contract

`memfold dream run`

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `--scope-type` | enum | yes | `user / repo / family` |
| `--scope-id` | string | yes | target scope |
| `--trigger` | enum | yes | `manual / scheduled` |

success:

```json
{
  "job_id": "dj_...",
  "promoted": 1,
  "held": 2,
  "quarantined": 0,
  "discarded": 4
}
```

named errors:
- `DREAM_NOT_ELIGIBLE`
- `INVALID_SOURCE_REF`
- `TRACE_VALIDATION_FAILED`
- `STABLE_WRITE_FAILED`
- `BOOT_REBUILD_FAILED`

## 8. Failure Cases and Acceptance

Capability: promoted user preferences are traceable  
Failure example: `stable` contains a user preference but no raw user entry exists in session_log  
Expected: every promoted user-origin item resolves to one explicit raw user message  
Completion signal: trace miss rate for promoted user-origin items = 0

Capability: history is sufficient as first gather layer  
Failure example: dreaming must full-scan all session logs to promote common stable rules  
Expected: history provides first-pass signals, session_log is only targeted fallback  
Completion signal: full session_log scan rate during dream = 0 in standard cases

Capability: tombstones block re-promotion  
Failure example: rejected claim returns with different wording  
Expected: fingerprint match blocks stable write  
Completion signal: tombstone bypass rate = 0

Capability: repair does not resurrect rejected memory
Failure example: a tombstoned claim disappears from SQLite state but survives in `stable` markdown / boot / qmd and returns after repair
Expected: repair removes tombstoned stable blocks and rebuilt sidecars continue to exclude the rejected claim
Completion signal: tombstoned-claim resurrection rate after repair = 0

## 9. Implementation Phases

Phase 1: update gather order and source refs  
Done when: dreaming reads `history` first and only targeted `session_log` entries second

Phase 2: update promotion validation  
Done when: promoted user-origin items require raw trace validation

Phase 3: update outputs  
Done when: `stable` writes remain human-readable while boot/qmd rebuilds remain correct
