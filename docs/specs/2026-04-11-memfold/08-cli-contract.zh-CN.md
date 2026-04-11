# 08 CLI Contract

## 1. 目标

这份文档定义 MemFold 核心 CLI 的接口契约。

它解决两个问题：

1. 宿主怎么调 MemFold
2. MemFold 返回什么，失败时怎么报错

## 2. 命令清单

### 2.1 `memfold init`

作用：

- 初始化 `~/.memfold/` 目录
- 初始化 SQLite
- 写默认配置

入参：

- `--root <path>` 可选

成功返回：

- exit code `0`
- stdout 输出创建结果摘要

失败返回：

- `10` 路径不可写
- `11` SQLite 初始化失败

### 2.2 `memfold load`

作用：

- 读取启动包
- 按 mode / scope / intent 决定是否继续加载

入参：

- `--mode normal|fresh|sterile`
- `--scope-type user|project`
- `--scope-id <id>`
- `--intent startup|continue|knowledge_lookup|reset`
- `--budget <tokens>` 可选

成功返回：

```json
{
  "mode": "normal",
  "scope": {"type": "project", "id": "foo"},
  "items": [
    {"item_key": "user.language", "text": "默认中文回答"},
    {"item_key": "project.rule.1", "text": "不要自动带入旧错误路径"}
  ],
  "total_tokens_estimate": 128,
  "degraded": false
}
```

失败返回：

- `20` scope 不存在
- `21` 启动包缺失
- `22` mutation 未稳定且无法降级

### 2.3 `memfold write-evidence`

作用：

- 写一条工作记录

入参：

- `--scope-type`
- `--scope-id`
- `--session-id`
- `--source-kind`
- `--summary`
- `--promotable 0|1`

成功返回：

```json
{"evidence_id": "ev_123", "stored": true}
```

失败返回：

- `30` 安全清洗失败
- `31` session 不存在
- `32` JSONL 写入失败

### 2.4 `memfold search`

作用：

- 按 intent 和预算执行检索

入参：

- `--scope-type`
- `--scope-id`
- `--intent continue|knowledge_lookup`
- `--query <text>`
- `--budget <tokens>`

成功返回：

```json
{
  "results": [
    {
      "source_type": "stable",
      "doc_id": "m_1",
      "pointer": "memory/projects/foo/stable/project-card.md#rule-1",
      "summary": "项目稳定约束..."
    }
  ]
}
```

失败返回：

- `40` query 为空
- `41` QMD 不可用且无回退方案

### 2.5 `memfold dream run`

作用：

- 执行一次 dreaming 作业

入参：

- `--scope-type`
- `--scope-id`
- `--trigger manual|session_end|scheduled`

成功返回：

```json
{
  "job_id": "dj_1",
  "promoted": 2,
  "held": 5,
  "quarantined": 1,
  "discarded": 8
}
```

失败返回：

- `50` dreaming 锁被占用
- `51` 输入不满足触发条件

### 2.6 `memfold repair`

作用：

- 修复内容层、SQLite 投影、QMD 之间的不一致

入参：

- `--scope-type` 可选
- `--scope-id` 可选

成功返回：

- exit code `0`
- stdout / json 输出修复摘要

失败返回：

- `60` repair 锁被占用
- `61` 内容层损坏无法重建

## 3. 统一错误返回

所有错误都应返回：

```json
{
  "error_code": 22,
  "error_name": "UNSTABLE_MUTATION",
  "message": "pending mutation prevents stable load",
  "retryable": true
}
```

## 4. 第一阶段完成标准

`CLI contract` 做完，不是说所有逻辑都完成，而是要满足：

- `init / load / write-evidence / search / dream run / repair` 都有固定入参
- 成功返回是结构化结果，不是散乱文本
- 失败返回有稳定错误码
- 宿主可以不读源码，只靠契约接入
