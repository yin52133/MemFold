# 05 Runtime And Implementation

## 1. 宿主和核心怎么分工

宿主只负责“触发”。

核心负责“做决定”。

### 宿主负责

- 启动 session
- 调用 `load`
- 调用 `write_evidence`
- 提交 `feedback`
- 触发显式搜索或 `dream_run`

### 核心负责

- 读取哪些层
- 什么时候继续往下读
- 什么时候写长期记忆
- 什么时候隔离或丢弃
- 什么时候需要 repair / rebuild

## 2. 为什么核心接口是 CLI

第一版选择 CLI 不是为了让用户手动操作，而是为了让不同宿主都能稳定调用同一个本地核心。

这样做的好处：

- 不绑某个宿主实现
- 本地分发简单
- 出问题时容易复现
- 不需要先引入常驻服务复杂度

所以正确理解是：

- `plugin / hook / skill` 是宿主入口
- `CLI` 是 MemFold 核心入口

## 3. 最小接口

- `load(mode, scope, intent)`
- `write_evidence(scope, source_kind, summary, refs...)`
- `feedback(target, verdict, reason)`
- `search(scope, query, intent, budget)`
- `dream_run(scope, trigger)`

宿主层只做薄转换，不在宿主里复制记忆逻辑。

## 4. 模块拆分

- `config`
- `state`
- `memory_fs`
- `mutations`
- `boot`
- `evidence`
- `retrieval`
- `qmd_adapter`
- `feedback`
- `dreaming`
- `wiki`
- `experiments`

拆分原则：

- 每个模块只做一类事
- 不跨层偷改状态
- 所有跨存储修改都经过 `mutations`

## 5. CLI 命令

- `memfold init`
- `memfold load`
- `memfold write-evidence`
- `memfold feedback`
- `memfold dream run`
- `memfold qmd sync`
- `memfold search`
- `memfold bundle compile`
- `memfold repair`
- `memfold experiment run`

## 6. 实施顺序

第一阶段先做：

1. `config + state + memory_fs`
2. `mutations`
3. `boot`
4. `evidence`
5. `retrieval`

第一阶段完成标准：

- `memfold init` 能生成最小目录
- SQLite 能初始化并建表
- `memfold load` 能返回启动包
- `memfold write-evidence` 能成功写一条工作记录
- `memfold search` 能返回结构化结果

第二阶段再做：

6. `qmd_adapter`
7. `feedback`
8. `dreaming`

第二阶段完成标准：

- QMD 能完成一次全量索引
- `feedback` 能把长期记忆打到 `disputed/rejected`
- dreaming 能完成一次“保留 / 暂存 / 隔离 / 丢弃”决策
- dreaming 不会把分析草稿直接晋升

第三阶段再做：

9. `experiments`

第三阶段完成标准：

- 能跑离线失败样例
- 能对规则改动给出通过 / 失败结论
- 不会误写生产目录

这样可以保证系统先具备：

- 稳定启动
- 稳定写入
- 稳定回读

然后才去做复杂整理。

## 7. 发布与提交策略

你前面提的要求是对的：  
错误的中间修改不能进最终公开仓库。

所以规则应该写死：

- 本地可以有探索性提交
- 公开仓库只接受 feature 级、范围明确、结论稳定的提交
- 被否定的设计迭代不能进入最终云上历史

这意味着在真正 push 前，要做：

- 设计收敛
- feature 切分
- 历史整理

而不是把所有试错痕迹一起推上去。

## 8. V1 范围

V1 只做最核心闭环：

- 启动包
- 长期记忆
- 工作记录
- 历史档案
- QMD sidecar
- `normal/fresh/sterile`
- 基础 dreaming
- error quarantine
- rebuild / repair

V1 不追求：

- 常驻服务
- 复杂图谱
- 自动调参
- 多宿主深度包装
