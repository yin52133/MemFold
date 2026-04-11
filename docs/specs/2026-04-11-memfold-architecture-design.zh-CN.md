# MemFold 架构设计总览

- 状态：Draft
- 日期：2026-04-11
- 语言：中文工作稿

## 1. 要解决的问题

一个 agent 在多个项目、多次 session 中工作时，记忆系统会在两个方向上坏掉：

**方向一：什么都带着走**
- token 膨胀
- 旧错误路径反复注入当前上下文
- 旧项目判断污染新项目工作

**方向二：什么都不敢记**
- 每次 session 像第一次来这个项目
- 用户稳定偏好永远要重复说
- 项目级稳定约束总是丢

MemFold 的目标是同时压住这两个方向。核心策略不是"存多少"，而是**分层筛选**：默认只带最小稳定信息，需要时才逐层展开，dreaming 负责周期性把有价值的信息从工作记录提升到长期层。

---

## 2. 实现约束

这份规格在实现层新增以下硬约束，避免后续施工时再临时换栈：

- **核心实现语言固定为 Rust**
- **状态层固定使用 SQLite**
- **检索索引固定通过 QMD 接入本地小模型索引能力**
- **宿主集成只通过 CLI / hook / tool 调用 Rust 核心，不在宿主层复制记忆决策**

这意味着：

- `memfold` 是一个 Rust CLI，而不是 Python 脚本或常驻服务
- SQLite 继续只承担状态、关系、锁、作业，不承担正文真相源
- QMD 仍然是可丢弃重建的 sidecar，但其检索能力依赖本地小模型索引，不依赖远端托管服务
- 后续如果替换索引模型，只允许在 `qmd_adapter` 后面替换，不允许改动记忆分层和真相源设计

---

## 3. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                        宿主层                                │
│   Claude Code / Codex / OpenClaw / 其他 agent 宿主          │
│                                                              │
│   session 启动时调 memfold load                              │
│   工作中调 write_evidence                                    │
│   用户反馈时调 feedback                                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ CLI 接口
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      MemFold 核心                            │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │   boot   │  │ evidence │  │ dreaming │  │ retrieval │  │
│  └──────────┘  └──────────┘  └──────────┘  └───────────┘  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │ memory_fs│  │mutations │  │ feedback │  │qmd_adapter│  │
│  └──────────┘  └──────────┘  └──────────┘  └───────────┘  │
└──────────────────┬──────────────────────────────────────────┘
                   │ 读写
          ┌────────┴────────┐
          ▼                 ▼
┌──────────────┐   ┌─────────────────────────────────────────┐
│  SQLite      │   │  文件系统（正文真相源）                   │
│  state only  │   │  Markdown / JSONL                        │
│  投影/状态/锁 │   │  stable/ wiki/ archive/ sessions/        │
└──────────────┘   └─────────────────────────────────────────┘
                            │
                            ▼
                   ┌─────────────────┐
                   │   QMD 索引      │
                   │   派生物，可丢弃 │
                   │   重建          │
                   └─────────────────┘
```

**关键分工：**
- 宿主只触发，不决策
- 核心决定读哪些层、写哪里、怎么晋升
- SQLite 只存状态/关系/锁，不存正文
- 文件系统是唯一正文真相源
- QMD 是检索加速器，不是真相源，随时可以丢弃重建

---

## 4. 五层结构

```
┌─────────────────────────────────────────────────┐
│  Layer 1: 启动包 (boot bundle)                   │
│  编译产物，非真相源                               │
│  300~700 tokens，启动时默认读                     │
├─────────────────────────────────────────────────┤
│  Layer 2: 长期记忆 (stable)                      │
│  真相源：stable/*.md                             │
│  默认决策层，dreaming 四选一之后才能进来           │
├─────────────────────────────────────────────────┤
│  Layer 3: 工作记录 (evidence)                    │
│  真相源：sessions/*/evidence.jsonl               │
│  当前和近期工作产生的结构化结果                   │
├─────────────────────────────────────────────────┤
│  Layer 4: 知识库 (wiki)                          │
│  真相源：wiki/*.md                               │
│  背景知识、术语、项目介绍，不默认载入             │
├─────────────────────────────────────────────────┤
│  Layer 5: 历史档案 (archive)                     │
│  真相源：archive/memory-YYYY-MM-DD.md            │
│  按日期归档的历史快照，成本最高，最后才读          │
└─────────────────────────────────────────────────┘
```

默认读取顺序固定：**启动包 → 长期记忆 → 工作记录 → 知识库 → 历史档案**

继续往下读需要明确触发条件，不能因为"项目名相似"或"主题相近"就自动展开。

---

## 5. 记忆完整生命周期

```
session 工作中
│
├── 工具调用 / 用户表达 / 代码变更 / 测试结果
│         │
│         ▼
│   write_evidence
│         │
│         ├──► sessions/<id>/evidence.jsonl    工作记录真相源
│         ├──► SQLite evidence_items 投影
│         └──► archive/memory-YYYY-MM-DD.md    追加到当日日志
│
│
dreaming 触发（门控：距上次 ≥24h 且新 session ≥5）
│
├── Phase 1 Orient（定位）
│     读现有 stable/*.md
│     了解长期记忆现状，避免重复晋升
│
├── Phase 2 Gather（采集信号）
│     优先读 archive/memory-YYYY-MM-DD.md（日志，人可读）
│     次读过时记忆（与当前状态矛盾的条目）
│     最后窄关键词 grep evidence.jsonl（不全文读）
│
├── Phase 3 Consolidate（整合）
│     四选一决策：保留 / 暂存 / 隔离 / 丢弃
│     合并到现有主题文件，不创建近似重复
│     相对日期转绝对日期
│     删除被推翻的事实
│
└── Phase 4 Prune（修剪与索引）
      重编 boot/bundle.md
      同步 QMD 索引
      更新 SQLite 投影
      每个主题文件保持合理大小
```

---

## 6. Dreaming 四选一决策

```
工作记录候选条目
      │
      ▼
  claim_fingerprint 命中 tombstone？
      │
     是│                    │否
      ▼                     ▼
   直接丢弃        满足晋升条件之一？
                  · 用户明确说"记住这个"
                  · 稳定用户偏好
                  · 稳定项目约束
                  · 跨多 session 复现且未被纠正
                       │
                      是│                    │否
                       ▼                     ▼
                保留，进 stable       证据不足，只出现过一次？
                                           │
                                          是│                │否
                                           ▼                 ▼
                                         暂存          与长期记忆冲突？
                                      继续观察         或用户说"不对"？
                                                           │
                                                          是│     │否
                                                           ▼      ▼
                                                          隔离   丢弃
                                                      写 tombstone
```

**为什么不能只靠 status=rejected：**

错误 claim 可以换一个 summary 绕过状态检查，系统当成新东西再次晋升。tombstone 封的是 `claim_fingerprint`（语义断言本身），不是某条记录。换壳也会被命中。

---

## 7. QMD 的角色

QMD 是索引侧边车，不是记忆真相源。

```
需要往下钻时（续做 / 知识检索）
      │
      ▼
  直接全文扫描所有 .md / .jsonl？
      │
      │  问题：慢，而且引入无关噪声
      ▼
  QMD 精确定位
      │
      ├── path / topic / collection 查询
      ├── 返回 pointer 指回文件系统真相源
      └── 热路径精确查找 p50 < 50ms
```

QMD 始终应被视为可丢弃重建缓存。只要内容层和 SQLite 完整，就可以从头重建，结果必须一致。

repair 顺序只能是：**内容层 → SQLite 投影 → 派生层（QMD/启动包）**，不能反过来。

---

## 8. 启动链详解

```
宿主启动 session
      │
      ▼
memfold load
  --mode normal|fresh|sterile
  --scope-type user|project
  --scope-id <slug>
  --intent startup|continue|knowledge_lookup|reset
      │
      ▼
读 memory/user/boot/bundle.md         用户级启动包
读 memory/projects/<slug>/boot/bundle.md  项目级启动包
      │
      ▼
返回 JSON
  {
    "mode": "normal",
    "scope": {"type": "project", "id": "foo"},
    "items": [...],
    "total_tokens_estimate": 128,
    "degraded": false
  }
      │
      ▼
宿主静默注入上下文（不显示给用户）
```

启动包是编译产物，不是真相源。每次 dreaming 结束后、每次长期记忆变更后，都要重编。

---

## 8. 跨存储写入顺序

多存储写入如果没有固定顺序，系统一定会 split-brain。

```
1. SQLite 建 mutation 记录（pending）
      │
      ▼
2. 内容层写临时文件
      │
      ▼
3. 原子替换内容层（真相源落地）
      │
      ▼
4. mutation 标记 applied_to_content
      │
      ▼
5. 生成派生作业（重编启动包 / QMD 同步）
      │
      ▼
6. 重编 boot/bundle.md
      │
      ▼
7. 重建/同步 QMD
      │
      ▼
8. mutation 标记 fully_applied
```

如果在任意步骤中断，repair 从内容层开始向派生层重建，不能从派生层往内容层推。

---

## 9. 防错误记忆机制

```
用户说"这个不对"
      │
      ▼
feedback --verdict rejected
      │
      ├──► memory_item.status → rejected
      ├──► 写入 tombstone（claim_fingerprint + reason）
      ├──► 关联 evidence 标记 promotable=0
      └──► 建议宿主切换 fresh 或 sterile 模式

后续 dreaming 运行时
      │
      ▼
candidate 条目 claim_fingerprint 与 tombstone 比对
      │
   命中 ▼            │ 未命中
  直接丢弃    继续正常四选一决策
```

**分析草稿永不晋升（硬规则）：**

系统自己的推断解释只帮助解释，不帮助记住。允许分析草稿进工作记录 = 系统把自己的猜测升成长期事实。这条规则没有例外。

---

## 10. 三种启动模式

| 模式 | 读什么 | token 预算 | 自动回忆 | 写入后晋升 | 适合场景 |
|------|--------|-----------|---------|----------|---------|
| `normal` | 启动包 | 300~700，上限 900 | 可以，受门控 | 可以 | 正常工作 |
| `fresh` | 最小启动包 | 60~180，上限 250 | 不可以 | 不可以 | 重新开始但保留最小用户画像 |
| `sterile` | 不读记忆 | 0 | 不可以 | 不可以 | 怀疑旧记忆已带歪方向 |

---

## 11. 模块依赖关系

```
config ◄─────────────────── 被所有模块依赖

state (SQLite)
  ▲
  └── 被 boot / evidence / mutations / dreaming / retrieval / feedback 依赖

memory_fs（文件系统读写）
  ▲
  └── 被 boot / evidence / dreaming / retrieval / qmd_adapter 依赖

mutations（跨存储提交协调）
  依赖：state, memory_fs
  被：evidence / dreaming / feedback 依赖

boot（启动包编译）
  依赖：state, memory_fs, mutations

evidence（工作记录写入）
  依赖：mutations, memory_fs

retrieval（分层检索）
  依赖：state, memory_fs, qmd_adapter

qmd_adapter（索引侧边车）
  依赖：memory_fs, state

feedback（用户反馈）
  依赖：mutations, state

dreaming（记忆整理）
  依赖：mutations, memory_fs, state, retrieval
```

---

## 12. 实施阶段

```
第一阶段：核心闭环
┌────────────────────────────────────────┐
│  config + state + memory_fs            │  基础读写能力
│  mutations                             │  跨存储提交协调
│  boot                                  │  启动包编译和读取
│  evidence                              │  工作记录写入
└────────────────────────────────────────┘
完成标准：能稳定启动、稳定写入、稳定回读

第二阶段：检索和整理
┌────────────────────────────────────────┐
│  retrieval                             │  分层检索
│  qmd_adapter                           │  索引加速
│  feedback                              │  用户反馈接入
│  dreaming                              │  记忆整理
└────────────────────────────────────────┘
完成标准：dreaming 可运行，tombstone 机制有效

第三阶段：实验
┌────────────────────────────────────────┐
│  experiments                           │  离线验证框架
└────────────────────────────────────────┘
完成标准：可离线验证晋升率、污染率、复活率
```

---

## 13. 文档结构

- [01-system-overview.zh-CN.md](./2026-04-11-memfold/01-system-overview.zh-CN.md) — 系统语义、五层结构、启动链
- [02-loading-and-retrieval.zh-CN.md](./2026-04-11-memfold/02-loading-and-retrieval.zh-CN.md) — 四条运行流、三种模式、source gating
- [03-storage-and-qmd.zh-CN.md](./2026-04-11-memfold/03-storage-and-qmd.zh-CN.md) — 真相源、跨存储提交、QMD 契约
- [04-dreaming-and-retention.zh-CN.md](./2026-04-11-memfold/04-dreaming-and-retention.zh-CN.md) — dreaming 四阶段、四选一决策、防错误复活
- [05-runtime-and-implementation.zh-CN.md](./2026-04-11-memfold/05-runtime-and-implementation.zh-CN.md) — 宿主/核心分工、CLI、模块、实施顺序
- [06-filesystem-layout.zh-CN.md](./2026-04-11-memfold/06-filesystem-layout.zh-CN.md) — 目录结构、每个路径的作用
- [07-sqlite-schema.zh-CN.md](./2026-04-11-memfold/07-sqlite-schema.zh-CN.md) — 所有表和字段
- [08-cli-contract.zh-CN.md](./2026-04-11-memfold/08-cli-contract.zh-CN.md) — 每条命令的入参/出参/错误码
- [09-memory-item-schema.zh-CN.md](./2026-04-11-memfold/09-memory-item-schema.zh-CN.md) — 四类数据对象的最小字段
- [memfold-open-source-review.zh-CN.md](../references/memfold-open-source-review.zh-CN.md) — 参考库与设计决策对应关系
