# Codex QMD Guide

[English](./qmd.en.md) | [中文](./qmd.zh-CN.md)

## 1. 目标

说明 MemFold 在 Codex 里的 QMD 部署、首次 embedding model 配置和后续索引流程。

## 2. 当前 QMD 形态

QMD 是 sidecar，不是正文真相源。

它负责：

- 给 `stable / session_log / history` 建索引
- 返回回指正文真相源的 `pointer`
- 在 embedding 可用时使用向量相似度增强检索

## 3. 首次配置 embedding model

第一次需要先执行：

```bash
memfold --root ~/.codex/memfold qmd init-model --model multilingual-e5-small
```

这一步会：

- 创建 `~/.codex/memfold/qmd/config/model.json`
- 首次下载 embedding model 到 `~/.codex/memfold/qmd/models`
- 不需要单独启动 `Ollama` 或 `LM Studio`

## 4. 推荐模型

- `multilingual-e5-small`
  - 默认推荐
  - 适合中英混合和通用检索

可选：

- `bge-small-zh-v1.5`
- `bge-m3`

测试专用：

- `mock-test`

## 5. 建索引

```bash
memfold --root ~/.codex/memfold qmd sync --scope-type project --scope-id my-project
```

会同步：

- `stable`
- `session_log`
- `history`

## 6. 检索

```bash
memfold --root ~/.codex/memfold search \
  --scope-type project \
  --scope-id my-project \
  --intent continue \
  --query "中文回答" \
  --budget 400
```

检索顺序：

- `continue`: stable > history > session_log
- `knowledge_lookup`: stable > history > session_log

embedding 可用时：

- 使用 embedding + 词法混合评分
- 原始 query 或归一化 query 的精确命中，会优先于泛化 token 重叠结果
- 一旦存在精确 token 类 query 命中，结果会收敛到 exact hits，而不是继续混入泛化 history 噪音
- 有 `raw_text` 的 `session_log` 会参与检索
- `stable / history / session_log` 的重复摘要会去重
- 如果 raw-text 命中的 claim 已经晋升为 `stable`，优先返回 `stable`

embedding 不可用时：

- 回退到词法检索

## 7. slash command

plugin command：

```text
/qmd
```

支持两个模式：

- `mode=init-model`
- `mode=sync`

## 8. 调试

看配置：

```bash
cat ~/.codex/memfold/qmd/config/model.json
```

看索引：

```bash
find ~/.codex/memfold/qmd/collections -type f | sort
```

## 9. 常见问题

### 为什么第一次比后面慢？

因为第一次需要下载 embedding model。

### 为什么不需要 Ollama / LM Studio？

因为当前用的是内嵌本地 embedding provider，而不是外置推理服务。
