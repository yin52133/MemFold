# MemFold 架构总览

> 状态：目标态设计 Draft。当前已发货的宿主接口仍以 `write-evidence --scope-type project` 等现有命令为准；这里描述的是在保持原五层骨架下的目标增量演进。

## 1. Problem and Goal

agent 在多个仓库、多次 session 中工作时，容易同时出现两类问题：一是把旧上下文和错误路径默认带进新任务，二是稳定偏好和长期约束不断丢失。MemFold 的目标是在本地提供一套分层记忆系统：启动时只加载最小稳定上下文，工作中记录可验证的会话日志，周期性整理为历史总结和长期记忆。

## 2. Scope and Non-Goals

In scope:
- 启动时加载用户级和仓库级最小稳定上下文
- 工作中记录会话日志，并保存用户原话以支持追溯验证
- 生成按日历史总结，供人读和 dreaming 采集信号
- 通过 dreaming 把稳定信息提升到长期记忆
- 自动识别 `repo scope` 和 `family scope`
- 记录运行阶段耗时与失败信息，并写入落盘日志

Out of scope:
- 保存完整 tool 原始输出全文
- 把 family 记忆默认注入启动上下文
- 多机同步或远端托管记忆
- 图数据库或关系图谱

## 3. System Boundaries

```text
Codex
  └── calls (env vars) ──► memfold CLI ──► ~/.codex/memfold/
                                              ├── memory/
                                              ├── state/memfold.db
                                              ├── qmd/
                                              └── runtime/logs/

Claude Code
  └── calls (stdin JSON) ──► memfold CLI ──► ~/.claude/memfold/
                                                ├── memory/
                                                ├── state/memfold.db
                                                ├── qmd/
                                                └── runtime/logs/
```

两个宿主共享同一个 memfold 二进制，但记忆完全隔离。

MemFold 负责：
- scope 路由
- 分层加载与检索
- 会话日志记录
- 历史总结整理
- dreaming、反馈、repair、bundle 重编

宿主只负责：
- 触发 CLI / hook / skill
- 提供当前 workspace、session 和用户输入

## 4. Source of Truth Declaration

| Layer | Owns | Disposable |
|-------|------|------------|
| `memory/*/stable/*.md` | 长期记忆正文 | No |
| `memory/*/sessions/*/session_log.jsonl` | 会话级原始记录 | No |
| `memory/*/history/daily/*.md` | 历史总结正文 | Yes |
| `memory/*/boot/bundle.md` | nothing, compiled startup bundle | Yes |
| `state/memfold.db` | 状态、索引、路由、锁、作业 | No |
| `qmd/` | 检索索引 sidecar | Yes |
| `runtime/logs/*.jsonl` | 运行日志归档 | Yes |

说明：
- `session_log.jsonl` 是“是否真是用户说过”的唯一验证基础。
- `history/daily/*.md` 不是会话日志镜像，而是整理后的历史总结。
- `boot/bundle.md` 只服务启动加载，不承担追溯职责。

## 5. Core Decisions

1. **What:** 保留五层结构，但把原 `evidence` 明确为 `session_log`，把原 `archive` 明确为 `history`。 **Why:** 原设计的分层是对的，但“工作记录”和“历史档案”职责过于混杂。 **Reversal condition:** 如果五层结构被证明无法支撑启动最小化和后续检索分离。
2. **What:** 只在 `session_log` 中保存用户原话，系统动作只保存结构化摘要。 **Why:** 需要可验证“是不是用户真说过”，但不需要保存全部 tool 原文。 **Reversal condition:** 如果后续发现系统动作的全文也是关键证据。
3. **What:** `history` 只保存整理后的关键事件和结论，不再镜像 `session_log`。 **Why:** 避免同一条信息在 JSONL、Markdown、SQLite 多次重复。 **Reversal condition:** 如果 dreaming 或人工追溯必须依赖逐条 Markdown 镜像。
4. **What:** 启动只加载 `user + repo`，`family` 只参与检索。 **Why:** family 共享记忆有用，但不应默认污染当前上下文。 **Reversal condition:** 如果多个 family 工程的共享约束被证明必须在启动时默认注入。
5. **What:** `repo_id` 必须由稳定 git 身份自动生成，不允许继续用目录 basename。 **Why:** basename 会冲突，且移动本地路径会改变身份。 **Reversal condition:** 如果宿主无法稳定提供 git 身份。
6. **What:** Markdown 面向人读，长 hash 和内部追溯字段留在 SQLite。 **Why:** 当前 Markdown token 成本高，且对人没有直接帮助。 **Reversal condition:** 如果人工编辑和 repair 必须依赖这些字段在文件正文中显式存在。
7. **What:** 运行阶段必须同时有实时进度和落盘日志。 **Why:** 长耗时阶段没有可观测性，会让系统看起来卡死。 **Reversal condition:** 如果所有关键路径都足够快且无排障需求。

## 6. Runtime Flows

### 6.1 Startup Loading（current shipped host surface）

```text
host starts session
  └──► memfold load --scope-type project --scope-id <project_slug> --intent startup
          ├── resolve current project scope
          ├── load user boot bundle
          ├── load project boot bundle
          ├── if scope resolution fails ──► return named error
          └── success ──► {items, total_tokens_estimate, degraded}
```

### 6.2 Session Recording（current shipped host surface）

```text
user message / key system action
  └──► memfold write-evidence
          ├── append sessions/<id>/session_log.jsonl
          ├── project into SQLite session_log_entries
          ├── update sessions table
          ├── if write fails ──► mutation stays pending, return named error
          └── success ──► {evidence_id, stored:true}
```

### 6.3 History Summarization（target migration）

```text
session end / manual summarize / repair rebuild
  └──► memfold summarize-history
          ├── read session_log + runtime logs
          ├── extract key events, decisions, failures, feedback
          ├── write history/daily/<date>.md
          ├── if summarization fails ──► leave session_log intact, return named error
          └── success ──► {summary_id, updated:true}
```

### 6.4 Dreaming

```text
manual run / scheduled gate satisfied
  └──► memfold dream run
          ├── read stable/*
          ├── read history/daily/*.md
          ├── targeted read session_log when needed
          ├── decide keep / hold / quarantine / discard
          ├── write stable/*
          ├── rebuild boot bundle and qmd
          ├── if decision/write fails ──► job stays failed with reason
          └── success ──► {promoted, held, quarantined, discarded}
```

### 6.5 Trace Query（target migration）

```text
user asks "is this really what I said?"
  └──► memfold trace find --query <...>
          ├── search SQLite indexes
          ├── locate repo_id / session_id / line_no
          ├── open session_log.jsonl line
          ├── if not found ──► return TRACE_NOT_FOUND
          └── success ──► raw user entry + trace metadata
```

## 7. Data and Interface Contracts

Canonical persistent objects:
- `memory_item` → see `memory-item-schema.zh-CN.md`
- `session_log_entry` → see `memory-item-schema.zh-CN.md`
- `history_summary_block` → see `memory-item-schema.zh-CN.md`
- `tombstone` → see `memory-item-schema.zh-CN.md`
- `boot_entry` → see `memory-item-schema.zh-CN.md`

Canonical runtime contracts:
- `memfold load`
- `memfold write-evidence`（current shipped）
- `memfold summarize-history`
- `memfold search`
- `memfold feedback`
- `memfold dream run`
- `memfold qmd sync`
- `memfold repair`

Detailed command contracts live in `cli-contract.zh-CN.md`.

## 8. Failure Cases and Acceptance

Capability: startup loading is deterministic  
Failure example: same repo snapshot returns different boot items across consecutive calls  
Expected: same `mode + scope + intent` returns same item set and order  
Completion signal: two consecutive `load` calls return byte-identical item lists

Capability: exact token retrieval beats generic history overlap
Failure example: searching for a long token-like query returns old generic history rows before the exact fresh hook/session hit
Expected: once an exact token match exists, retrieval prioritizes exact hits and can narrow the result set to those hits
Completion signal: exact-token miss / mis-rank rate = 0

Capability: user quote trace is verifiable  
Failure example: history/stable claims a user preference, but no raw user entry can be found  
Expected: every promoted user preference can be traced back to a `session_log` user entry  
Completion signal: trace miss rate for promoted user-origin items = 0

Capability: history is not a session mirror  
Failure example: every `session_log` entry is copied into `history/daily/*.md` one by one  
Expected: history only keeps key events and conclusions  
Completion signal: average history blocks per session is materially lower than session_log entries

Capability: rejected claims cannot re-enter stable  
Failure example: tombstoned claim reappears in long-term memory  
Expected: `claim_fingerprint` match blocks re-promotion regardless of wording changes  
Completion signal: tombstone bypass rate = 0

Capability: sensitive content never enters stable or history  
Failure example: token / key / cookie appears in Markdown memory layers  
Expected: raw sensitive content remains absent from `stable` and `history`  
Completion signal: sensitive content leak rate in human-readable layers = 0

## 9. Implementation Phases

Phase 1: canonical docs rewrite  
Includes: architecture, storage, filesystem, schema, CLI, host integration  
Done when: `docs/specs/` no longer contains dated canonical specs and all layer/contract names are consistent  
Blocks: implementation changes depend on stable canonical spec

Phase 2: naming and schema migration  
Includes: `session_log` rename, `history` responsibility update, SQLite table renames  
Done when: file paths and table names migrate while shipped CLI remains explicitly documented until cutover
Blocks: data reset and new write path depend on this migration

Phase 3: write path and history generation  
Includes: session_log write, history summarize, raw user quote persistence  
Done when: one user message can be written, summarized, and traced back from SQLite to JSONL  
Blocks: dreaming update depends on this path

Phase 4: dreaming and retrieval update  
Includes: new gather order, repo/family routing, boot rebuild, QMD sync  
Done when: promoted stable items come from traceable session_log entries and family scope only appears in retrieval  
Blocks: runtime observability depends on flow stability

Phase 5: runtime observability and data reset  
Includes: progress output, runtime logs, clearing old memory data, rebuilding clean state  
Done when: long-running commands show stage progress and old duplicated memory layout is removed
