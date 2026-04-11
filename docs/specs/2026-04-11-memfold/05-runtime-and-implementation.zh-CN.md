# 05 Runtime And Implementation

## 1. 运行时边界

- 核心语言：`Rust`
- 形态：`CLI-first`
- 后续可扩展：本地后台进程
- 不以 MCP 为主接口

## 2. 宿主与核心的职责分工

宿主：

- 触发 `load`
- 触发 `write_evidence`
- 提交 `feedback`
- 控制 `memory mode`
- 触发显式搜索或 `dream_run`

核心：

- 读取筛选
- 状态机
- 检索路由
- dreaming
- 自我进化评估

## 3. 最小接口

- `load(mode, scope, intent)`
- `write_evidence(scope, source_kind, summary, refs...)`
- `feedback(target, verdict, reason)`
- `search(scope, query, intent, budget)`
- `dream_run(scope, trigger)`

宿主层只负责把各家 skill / plugin / hook 转成这套接口。

## 4. Rust 模块

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

每个模块只做一类事，不跨层偷改状态。

## 5. CLI

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

1. `config + state + memory_fs`
2. `mutations`
3. `boot`
4. `evidence`
5. `retrieval`
6. `qmd_adapter`
7. `feedback`
8. `dreaming`
9. `experiments`

原因：

- 先把真相源和提交协议做稳
- 再做默认读取
- 最后再做复杂整理与实验

## 7. 版本范围

### V1

- Rust CLI 骨架
- 混合目录结构
- SQLite 状态库
- Boot View 编译与加载
- Stable Memory 文件读写
- Evidence JSONL + SQLite 投影
- Trace Archive 索引
- 基础 dreaming
- QMD sidecar 集成
- `normal/fresh/sterile`
- rejected / disputed / quarantine
- evidence 安全清洗
- rebuild / repair
- retention / prune
- 实验环境与生产环境隔离

### V1.1

- conflict analyzer
- boot pollution 评估
- wiki 编译层
- 更完整的 feedback 回路

### V2

- 本地后台进程
- 更复杂的实体图谱
- richer trust model
- 自动调参框架
- 多宿主官方包装层

## 8. 主要风险

- 混合架构复杂度高
- 若绕过 mutation 协议，系统会 split-brain
- 如果 evidence 写入太宽、dreaming 阈值太低，系统会迅速膨胀
