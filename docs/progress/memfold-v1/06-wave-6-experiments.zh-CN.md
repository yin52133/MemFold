# Wave 6 Experiments

- 状态：`done`
- Owner：`controller`
- 最后更新：`2026-04-12`
- 依赖：`Wave 1-5`
- 退出条件：`experiment run 和离线回归集可执行，关键指标可重复验证`

## 1. 范围

这一波负责离线验证，不直接改变生产规则边界。

## 1.1 模块状态页

- `wave-6/01-experiments.zh-CN.md`

## 2. Checklist

- [x] 建立离线 fixtures / 回归样例集
- [x] CLI：`memfold experiment run`
- [x] tombstone 复活率指标验证
- [x] 分析草稿晋升率指标验证
- [x] sterile 来源晋升率指标验证
- [x] 原文泄漏率指标验证
- [x] 结果输出格式固定
- [x] 实验回归文档补齐
- [x] 模块级 tests 全绿
- [x] 回归集执行通过

## 3. 验证记录

- `cargo test --test experiments_run`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 4. 阻塞记录

- 暂无

## 5. 更新日志

- `2026-04-12`: 初始化 Wave 6 进度文件
- `2026-04-12`: Wave 6 完成，`experiment run` 与离线回归集通过
