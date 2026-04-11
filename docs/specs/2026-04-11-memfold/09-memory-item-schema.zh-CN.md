# 09 Memory Item Schema

## 1. 目标

这份文档定义最小数据对象，避免施工时每一步都重新拍板。

要定义的对象有四个：

1. 长期记忆条目
2. 工作记录
3. 墓碑
4. 启动包条目

## 2. 长期记忆条目

最小字段：

| 字段 | 必填 | 说明 |
|------|------|------|
| `item_key` | 是 | 文件内稳定 key |
| `title` | 是 | 条目标题 |
| `summary` | 是 | 简短内容 |
| `status` | 是 | `stable/candidate/disputed/rejected/quarantined` |
| `claim_fingerprint` | 是 | 语义指纹 |
| `autoload` | 是 | `none/boot_user/boot_project/manual_only` |
| `content_hash` | 是 | 内容 hash |
| `revision` | 是 | 版本号 |
| `supersedes_id` | 否 | 覆盖旧条目 |

Markdown 形态示例：

```md
## item_key: project.rule.1
title: 不要自动带入旧错误路径
status: stable
autoload: boot_project
claim_fingerprint: cfp_xxx
revision: 3

本项目中，旧错误路径不得自动进入新 session。
```

## 3. 工作记录

最小字段：

| 字段 | 必填 | 说明 |
|------|------|------|
| `evidence_id` | 是 | 记录 id |
| `session_id` | 是 | 所属 session |
| `source_kind` | 是 | `user/tool/code/test/decision/feedback` |
| `summary` | 是 | 安全摘要 |
| `promotable` | 是 | 0 / 1 |
| `origin_mode` | 是 | `normal/fresh/sterile` |
| `claim_fingerprint` | 否 | 语义指纹 |
| `created_at` | 是 | 时间 |

JSONL 示例：

```json
{"evidence_id":"ev_1","session_id":"sess_1","source_kind":"user","summary":"用户明确要求默认中文回答","promotable":1,"origin_mode":"normal","claim_fingerprint":"cfp_lang_cn","created_at":"2026-04-11T10:00:00Z"}
```

## 4. 墓碑

墓碑不是 status 的替代物，而是“同一个错误 claim 不能换壳回来”的约束。

最小字段：

| 字段 | 必填 | 说明 |
|------|------|------|
| `claim_fingerprint` | 是 | 被拒绝的 claim |
| `scope_type` | 是 | `user/project` |
| `scope_id` | 是 | 作用域 |
| `reason` | 是 | 为什么拒绝 |
| `created_at` | 是 | 时间 |

## 5. 启动包条目

启动包条目不是独立真相源，只是从长期记忆编译出来的投影。

最小字段：

| 字段 | 必填 | 说明 |
|------|------|------|
| `item_key` | 是 | 来源条目 key |
| `text` | 是 | 启动时实际注入内容 |
| `source_type` | 是 | `user_profile/project_card` |
| `token_estimate` | 是 | 估算 token |

## 6. 第一阶段完成标准

做到这里，施工者至少不需要再临时决定：

- 长期记忆条目长什么样
- 工作记录长什么样
- 墓碑怎么表达
- 启动包条目长什么样

如果实现时还要反复问“这个字段到底有没有”，说明 schema 还没定住。
