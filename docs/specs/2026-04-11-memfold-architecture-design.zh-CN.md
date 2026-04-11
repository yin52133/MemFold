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

### 3.7 敏感信息默认不入记忆

MemFold 不应把密钥、token、cookie、私钥、助记词、密码、个人隐私信息、支付信息等敏感内容持久化到 boot、effective memory、wiki、observation 或 archive 正文中。

如果运行时确实遇到敏感材料，系统只能：

- 丢弃
- 脱敏后保留摘要
- 保留不含敏感正文的外部引用

绝不允许把敏感原文直接写入长期记忆层。

### 3.8 预算优先于“尽量全读”

记忆系统的目标不是尽可能多读，而是在最小 token 成本下给出足够正确的上下文。只要当前回答已经有足够支撑，就不继续下钻。

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

启动时自动载入的最小记忆包。它是从 stable memory 中按规则编译出来的加载视图，不是独立长期真相源。

### 5.2 Effective Memory

真正意义上的长期记忆层。默认优先检索，允许自动注入上下文，但必须经过筛选、整理和状态控制。

### 5.3 Observation

从用户表达、工具调用、代码修改、验证结果中抽取出的结构化事件，是 archive 和 effective memory 之间的中间层。

### 5.4 Archive

经安全清洗后的低层材料集合。它包含：

- `archive summary`
- `archive refs`
- 必要时的 `raw transcript refs`

其中 `raw transcript` 在 MemFold 中默认只以安全引用或安全片段形式存在，不作为无条件持久化的正文真相源。

### 5.5 Knowledge Wiki

面向浏览和综合的知识页层。适合表达概念、实体、主题、外部资料整合，不直接等同于有效长期记忆。

### 5.6 Dreaming

一套异步整理流程，用于把 observation 和 archive 中重复出现且高价值的信息，晋升、合并或淘汰到 effective memory。

### 5.7 Memory Mode

会话级载入模式，控制启动时默认读什么，以及后续是否允许自动扩展检索。

### 5.8 Reflection Note

Dreaming 或检索过程中产生的分析性反思、假设、推测与说明性注记。它可用于解释和调试，但默认不是 observation，也不能直接晋升为长期记忆。

### 5.9 Autoload

决定某条 stable memory 是否有资格参与 boot bundle 编译的加载属性。它不是记忆层级本身，而是一个装载资格标记。

建议枚举：

- `none`
- `boot_user`
- `boot_project`
- `manual_only`

### 5.10 User Profile

用户级稳定画像，是 effective memory 的一个受控子集，用于表达语言偏好、风格偏好和长期硬约束。

### 5.11 Project Card

项目级稳定卡片，是 effective memory 的一个受控子集，用于表达项目身份、核心术语、关键入口和长期约束。

### 5.12 Knowledge Lookup

一种显式任务意图，表示当前需要查背景、概念、综述或外部知识，而不是继续当前行为决策。只有在这种意图下，wiki 和 archive summary 才允许更早参与检索。

## 6. 总体架构

MemFold 的持久化主体采用四层结构：

1. `Effective Memory Layer`
2. `Observation Layer`
3. `Archive Layer`
4. `Knowledge Wiki Layer`

此外还有一个派生视图：

- `Boot Layer`
  - 仅用于安全启动和最小上下文加载
  - 不与四个持久化主体层并列作为长期真相源

配套两个横切子系统：

- `Dreaming System`
- `Autoresearch System`

### 6.1 架构摘要

- `Effective Memory` 负责默认长期记忆读取
- `Observation` 负责承接运行时事件与候选事实
- `Archive` 负责安全清洗后的追溯材料与引用
- `Knowledge Wiki` 负责面向主题的可浏览知识组织
- `Boot Layer` 负责把 stable memory 编译成最小启动视图
- `Dreaming` 负责整理、晋升、去重、降权和淘汰
- `Autoresearch` 负责优化记忆系统自己的策略，不直接替代生产写入流程

### 6.2 各层边界速览

- `Effective Memory`
  - 是什么：默认长期记忆层
  - 不是什么：原始档案，不是自动生成的背景页
- `Observation`
  - 是什么：结构化证据层和候选事实层
  - 不是什么：长期记忆本体，也不是模型自由反思
- `Archive`
  - 是什么：安全清洗后的追溯材料与引用
  - 不是什么：无条件保存的完整原始对话正文
- `Knowledge Wiki`
  - 是什么：浏览和综述层
  - 不是什么：默认行为指导层
- `Boot Layer`
  - 是什么：加载视图
  - 不是什么：独立真相源

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

Boot bundle 不应按“能塞多少塞多少”构建，而应按固定 slot 编译。建议第一版采用以下上限：

- 语言偏好：最多 1 条
- 风格偏好：最多 2 条
- 用户硬约束：最多 3 条
- 项目身份：最多 1 条
- 项目术语映射：最多 5 对
- 关键入口：最多 3 个
- 项目长期约束：最多 3 条

### 7.3 会话模式

MemFold 支持三种会话模式：

- `normal`
  - 自动加载 boot bundle
  - 允许后续按任务自动检索高置信 effective memory
  - 正常写 observation，允许 dreaming 晋升
- `fresh`
  - 只加载最小 boot bundle
  - 默认不自动扩展读取项目历史
  - 允许写 observation，但默认标记为 `non_promotable_until_review`
- `sterile`
  - 不加载任何长期记忆
  - 不执行后台记忆检索
  - 仅保留当前会话和必要的安全清洗后 archive 引用
  - 默认不把该模式下产生的 observation 投入 dreaming 队列

### 7.4 模式矩阵

| 模式 | 读 boot | 自动扩展检索 | 写 observation | 进入 dreaming | 自动晋升 |
|------|---------|--------------|----------------|---------------|----------|
| `normal` | 是 | 是，受预算和门控限制 | 是 | 是 | 是 |
| `fresh` | 是，最小集 | 否，除非显式请求 | 是，带 `origin_mode=fresh` | 默认否 | 否 |
| `sterile` | 否 | 否 | 可选，仅安全清洗摘要 | 否 | 否 |

### 7.5 扩展读取触发器

只有以下场景允许额外读取记忆：

- 用户显式要求继续上次工作
- 用户提到“按我一贯习惯”“你之前记得”
- 当前上下文出现冲突或缺信息
- 用户明确指出理解偏差，要求回溯对齐
- 用户明确发起知识检索或背景查询

除此之外，不自动继续扩展检索。

说明：

- “当前任务与项目、主题、实体高匹配”本身不构成自动扩展理由
- 第一次自动扩展后，如果仍不足，原则上应优先缩小问题范围，而不是继续加旧记忆

### 7.6 读取预算与源门控

为了控制 token 污染，扩展读取必须带预算和源门控。

建议规则：

- 每次扩展读取，默认最多补充 `1` 组高置信结果
- 单次自动补充的总预算建议不超过 `250` tokens
- 每轮回答默认最多执行 `1` 次隐式扩展检索
- 在未触发“继续上次工作/回溯对齐/知识检索”前，不自动读取 `archive`
- 在未触发“知识检索”前，不自动读取 `wiki`
- `QMD` 返回的结果不能直接原样进入 prompt，必须经 `retrieval` 层裁剪成 `summary/context/details`

默认自动扩展优先级建议为：

1. `effective memory`
2. `observation`
3. `project wiki summary`
4. `archive summary`

其中 `project wiki summary` 和 `archive summary` 只有在任务类型匹配时才允许启用。若第一次自动扩展后仍不足，第二次扩展应转为显式用户确认或显式子命令触发。

### 7.7 Fresh 与 Sterile 的安全语义

- `fresh`
  - 允许最小用户画像进入上下文
  - 允许显式请求时再查项目级记忆
  - 不允许因为“相似查询”自动扩展旧记忆
- `sterile`
  - 任何长期记忆都不读
  - 不做隐式回忆
  - 只允许当前工作区事实和当前对话作为依据

当用户明确要求“重新开始”“不要沿用之前判断”时，应优先切换到 `fresh` 或 `sterile`。

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
- `trust_score` 达到阈值
- `freshness_score` 达到阈值
- 未过期
- 不在冲突窗口中

### 8.4 语义墓碑与复活约束

为了防止错误记忆换一组 observation 或换一个 item id 后重新进入系统，MemFold 需要 claim-level 的墓碑机制。

建议新增：

- `claim_fingerprint`
  - 代表语义级 claim 的稳定指纹
- `tombstones`
  - 记录被拒绝或被冻结的 claim
- `supersedes/replaces`
  - 标记新旧记忆之间的覆盖关系

规则：

- 与 tombstone 高相似的新 candidate 不能直接 promote
- 若要恢复曾被拒绝的 claim，必须提供新的证据链和显式复核

## 9. 数据层设计

MemFold 采用混合存储：

- `SQLite`：状态、索引控制、作业、关系、分数、锁
- `Markdown`：effective memory、project card、user profile、knowledge wiki
- `JSONL`：observations、archive、raw events、session transcript 片段

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

### 9.4 恢复、重建与并发原则

为了避免数据库或索引损坏导致整体记忆不可用，必须坚持以下原则：

- `SQLite` 只保存状态和可重建元数据，不保存唯一正文真相源
- `Markdown + JSONL` 才是内容真相源
- `QMD` 索引必须可全量重建
- `SQLite` 状态库必须支持完整性检查与备份快照
- 锁真相源统一为 `SQLite lease`
- `runtime/locks/` 若存在，只能作为调试和 crash hint，不得与 SQLite 并列为锁真相源
- 第一版默认仅支持本地磁盘，不支持网络文件系统或共享盘上的状态库

这意味着即使 `memfold.db` 或 `QMD` 索引异常，系统也应该退化为“重建索引和状态”，而不是“记忆内容不可恢复”。

### 9.5 跨存储提交与恢复协议

一次记忆变更可能同时影响：

- `Markdown/JSONL` 内容层
- `SQLite` 状态层
- `boot bundle` 派生层
- `QMD` 索引层

因此必须定义提交顺序，避免 split-brain。

第一版建议采用：

1. 在 `SQLite` 中创建 mutation 记录，状态为 `pending`
2. 生成目标文件的新内容到临时文件
3. 以原子 rename 方式替换目标文件
4. 在 `SQLite` 中把 mutation 更新为 `applied_to_content`
5. 生成 `pending_bundle_compile` 和 `pending_qmd_sync` 派生作业
6. 派生作业成功后，将 mutation 标记为 `fully_applied`

恢复规则：

- 启动时先扫描 `pending`/`applied_to_content`/`pending_qmd_sync` 状态的 mutation
- 若内容层已落地但索引未更新，则只补做派生步骤
- 若 mutation 未完成且文件落地不完整，则回滚到上一个已知内容版本

明确约束：

- `boot bundle` 是派生物，不是可编辑真相源
- `QMD` 索引是派生缓存，不是可编辑真相源
- `repair` 的优先级是“内容层 -> SQLite 投影 -> 派生层”

### 9.6 保留期与删除传播

MemFold 必须定义 retention 与删除传播规则，避免“虽然不再自动加载，但原文永久残留”。

第一版建议矩阵：

- `boot bundle`
  - 只保留当前编译结果
  - 每次重编覆盖旧产物
- `effective memory`
  - 长期保留
  - 被 supersede 后保留历史版本引用，但不自动加载旧版本
- `observation`
  - 默认保留较长时间，但应支持按项目、按会话、按状态清理
- `archive`
  - 默认保留，但必须支持按项目结束、人工删除、敏感命中、模式来源进行裁剪
- `benchmark/experiment artifacts`
  - 默认短期保留
  - 不得无限增长
- `backups`
  - 必须有 prune 规则，不能无限累积

删除或脱敏传播要求：

- 删除 archive 条目时，必须同步清理 SQLite 索引记录
- 脱敏后必须同步重建相关 QMD 索引
- 被删除或脱敏的数据不得继续出现在 benchmark、backup、cache 中
- `sterile` 模式下产生的临时材料必须可一键清理

### 9.7 Observation 的真相源

Observation 既是状态机输入，也是证据层，必须明确真相源。

第一版建议：

- observation 正文真相源：append-only JSONL
- SQLite：observation 的投影、状态、索引、关系和分数

这样可以保证：

- DB 损坏时 observation 可重放
- archive/raw events 与 observation 不混成同一层
- experimentation 和 repair 可基于 observation log 回放

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
│   │           └── <session-id>/
│   │               ├── observations.jsonl
│   │               └── raw-events.jsonl
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
- `boot/bundle.md` 是派生物，不是人工维护的长期真相源
- `runtime/locks/` 只是辅助诊断目录，不是锁真相源

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
- `mutations`
  - 跨存储变更状态
- `mode_sessions`
  - 会话模式与加载历史
- `sessions`
  - session 元数据
- `lock_leases`
  - 运行时租约锁
- `strategy_experiments`
  - autoresearch 实验记录
- `strategy_metrics`
  - 实验结果指标
- `tombstones`
  - 被拒绝 claim 的语义墓碑

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
- `content_hash`
- `last_used_at`
- `decision_version`
- `redaction_status`
- `claim_fingerprint`
- `supersedes_id`
- `pollution_risk_score`
- `item_key`
- `revision`
- `created_at`
- `updated_at`
- `deleted_at`

### 11.3 Memory Item 的最小单位

第一版建议把一个 memory item 定义为：

- 一个 Markdown 文件中的一个稳定 block

该 block 必须拥有：

- `item_key`
- `title`
- `summary/body`
- 可选 metadata

这意味着：

- `file_path` 只负责定位文件
- `item_key` 才负责定位文件内的逻辑记忆项
- 手工编辑 Markdown 后，系统通过 `item_key + content_hash` 检测变化
- `revision/supersedes_id/deleted_at` 负责版本链和墓碑链

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
- `redaction_status`
- `sensitivity_flags`
- `origin_mode`
- `promotable`
- `evidence_kind`
- `projection_version`

`promotion_candidates`：

- `id`
- `observation_id`
- `candidate_type`
- `cluster_key`
- `phase`
- `promotion_score`
- `decision`
- `explanation`
- `decision_version`
- `claim_fingerprint`

`mutations`：

- `id`
- `mutation_kind`
- `scope_type`
- `scope_id`
- `target_ref`
- `status`
- `content_version_before`
- `content_version_after`
- `created_at`
- `updated_at`

`sessions`：

- `id`
- `scope_type`
- `scope_id`
- `origin_host`
- `mode`
- `started_at`
- `ended_at`

`lock_leases`：

- `id`
- `lock_key`
- `owner`
- `lease_until`
- `heartbeat_at`
- `idempotency_key`

`tombstones`：

- `id`
- `claim_fingerprint`
- `scope_type`
- `scope_id`
- `reason`
- `evidence_ref`
- `created_at`

`dream_jobs`：

- `id`
- `scope_type`
- `scope_id`
- `job_kind`
- `status`
- `attempt`
- `lock_key`
- `created_at`
- `applied_at`
- `verified_at`
- `failed_at`
- `rollback_ref`

## 12. Observation 设计

Observation 是 MemFold 的核心中间层。

### 12.1 Observation 的职责

- 接住运行时产生的结构化信号
- 让 dreaming 处理结构化事件，而不是直接处理大段 transcript
- 为 effective memory 提供可追溯证据
- 为 archive 提供更高层摘要入口
- 作为可回放的结构化证据日志

### 12.2 Observation 的来源

- 用户显式表达
- 工具调用结果
- 代码修改结果
- 测试/验证结果
- 设计决策
- 用户反馈

### 12.3 Observation 写入前的安全清洗

Observation 在写入前必须经过安全清洗流程，至少包括：

- 密钥与 token 模式识别
- cookie、session、私钥、助记词识别
- 明显的个人隐私信息识别
- 长文本截断与摘要化
- 将高风险原文替换为类型标签或外部引用

如果无法确认内容是否敏感，默认不写入 observation 正文。

### 12.4 Reflection Note 与 Observation 的边界

以下内容只能进入 `reflection_note`，不能直接进入 observation：

- 模型自己的猜测
- dreaming 过程中的反思
- 未绑定外部证据的总结
- 检索路径解释

`reflection_note` 默认：

- `non_promotable`
- `non_bootable`
- 不参与 boot bundle 编译

只有在补齐外部证据、验证结果或用户显式确认后，才允许转换为正式 observation。

### 12.5 Observation 不是长期记忆

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
5. `Archive summary`
6. `Raw transcript refs`

注意：

- `QMD 索引摘要` 不是单一来源，而是 `retrieval` 层基于 source gating 选出的摘要
- 默认情况下，`wiki` 和 `archive` 不应因为单次模糊搜索就早于 `observation` 进入上下文
- `normal` 与 `fresh` 模式下，QMD 默认只查询 `effective` collection；`wiki/archive` 需要显式 intent 或显式二次确认

### 13.2 显式知识检索路径

当 intent 为 `knowledge_lookup` 时，可采用不同于默认续做路径的顺序：

1. `Boot Layer`
2. `Effective Memory`
3. `Wiki summary`
4. `Archive summary`
5. `Observation`
6. `Raw transcript refs`

这一路径只适用于背景查阅和主题综述，不适用于默认行为决策。

### 13.3 QMD 集成方式

QMD 作为 sidecar search，主要负责：

- `effective/` 目录索引
- `wiki/` 目录索引
- `archive/` 目录索引
- 支持基于 path、topic、collection 的搜索

QMD 的查询结果必须标注来源类型：

- `effective`
- `wiki`
- `archive`

并附带最少 metadata：

- `source_type`
- `updated_at`
- `status_or_trust`
- `scope`
- `doc_id`
- `content_hash`
- `pointer`

其中：

- `doc_id` 是 QMD 索引文档的稳定标识
- `pointer` 指向 `relative_path + line_range` 或等价定位信息

QMD 不负责：

- 记忆状态更新
- 自动加载决策
- 冲突解决
- 梦境作业调度
- rejected/quarantine 管理

### 13.4 Progressive Disclosure

检索输出分三层：

- `summary`
  - 短摘要，适合 prompt 内联
- `context`
  - 中等长度内容，适合补齐背景
- `details`
  - 深度材料，仅在必要时展开

### 13.5 Token 预算建议

单轮检索建议预算：

- `summary`：80 到 180 tokens
- `context`：最多 300 tokens
- `details`：仅在显式下钻时使用

默认情况下，同一轮回答不应同时拼接多组 `context` 和 `details` 结果。

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

### 14.5 Dreaming 执行门控

Dreaming 不能在每轮交互后立即运行。建议至少受以下门控约束：

- 会话数阈值
- 时间阈值
- 待处理 observation 数量阈值
- 当前不存在同类作业锁

这样可以避免频繁整理造成的噪声放大和状态抖动。

### 14.6 Dreaming 的输入约束

Dreaming 的正式输入只能是：

- `promotable = true` 的 observation
- 安全清洗后的 archive 摘要
- 已有 stable memory 的比较项

Dreaming 不直接消费 `reflection_note`。

### 14.7 Dreaming 状态机

Dreaming 作业至少要经过以下状态：

1. `queued`
2. `clustered`
3. `scored`
4. `decided`
5. `applied`
6. `verified`
7. `indexed_or_compiled`

失败路径：

- 任一阶段失败可进入 `failed`
- 若已落库但验证失败，必须进入 `rolled_back`

每个阶段要求：

- `decided`
  - 只产生决策，不直接修改正文真相源
- `applied`
  - 修改 Markdown、SQLite 状态与相关关系
- `verified`
  - 检查正文、状态、索引的一致性
- `indexed_or_compiled`
  - 触发必要的 QMD 重建和 boot bundle 重编

必须保证：

- 幂等
- 可追踪
- 可回滚
- 不因单次失败污染 stable memory

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

### 16.3 Wiki 的检索使用规则

Wiki 只适合用于：

- 项目背景说明
- 术语解释
- 主题综述
- 外部知识查阅

Wiki 不适合用于：

- 直接指导当前轮行为决策
- 覆盖 stable memory
- 替代用户当前显式意图

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

### 17.6 Autoresearch 的数据边界

Autoresearch 默认只处理：

- 脱敏后的回放样本
- 已标注 benchmark
- 受控实验数据

Autoresearch 不应直接把生产中的完整用户记忆正文拿去做自由试验，尤其不能把敏感或争议材料直接当训练样本反复重放。

### 17.7 实验环境硬隔离

`memfold experiment run` 必须运行在独立实验环境中，至少满足：

- 使用只读内容快照
- 使用独立的 SQLite 状态路径
- 使用独立的 QMD collection 或索引路径
- 不允许调用生产写入命令
- 不允许修改生产 boot bundle
- 不允许修改 live memory 文件

策略从实验环境进入生产前，应经过显式 promote 流程，而不是由实验任务直接覆盖生产配置。

### 17.8 指标硬门槛与软目标

指标分为两类：

- `hard gate`
  - `false_memory_rate`
  - `contradiction_rate`
  - `boot_pollution_rate`
- `soft objective`
  - `useful_recall_at_k`
  - `promotion_precision`
  - `promotion_recall`
  - `token_cost`
  - `latency`

规则：

- 任一 `hard gate` 超阈值，策略不得进入生产
- `soft objective` 只用于比较收益，不足以覆盖安全退化
- 每次实验都必须记录样本量、基线版本和切片结果

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

### 19.3 宿主最小集成协议

为了避免不同宿主各自发明一套错误接入方式，第一版应定义一个稳定的最小协议，至少包括：

- `load(mode, scope, intent)`
- `write_observation(scope, source_kind, summary, refs...)`
- `feedback(target, verdict, reason)`
- `search(scope, query, intent, budget)`
- `dream_run(scope, trigger)`

这样 Codex、Claude Code、OpenClaw 的 skill/plugin/hook 包装层都只需做薄转换。

### 19.4 路径与可移植性约束

为了保证仓库可搬迁、可 fork、可恢复，主状态中只存：

- canonical relative path
- content hash
- stable item key

主状态中不应写入：

- 依赖机器的绝对路径
- 宿主特定临时目录
- 不可重建的 QMD 内部路径

QMD 索引应始终视为可丢弃重建缓存。

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
- observation 安全清洗
- rebuild/repair 基础能力
- retention/prune 基础能力
- 实验环境与生产环境隔离

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
  - 负责 SQLite 连接、迁移、事务、租约锁管理
- `memory_fs`
  - 负责 `memory/` 目录下的 Markdown/JSONL 原子读写
- `boot`
  - 负责 boot bundle 编译、预算控制、加载筛选
- `observation`
  - 负责 observation 创建、去重、聚类前预处理
- `retrieval`
  - 负责检索路由、分级下钻、结果裁剪
- `qmd_adapter`
  - 负责与 QMD 的索引同步、collection 映射、查询适配
- `mutations`
  - 负责跨存储变更协议、恢复与补偿
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

QMD 文档模型至少需要：

- `doc_id`
- `source_type`
- `relative_path`
- `pointer`
- `content_hash`
- `last_indexed_at`

以下能力不进入 `qmd_adapter`：

- 记忆状态迁移
- dreaming phase 决策
- boot bundle 编译规则
- rejected/disputed/quarantine 管理
- 跨存储提交事务

### 21.4 第一版交付顺序

建议实现顺序：

1. `config + state + memory_fs`
2. `mutations`
3. `boot`
4. `observation`
5. `retrieval`
6. `qmd_adapter`
7. `feedback`
8. `dreaming`
9. `experiments`

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
- 派生层：boot bundle + QMD index

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
