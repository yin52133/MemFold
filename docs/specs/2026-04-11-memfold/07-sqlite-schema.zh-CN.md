# 07 SQLite Schema

## 1. 目标

SQLite 只保存状态，不保存唯一正文真相源。

第一版要支持：
- 启动加载
- 工作记录写入
- dreaming 决策
- 错误记忆隔离
- QMD 同步
- repair / rebuild

## 2. 表清单

| 表名 | 用途 |
|------|------|
| `memory_items` | 长期记忆条目投影 |
| `evidence_items` | 工作记录投影 |
| `trace_archives` | 历史档案投影 |
| `boot_entries` | 启动包编译结果索引 |
| `mutations` | 跨存储提交状态 |
| `sessions` | session 元数据 |
| `lock_leases` | 运行时租约锁 |
| `dream_jobs` | dreaming 作业 |
| `tombstones` | 被拒绝 claim 的语义墓碑 |
| `feedback_events` | 用户反馈 |
| `retrieval_events` | 检索命中记录 |

## 3. 核心字段

### 3.1 `memory_items`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 条目 id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 用户或项目 slug |
| `item_key` | TEXT | 文件内稳定条目标识 |
| `file_path` | TEXT | 相对路径 |
| `title` | TEXT | 标题 |
| `status` | TEXT | `stable/candidate/disputed/rejected/quarantined` |
| `autoload` | TEXT | `none/boot_user/boot_project/manual_only` |
| `claim_fingerprint` | TEXT | 语义指纹（SHA-256 前 16 字节） |
| `content_hash` | TEXT | 内容 hash，用于检测人工修改 |
| `revision` | INTEGER | 版本号，单调递增 |
| `supersedes_id` | TEXT NULL | 覆盖旧条目的 id |
| `trust_score` | REAL | 信任分（0~1） |
| `freshness_score` | REAL | 新鲜度（0~1） |
| `created_at` | TEXT | 创建时间（ISO 8601） |
| `updated_at` | TEXT | 更新时间 |
| `deleted_at` | TEXT NULL | 逻辑删除时间，NULL 表示未删除 |

### 3.2 `evidence_items`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 记录 id |
| `session_id` | TEXT | 所属 session |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug 等 |
| `source_kind` | TEXT | `user/tool/code/test/decision/feedback` |
| `summary` | TEXT | 安全摘要（不含敏感原文） |
| `jsonl_path` | TEXT | 正文所在 JSONL 的相对路径 |
| `line_no` | INTEGER | 所在行号 |
| `promotable` | INTEGER | 0=不可晋升 / 1=可晋升 |
| `origin_mode` | TEXT | `normal/fresh/sterile` |
| `claim_fingerprint` | TEXT NULL | 语义指纹（可选） |
| `created_at` | TEXT | 创建时间 |

### 3.3 `trace_archives`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 档案 id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `archive_date` | TEXT | 归档日期（YYYY-MM-DD） |
| `archive_kind` | TEXT | `daily_log/summary/ref` |
| `file_path` | TEXT | 相对路径（如 `archive/memory-2026-04-11.md`） |
| `line_no` | INTEGER NULL | 如果是 JSONL，记录行号 |
| `content_hash` | TEXT | 内容 hash |
| `created_at` | TEXT | 创建时间 |
| `deleted_at` | TEXT NULL | 逻辑删除时间 |

### 3.4 `boot_entries`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 条目 id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `item_key` | TEXT | 来源 memory_item 的 item_key |
| `source_item_id` | TEXT | 来源 memory_items.id |
| `text` | TEXT | 启动时实际注入内容 |
| `token_estimate` | INTEGER | 估算 token 数 |
| `compiled_at` | TEXT | 编译时间 |

### 3.5 `mutations`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | mutation id |
| `mutation_kind` | TEXT | `write/promote/reject/delete/repair` |
| `target_ref` | TEXT | 目标条目 id 或路径 |
| `status` | TEXT | `pending/applied_to_content/fully_applied/failed` |
| `idempotency_key` | TEXT | 幂等键，防重复执行 |
| `created_at` | TEXT | 创建时间 |
| `updated_at` | TEXT | 更新时间 |

### 3.6 `sessions`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | session id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `mode` | TEXT | `normal/fresh/sterile` |
| `intent` | TEXT | `startup/continue/knowledge_lookup/reset` |
| `host` | TEXT NULL | 宿主标识（如 `claude-code`） |
| `started_at` | TEXT | 开始时间 |
| `ended_at` | TEXT NULL | 结束时间，NULL 表示进行中 |
| `evidence_count` | INTEGER | 本次 session 写入的 evidence 数量 |

### 3.7 `lock_leases`

| 字段 | 类型 | 说明 |
|------|------|------|
| `lock_key` | TEXT PK | 锁名（如 `dream:project-foo`） |
| `owner` | TEXT | 占有者（进程 id / session id） |
| `lease_until` | TEXT | 过期时间 |
| `heartbeat_at` | TEXT | 心跳时间 |
| `idempotency_key` | TEXT | 幂等键 |

### 3.8 `dream_jobs`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 作业 id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `trigger` | TEXT | `manual/session_end/scheduled` |
| `status` | TEXT | `queued/running/applied/failed` |
| `promoted` | INTEGER NULL | 晋升数量 |
| `held` | INTEGER NULL | 暂存数量 |
| `quarantined` | INTEGER NULL | 隔离数量 |
| `discarded` | INTEGER NULL | 丢弃数量 |
| `created_at` | TEXT | 创建时间 |
| `updated_at` | TEXT | 更新时间 |

### 3.9 `tombstones`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 墓碑 id |
| `claim_fingerprint` | TEXT | 被拒绝的语义指纹 |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `reason` | TEXT | 拒绝原因 |
| `source_item_id` | TEXT NULL | 原始 memory_item id |
| `created_at` | TEXT | 创建时间 |

### 3.10 `feedback_events`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 事件 id |
| `target_id` | TEXT | 目标 memory_item id 或 evidence_item id |
| `target_type` | TEXT | `memory_item/evidence_item` |
| `verdict` | TEXT | `confirmed/disputed/rejected` |
| `reason` | TEXT NULL | 用户说了什么 |
| `session_id` | TEXT NULL | 反馈发生时的 session |
| `created_at` | TEXT | 创建时间 |

### 3.11 `retrieval_events`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 事件 id |
| `session_id` | TEXT | 所属 session |
| `query` | TEXT NULL | 查询词（如果是语义检索） |
| `intent` | TEXT | `startup/continue/knowledge_lookup` |
| `source_type` | TEXT | 命中的层（stable/wiki/archive/evidence） |
| `doc_id` | TEXT | 命中的文档 id |
| `latency_ms` | INTEGER | 检索延迟 ms |
| `created_at` | TEXT | 创建时间 |

## 4. 最小索引

第一版至少建这些索引：

| 索引名 | 覆盖字段 |
|--------|---------|
| `idx_memory_scope_status` | `scope_type, scope_id, status` |
| `idx_memory_claim_fingerprint` | `claim_fingerprint` |
| `idx_memory_autoload` | `autoload, status` |
| `idx_evidence_session_created` | `session_id, created_at` |
| `idx_evidence_claim_fingerprint` | `claim_fingerprint` |
| `idx_evidence_promotable` | `promotable, origin_mode` |
| `idx_trace_scope_date` | `scope_type, scope_id, archive_date` |
| `idx_mutations_status` | `status, created_at` |
| `idx_sessions_scope` | `scope_type, scope_id, started_at` |
| `idx_dream_jobs_status` | `status, created_at` |
| `idx_tombstones_claim_fingerprint` | `claim_fingerprint, scope_type, scope_id` |

## 5. 第一阶段完成标准

做到这里，才算"state 模块完成"：
- 所有 11 张表能初始化
- 核心索引能创建
- `memory_items / evidence_items / mutations / lock_leases / sessions` 可读写
- session 启动后可生成一条 `sessions` 记录
- mutation 状态能从 `pending` 正常推进到 `fully_applied`
