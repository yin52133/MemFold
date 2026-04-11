# Wave 2 Module Mutations

- 状态：`done`
- Owner：`worker-mutations`
- 最后更新：`2026-04-12`
- 写入范围：`src/mutations.rs`, `tests/mutations_flow.rs`, `docs/progress/memfold-v1/wave-2/01-mutations.zh-CN.md`

## Checklist

- [x] mutation 记录创建
- [x] status 推进：`pending -> applied_to_content -> fully_applied`
- [x] scope 级 pending mutation 检查
- [x] 对 Wave 2 集成开放最小接口

## 验证记录

- `cargo test --test mutations_flow`
  - 结果：PASS
  - 说明：`target_ref` 按 `scope_key::raw_target_ref` 方式存储，`has_unstable_scope_mutations` 将所有非 `fully_applied` 状态视为 unstable
- `cargo test`
  - 结果：FAIL
  - 原因：`tests/cli_wave2.rs` 仍然引用未实现的 `memfold::boot::compile_scope_bundle`，超出本次写入范围

## 阻塞记录

- `2026-04-12`: 实现 mutation 生命周期，补充回归测试，target scope 通过；全量测试仍受 CLI 集成未完成阻塞

## 更新日志

- `2026-04-12`: 模块页初始化
