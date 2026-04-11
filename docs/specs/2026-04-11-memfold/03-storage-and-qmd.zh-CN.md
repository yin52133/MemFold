# 03 Storage And QMD

## 1. 这份文档讲什么

这里只回答三个问题：

1. 什么是真相源
2. QMD 到底负责什么
3. 多存储一起改时怎么不把状态搞坏

## 2. 真相源

MemFold 只承认两类真相源：

- `Markdown`
- `JSONL`

具体分工：

- `Markdown`
  - 长期记忆
  - 知识库
  - 用户画像
  - 项目卡片
- `JSONL`
  - 工作记录
  - 历史档案
  - raw event 投影

`SQLite` 不是正文真相源，它只存：

- 状态
- 关系
- 分数
- 作业
- 锁

## 3. QMD 的角色

QMD 是索引侧边车，不是记忆真相源。

它只负责：

- 给长期记忆建立索引
- 给知识库建立索引
- 给历史档案建立索引
- 做 path/topic/collection 查询

它不负责：

- 状态迁移
- 自动加载决策
- 冲突处理
- dreaming 决策
- 跨存储事务

一句话说：

QMD 帮你找，不能替你判。

## 4. 工作记录和 memory item 的最小单位

### 4.1 工作记录

工作记录的正文真相源是 append-only JSONL。

SQLite 里只保存它的：

- 投影
- 状态
- 关系
- 分数

### 4.2 memory item

一个 memory item = 一个 Markdown 文件中的一个稳定 block。

每个 block 至少有：

- `item_key`
- `title`
- `summary/body`

因此：

- `file_path` 只定位文件
- `item_key` 才定位文件内条目
- `content_hash` 用于检测人工修改
- `revision / supersedes_id / deleted_at` 表示版本链

## 5. 跨存储提交

多存储写入如果没有顺序，系统一定会 split-brain。

固定顺序：

```text
1. SQLite 建 mutation
2. 内容层写临时文件
3. 原子替换内容层
4. mutation 标记 applied_to_content
5. 生成派生作业
6. 重编启动包
7. 重建/同步 QMD
8. mutation 标记 fully_applied
```

这里最重要的是：

- 启动包不是人工真相源
- QMD 不是人工真相源
- repair 顺序只能是：
  `内容层 -> SQLite 投影 -> 派生层`

不能反过来。

## 6. 锁与并发

锁真相源只允许一个：

- `SQLite lease`

`runtime/locks/` 如果存在，只能拿来做诊断，不能和 SQLite 并列。

最小锁字段：

- `lock_key`
- `owner`
- `lease_until`
- `heartbeat_at`
- `idempotency_key`

这些锁要覆盖的命令：

- `write_evidence`
- `dream_run`
- `bundle_compile`
- `qmd_sync`
- `repair`

## 7. QMD 文档契约

QMD 中的每个文档至少带：

- `doc_id`
- `source_type`
- `relative_path`
- `pointer`
- `content_hash`
- `last_indexed_at`

要求：

- `pointer` 必须能稳定回指真相源
- `relative_path` 必须是 canonical relative path
- QMD 始终应被视为可丢弃重建缓存

## 8. 索引检索验收

第一版最低门槛：

- 热路径精确查找：`p50 < 50ms`，`p95 < 150ms`
- 常规混合检索：`p50 < 150ms`，`p95 < 400ms`
- 全量重建必须可执行
- 增量同步不能破坏回指稳定性

最小失败样例：

- rename 后回指失效
- 删除或脱敏后旧索引仍命中
- 增量同步后同一内容重复命中
- QMD 可用但 SQLite 状态缺失时，结果仍被误当成稳定记忆

## 9. 删除传播

删除或脱敏不能只改一处。

必须同步传播到：

- Markdown / JSONL 真相源
- SQLite 投影
- QMD 索引
- benchmark
- backup
- cache

`sterile` 产生的临时材料必须支持一键清理。
