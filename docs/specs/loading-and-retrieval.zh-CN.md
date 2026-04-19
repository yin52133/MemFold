# MemFold 加载与检索

> 状态：目标态设计 Draft。当前已发货宿主接口仍以 `scope-type user|project` 和 `write-evidence` 为准；这里描述的是在原五层骨架上演进后的加载与检索目标。

## 1. 范围

这份文档定义四条运行流：
- 启动加载流
- 续做检索流
- 知识检索流
- 证据追溯流

## 2. 启动加载流

```text
host starts session
  └──► resolve current project scope
          ├── success ──► memfold load --scope-type project --scope-id <project_slug> --intent startup
          │                 ├── read user boot bundle
          │                 ├── read project boot bundle
          │                 └── return item list
          └── failure ──► REPO_SCOPE_RESOLUTION_FAILED
```

启动时只返回：
- 用户级启动包条目
- 当前 repo 启动包条目

不会自动返回：
- family 共享记忆
- `session_log` 原始记录
- `wiki`
- `history`

## 3. 续做检索流

```text
user asks to continue / references prior repo work
  └──► memfold search --intent continue
          ├── query repo scope first
          ├── query family scope second
          ├── query user scope last
          ├── hide tombstoned claims from retrieval results
          ├── dedupe identical summaries across layers
          ├── prefer stable when a raw-text hit maps to the same promoted claim
          ├── if exact trace needed ──► open session_log
          └── return ranked results with source labels
```

返回结果必须标注来源：
- `user`
- `repo`
- `family`
- `history`
- `session_log`

## 4. 知识检索流

```text
user asks about framework / background / concept
  └──► memfold search --intent knowledge_lookup
          ├── query repo wiki
          ├── query family wiki/stable
          ├── query history summaries
          ├── only then query session_log
          └── return ranked results
```

知识检索不应默认回到原始 session 记录，除非：
- 用户显式要求验证原话
- 高层结果互相冲突
- 需要确认某条结论是否真实发生

## 5. 证据追溯流

```text
user asks "is this really what I said?"
  └──► memfold trace find --query <...>
          ├── search SQLite session_log_entries
          ├── resolve repo_id + session_id + line_no
          ├── open sessions/<session-id>/session_log.jsonl
          ├── extract raw user entry
          └── return raw record + trace metadata
```

这条流的目标不是找“总结”，而是找“证据”。

## 6. 加载预算

启动预算只约束 `boot bundle`：
- `normal`：用户 + repo
- `fresh`：只读用户
- `sterile`：不读任何记忆层
- 如果 `user + repo` 最终正文完全相同，启动注入只保留一份正文，避免重复占用 token

检索预算作用于：
- 返回条目数
- 下钻层数
- QMD 命中后回读文件数

## 7. Failure Cases

| 场景 | Named error | 行为 |
|------|-------------|------|
| repo_id 无法解析 | `REPO_SCOPE_RESOLUTION_FAILED` | 不加载 repo 记忆，宿主可退回 user-only |
| family 路由缺失 | `FAMILY_ROUTE_NOT_FOUND` | 只查 repo + user |
| trace 找不到原始记录 | `TRACE_NOT_FOUND` | 返回失败，不得用 summary 冒充原话 |
| session_log 文件损坏 | `SESSION_LOG_CORRUPTED` | 返回失败，并建议 repair |

## 8. Acceptance

Capability: startup only loads minimal context  
Failure example: family/shared items被默认注入启动上下文  
Expected: startup item set only contains `user + repo` boot items  
Completion signal: startup family injection rate = 0

Capability: startup does not duplicate identical prose across scopes
Failure example: user boot 和 repo boot 各有一条正文完全相同的 memory，启动时被注入两次
Expected: identical injected text is deduplicated before entering startup context
Completion signal: duplicate startup text block count = 0

Capability: trace can verify promoted user claims  
Failure example: stable 中存在用户偏好，但无法回到原始 user entry  
Expected: promoted user-origin item can resolve to one raw `session_log` entry  
Completion signal: trace success rate for promoted user-origin items = 100%

Capability: rejected claims stop resurfacing through search
Failure example: feedback 已明确拒绝的 claim 仍然从 stable / session_log 检索结果里返回
Expected: tombstoned claim_fingerprint is filtered out before results are returned
Completion signal: rejected-claim search resurfacing rate = 0
