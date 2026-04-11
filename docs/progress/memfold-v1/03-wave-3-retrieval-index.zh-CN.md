# Wave 3 Retrieval And Index

- 状态：`done`
- Owner：`controller + agent-team`
- 最后更新：`2026-04-12`
- 依赖：`Wave 1-2`
- 退出条件：`search、bundle compile、qmd sync 路径打通，分层检索规则可验证`

## 1. 范围

这一波负责检索和索引 sidecar，不负责 feedback 与 dreaming 决策。

## 1.1 模块状态页

- `wave-3/01-qmd-retrieval.zh-CN.md`
- `wave-3/02-cli-search.zh-CN.md`

## 2. Checklist

- [x] `qmd_adapter` 抽象与本地索引接入
- [x] QMD 文档回指 `pointer` 契约落地
- [x] `qmd sync` 增量同步与重建路径
- [x] `retrieval` 分层读取顺序实现
- [x] `retrieval`：`startup / continue / knowledge_lookup` 规则实现
- [x] CLI：`memfold search`
- [x] CLI：`memfold bundle compile`
- [x] CLI：`memfold qmd sync`
- [x] 检索命中元数据回填
- [x] 模块级 unit tests 全绿
- [x] `search / bundle compile / qmd sync` 集成测试全绿

## 3. 验证记录

- `cargo test --test search_flow --test cli_e2e`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 4. 阻塞记录

- 暂无

## 5. 更新日志

- `2026-04-12`: 初始化 Wave 3 进度文件
- `2026-04-12`: 切换为 `in_progress`，开始 retrieval/index 实施
- `2026-04-12`: Wave 3 完成，检索、qmd sync、bundle compile 路径与 CLI E2E 全部打通
