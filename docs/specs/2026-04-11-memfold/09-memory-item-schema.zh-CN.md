# 09 Memory Item Schema

## 1. 目标

定义四类最小数据对象，避免施工时每一步都重新拍板。

四类对象：
1. 长期记忆条目（memory item）
2. 工作记录（evidence）
3. 墓碑（tombstone）
4. 启动包条目（boot entry）

## 2. 长期记忆条目

存放于 `stable/*.md`，每个文件包含一到多个 block，每个 block 是一个 memory item。

**最小字段：**

| 字段 | 必填 | 说明 |
|------|------|------|
| `item_key` | 是 | 文件内稳定 key，格式：`<scope>.<category>.<name>` |
| `title` | 是 | 条目标题 |
| `summary` | 是 | 简短内容（不含敏感原文） |
| `status` | 是 | `stable/candidate/disputed/rejected/quarantined` |
| `claim_fingerprint` | 是 | 语义指纹（见 04 文档第 9 节） |
| `autoload` | 是 | `none/boot_user/boot_project/manual_only` |
| `content_hash` | 是 | 内容 hash，用于检测人工修改 |
| `revision` | 是 | 版本号，单调递增，从 1 开始 |
| `supersedes_id` | 否 | 覆盖旧条目的 item_key |

**Markdown 格式示例：**

```markdown
## item_key: project.rule.no-stale-path
title: 不要自动带入旧错误路径
status: stable
autoload: boot_project
claim_fingerprint: cfp_3a7f2e1b4c9d8e6f
content_hash: sha256:abc123
revision: 3

本项目中，旧任务的错误路径不得自动进入新 session。
如果用户没有明确说"继续上次"，不要主动恢复旧推理链。
```

**item_key 命名规则：**

```
user.preference.language       用户偏好：语言
user.preference.style          用户偏好：风格
project.rule.<name>            项目规则
project.constraint.<name>      项目约束
project.context.<name>         项目背景（不进启动包）
```

## 3. 工作记录

存放于 `sessions/<session-id>/evidence.jsonl`，每行一条，append-only。

**最小字段：**

| 字段 | 必填 | 说明 |
|------|------|------|
| `evidence_id` | 是 | 记录 id，格式：`ev_<uuid>` |
| `session_id` | 是 | 所属 session id |
| `source_kind` | 是 | `user/tool/code/test/decision/feedback` |
| `summary` | 是 | 安全摘要（不含 token/key/cookie 等原文） |
| `promotable` | 是 | `0` = 不可晋升 / `1` = 可晋升 |
| `origin_mode` | 是 | `normal/fresh/sterile` |
| `claim_fingerprint` | 否 | 语义指纹（有明确断言时填） |
| `created_at` | 是 | ISO 8601 时间 |

**JSONL 示例：**

```json
{"evidence_id":"ev_a1b2c3","session_id":"sess_xyz","source_kind":"user","summary":"用户明确要求默认用中文回答所有问题","promotable":1,"origin_mode":"normal","claim_fingerprint":"cfp_3a7f2e1b4c9d8e6f","created_at":"2026-04-11T10:00:00Z"}
{"evidence_id":"ev_d4e5f6","session_id":"sess_xyz","source_kind":"feedback","summary":"用户说这个路径判断不对，要求忽略上次结论","promotable":0,"origin_mode":"normal","created_at":"2026-04-11T14:30:00Z"}
{"evidence_id":"ev_g7h8i9","session_id":"sess_xyz","source_kind":"decision","summary":"确认 claim_fingerprint 用 SHA-256 前 16 字节","promotable":1,"origin_mode":"normal","claim_fingerprint":"cfp_9f8e7d6c5b4a3f2e","created_at":"2026-04-11T16:00:00Z"}
```

**source_kind 枚举说明：**

| 值 | 含义 |
|----|------|
| `user` | 用户明确表达的偏好或要求 |
| `tool` | 工具调用结果 |
| `code` | 代码修改结果 |
| `test` | 测试与验证结果 |
| `decision` | 设计决策 |
| `feedback` | 用户纠正或反馈 |

## 4. 历史档案日志

存放于 `archive/memory-YYYY-MM-DD.md`，每天一个文件，append-only，人可读。

Dreaming Phase 2 优先读这里，不是全文扫 evidence.jsonl。

**格式示例：**

```markdown
# 2026-04-11

## 10:32 [user] 用户明确要求默认用中文回答
source_kind: user
session: sess_xyz
evidence_id: ev_a1b2c3
promotable: true

## 14:30 [feedback] 用户说路径判断不对
source_kind: feedback
session: sess_xyz
evidence_id: ev_d4e5f6
promotable: false

## 16:00 [decision] 确认 claim_fingerprint 方案
source_kind: decision
session: sess_xyz
evidence_id: ev_g7h8i9
promotable: true
```

## 5. 墓碑

墓碑是"同一个错误 claim 不能换壳回来"的约束，不是 status 的替代物。

**最小字段：**

| 字段 | 必填 | 说明 |
|------|------|------|
| `claim_fingerprint` | 是 | 被拒绝的语义指纹（唯一约束键） |
| `scope_type` | 是 | `user/project` |
| `scope_id` | 是 | 作用域 |
| `reason` | 是 | 为什么拒绝（用户原话或系统判断） |
| `source_item_id` | 否 | 原始 memory_item id |
| `created_at` | 是 | 时间 |

**写入时机：**

```
用户说"这个不对" → feedback verdict=rejected
      │
      ▼
1. memory_item.status → rejected
2. 写入 tombstone（claim_fingerprint + reason）

dreaming 发现冲突 → 四选一决策=隔离
      │
      ▼
1. memory_item.status → quarantined
2. 写入 tombstone
```

## 6. 启动包条目

启动包条目是从长期记忆编译出来的投影，不是独立真相源。存于 `boot/bundle.md`。

**最小字段：**

| 字段 | 必填 | 说明 |
|------|------|------|
| `item_key` | 是 | 来源 memory_item 的 item_key |
| `source_item_id` | 是 | 来源 memory_items.id |
| `text` | 是 | 启动时实际注入的内容 |
| `source_type` | 是 | `user_profile/project_card/project_rule` |
| `token_estimate` | 是 | 估算 token 数 |

**bundle.md 格式示例：**

```markdown
<!-- compiled: 2026-04-11T18:00:00Z | scope: project/foo | tokens: 312 -->

## user.preference.language
默认用中文回答所有问题。

## project.rule.no-stale-path
旧任务的错误路径不得自动进入新 session。
如果用户没有明确说"继续上次"，不要主动恢复旧推理链。

## project.constraint.freeze
合并冻结从 2026-03-05 开始，到 2026-04-30 结束。
```

**编译规则：**

1. 从 `memory_items` 筛选 `autoload IN (boot_project, boot_user) AND status = stable AND deleted_at IS NULL`
2. 按 `trust_score DESC, updated_at DESC` 排序
3. 从高到低累加 `token_estimate`，超过预算则截断
4. 写入 `boot/bundle.md`，带编译时间戳注释

## 7. 第一阶段完成标准

做到这里，施工者在写代码时不需要再临时决定：
- 长期记忆条目长什么样
- 工作记录 JSONL 格式是什么
- 历史档案日志怎么写
- 墓碑什么时候写、写什么
- 启动包怎么编译、格式是什么

如果实现时还要反复问"这个字段到底有没有"，说明 schema 还没定住。
