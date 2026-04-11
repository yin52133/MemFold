# 10 Host Integration

## 1. 这份文档讲什么

MemFold 核心是 CLI，宿主（Claude Code / Codex）通过两种机制调用它：

```
宿主集成机制
  │
  ├── hook（自动触发，无需 Claude 决策）
  │     · session 生命周期各节点自动执行
  │     · 保证基本信息一定被载入
  │     · 保证工作流水一定被记录
  │
  └── skill / tool（Claude 主动决策触发）
        · 按需检索
        · 主动标记值得晋升的条目
        · 用户显式要求时写入/查询
```

**核心原则：hook 保证不漏，skill 保证不乱。**

hook 负责兜底——不管 Claude 有没有意识到，启动包一定载入，工作流水一定记录。
skill 负责精度——promotable=1 的条目由 Claude 判断，不靠随机触发。

---

## 2. 两层触发机制的职责划分

```
┌─────────────────────────────────────────────────────────┐
│  hook 层（自动，低门槛）                                  │
│                                                         │
│  触发时机：session 生命周期事件                           │
│  write-evidence 默认 promotable=0                        │
│  目的：留审计轨迹，喂给 dreaming                          │
│                                                         │
│  session_start ──► memfold load（载入启动包）             │
│  each_turn_end ──► memfold write-evidence（流水）         │
│  session_end   ──► memfold write-evidence（总结）         │
│                    + 可选 memfold dream run               │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  skill 层（Claude 主动判断，高门槛）                      │
│                                                         │
│  触发条件：Claude 自己判断 OR 用户显式触发               │
│  write-evidence 带 promotable=1                          │
│  目的：标记值得晋升的高质量候选                           │
│                                                         │
│  用户说"记住这个" ──► write-evidence --promotable 1      │
│  发现"上次"/"之前" ──► memfold search                    │
│  用户说"这个不对" ──► memfold feedback + tombstone        │
│  Claude 判断需要上下文 ──► memfold search                 │
└─────────────────────────────────────────────────────────┘
```

**为什么 hook 写 promotable=0：**

hook 写的是完整工作流水，信噪比未经筛选。dreaming 在 Phase 2 会读这些流水，但不会自动把它们全部晋升。promotable=0 意味着这些条目只能被 dreaming 当信号参考，不会自动进入长期记忆。

**为什么 skill 写 promotable=1：**

Claude 判断"这条值得长期保留"时才标记 promotable=1。这类条目在下次 dreaming 时会进入候选池，经过四选一决策后才能晋升。

---

## 3. Session 生命周期全图

```
用户启动 Claude Code / Codex
        │
        ▼
  ┌─────────────────────────────────────┐
  │  hook: session_start                │
  │                                     │
  │  1. memfold load                    │
  │     --mode normal|fresh|sterile     │
  │     --scope-type project            │
  │     --scope-id <slug>               │
  │     --intent startup                │
  │                                     │
  │  2. 返回 bundle.md 内容             │
  │     宿主静默注入 system prompt      │
  └─────────────────────────────────────┘
        │
        ▼
  ┌─────────────────────────────────────┐
  │  对话进行中（每轮）                  │
  │                                     │
  │  hook: each_turn_end                │
  │    memfold write-evidence           │
  │    --source-kind decision/user/...  │
  │    --promotable 0                   │
  │    --summary <安全摘要>             │
  │                                     │
  │  skill: 按需触发（见第 4 节）        │
  └─────────────────────────────────────┘
        │
        ▼
  ┌─────────────────────────────────────┐
  │  hook: session_end                  │
  │                                     │
  │  1. memfold write-evidence          │
  │     --source-kind decision          │
  │     --summary <session 总结>        │
  │     --promotable 0                  │
  │                                     │
  │  2. 可选：memfold dream run         │
  │     --trigger session_end           │
  │     （V1 可跳过，改为 manual）      │
  └─────────────────────────────────────┘
```

---

## 4. Skill 触发条件表

Claude 在以下场景应主动调用对应 skill：

| 触发信号 | 调用命令 | 说明 |
|---------|---------|------|
| 用户说"记住这个" / "这很重要" | `write-evidence --promotable 1` | 标记晋升候选 |
| 用户说"你记得吗" / "上次" / "之前" | `memfold search --intent continue` | 按需检索 |
| 用户提到特定技术术语 / 背景概念 | `memfold search --intent knowledge_lookup` | 知识检索 |
| Claude 自己意识到缺少上下文 | `memfold search --intent continue` | Claude 主动补全 |
| 用户说"这个不对" / "忽略之前的" | `memfold feedback --verdict rejected` | 纠错 + tombstone |
| 用户明确偏好（语言/风格/习惯） | `write-evidence --promotable 1 --source-kind user` | 稳定偏好记录 |

**Claude 不应该触发 skill 的情况：**
- 仅仅因为话题相关就做检索（会产生噪声）
- 没有明确"记住"信号就写 promotable=1
- 在 sterile 模式下做任何检索

---

## 5. Claude Code 集成方案

Claude Code 通过 **hook** + **slash command（skill）** 集成。

### 5.1 Hook 配置

Claude Code 支持在 `.claude/settings.json` 中配置 hooks：

```json
{
  "hooks": {
    "PreToolUse": [],
    "PostToolUse": [],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "memfold write-evidence --scope-type project --scope-id $MEMFOLD_SCOPE_ID --session-id $MEMFOLD_SESSION_ID --source-kind decision --summary \"$MEMFOLD_TURN_SUMMARY\" --promotable 0 --origin-mode normal"
          }
        ]
      }
    ]
  }
}
```

Session 启动时，在 `CLAUDE.md` 或 system prompt 注入区执行：

```bash
memfold load \
  --mode normal \
  --scope-type project \
  --scope-id $(basename $PWD) \
  --intent startup
```

返回的 JSON 中 `items[].text` 拼接后注入 system prompt。

### 5.2 Skill（Slash Command）

在 `.claude/commands/` 下放置 skill 文件：

```
.claude/commands/
  memfold-search.md       # /memfold-search
  memfold-remember.md     # /memfold-remember
  memfold-forget.md       # /memfold-forget
  memfold-dream.md        # /memfold-dream
```

**`/memfold-search` 示例：**

```markdown
---
description: 从 MemFold 长期记忆中检索相关上下文
---

根据当前对话内容，调用：

memfold search \
  --scope-type project \
  --scope-id <当前项目 slug> \
  --intent continue \
  --query "<当前问题关键词>" \
  --budget 400

将返回结果注入当前回答的上下文中。
```

**`/memfold-remember` 示例：**

```markdown
---
description: 将当前对话中的重要结论标记为可晋升记忆
---

将用户明确要求记住的内容，调用：

memfold write-evidence \
  --scope-type project \
  --scope-id <当前项目 slug> \
  --session-id <当前 session id> \
  --source-kind user \
  --summary "<用户要求记住的内容摘要>" \
  --promotable 1 \
  --origin-mode normal
```

### 5.3 Claude Code 完整触发时序

```
用户打开 Claude Code（项目目录）
        │
        ▼
CLAUDE.md 中 memfold load 执行
返回 bundle.md 内容注入 system prompt
        │
        ▼
对话进行中
        │
  ┌─────┴──────────────────────────────────┐
  │  用户每轮对话结束                       │
  │  hook Stop 触发 write-evidence          │
  │  (promotable=0, source-kind=decision)   │
  └─────────────────────────────────────────┘
        │
  ┌─────┴──────────────────────────────────┐
  │  Claude 检测到触发信号                  │
  │  调用对应 skill（见第 4 节）            │
  └─────────────────────────────────────────┘
        │
        ▼
session 结束（用户关闭）
hook Stop 最后一次触发 write-evidence
（source-kind=decision, summary=session总结）
```

---

## 6. Codex 集成方案

Codex 通过 **`AGENTS.md` + tool definition** 集成。

### 6.1 AGENTS.md 中声明 MemFold 工具

在项目根目录 `AGENTS.md` 中加入：

```markdown
## Memory Tools

At the start of every session, run:

```bash
memfold load --mode normal --scope-type project --scope-id <project-slug> --intent startup
```

Inject the returned `items[].text` into context before answering.

Available memory tools:
- `memfold search` — retrieve relevant context when user references past work
- `memfold write-evidence --promotable 1` — when user says "remember this"
- `memfold feedback --verdict rejected` — when user says "this is wrong"
- `memfold dream run --trigger manual` — when explicitly asked to consolidate memory
```

### 6.2 Tool 定义（JSON Schema）

Codex 支持在 `AGENTS.md` 中声明 shell tool，或通过 OpenAI tool use 格式定义：

```json
{
  "name": "memfold_search",
  "description": "Search MemFold long-term memory for relevant context about past work, user preferences, or project constraints.",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query derived from current conversation"
      },
      "intent": {
        "type": "string",
        "enum": ["continue", "knowledge_lookup"],
        "description": "continue=retrieving past task context, knowledge_lookup=retrieving background knowledge"
      },
      "budget": {
        "type": "integer",
        "description": "Token budget for results, default 400"
      }
    },
    "required": ["query", "intent"]
  }
}
```

```json
{
  "name": "memfold_write_evidence",
  "description": "Record a work evidence entry. Use promotable=1 only when user explicitly says to remember something, or when a stable preference/constraint is established.",
  "parameters": {
    "type": "object",
    "properties": {
      "source_kind": {
        "type": "string",
        "enum": ["user", "tool", "code", "test", "decision", "feedback"]
      },
      "summary": {
        "type": "string",
        "description": "Safe summary, must not contain tokens/keys/cookies/passwords"
      },
      "promotable": {
        "type": "integer",
        "enum": [0, 1],
        "description": "0=audit trail only, 1=dreaming promotion candidate"
      }
    },
    "required": ["source_kind", "summary", "promotable"]
  }
}
```

### 6.3 Codex 完整触发时序

```
Codex 初始化 agent
        │
        ▼
AGENTS.md 中执行 memfold load
返回结果注入初始 context
        │
        ▼
agent 执行任务循环
        │
  ┌─────┴──────────────────────────────────┐
  │  每个 tool call 完成后                  │
  │  自动调用 memfold_write_evidence        │
  │  source_kind=tool, promotable=0         │
  └─────────────────────────────────────────┘
        │
  ┌─────┴──────────────────────────────────┐
  │  agent 判断触发条件                     │
  │  调用 memfold_search / write(p=1)       │
  └─────────────────────────────────────────┘
        │
        ▼
任务结束
调用 memfold_write_evidence
source_kind=decision, summary=<任务总结>, promotable=0
```

---

## 7. 两种宿主的对比

| 维度 | Claude Code | Codex |
|------|------------|-------|
| 自动载入机制 | `CLAUDE.md` + `hooks.Stop` | `AGENTS.md` 声明 |
| skill 入口 | `.claude/commands/*.md` slash command | tool definition (JSON Schema) |
| 每轮写入 | hook Stop 自动触发 | tool call 后自动触发 |
| 检索触发 | Claude 判断 + 用户显式 `/` 命令 | agent 判断调用 tool |
| session 边界 | 用户开关 Claude Code | agent 任务循环 |
| mode 控制 | `--mode` 参数 | `AGENTS.md` 中声明默认 mode |

---

## 8. 保证载入不漏的验收条件

以下任意一条失败，都属于集成失效：

```
□ 新 session 开始时，bundle.md 内容没有出现在 system prompt
□ 对话结束后，SQLite evidence_items 没有新增记录
□ 用户说"记住这个"后，没有出现 promotable=1 的条目
□ sterile 模式下出现了 memfold search 调用
□ fresh 模式下启动包包含了旧项目的 decision 类条目
```

---

## 9. 环境变量约定

hook 和 skill 需要读取的环境变量：

| 变量名 | 说明 | 示例 |
|--------|------|------|
| `MEMFOLD_ROOT` | MemFold 根目录 | `~/.memfold` |
| `MEMFOLD_SCOPE_ID` | 当前项目 slug | `my-project` |
| `MEMFOLD_SCOPE_TYPE` | `user` 或 `project` | `project` |
| `MEMFOLD_SESSION_ID` | 当前 session id | `sess_abc123` |
| `MEMFOLD_MODE` | 启动模式 | `normal` |

这些变量由宿主在启动时写入，或由 `memfold load` 执行后通过 stdout 返回供宿主设置。
