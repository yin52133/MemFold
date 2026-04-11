# MemFold 架构设计总览

- 状态：Draft
- 日期：2026-04-11
- 语言：中文工作稿

MemFold 的目标很简单：

- 默认少载入
- 需要时再逐层下钻
- 把真正长期有用的信息留下来
- 把错误记忆挡在长期层之外

它不是“把所有历史都存起来”的系统，而是“把长期有用的信息筛出来”的系统。

## 入口链

MemFold 的入口不是让人手动敲 CLI。

真正的启动链是：

1. 宿主启动 session
2. 宿主插件 / hook / skill wrapper 调 `memfold load`
3. MemFold 核心返回一小段启动信息
4. 宿主把这段信息静默注入当前上下文

所以：

- 对宿主来说，入口是 `plugin / hook / skill`
- 对 MemFold 核心来说，执行接口是 `CLI`

## 层级

MemFold 只保留 5 个需要理解的层：

1. `启动包`
2. `长期记忆`
3. `工作记录`
4. `知识库`
5. `历史档案`

另有一个不进入长期层的对象：

- `分析草稿`

## 主次关系

默认读取顺序固定：

1. `启动包`
2. `长期记忆`
3. `工作记录`
4. `知识库`
5. `历史档案`

这不是“可以商量的建议”，而是 MemFold 的主次关系。

## 文档结构

- [01-system-overview.zh-CN.md](/home/ps/project/MemFold/docs/specs/2026-04-11-memfold/01-system-overview.zh-CN.md)
  系统语义、层级和入口链
- [02-loading-and-retrieval.zh-CN.md](/home/ps/project/MemFold/docs/specs/2026-04-11-memfold/02-loading-and-retrieval.zh-CN.md)
  启动、续做、知识检索、重开四条运行流
- [03-storage-and-qmd.zh-CN.md](/home/ps/project/MemFold/docs/specs/2026-04-11-memfold/03-storage-and-qmd.zh-CN.md)
  真相源、状态层、QMD、恢复与删除传播
- [04-dreaming-and-retention.zh-CN.md](/home/ps/project/MemFold/docs/specs/2026-04-11-memfold/04-dreaming-and-retention.zh-CN.md)
  dreaming 保留逻辑、错误记忆隔离、规则与验收
- [05-runtime-and-implementation.zh-CN.md](/home/ps/project/MemFold/docs/specs/2026-04-11-memfold/05-runtime-and-implementation.zh-CN.md)
  CLI、模块、实施顺序、提交策略
- [memfold-open-source-review.zh-CN.md](/home/ps/project/MemFold/docs/references/memfold-open-source-review.zh-CN.md)
  参考库与致敬
