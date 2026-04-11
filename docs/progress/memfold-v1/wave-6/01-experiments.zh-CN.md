# Wave 6 Module Experiments

- 状态：`done`
- Owner：`controller`
- 最后更新：`2026-04-12`
- 写入范围：`src/experiments.rs`, `tests/experiments_run.rs`, `tests/fixtures/experiments/*`, `src/main.rs`

## Checklist

- [x] fixture schema 定义
- [x] passing/failing 离线回归集
- [x] `memfold experiment run`
- [x] 指标计算与固定输出
- [x] regression tests

## 验证记录

- `cargo test --test experiments_run`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 阻塞记录

- 暂无

## 更新日志

- `2026-04-12`: 建立离线 fixture 驱动的 experiments 模块与 CLI，并完成回归验证
