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
| `claim_fingerprint` | TEXT | 语义指纹 |
| `content_hash` | TEXT | 内容 hash |
| `revision` | INTEGER | 版本号 |
| `supersedes_id` | TEXT NULL | 覆盖旧条目 |
| `trust_score` | REAL | 信任分 |
| `freshness_score` | REAL | 新鲜度 |
| `created_at` | TEXT | 创建时间 |
| `updated_at` | TEXT | 更新时间 |
| `deleted_at` | TEXT NULL | 逻辑删除时间 |

### 3.2 `evidence_items`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 记录 id |
| `session_id` | TEXT | 所属 session |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug 等 |
| `source_kind` | TEXT | `user/tool/code/test/decision/feedback` |
| `summary` | TEXT | 安全摘要 |
| `jsonl_path` | TEXT | 正文所在 JSONL |
| `line_no` | INTEGER | 所在行 |
| `promotable` | INTEGER | 0 / 1 |
| `origin_mode` | TEXT | `normal/fresh/sterile` |
| `claim_fingerprint` | TEXT | 语义指纹 |
| `created_at` | TEXT | 创建时间 |

### 3.3 `trace_archives`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 档案 id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `archive_kind` | TEXT | `summary/ref/raw-ref` |
| `jsonl_path` | TEXT | 正文所在 JSONL |
| `line_no` | INTEGER | 所在行 |
| `content_hash` | TEXT | 内容 hash |
| `created_at` | TEXT | 创建时间 |

### 3.4 `mutations`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | mutation id |
| `mutation_kind` | TEXT | `write/promote/reject/delete/repair` |
| `target_ref` | TEXT | 目标条目 |
| `status` | TEXT | `pending/applied_to_content/fully_applied/failed` |
| `created_at` | TEXT | 创建时间 |
| `updated_at` | TEXT | 更新时间 |

### 3.5 `lock_leases`

| 字段 | 类型 | 说明 |
|------|------|------|
| `lock_key` | TEXT PK | 锁名 |
| `owner` | TEXT | 占有者 |
| `lease_until` | TEXT | 过期时间 |
| `heartbeat_at` | TEXT | 心跳时间 |
| `idempotency_key` | TEXT | 幂等键 |

### 3.6 `dream_jobs`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 作业 id |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `status` | TEXT | `queued/running/applied/failed` |
| `created_at` | TEXT | 创建时间 |
| `updated_at` | TEXT | 更新时间 |

### 3.7 `tombstones`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PK | 墓碑 id |
| `claim_fingerprint` | TEXT | 被拒绝 claim |
| `scope_type` | TEXT | `user` / `project` |
| `scope_id` | TEXT | 项目 slug |
| `reason` | TEXT | 拒绝原因 |
| `created_at` | TEXT | 创建时间 |

## 4. 最小索引

第一版至少建这些索引：

- `idx_memory_scope_status`
- `idx_memory_claim_fingerprint`
- `idx_evidence_session_created`
- `idx_evidence_claim_fingerprint`
- `idx_trace_scope_created`
- `idx_mutations_status`
- `idx_dream_jobs_status`
- `idx_tombstones_claim_fingerprint`

## 5. 第一阶段完成标准

做到这里，才算“state 模块完成”：

- 所有表能初始化
- 核心索引能创建
- `memory_items / evidence_items / mutations / lock_leases` 可读写
- session 启动后可生成一条 `sessions` 记录
- mutation 状态能从 `pending` 正常推进到 `fully_applied`
