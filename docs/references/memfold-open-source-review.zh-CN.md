# MemFold 参考库与设计决策映射

这份文档记录对 MemFold 有启发的公开项目，以及它们各自的具体设计启发点如何映射到 MemFold 的实现决策。

MemFold 不是这些项目的复制，而是从中提取真正清晰、可落地的部分，重新组合为一套适合本地 CLI 宿主的记忆系统。

---

## 1. OpenClaw

**影响范围：dreaming 四阶段、历史档案格式、分层记忆结构**

### 1.1 分层记忆结构

OpenClaw 把记忆分为可审查的长期层与临时工作层，长期层人工可读，工作层按 session 隔离。

MemFold 的映射：

```
OpenClaw 分层          MemFold 对应层
─────────────────────────────────────────
长期稳定记忆   ──►  stable/*.md（唯一正文真相源）
工作临时记忆   ──►  sessions/*/evidence.jsonl
启动注入       ──►  boot/bundle.md（编译产物，非真相源）
```

### 1.2 Dreaming 四阶段

OpenClaw 的 ccb auto-dream 把记忆整理分为四个阶段，设置了时间和 session 数量的双门控。

MemFold 的映射：

```
ccb auto-dream 阶段     MemFold dreaming 对应阶段
────────────────────────────────────────────────
定位现状          ──►  Phase 1: Orient（读 stable/*.md，了解现状）
采集工作信号      ──►  Phase 2: Gather（优先读 archive/*.md，再 grep evidence）
整合决策          ──►  Phase 3: Consolidate（四选一：保留/暂存/隔离/丢弃）
索引修剪          ──►  Phase 4: Prune（重编 bundle，同步 QMD，修剪大文件）

触发门控          ──►  minHours: 24 + minSessions: 5
```

### 1.3 每日历史档案

OpenClaw 用 `memory-YYYY-MM-DD.md` 保存每日人可读工作日志，dreaming 时优先读这些文件，而不是全文扫描 evidence。

MemFold 的映射：

```
archive/memory-2026-04-11.md 格式：

# 2026-04-11

## 10:32 [user] 用户明确要求默认用中文回答
source_kind: user
session: sess_xyz
evidence_id: ev_a1b2c3
promotable: true
```

**关键决策**：archive 文件是永久审计轨迹，dreaming 消费它但不清理它。发现 stable 里的记忆对齐有误时，回查 archive 是唯一溯源手段。

---

## 2. QMD（Query Metadata）

**影响范围：索引侧边车设计、检索层独立性、可丢弃重建原则**

### 2.1 索引侧边车

QMD 把检索索引作为独立侧边车（sidecar），与内容层分离，可以随时丢弃重建，不影响内容真相源。

MemFold 的映射：

```
内容层（真相源）          QMD 侧边车（可丢弃）
────────────────────────────────────────────
stable/*.md          ──►  qmd/collections/stable/
sessions/evidence    ──►  qmd/collections/evidence/
archive/*.md         ──►  qmd/collections/archive/

关系：
  内容层是唯一真相源
  QMD 只是加速检索的投影
  QMD 损坏 → memfold qmd sync 重建，不丢数据
```

### 2.2 Collection 组织

QMD 按 collection 组织索引，每个 collection 有独立的检索策略。

MemFold 的映射：

```
检索层          QMD collection    检索策略
──────────────────────────────────────────
stable          stable_col        精确 key + 语义相似
wiki            wiki_col          语义相似优先
archive         archive_col       日期范围 + 关键词
evidence        evidence_col      session_id + claim_fingerprint
```

### 2.3 检索层独立性

QMD 的设计强调检索不应该与存储逻辑耦合，检索层可以升级替换，不影响存储层。

MemFold 的映射：qmd_adapter 模块封装所有 QMD IO，其他模块不直接调用 QMD。如果未来替换索引引擎，只改 qmd_adapter，不动 retrieval / dreaming / boot。

---

## 3. Hermes Agent

**影响范围：记忆系统生命周期边界、宿主与核心分工**

### 3.1 记忆系统有生命周期边界

Hermes Agent 把记忆系统看成有独立生命周期的能力，不是宿主的附属逻辑。记忆系统有自己的初始化、运行、清理边界。

MemFold 的映射：

```
生命周期边界映射：

宿主（Claude Code / Codex）
  只负责触发生命周期事件
  ├── session 开始 → memfold load
  ├── 每轮结束    → memfold write-evidence
  └── session 结束 → memfold write-evidence + 可选 dream run

MemFold 核心
  拥有完整生命周期控制权
  ├── init（初始化目录和 SQLite）
  ├── load（启动包读取和注入）
  ├── write（工作记录写入和归档）
  ├── dream（记忆整理）
  └── repair（一致性修复）
```

### 3.2 宿主不复制记忆逻辑

Hermes Agent 的核心原则：宿主只做薄转换，不在宿主层复制任何记忆决策逻辑。

MemFold 的映射：Claude Code 的 hook 只调 CLI，不判断"这条值不值得记"。Codex 的 tool definition 只传参数，不做内容决策。所有决策在 MemFold 核心内部。

---

## 4. Codex Autoresearch

**影响范围：离线验证框架、改进规则的方法论**

### 4.1 用迭代实验改系统，不在生产里试错

Codex Autoresearch 的核心方法：先定义失败样例，跑离线数据集，验证指标下降后才改生产系统。

MemFold 的映射：

```
改进 dreaming 决策规则的正确顺序：

1. 定义失败样例
   ├── 错误记忆复活率（tombstone 被绕过）
   ├── 自我污染率（分析草稿进入 stable）
   └── 原文泄漏率（token/key 出现在 summary）

2. 改规则（claim_fingerprint 生成逻辑、四选一边界）

3. 跑 memfold experiment run
   验证上述指标是否下降

4. 指标通过后才更新生产规则
```

### 4.2 指标驱动，不靠主观感觉

MemFold experiments 模块定义的核心指标直接来自这个思路：

| 指标 | 通过门槛 |
|------|---------|
| tombstone 命中后的自动晋升率 | 0 |
| 纯分析输入的晋升率 | 0 |
| sterile 来源的晋升率 | 0 |
| 原文泄漏率 | 0 |

---

## 5. MemPalace

**影响范围：历史追溯层的重要性、不要只看"当前最短上下文"**

### 5.1 追溯层是系统可信度的基础

MemPalace 强调：只保留"当前最优化上下文"的系统，在出错时没有任何可追溯的轨迹，无法诊断为什么记忆出了问题。

MemFold 的映射：

```
不追溯的风险：
  stable/*.md 有错误条目
        │
        ▼
  用户说"这个不对"
        │
        ▼
  系统无法知道：这条是从哪来的？dreaming 当时看到了什么？
        │
        ▼
  无法修正根本原因，只能删条目

有追溯的能力：
  stable/*.md 有错误条目
        │
        ▼
  回查 archive/memory-*.md
  找到当时的 evidence_id
        │
        ▼
  检查 dreaming 当时的判断依据
  修正规则或修正 stable 条目
```

### 5.2 追溯层不进启动包

MemPalace 同时提醒：追溯层不应该自动注入上下文，否则每次启动都带着大量历史，变成了另一种污染。

MemFold 的映射：archive / sessions / wiki 默认不进启动包。只有 stable 里 `autoload=boot_project/boot_user` 的条目才进 bundle.md。

---

## 6. llm-wiki-skill / llm-wiki-agent

**影响范围：wiki 层独立于长期记忆、背景知识与稳定偏好的分离**

### 6.1 背景知识页应该独立于长期记忆层

llm-wiki-skill 的核心洞察：把"关于某个主题的背景知识"和"用户的稳定偏好/项目规则"混在一起，会导致两类问题：

1. 背景知识条目太多，长期记忆被稀释
2. 规则条目被当成知识查询，语义检索时带出不相关的东西

MemFold 的映射：

```
llm-wiki 启发的分层：

stable/*.md（长期记忆）
  ├── user.preference.*    用户偏好
  ├── project.rule.*       项目规则
  └── project.constraint.* 项目约束
  → 进入启动包，session 开始时注入

wiki/*.md（背景知识）
  ├── 术语解释
  ├── 技术背景
  └── 领域综述
  → 不进启动包，只在 knowledge_lookup 流时检索

两层分开的好处：
  · 启动包只含"总是需要知道的"，不含"偶尔需要查的"
  · 知识库可以无限扩展，不影响启动 token 预算
  · dreaming 整理 stable 时不会误升知识性条目
```

### 6.2 wiki 的写入和更新不走 dreaming

llm-wiki-agent 的 wiki 条目由人工或显式工具写入，不经过自动整理流程。

MemFold 的映射：wiki 模块独立于 dreaming 流程。dreaming 四选一决策只针对 evidence → stable 的晋升路径，不会把 evidence 自动升到 wiki。wiki 条目通过 `memfold wiki write` 显式写入。

---

## 7. 设计决策汇总

| 设计决策 | 来源启发 | 具体体现 |
|---------|---------|---------|
| 五层记忆结构 | OpenClaw 分层 | boot → stable → wiki → archive → evidence |
| dreaming 四阶段 | OpenClaw ccb auto-dream | Orient / Gather / Consolidate / Prune |
| 每日 archive 文件 | OpenClaw 日志格式 | `archive/memory-YYYY-MM-DD.md`，append-only |
| QMD 为可丢弃侧边车 | QMD 索引设计 | `qmd/` 损坏可 `memfold qmd sync` 重建 |
| 检索层不耦合存储层 | QMD collection 组织 | qmd_adapter 模块封装全部 QMD IO |
| 宿主只触发，核心决策 | Hermes Agent 生命周期边界 | hook 只调 CLI，不复制记忆逻辑 |
| 离线验证改规则 | Codex Autoresearch | experiments 模块，指标驱动，不在生产试错 |
| archive 是永久审计轨迹 | MemPalace 追溯层重要性 | dreaming 读 archive 但不清理 |
| wiki 独立于长期记忆 | llm-wiki-skill 分层 | wiki/*.md 只在 knowledge_lookup 时读 |
| claim_fingerprint 防复活 | (MemFold 原创) | tombstone 封语义断言，不封条目 |
