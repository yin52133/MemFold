# Wave 4 Feedback Dream Repair

- 状态：`done`
- Owner：`controller + agent-team`
- 最后更新：`2026-04-12`
- 依赖：`Wave 1-3`
- 退出条件：`feedback、dream run、repair 可用，tombstone 与 quarantine 规则可验证`

## 1. 范围

这一波负责纠错、记忆整理和修复，不负责最终宿主集成验收。

## 1.1 模块状态页

- `wave-4/01-feedback-dreaming.zh-CN.md`
- `wave-4/02-repair-e2e.zh-CN.md`

## 2. Checklist

- [x] `feedback`：`confirmed / disputed / rejected` 事件写入
- [x] `feedback`：rejected -> tombstone 写入
- [x] `dream_jobs` 生命周期实现
- [x] `dreaming` Phase 1: Orient
- [x] `dreaming` Phase 2: Gather
- [x] `dreaming` Phase 3: Consolidate
- [x] `dreaming` Phase 4: Prune
- [x] CLI：`memfold dream run`
- [x] CLI：`memfold repair`
- [x] repair 顺序实现：内容层 -> SQLite -> 派生层
- [x] tombstone 防复活规则测试通过
- [x] sterile 来源不可晋升测试通过
- [x] 模块级 unit tests 全绿
- [x] `feedback / dream / repair` 集成测试全绿

## 3. 验证记录

- `cargo test --test feedback_dream_repair --test cli_e2e`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 4. 阻塞记录

- 暂无

## 5. 更新日志

- `2026-04-12`: 初始化 Wave 4 进度文件
- `2026-04-12`: 切换为 `in_progress`，开始 feedback/dream/repair 实施
- `2026-04-12`: Wave 4 完成，反馈、dream、repair 与相关 CLI/E2E 全部通过
