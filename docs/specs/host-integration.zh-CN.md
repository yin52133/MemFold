# 宿主集成

> 状态：目标态设计 Draft。当前已发货 hook/skill 仍以 `load / write-evidence / search / feedback / dream / repair` 和 `scope-type user|project` 为准。

## 1. Problem and Goal

MemFold 核心是 CLI，但真正的使用入口来自宿主的 hook、skill 和自动路由。集成设计必须保证：启动不漏、记录不乱、family 记忆不误注入、需要时能检索到相关 repo/family 记录。

## 2. Scope and Non-Goals

In scope:
- session_start / turn_end / session_end hook 责任
- skill/tool 的精细入口
- repo/family 自动路由
- trace 与运行日志集成

Out of scope:
- 宿主内部复制记忆规则
- 依赖 repo 自己维护临时 scope 声明文件

## 3. System Boundaries

```text
host hooks
  ├── session_start  ──► memfold load
  ├── turn_end       ──► memfold write-evidence
  └── session_end    ──► write-evidence summary + optional dream gate

host skills/tools
  ├── search         ──► memfold search
  ├── feedback       ──► memfold feedback
  ├── remember       ──► memfold write-evidence --promotable 1
  └── trace          ──► memfold trace find
```

## 4. Source of Truth Declaration

| Integration path | Owns |
|------------------|------|
| hooks | lifecycle-triggered baseline behavior |
| skills/tools | precision operations |
| memfold core | all actual memory decisions |

## 5. Core Decisions

1. **What:** hook 保证不漏，skill 保证不乱。 **Why:** lifecycle 记录要稳定触发，而长期记忆候选要有精度控制。 **Reversal condition:** 如果宿主能稳定提供更细粒度统一事件流。
2. **What:** session_end hook 负责触发 history summarization。 **Why:** history 是会话结束后的整理层。 **Reversal condition:** 如果总结生成被证明必须完全交给 dreaming。
3. **What:** family 路由由全局规则自动识别，不依赖 repo 临时声明文件。 **Why:** 设计应规避冲突，而不是把问题丢给使用者。 **Reversal condition:** 如果自动路由在关键仓库上稳定失败。

## 6. Runtime Flows

### 6.1 Session Start（current shipped）

```text
host starts session
  └──► resolve project scope
          ├── load user + project boot
          ├── do not load family boot
          └── return startup items
```

### 6.2 Turn End（current shipped）

```text
turn ends with meaningful state change
  └──► memfold write-evidence
          ├── user or system event -> sanitized summary required
          ├── duplicate/noise -> skip
          └── runtime logs record timing
```

### 6.3 Session End（current shipped + target extension）

```text
session ends
  └──► write-evidence session summary
          ├── update session metadata
          ├── maybe run scheduled dream gate
          ├── target extension: summarize-history
          └── append runtime completion logs
```

### 6.4 Retrieval with Family

```text
user mentions repo / project / directory
  └──► resolve repo scope
          ├── query repo first
          ├── query linked family second
          ├── query user third
          └── return source-labeled results
```

## 7. Data and Interface Contracts

### 7.1 Hook Inputs

| Variable | Required | Notes |
|----------|----------|-------|
| `MEMFOLD_SCOPE_TYPE` | yes | current shipped: `user / project`; target migration extends routing internally |
| `MEMFOLD_SCOPE_ID` | yes | resolved scope |
| `MEMFOLD_SESSION_ID` | yes | session identity |
| `MEMFOLD_MODE` | yes | `normal / fresh / sterile` |
| `MEMFOLD_SOURCE_KIND` | conditional | entry source kind |

### 7.2 Hook Guarantees

| Hook | Must do |
|------|---------|
| `session_start` | load user + project boot |
| `turn_end` | record meaningful structured entry or skip with reason |
| `session_end` | write session summary, close session metadata, optionally trigger dream gate |

## 8. Failure Cases and Acceptance

Capability: family memory does not leak into startup  
Failure example: `family-alpha` rule appears automatically in unrelated repo startup items  
Expected: family only appears in retrieval results  
Completion signal: family startup injection rate = 0

Capability: user-origin remember path remains precise  
Failure example: random hook summary becomes stable user preference  
Expected: only explicit remember/tool path writes promotable user preference candidates  
Completion signal: hook-origin accidental stable promotion rate = 0

Capability: session_end produces readable history  
Failure example: history is missing or is just a raw session mirror  
Expected: session_end writes a concise summary block to daily history  
Completion signal: history generation success rate = 100% for completed sessions

## 9. Implementation Phases

Phase 1: update hook contracts and host docs  
Done when: hook variable names and responsibilities are explicit

Phase 2: add family routing rules  
Done when: repo mention retrieval can automatically include linked family results

Phase 3: replace canonical evidence terminology  
Done when: host docs use `session_log` language as primary terminology
