# Wave 3 Module QMD Retrieval

- 状态：`done`
- Owner：`controller`
- 最后更新：`2026-04-12`
- 写入范围：`src/qmd_adapter.rs`, `src/retrieval.rs`, `tests/search_flow.rs`, `docs/progress/memfold-v1/wave-3/01-qmd-retrieval.zh-CN.md`

## Checklist

- [x] qmd sidecar 文档记录
- [x] qmd sync / rebuild
- [x] stable/evidence/archive 检索
- [x] search 结果元数据
- [x] 检索测试

## 验证记录

- `cargo test --test search_flow`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 阻塞记录

- 暂无

## 更新日志

- `2026-04-12`: 模块页初始化
- `2026-04-12`: 本地接管实现并完成 `qmd/retrieval`，`search_flow` 与全量测试通过
