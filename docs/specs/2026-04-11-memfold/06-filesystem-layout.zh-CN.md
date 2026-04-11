# 06 Filesystem Layout

## 1. 根目录

第一版约定 MemFold 的本地根目录为：

```
~/.memfold/
```

如果宿主需要按项目隔离，使用：

```
~/.memfold/projects/<project-slug>/
```

## 2. 完整布局

```
~/.memfold/
├── config/
│   ├── config.toml                    全局配置
│   └── hosts/
│       ├── claude-code.toml           Claude Code 宿主配置
│       ├── codex.toml                 Codex 宿主配置
│       └── openclaw.toml              OpenClaw 宿主配置
│
├── state/
│   ├── memfold.db                     SQLite（唯一状态真相源）
│   ├── backups/                       备份
│   └── migrations/                   迁移脚本
│
├── memory/
│   ├── user/
│   │   ├── stable/
│   │   │   ├── profile.md             用户画像（user/feedback 类记忆）
│   │   │   └── preferences.md        用户稳定偏好
│   │   ├── boot/
│   │   │   └── bundle.md             用户级启动包（编译产物，非真相源）
│   │   └── wiki/                     用户级背景知识页
│   │
│   └── projects/
│       └── <project-slug>/
│           ├── stable/
│           │   ├── project-card.md    项目介绍 + 稳定约束
│           │   └── rules.md           项目规则
│           ├── boot/
│           │   └── bundle.md         项目级启动包（编译产物，非真相源）
│           ├── wiki/                 项目背景知识页
│           ├── archive/
│           │   ├── memory-2026-04-01.md   日期归档（人可读，dreaming 信号源）
│           │   ├── memory-2026-04-02.md
│           │   └── ...
│           └── sessions/
│               └── <session-id>/
│                   ├── evidence.jsonl     工作记录正文（append-only）
│                   ├── raw-events.jsonl   原始事件投影
│                   └── run-meta.json      session 元数据
│
├── qmd/
│   ├── config/                        QMD 配置
│   ├── collections/                   索引集合
│   └── cache/                        索引缓存
│
└── runtime/
    ├── cache/                         运行时缓存
    ├── jobs/                          待执行作业
    └── logs/                          运行日志
```

## 3. 每个目录的职责

| 路径 | 作用 | 是否正文真相源 |
|------|------|-------------|
| `config/` | 全局配置、宿主配置 | 是 |
| `state/memfold.db` | SQLite 状态库 | 是（状态真相源） |
| `memory/user/stable/` | 用户级长期记忆 | 是 |
| `memory/projects/*/stable/` | 项目级长期记忆 | 是 |
| `memory/*/boot/bundle.md` | 启动包编译结果 | 否（派生物） |
| `memory/*/wiki/` | 背景知识页 | 是 |
| `memory/*/archive/memory-*.md` | 按日期归档的工作日志 | 是 |
| `memory/*/sessions/*/evidence.jsonl` | 工作记录正文 | 是 |
| `memory/*/sessions/*/raw-events.jsonl` | 原始事件投影 | 是 |
| `qmd/` | 索引侧边车 | 否（可丢弃重建） |
| `runtime/` | 运行时缓存、作业、日志 | 否 |

## 4. 历史档案：按日期归档

每次 `write-evidence` 调用时，除了写入 `sessions/*/evidence.jsonl`，还追加到当日的历史档案文件：

```
archive/memory-YYYY-MM-DD.md
```

这个文件是**人可读的工作日志**，不是机器格式。每天一个文件，append-only。

格式示例：

```markdown
# 2026-04-11

## 10:32 [user] 用户明确要求默认用中文回答
source_kind: user
session: sess_abc123
promotable: true

## 14:15 [decision] 确认 claim_fingerprint 用 SHA-256 前 16 字节
source_kind: decision
session: sess_abc123
promotable: true

## 16:40 [feedback] 用户说"这个路径不对，忽略之前的判断"
source_kind: feedback
session: sess_xyz789
promotable: false
```

Dreaming 在 Phase 2 Gather 时优先读这些文件，而不是全文扫描 evidence.jsonl。

## 5. 启动时实际读哪些文件

默认只读：

```
memory/user/boot/bundle.md
memory/projects/<project-slug>/boot/bundle.md
```

不会在启动时自动读：
- `wiki/`
- `archive/`
- `sessions/`

这些只在续做流、知识检索流、dreaming 时才会被访问。

## 6. 哪些文件允许手工编辑

允许手工编辑：
- `stable/*.md`
- `wiki/*.md`
- `config/*.toml`

不建议手工编辑：
- `boot/bundle.md`（应由 `bundle compile` 生成）
- `state/memfold.db`
- `qmd/`
- `runtime/`
- `sessions/*.jsonl`（append-only，手工改会破坏 content_hash）
- `archive/memory-*.md`（append-only）

如果手工编辑了 `stable/*.md`，需要手动触发 `bundle compile` 和 `qmd sync` 来重建派生层。

## 7. Archive 不清理原则与删除传播要求

### 7.1 Archive 是永久审计轨迹

**Archive 文件不会因为 dreaming 而被清理。**

```
正常流：
  archive/memory-*.md  ──► dreaming Phase 2 读取信号
        │                        │
        │ (append-only，不动)    ▼
        │               整理结果写入 stable/*.md
        └──────────────────────────────────────────
                archive 原封不动保留

异常流（发现 stable 里的记忆不对齐）：
  stable/*.md 有问题条目
        │
        ▼
  回查 archive/memory-*.md
  检查 dreaming 当时的原始信号是否归档有误
        │
        ▼
  修正 stable/*.md（不动 archive）
```

Archive 的价值正是它的完整性——它记录的是"系统当时看到了什么"，而不是"系统最终认为是什么"。dreaming 的结论有误时，archive 是唯一可以溯源的真相轨迹。

### 7.2 何时会触发删除

删除不是常规操作，只在以下三种情况发生：

| 情况 | 说明 |
|------|------|
| 用户主动要求删除 / 脱敏 | "帮我删掉那条记录"、隐私合规要求 |
| 发现敏感信息泄漏 | token / key / cookie 意外写入了 summary |
| sterile 模式结束后一键清理 | 临时实验 session 的全部痕迹 |

### 7.3 删除传播规则

触发删除后，必须同步到所有层：

```
删除 stable/*.md 中的条目
      │
      ├──► SQLite memory_items: deleted_at 落地
      ├──► boot/bundle.md 重编（如果该条目在启动包中）
      └──► QMD 索引：移除相关条目

删除 archive/memory-*.md 中的内容（仅隐私/脱敏请求时）
      │
      ├──► SQLite trace_archives: deleted_at 落地
      └──► QMD 索引：移除相关条目
      注意：archive 是 append-only，脱敏只能替换原文为 [REDACTED]，
            不能物理删除行（否则破坏审计轨迹完整性）

删除 sessions/*/evidence.jsonl 中的条目（仅隐私/脱敏请求时）
      │
      └──► SQLite evidence_items: deleted_at 落地
      注意：同上，只替换敏感字段为 [REDACTED]，不删行

sterile 模式结束后一键清理
      │
      ├──► 删除 sessions/<sterile-session-id>/ 整个目录
      ├──► SQLite evidence_items: deleted_at 落地
      └──► archive 中该 session 产生的条目标记 [STERILE-PURGED]
```

## 8. 第一阶段完成标准

`memfold init` 执行后满足：
- `~/.memfold/` 完整目录结构已生成
- `config.toml` 已写入默认配置
- `state/memfold.db` 已初始化（所有表已建）
- `boot/`、`stable/`、`sessions/`、`archive/` 路径可按 `project-slug` 正确解析
- 项目级路径可按 `project-slug` 正确落盘
