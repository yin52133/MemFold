# 05 Runtime And Implementation

## 1. 运行时结论

- 核心语言：`Rust`
- 形态：`CLI-first`
- 后续可扩展：本地后台进程
- 不以 MCP 为主接口

## 2. 宿主集成边界

服务对象：

- Codex
- Claude Code
- OpenClaw

宿主层负责：

- 触发 load
- 触发 write_observation
- 提交 feedback
- 控制 memory mode
- 触发显式搜索或 dream run

MemFold 核心负责：

- 读取筛选
- 状态机
- 检索路由
- dreaming
- 自我进化评估

## 3. 最小集成协议

建议对外稳定成：

- `load(mode, scope, intent)`
- `write_observation(scope, source_kind, summary, refs...)`
- `feedback(target, verdict, reason)`
- `search(scope, query, intent, budget)`
- `dream_run(scope, trigger)`

这样各家 skill/plugin/hook 只需要做薄转换。

## 4. Rust 模块拆分

- `config`
  - 配置、路径解析、scope 解析
- `state`
  - SQLite、迁移、租约锁
- `memory_fs`
  - Markdown / JSONL 原子读写
- `mutations`
  - 跨存储变更协议和恢复
- `boot`
  - boot bundle 编译与加载
- `observation`
  - observation 创建与预处理
- `retrieval`
  - 检索路由、结果裁剪
- `qmd_adapter`
  - QMD 同步与查询适配
- `feedback`
  - 正负反馈与纠正
- `dreaming`
  - phase 流程与决策
- `wiki`
  - 知识页编译入口
- `experiments`
  - autoresearch 回放与指标

## 5. CLI 子命令建议

- `memfold init`
- `memfold load`
- `memfold write-observation`
- `memfold feedback`
- `memfold dream run`
- `memfold qmd sync`
- `memfold search`
- `memfold bundle compile`
- `memfold repair`
- `memfold experiment run`

## 6. 建议实现顺序

1. `config + state + memory_fs`
2. `mutations`
3. `boot`
4. `observation`
5. `retrieval`
6. `qmd_adapter`
7. `feedback`
8. `dreaming`
9. `experiments`

理由：

- 先把安全加载和内容真相源做稳
- 再上跨存储变更协议
- 然后才做复杂整理和实验系统

## 7. 版本范围

### 8.1 V1 必做

- Rust CLI 骨架
- 混合目录结构
- SQLite 状态库
- boot bundle 编译与加载
- effective memory 文件读写
- observation JSONL + SQLite 投影
- archive 索引
- 基础 dreaming phase
- QMD sidecar 集成
- normal/fresh/sterile
- rejected/disputed/quarantine
- observation 安全清洗
- rebuild / repair
- retention / prune
- 实验环境与生产环境隔离

### 8.2 V1.1 建议补充

- conflict analyzer
- boot pollution 评估
- wiki 编译层
- 更完整的 feedback 回路

### 8.3 V2 再做

- 本地后台进程
- 更复杂的实体图谱
- richer trust model
- 自动调参框架
- 多宿主官方包装层

## 8. 主要风险

### 9.1 复杂度风险

混合架构比纯文件更复杂，但换来更稳的状态机和更低的错误复活概率。

### 9.2 split-brain 风险

如果绕过 mutation 协议直接改内容、状态或索引，系统会分裂。

### 9.3 过度记忆风险

如果 observation 抽取太宽、dreaming 阈值太低，系统会快速膨胀。
