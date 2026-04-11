# 03 Storage And QMD

## 1. 这份文档讲什么

三个问题：
1. 什么是正文真相源
2. QMD 到底负责什么
3. 多存储同时写时怎么保持一致

## 2. 真相源分工

```
┌─────────────────────────────────────────────────────┐
│  正文真相源（唯一权威）                               │
│                                                      │
│  Markdown (.md)                                      │
│    · stable/*.md       长期记忆                      │
│    · wiki/*.md         知识库                        │
│    · user/stable/      用户画像                      │
│    · archive/memory-YYYY-MM-DD.md  历史档案          │
│                                                      │
│  JSONL                                               │
│    · sessions/*/evidence.jsonl  工作记录             │
│    · archive/*.jsonl            历史档案投影          │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  SQLite（只存状态，不存正文）                         │
│    · 状态（status / autoload）                       │
│    · 关系（supersedes_id / session_id）               │
│    · 分数（trust_score / freshness_score）            │
│    · 作业（dream_jobs / mutations）                   │
│    · 锁（lock_leases）                               │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  QMD（索引侧边车，派生物，可丢弃重建）                 │
│    · path / topic / collection 索引                  │
│    · 快速定位到真相源文件                             │
│    · 不负责任何决策                                   │
└─────────────────────────────────────────────────────┘
```

repair 顺序只能是：**正文真相源 → SQLite 投影 → 派生层（QMD / 启动包）**，不能反过来。

## 3. QMD 的角色

```
QMD 负责：
  ├── 给 stable/*.md 建立索引
  ├── 给 wiki/*.md 建立索引
  ├── 给 archive/memory-*.md 建立索引
  └── 做 path / topic / collection 查询

QMD 不负责：
  ├── 状态迁移
  ├── 自动加载决策
  ├── 冲突处理
  ├── dreaming 决策
  └── 跨存储事务
```

**一句话：QMD 帮你找，不能替你判。**

任何时候内容层完整，就可以从头重建 QMD，结果必须一致。

## 4. Memory Item 最小单位

一个 memory item = 一个 Markdown 文件中的一个稳定 block。

```markdown
## item_key: project.rule.1
title: 不要自动带入旧错误路径
status: stable
autoload: boot_project
claim_fingerprint: cfp_abc123
revision: 3

本项目中，旧错误路径不得自动进入新 session。
```

定位关系：
- `file_path` 只定位文件
- `item_key` 才定位文件内条目
- `content_hash` 用于检测人工修改
- `revision / supersedes_id / deleted_at` 表示版本链

## 5. 跨存储写入顺序

多存储写入必须按固定顺序，否则出现 split-brain：

```
Step 1  SQLite 建 mutation 记录（status=pending）
          │
Step 2  内容层写临时文件（不直接覆盖）
          │
Step 3  原子替换内容层（真相源落地）
          │
Step 4  mutation 标记 applied_to_content
          │
Step 5  生成派生作业（重编启动包 / QMD 同步）
          │
Step 6  重编 boot/bundle.md
          │
Step 7  重建/同步 QMD
          │
Step 8  mutation 标记 fully_applied
```

如果在任意步骤中断，repair 从内容层开始重建，不能从 QMD 或 SQLite 往正文推。

## 6. 锁

锁真相源只有一个：SQLite `lock_leases` 表。

`runtime/locks/` 目录如果存在，只能用于诊断，不能和 SQLite 并列作为锁真相源。

必须加锁的命令：

| 命令 | lock_key |
|------|---------|
| `write_evidence` | `write:<scope>` |
| `dream_run` | `dream:<scope>` |
| `bundle_compile` | `bundle:<scope>` |
| `qmd_sync` | `qmd:<scope>` |
| `repair` | `repair:<scope>` |

最小锁字段：

| 字段 | 说明 |
|------|------|
| `lock_key` | 锁名 |
| `owner` | 占有者（进程 id / session id） |
| `lease_until` | 过期时间 |
| `heartbeat_at` | 心跳时间 |
| `idempotency_key` | 幂等键，防止重复执行 |

## 7. QMD 文档契约

QMD 中每个文档至少带：

| 字段 | 说明 |
|------|------|
| `doc_id` | 文档标识 |
| `source_type` | stable / wiki / archive / evidence |
| `relative_path` | canonical relative path |
| `pointer` | 稳定回指真相源的指针（含 item_key） |
| `content_hash` | 内容 hash |
| `last_indexed_at` | 最后索引时间 |

要求：
- `pointer` 必须能稳定回指真相源
- `relative_path` 必须是 canonical 格式
- QMD 始终应被视为可丢弃重建缓存

## 8. 索引检索验收

第一版最低门槛：

| 场景 | p50 | p95 |
|------|-----|-----|
| 热路径精确查找 | < 50ms | < 150ms |
| 常规混合检索 | < 150ms | < 400ms |

最小失败样例：

```
· rename 后 pointer 回指失效
· 删除或脱敏后旧索引仍命中
· 增量同步后同一内容重复命中
· QMD 可用但 SQLite 状态缺失时，结果仍被误当成稳定记忆
```

## 9. 删除传播

删除或脱敏一条记忆，必须同步传播到所有存储：

```
Markdown / JSONL 正文真相源
      │
      ▼
SQLite 投影（status → deleted_at 落地）
      │
      ▼
QMD 索引（移除相关条目）
      │
      ▼
benchmark / backup / cache（清理旧版本）
```

`sterile` 模式产生的临时材料必须支持一键清理，不得残留在任何存储层。
