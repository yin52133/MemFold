# 05 Runtime And Implementation

## 1. 宿主和核心怎么分工

```
宿主（Claude Code / Codex / OpenClaw）
  │  只负责触发
  │
  ├── 启动 session
  ├── 调用 memfold load
  ├── 调用 memfold write-evidence
  ├── 提交 memfold feedback
  └── 触发显式搜索或 memfold dream run

核心（MemFold）
  │  负责所有决策
  │
  ├── 读取哪些层
  ├── 什么时候继续往下读
  ├── 什么时候写长期记忆
  ├── 什么时候隔离或丢弃
  └── 什么时候需要 repair / rebuild
```

宿主层只做薄转换，不在宿主里复制任何记忆逻辑。

## 2. 为什么核心接口是 CLI

第一版选择 CLI 是为了让不同宿主都能稳定调用同一个本地核心。

好处：
- 不绑某个宿主实现
- 本地分发简单
- 出问题时容易复现
- 不需要先引入常驻服务复杂度

正确理解：
- `plugin / hook / skill` 是宿主入口
- `CLI` 是 MemFold 核心入口，不是用户手敲的工具

## 3. CLI 命令清单

| 命令 | 用途 |
|------|------|
| `memfold init` | 初始化目录和 SQLite |
| `memfold load` | 读取启动包，按模式决定展开层数 |
| `memfold write-evidence` | 写工作记录 |
| `memfold feedback` | 接收用户反馈，标记 rejected/disputed |
| `memfold dream run` | 执行 dreaming 整理 |
| `memfold qmd sync` | 重建/同步 QMD 索引 |
| `memfold search` | 按 intent 和预算执行检索 |
| `memfold bundle compile` | 重编启动包 |
| `memfold repair` | 修复存储层不一致 |
| `memfold experiment run` | 运行离线验证实验 |

## 4. 模块清单与职责

```
config
  读取 config.toml 和宿主配置
  被所有模块依赖

state
  SQLite 初始化和读写
  被大多数模块依赖

memory_fs
  文件系统读写（Markdown block 解析 / JSONL append / 原子替换）
  不做任何决策，只做 IO

mutations
  跨存储提交协调
  按固定顺序写 SQLite → 内容层 → 派生层
  所有跨存储修改必须经过这里

boot
  读取和编译启动包
  从 stable/*.md 的 autoload 条目编译 bundle.md
  验证 token 预算

evidence
  写入工作记录
  调 mutations 保证顺序
  追加到 sessions/*/evidence.jsonl
  追加到 archive/memory-YYYY-MM-DD.md

retrieval
  分层检索
  按 mode / intent / budget 决定读哪些层
  调 qmd_adapter 做精确查找

qmd_adapter
  QMD 索引侧边车的接口封装
  同步 / 增量更新 / 重建
  不做决策，只做索引 IO

feedback
  接收用户反馈
  调 mutations 更新 status
  写入 tombstone

dreaming
  记忆整理核心
  四阶段：Orient / Gather / Consolidate / Prune
  调 retrieval 采集信号
  调 mutations 落地决策结果
  调 boot 重编启动包

wiki
  知识库读写接口（wiki/*.md CRUD）

experiments
  离线验证框架
  跑晋升率 / 污染率 / 复活率等指标
```

## 5. 模块依赖图

```
config ◄──────────────────────── 所有模块都依赖

state ◄──────────── boot, evidence, mutations,
                    dreaming, retrieval, feedback, qmd_adapter

memory_fs ◄──────── boot, evidence, dreaming,
                    retrieval, qmd_adapter, wiki

mutations ◄──────── evidence, dreaming, feedback
  └── 依赖 state, memory_fs

boot
  └── 依赖 state, memory_fs, mutations

evidence
  └── 依赖 mutations, memory_fs

retrieval
  └── 依赖 state, memory_fs, qmd_adapter

qmd_adapter
  └── 依赖 memory_fs, state

feedback
  └── 依赖 mutations, state

dreaming
  └── 依赖 mutations, memory_fs, state, retrieval
```

## 6. 实施顺序

```
第一阶段（核心闭环，先做）
┌───────────────────────────────────────────────┐
│  1. config + state + memory_fs                │
│     完成标准：memfold init 可执行             │
│                                               │
│  2. mutations                                 │
│     完成标准：mutation 状态可从 pending       │
│       推进到 fully_applied                    │
│                                               │
│  3. boot                                      │
│     完成标准：memfold load 返回稳定结果       │
│                                               │
│  4. evidence                                  │
│     完成标准：write-evidence 可执行，         │
│       JSONL 落地，SQLite 投影正确             │
└───────────────────────────────────────────────┘
目标：能稳定启动、稳定写入、稳定回读

第二阶段（检索和整理）
┌───────────────────────────────────────────────┐
│  5. retrieval                                 │
│  6. qmd_adapter                               │
│  7. feedback                                  │
│  8. dreaming                                  │
└───────────────────────────────────────────────┘
目标：dreaming 可运行，tombstone 机制有效

第三阶段（实验）
┌───────────────────────────────────────────────┐
│  9. experiments                               │
└───────────────────────────────────────────────┘
目标：可离线验证晋升率、污染率、复活率
```

## 7. V1 范围

V1 包含：
- 启动包
- 长期记忆
- 工作记录
- 历史档案（按日期 .md 归档）
- QMD sidecar
- `normal / fresh / sterile` 三种模式
- 基础 dreaming（manual 触发）
- error quarantine（tombstone 机制）
- rebuild / repair

V1 不包含：
- 常驻服务
- 复杂图谱
- 自动调参
- 多宿主深度包装
- scheduled dreaming 门控（可选后做）

## 8. 提交策略

- 本地可以有探索性提交
- 公开仓库只接受 feature 级、范围明确、结论稳定的提交
- 被否定的设计迭代不进入最终云上历史

在真正 push 前需要做：设计收敛 → feature 切分 → 历史整理
