# Wave 5 Codex And E2E

- 状态：`done`
- Owner：`controller`
- 最后更新：`2026-04-12`
- 依赖：`Wave 1-4`
- 退出条件：`Codex 集成工件齐备，CLI 主路径端到端验收通过`

## 1. 范围

这一波负责把核心能力拼成对 Codex 可用的完整闭环，并完成 E2E 验收。

## 2. Checklist

- [x] Codex 集成说明与调用约束整理
- [x] AGENTS / tool 对接工件落地
- [x] session_start -> `load` E2E
- [x] tool/audit -> `write-evidence` E2E
- [x] continue/lookup -> `search` E2E
- [x] reject -> `feedback` -> `tombstone` E2E
- [x] `dream run` E2E
- [x] `repair` E2E
- [x] fresh 模式验收通过
- [x] sterile 模式验收通过
- [x] 完整 CLI snapshot / acceptance tests 全绿

## 3. 验证记录

- `cargo test --test cli_wave2 --test cli_e2e`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 4. 阻塞记录

- 暂无

## 5. 更新日志

- `2026-04-12`: 初始化 Wave 5 进度文件
- `2026-04-12`: Codex 主路径 E2E 已跑通，Wave 5 切换为 `done`
