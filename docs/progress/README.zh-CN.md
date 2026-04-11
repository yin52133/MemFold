# Progress Tracking Rules

## 1. 目标

这个目录用于记录 MemFold V1 在并行施工时的真实进度，不用于写设计结论。

它解决三个问题：

1. 当前整体做到哪一波
2. 每个子模块是谁在做、做到哪、卡在哪里
3. 哪些验证已经做过，哪些还没做

## 2. 目录结构

```
docs/progress/
  README.zh-CN.md                    进度更新规则
  memfold-v1/
    00-master-checklist.zh-CN.md     总览板，只看整体状态
    01-wave-1-foundation.zh-CN.md    第一波：基础设施
    wave-1/                          Wave 1 模块级状态页
    02-wave-2-write-load.zh-CN.md    第二波：写入与启动载入
    03-wave-3-retrieval-index.zh-CN.md
    04-wave-4-feedback-dream-repair.zh-CN.md
    05-wave-5-codex-e2e.zh-CN.md
    06-wave-6-experiments.zh-CN.md
```

## 3. 状态词汇

- `todo`: 还没开始
- `in_progress`: 已开始施工
- `blocked`: 有明确阻塞
- `done`: 当前范围完成并通过本波要求的验证

## 4. 更新规则

- 主控只更新 `00-master-checklist.zh-CN.md`
- wave 负责人更新对应 `01-wave-*` 总表
- 每个 worker 或模块负责人只更新自己负责的模块状态页
- 开始施工前，先把自己负责项改成 `in_progress`
- 出现阻塞时，必须写明阻塞原因、影响范围、下一步
- 完成后，必须写明验证命令或验证类型，不能只写“已完成”
- 旧日志不覆盖，只追加，保留时间线

## 5. 更新节奏

- 领取任务时更新一次
- 本地单测通过后更新一次
- 小范围集成通过后更新一次
- 被阻塞或范围变化时立刻更新一次
- 主控在波次切换时同步总览板

## 6. 使用原则

- 总览板回答“整体进度”
- 波次文件回答“波次级进度”
- 模块状态页回答“某个 worker 正在做什么”
- 没写进度，视为没有完成
- E2E 不能替代模块进度记录
