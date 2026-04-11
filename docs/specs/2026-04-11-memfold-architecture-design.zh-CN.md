# MemFold 架构设计稿

- 状态：Draft
- 日期：2026-04-11
- 语言：中文工作稿
- 目标仓库：`MemFold`

## 1. 文档目的

本文定义 MemFold 的第一版总体架构。MemFold 是一个面向本地单用户、多项目、跨 session 复用的分层记忆系统，服务于 Codex、Claude Code、OpenClaw 这类 agent 运行时，但不绑定某一个宿主。

本文重点回答以下问题：

- 长期记忆、观察记忆、原始档案、知识页之间如何分层
- 每次 agent 启动默认读什么，什么情况下才扩展读取
- 如何把错误记忆隔离，避免污染新 session
- QMD 在系统中的角色是什么，为什么不能直接替代状态层
- dreaming 如何整理、晋升、降权和淘汰记忆
- 如何把 autoresearch 的自我进化逻辑接到记忆系统本身
- 第一版需要实现哪些核心模块，哪些能力留到后续版本

## 2. 产品定位

MemFold 不是：

- 通用向量数据库
- 纯 wiki 系统
- 单次会话摘要器
- 只靠 prompt 自动堆上下文的“记忆增强器”

MemFold 是：

- 一个本地优先的分层记忆底座
- 一个默认低噪声、按需下钻的记忆加载系统
- 一个把原始材料整理为有效长期记忆的 dreaming 系统
- 一个带有自我评估与策略迭代能力的 agent memory runtime

## 3. 核心设计原则

### 3.1 默认少载入

默认只加载最稳定、最短、最不容易误导的记忆。原始会话、候选记忆、争议记忆、旧推理过程一律不自动加载。

### 3.2 分层隔离

能长期保存，不等于就是长期记忆。长期有效信息、结构化观察、原始档案、知识页必须分开存放，避免混淆。

### 3.3 文件可审查，状态可查询

人类长期记忆内容要能直接审查、编辑、diff；状态机、分数、锁、作业、检索计数则要有结构化存储。

### 3.4 错误记忆不可自动复活

一旦记忆被用户纠正、否决或标记为错误，不能在新 session 中被自动重新注入。

### 3.5 检索优先索引，必要时再下钻原文

先看有效记忆和索引摘要，不够再查 observation，不够再查 archive，最后才看原始 transcript。

### 3.6 系统和宿主解耦

核心系统不依赖 MCP，不直接嵌入某个 agent 运行时。宿主通过 skill、plugin、hook、script 或 CLI 命令与 MemFold 交互。

## 4. 借鉴来源与边界

本项目设计借鉴多个开源项目的架构思想。借鉴边界必须明确，避免把不同系统的不兼容假设混在一起。

### 4.1 借鉴映射

- `OpenClaw`
  - 借鉴内容：长期记忆与短期材料分层、dreaming phase、可审查记忆文件、promotion 思路
  - 不直接采用：其文件层次中的冗余组织方式
- `QMD`
  - 借鉴内容：本地索引、collection/context tree、混合检索、sidecar 检索引擎角色
  - 不直接采用：把 QMD 当作记忆状态真相源
- `Claude-Mem`
  - 借鉴内容：observation 作为中间层、progressive disclosure、hook 驱动思路
  - 不直接采用：代码实现与具体 hook 产物
- `Hermes Agent Memory`
  - 借鉴内容：memory provider 生命周期边界、状态分层、trust/entity/contradiction 方向
  - 不直接采用：其 provider 选择与具体后端绑定方式
- `Codex Autoresearch`
  - 借鉴内容：modify/verify/keep-discard/repeat、自进化实验闭环、保留失败轨迹
  - 不直接采用：把研究工作流直接混成生产记忆写入逻辑
- `MemPalace`
  - 借鉴内容：archive-first 冷存储理念、全量可追溯保底
  - 不直接采用：默认把 raw archive 暴露给主上下文
- `llm-wiki-skill` / `llm-wiki-agent`
  - 借鉴内容：raw 与 wiki 分层、entity/concept/source/synthesis 组织、lint/graph 思路
  - 不直接采用：让 wiki 页面进入默认 boot layer

### 4.2 License 判断

- `OpenClaw`：MIT
- `QMD`：MIT
- `Hermes Agent`：MIT
- `Codex Autoresearch`：MIT
- `MemPalace`：MIT
- `llm-wiki-skill`：MIT
- `llm-wiki-agent`：MIT
- `Claude-Mem`：AGPL-3.0-or-later

工程约束：

- MIT 项目可在遵守 notice 的前提下复用代码
- `Claude-Mem` 只借鉴架构思想，不直接复制代码、脚本、prompt 模板、文档原文

## 5. 名词定义

### 5.1 Boot Layer

启动时自动载入的最小记忆包，只包含稳定、低争议、高复用的信息。

### 5.2 Effective Memory

真正意义上的长期记忆层。默认优先检索，允许自动注入上下文，但必须经过筛选、整理和状态控制。

### 5.3 Observation

从用户表达、工具调用、代码修改、验证结果、总结反思中抽取出的结构化事件，是 archive 和 effective memory 之间的中间层。

### 5.4 Archive

原始会话记录、阶段性日志、完整 transcript、工具原始输出等低层材料。长期保存，但默认不自动进入上下文。

### 5.5 Knowledge Wiki

面向浏览和综合的知识页层。适合表达概念、实体、主题、外部资料整合，不直接等同于有效长期记忆。

### 5.6 Dreaming

一套异步整理流程，用于把 observation 和 archive 中重复出现且高价值的信息，晋升、合并或淘汰到 effective memory。

### 5.7 Memory Mode

会话级载入模式，控制启动时默认读什么，以及后续是否允许自动扩展检索。

## 6. 总体架构

MemFold 采用五层结构：

1. `Boot Layer`
2. `Effective Memory Layer`
3. `Observation Layer`
4. `Archive Layer`
5. `Knowledge Wiki Layer`

配套两个横切子系统：

- `Dreaming System`
- `Autoresearch System`

### 6.1 架构摘要

- `Boot Layer` 负责安全启动
- `Effective Memory` 负责默认长期记忆读取
- `Observation` 负责承接运行时事件与候选事实
- `Archive` 负责全量追溯
- `Knowledge Wiki` 负责面向主题的可浏览知识组织
- `Dreaming` 负责整理、晋升、去重、降权和淘汰
- `Autoresearch` 负责优化记忆系统自己的策略，不直接替代生产写入流程

## 7. 启动加载策略

启动加载策略是整个系统最关键的防污染机制。

### 7.1 默认加载内容

默认只自动加载两类信息：

- `用户稳定画像`
- `当前项目卡片`

用户稳定画像只允许包含：

- 默认语言偏好
- 回答风格偏好
- 稳定工作习惯
- 长期有效的硬性约束

当前项目卡片只允许包含：

- 项目身份说明
- 核心术语映射
- 关键目录或入口
- 长期有效的项目约束

默认禁止自动加载：

- 上一轮推理过程
- 上一轮错误路径
- 低置信 observation
- 候选记忆
- raw transcript
- 大段旧 session 总结
- disputed/rejected/quarantined 记忆

### 7.2 Boot Bundle 预算

Boot Layer 不直接整段读取若干文件，而是读取一份编译后的 `boot bundle`。

建议预算：

- `normal`：8 到 15 条短记忆，约 300 到 700 tokens，硬上限 900
- `fresh`：2 到 6 条短记忆，约 60 到 180 tokens，硬上限 250
- `sterile`：0

### 7.3 会话模式

MemFold 支持三种会话模式：

- `normal`
  - 自动加载 boot bundle
  - 允许后续按任务自动检索高置信 effective memory
  - 正常写 observation，允许 dreaming 晋升
- `fresh`
  - 只加载最小 boot bundle
  - 默认不自动扩展读取项目历史
  - 允许写 observation，但不自动晋升长期记忆
- `sterile`
  - 不加载任何长期记忆
  - 不执行后台记忆检索
  - 仅保留当前会话和必要的 raw archive

### 7.4 扩展读取触发器

只有以下场景允许额外读取记忆：

- 用户显式要求继续上次工作
- 用户提到“按我一贯习惯”“你之前记得”
- 当前任务与项目、主题、实体高匹配
- 当前上下文出现冲突或缺信息
- 用户明确指出理解偏差，要求回溯对齐

除此之外，不自动继续扩展检索。

## 8. 状态模型

所有候选和有效记忆都必须带状态，不能只有“存在/不存在”。

### 8.1 记忆状态

- `stable`
- `candidate`
- `disputed`
- `rejected`
- `quarantined`

### 8.2 状态规则

- `stable`
  - 可进入 effective memory
  - 若 `autoload` 允许，可进入 boot bundle
- `candidate`
  - 不自动进入 boot layer
  - 只在命中检索时作为候选证据使用
- `disputed`
  - 不自动加载
  - 仅用于冲突分析或人工复核
- `rejected`
  - 永不自动加载
  - dreaming 只能引用其历史，不得自动复活
- `quarantined`
  - 暂停使用，等待重新判断或人工处理

### 8.3 自动加载条件

只有同时满足以下条件的记忆项，才有资格进入 boot bundle：

- `status = stable`
- `autoload = boot_user` 或 `boot_project`
- `confidence` 达到阈值
- 未过期
- 不在冲突窗口中

## 9. 数据层设计

MemFold 采用混合存储：

- `SQLite`：状态、索引控制、作业、关系、分数、锁
- `Markdown`：effective memory、project card、user profile、knowledge wiki
- `JSONL`：archive、raw events、session transcript 片段

### 9.1 为什么不是纯 SQLite

纯 SQLite 会降低可审查性、可编辑性和可 diff 能力。长期记忆内容应保持文件化。

### 9.2 为什么不是纯文件

纯文件难以稳妥管理：

- 记忆状态
- confidence/trust/freshness
- recall/use count
- 作业队列
- locking
- disputed/rejected/quarantine
- dreaming phase signal

### 9.3 为什么 QMD 不能直接替代状态层

QMD 适合作为检索引擎，不适合作为状态真相源。原因包括：

- QMD 面向文档索引与搜索，不负责复杂状态机
- 它不天然表达记忆项状态迁移
- 它不天然管理 dreaming job、锁、晋升解释、冲突处理
- 它更适合做 sidecar search，而不是 production state store

因此 QMD 在 MemFold 中的角色是：

- 管理面向文件层的索引
- 为 effective memory、archive、wiki 提供混合检索
- 支持按主题、路径、collection 查询

而不是：

- 替代 SQLite
- 替代状态表
- 替代作业系统

## 10. 建议目录结构

```text
MemFold/
├── docs/
│   └── specs/
├── memory/
│   ├── user/
│   │   ├── effective/
│   │   │   ├── profile.md
│   │   │   └── anti-patterns.md
│   │   ├── boot/
│   │   │   └── bundle.md
│   │   ├── wiki/
│   │   └── archive/
│   ├── projects/
│   │   └── <project-slug>/
│   │       ├── effective/
│   │       │   ├── project-card.md
│   │       │   └── decisions.md
│   │       ├── boot/
│   │       │   └── bundle.md
│   │       ├── wiki/
│   │       ├── archive/
│   │       └── sessions/
│   └── shared/
├── state/
│   ├── memfold.db
│   └── backups/
├── qmd/
│   ├── config/
│   └── collections/
├── runtime/
│   ├── locks/
│   ├── jobs/
│   └── cache/
└── tools/
```

说明：

- `memory/` 是记忆内容层
- `state/memfold.db` 是状态数据库
- `qmd/` 是检索 sidecar 配置与 collection 管理
- `runtime/` 是运行时辅助目录

## 11. SQLite 表设计

第一版不追求过度复杂，但必须能支撑状态机和作业系统。

### 11.1 核心表

- `memory_items`
  - 存有效记忆条目元数据
- `observations`
  - 存结构化 observation
- `archives`
  - 存 archive 文件索引与 metadata
- `memory_links`
  - observation、memory、wiki、archive 之间的关系
- `entities`
  - 主题、实体、项目、人物、术语
- `memory_entity_links`
  - 记忆项与实体链接
- `dream_jobs`
  - dreaming 作业队列
- `promotion_candidates`
  - dreaming 候选池
- `conflicts`
  - 冲突检测结果
- `feedback_events`
  - 用户正负反馈
- `retrieval_events`
  - 记忆召回与使用记录
- `mode_sessions`
  - 会话模式与加载历史
- `strategy_experiments`
  - autoresearch 实验记录
- `strategy_metrics`
  - 实验结果指标

### 11.2 关键字段建议

`memory_items`：

- `id`
- `scope_type`
- `scope_id`
- `title`
- `summary`
- `status`
- `autoload`
- `confidence`
- `trust_score`
- `freshness_score`
- `last_verified_at`
- `expires_at`
- `source_kind`
- `file_path`

`observations`：

- `id`
- `scope_type`
- `scope_id`
- `session_id`
- `source_kind`
- `summary`
- `evidence_ref`
- `raw_ref`
- `confidence`
- `importance`
- `created_at`

`promotion_candidates`：

- `id`
- `observation_id`
- `candidate_type`
- `cluster_key`
- `phase`
- `promotion_score`
- `decision`
- `explanation`

## 12. Observation 设计

Observation 是 MemFold 的核心中间层。

### 12.1 Observation 的职责

- 接住运行时产生的结构化信号
- 让 dreaming 处理结构化事件，而不是直接处理大段 transcript
- 为 effective memory 提供可追溯证据
- 为 archive 提供更高层摘要入口

### 12.2 Observation 的来源

- 用户显式表达
- 工具调用结果
- 代码修改结果
- 测试/验证结果
- 设计决策
- 用户反馈
- dreaming 过程中的反思

### 12.3 Observation 不是长期记忆

Observation 默认不自动注入上下文。它必须经过：

- 去重
- 聚类
- 冲突检测
- 分数评估
- dreaming phase

才可能晋升为 effective memory。

## 13. 检索路径设计

MemFold 的检索路径必须严格分级。

### 13.1 默认检索顺序

1. `Boot Layer`
2. `Effective Memory`
3. `QMD 索引摘要`
4. `Observation`
5. `Archive`
6. `Raw transcript`

### 13.2 QMD 集成方式

QMD 作为 sidecar search，主要负责：

- `effective/` 目录索引
- `wiki/` 目录索引
- `archive/` 目录索引
- 支持基于 path、topic、collection 的搜索

QMD 不负责：

- 记忆状态更新
- 自动加载决策
- 冲突解决
- 梦境作业调度
- rejected/quarantine 管理

### 13.3 Progressive Disclosure

检索输出分三层：

- `summary`
  - 短摘要，适合 prompt 内联
- `context`
  - 中等长度内容，适合补齐背景
- `details`
  - 深度材料，仅在必要时展开

## 14. Dreaming 系统设计

Dreaming 是 observation/archives 到 effective memory 的整理系统。

### 14.1 Phase 设计

借鉴 OpenClaw 的多阶段思路，但在 MemFold 中采用更工程化定义：

- `light`
  - 去重、聚类、基本主题归并
- `rem`
  - 强化跨 session 复现信号、识别实体关系、发现冲突
- `deep`
  - 生成晋升、降权、淘汰决策，并写解释

### 14.2 Promotion 打分建议

建议综合以下维度：

- 跨 session 复现次数
- 用户显式确认
- 查询命中频次
- 使用后的正反馈
- 项目相关性
- 实体丰富度
- 最近验证时间
- 冲突惩罚
- 时效衰减
- 被否决历史

### 14.3 Dreaming 输出

Dreaming 可能输出三类结果：

- `promote`
  - 晋升为 effective memory
- `merge/update`
  - 与现有 stable memory 合并或修订
- `demote/reject/quarantine`
  - 降权、拒绝或隔离

### 14.4 Explain 能力

每一次 dreaming 决策必须产生解释，至少说明：

- 为什么被晋升、合并、降权或淘汰
- 依据了哪些 observation 或反馈
- 是否覆盖旧记忆
- 是否存在冲突风险

## 15. 错误记忆隔离机制

错误记忆污染是必须优先解决的问题。

### 15.1 触发条件

以下情况应触发错误记忆处理：

- 用户明确表示“这个不对”
- 用户要求“忽略上次结论”
- 当前事实与 stable memory 冲突
- 近期多次 retrieval 后负反馈集中出现

### 15.2 处理动作

- 将相关记忆从 `stable` 或 `candidate` 标记到 `disputed` 或 `rejected`
- 从 boot bundle 编译源中移除
- 记录冲突来源
- 标记关联 observation 为低信任或争议状态

### 15.3 禁止自动复活

被 `rejected` 的记忆不能因 dreaming 再次自动回到 stable。若确需恢复，必须：

- 产生新的证据链
- 通过显式复核流程

## 16. Knowledge Wiki 设计

Knowledge Wiki 是浏览层，不是默认主记忆层。

### 16.1 适合放入 Wiki 的内容

- 项目背景综述
- 外部资料综述
- 实体页
- 概念页
- 主题页
- 设计历史

### 16.2 不适合放入 Wiki 的内容

- 需要自动注入上下文的硬约束
- 易过时但未标记 freshness 的事实
- 单次 session 推理结果
- 未验证的观察结论

## 17. 自我进化系统

MemFold 的自我进化借鉴 codex-autoresearch，但只作用于“记忆策略”，不直接替代生产路径。

### 17.1 目标

不断优化：

- 什么时候写 observation
- 什么时候晋升 memory
- 检索路由顺序
- boot bundle 编译策略
- dreaming 分数与阈值

### 17.2 实验闭环

采用以下闭环：

1. `modify`
2. `verify`
3. `keep or discard`
4. `repeat`

### 17.3 回放基准

需要构建记忆系统专用 replay benchmark，包含：

- 历史 session 片段
- 典型查询
- 已知正例记忆
- 已知错误记忆
- 冲突样例
- 用户纠正样例

### 17.4 评估指标

- `useful_recall_at_k`
- `false_memory_rate`
- `promotion_precision`
- `promotion_recall`
- `contradiction_rate`
- `token_cost`
- `latency`
- `boot_pollution_rate`

### 17.5 生产隔离

Autoresearch 不能直接在线改生产策略。策略变更必须通过：

- 离线回放验证
- 基线对比
- 指标不退化

后才能进入生产配置。

## 18. 运行时形态

### 18.1 语言与实现

- 核心语言：`Rust`
- 接口形态：`CLI-first`
- 后续扩展：可加本地后台进程，但第一版以 CLI 为准

### 18.2 为什么不是 Python

不是因为 Python 性能一定不够，而是综合考虑：

- 开源后 fork 用户的可分发性
- 本地核心的长期形态
- 跨平台单二进制分发
- SQLite、文件系统、作业系统、后台 worker 的适配性

Rust 更适合作为生产核心。若后续需要实验脚本，可再补 Python 工具，但不进入主路径。

### 18.3 为什么不是 Go

Go 是合理备选，但当前设计优先级更偏：

- 性能余量
- 单二进制长期核心
- 与现有 agent 工具链的心理预期一致

因此当前拍板为 Rust。

## 19. 宿主集成策略

MemFold 不以 MCP 为主接口。

### 19.1 宿主原则

- `Codex`
- `Claude Code`
- `OpenClaw`

都应通过 skill、plugin、hook 或本地 script 调用 MemFold CLI，而不是依赖 MCP 常驻协议。

### 19.2 集成边界

宿主层负责：

- 触发加载
- 触发写入
- 提交用户反馈
- 控制 memory mode

MemFold 核心负责：

- 记忆读取与筛选
- 状态机
- dreaming
- 索引协同
- 自我进化评估

## 20. 第一版实现范围

### 20.1 V1 必做

- Rust CLI 骨架
- 混合目录结构
- SQLite 状态库
- boot bundle 编译与加载
- effective memory 文件读写
- observation 写入
- archive 写入索引
- 基础 dreaming phase
- QMD sidecar 集成
- normal/fresh/sterile 模式
- rejected/disputed/quarantine 机制

### 20.2 V1.1 建议补充

- conflict analyzer
- boot pollution 评估
- feedback 回路增强
- wiki 编译层

### 20.3 V2 再做

- 本地后台进程
- 更复杂的实体图谱
- richer trust model
- strategy autoresearch 自动调参框架
- 多宿主官方包装层

## 21. 实现模块拆分

第一版建议按职责拆成以下 Rust 模块，避免核心逻辑混在单文件或单命令里。

### 21.1 核心模块

- `config`
  - 负责全局配置、路径解析、项目作用域解析、模式默认值
- `state`
  - 负责 SQLite 连接、迁移、事务、锁管理
- `memory_fs`
  - 负责 `memory/` 目录下的 Markdown/JSONL 读写
- `boot`
  - 负责 boot bundle 编译、预算控制、加载筛选
- `observation`
  - 负责 observation 创建、去重、聚类前预处理
- `retrieval`
  - 负责检索路由、分级下钻、结果裁剪
- `qmd_adapter`
  - 负责与 QMD 的索引同步、collection 映射、查询适配
- `dreaming`
  - 负责 `light/rem/deep` 阶段流程和决策输出
- `feedback`
  - 负责用户纠正、正负反馈、rejected/disputed 写回
- `wiki`
  - 负责知识页编译与索引入口
- `experiments`
  - 负责 autoresearch 回放、指标记录、策略实验

### 21.2 CLI 子命令建议

- `memfold init`
  - 初始化目录、数据库、默认配置
- `memfold load`
  - 按模式加载 boot bundle 与可选扩展记忆
- `memfold write-observation`
  - 写入 observation
- `memfold feedback`
  - 提交记忆正负反馈或显式纠正
- `memfold dream run`
  - 执行 dreaming 作业
- `memfold qmd sync`
  - 同步 QMD collection 与索引
- `memfold search`
  - 执行分级检索
- `memfold bundle compile`
  - 编译 boot bundle
- `memfold repair`
  - 校验并修复 state/index 不一致
- `memfold experiment run`
  - 运行 autoresearch 策略实验

### 21.3 QMD 适配边界

`qmd_adapter` 的职责只限于：

- 注册或更新 collection
- 把 `effective/wiki/archive` 路径映射到合适 collection
- 执行查询并返回结构化摘要
- 在需要时触发重建或增量同步

以下能力不进入 `qmd_adapter`：

- 记忆状态迁移
- dreaming phase 决策
- boot bundle 编译规则
- rejected/disputed/quarantine 管理

### 21.4 第一版交付顺序

建议实现顺序：

1. `config + state + memory_fs`
2. `boot`
3. `observation`
4. `retrieval`
5. `qmd_adapter`
6. `feedback`
7. `dreaming`
8. `experiments`

这个顺序保证系统最先可用的是“安全加载 + 基础写入 + 基础检索”，而不是先追求复杂 dreaming。

## 22. 风险与取舍

### 22.1 复杂度风险

混合架构比纯文件更复杂，但换来的是：

- 更稳的状态机
- 更低的错误记忆复活风险
- 更明确的 dreaming 流程

### 22.2 双重真相源风险

如果 SQLite、Markdown、JSONL、QMD 的职责不清，系统会混乱。必须坚持：

- 内容真相源：Markdown + JSONL
- 状态真相源：SQLite
- 检索加速器：QMD

### 22.3 过度记忆风险

若 observation 抽取过宽或 dreaming 阈值过低，系统会迅速膨胀。V1 必须保守。

## 23. 当前结论

MemFold 第一版按以下结论推进：

- 架构采用混合分层
- 核心语言采用 Rust
- 接口形态采用 CLI-first
- 不以 MCP 为主接口
- QMD 作为检索 sidecar 集成
- 默认少载入，严格控制 boot layer
- effective memory、observation、archive、wiki 明确分层
- dreaming 负责晋升、修订、降权、淘汰
- autoresearch 只优化策略，不直接接管生产路径

这是一个偏保守、可审查、可回放、可自进化的本地 agent 记忆系统设计。
