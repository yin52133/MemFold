# 06 Filesystem Layout

## 1. 根目录

第一版约定 MemFold 的本地根目录为：

```text
~/.memfold/
```

如果宿主需要按项目隔离，使用：

```text
~/.memfold/projects/<project-slug>/
```

## 2. 完整布局

```text
~/.memfold/
├── config/
│   ├── config.toml
│   └── hosts/
│       ├── codex.toml
│       ├── claude-code.toml
│       └── openclaw.toml
├── state/
│   ├── memfold.db
│   ├── backups/
│   └── migrations/
├── memory/
│   ├── user/
│   │   ├── stable/
│   │   │   ├── profile.md
│   │   │   └── preferences.md
│   │   ├── boot/
│   │   │   └── bundle.md
│   │   ├── wiki/
│   │   └── archive/
│   └── projects/
│       └── <project-slug>/
│           ├── stable/
│           │   ├── project-card.md
│           │   └── rules.md
│           ├── boot/
│           │   └── bundle.md
│           ├── wiki/
│           ├── archive/
│           │   ├── refs.jsonl
│           │   └── summaries.jsonl
│           └── sessions/
│               └── <session-id>/
│                   ├── evidence.jsonl
│                   ├── raw-events.jsonl
│                   └── run-meta.json
├── qmd/
│   ├── config/
│   ├── collections/
│   └── cache/
└── runtime/
    ├── cache/
    ├── jobs/
    └── logs/
```

## 3. 每个目录干什么

| 路径 | 作用 | 是否真相源 |
|------|------|------------|
| `config/` | 全局配置、宿主配置 | 是 |
| `state/` | SQLite 状态库、备份、迁移 | 是，仅限状态 |
| `memory/user/stable/` | 用户级长期记忆 | 是 |
| `memory/projects/*/stable/` | 项目级长期记忆 | 是 |
| `memory/*/boot/` | 启动包编译结果 | 否 |
| `memory/*/wiki/` | 背景知识页 | 是 |
| `memory/*/archive/` | 历史档案摘要与引用 | 是 |
| `memory/*/sessions/*/evidence.jsonl` | 工作记录正文 | 是 |
| `memory/*/sessions/*/raw-events.jsonl` | 原始事件投影 | 是 |
| `qmd/` | 索引侧边车 | 否 |
| `runtime/` | 运行时缓存、作业、日志 | 否 |

## 4. 启动时实际会读哪些文件

默认只读：

- `memory/user/boot/bundle.md`
- `memory/projects/<project-slug>/boot/bundle.md`

不会在启动时自动读：

- `wiki/`
- `archive/`
- `sessions/`

## 5. 哪些文件允许手工编辑

允许手工编辑：

- `stable/*.md`
- `wiki/*.md`
- `config/*.toml`

不建议手工编辑：

- `boot/bundle.md`
- `state/memfold.db`
- `qmd/*`
- `runtime/*`
- `sessions/*.jsonl`

## 6. 删除传播要求

如果删除或脱敏以下内容：

- `stable/*.md`
- `wiki/*.md`
- `archive/*.jsonl`
- `sessions/*.jsonl`

必须同步：

- 更新 SQLite 投影
- 重编相关启动包
- 重建或同步 QMD

## 7. 第一阶段完成标准

`filesystem layout` 做完，不代表功能全有，而是要满足：

- `memfold init` 能生成最小目录
- `config.toml` 能被成功读取
- `state/memfold.db` 能初始化
- `boot/`、`stable/`、`sessions/` 路径可被正确解析
- 项目级路径可按 `project-slug` 正确落盘
