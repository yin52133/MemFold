# 文件系统布局

> 状态：目标态设计 Draft。当前文件系统布局仍有旧路径残留；这里定义的是迁移完成后的 canonical 布局。

## 1. 根目录

MemFold 的运行根目录固定为：

```text
~/.codex/memfold/
```

## 2. Canonical 布局

```text
~/.codex/memfold/
├── config/
│   ├── config.toml
│   └── hosts/
│
├── state/
│   └── memfold.db
│
├── memory/
│   ├── user/
│   │   ├── stable/
│   │   ├── boot/
│   │   │   └── bundle.md
│   │   ├── history/
│   │   │   └── daily/
│   │   └── sessions/
│   │       └── <session-id>/
│   │           └── session_log.jsonl
│   │
│   ├── repos/
│   │   └── <repo-id>/
│   │       ├── stable/
│   │       ├── wiki/
│   │       ├── boot/
│   │       │   └── bundle.md
│   │       ├── history/
│   │       │   └── daily/
│   │       │       └── <YYYY-MM-DD>.md
│   │       └── sessions/
│   │           └── <session-id>/
│   │               └── session_log.jsonl
│   │
│   └── families/
│       └── <family-id>/
│           ├── stable/
│           └── wiki/
│
├── qmd/
│   ├── config/
│   ├── collections/
│   └── cache/
│
└── runtime/
    └── logs/
        └── <session-id>.jsonl
```

## 3. 每个目录的职责

| 路径 | 作用 | 是否正文真相源 |
|------|------|----------------|
| `memory/user/stable/` | 用户长期记忆 | Yes |
| `memory/repos/*/stable/` | 仓库长期记忆 | Yes |
| `memory/families/*/stable/` | family 共享长期记忆 | Yes |
| `memory/*/wiki/` | 背景知识 | Yes |
| `memory/*/sessions/*/session_log.jsonl` | 会话原始记录 | Yes |
| `memory/*/history/daily/*.md` | 历史总结 | Yes |
| `memory/*/boot/bundle.md` | 启动派生产物 | No |
| `state/memfold.db` | 状态、索引、路由、锁 | No content truth, Yes state truth |
| `qmd/` | 检索 sidecar | No |
| `runtime/logs/*.jsonl` | 运行日志归档 | No |

## 4. 命名规则

### 4.1 `repo-id`

`repo-id` 必须由稳定 git 身份生成：
- 规范化 remote
- 仓库根身份

禁止：
- 直接使用 `basename(PWD)`
- 依赖当前本地绝对路径

### 4.2 `family-id`

`family-id` 由全局路由规则自动匹配，例如：
- `sata_coin`
- `sata_stock`

`family` 不应依赖每个 repo 自己维护临时声明文件。

## 5. 哪些文件允许手工编辑

允许手工编辑：
- `stable/*.md`
- `wiki/*.md`
- `history/daily/*.md` 仅在人工补总结时可编辑

不建议手工编辑：
- `boot/bundle.md`
- `session_log.jsonl`
- `state/memfold.db`
- `qmd/`
- `runtime/logs/`

## 6. 删除语义

| 数据类型 | 删除模型 |
|----------|----------|
| `stable/*.md` | soft delete via SQLite + file update |
| `session_log.jsonl` | redact only for sensitive fields |
| `history/daily/*.md` | redact summary block if needed |
| `boot/bundle.md` | hard rebuild |
| `runtime/logs/*.jsonl` | hard delete allowed after archival retention window |

## 7. 原布局迁移要求

当前 canonical 布局替换以下旧路径约定：
- `memory/projects/` -> `memory/repos/`
- `evidence.jsonl` -> `session_log.jsonl`
- `archive/` -> `history/daily/`
- 删除 `raw-events.jsonl`
- 删除 `run-meta.json`

迁移后不应继续在 canonical spec 中保留这些旧文件名。
