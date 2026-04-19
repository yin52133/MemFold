# MemFold V1 Master Checklist

- 状态：`done`
- 项目：`memfold-v1`
- 最后更新：`2026-04-19`
- 主控：`controller`

## 1. 里程碑

- [x] 规格文档已锁定实现技术栈：Rust + SQLite + QMD 本地小模型索引
- [x] 并行施工进度机制已建档
- [x] Wave 1 完成：foundation
- [x] Wave 2 完成：write/load
- [x] Wave 3 完成：retrieval/index
- [x] Wave 4 完成：feedback/dream/repair
- [x] Wave 5 完成：codex integration + e2e
- [x] Wave 6 完成：experiments
- [x] 完整 V1 E2E 验收通过

## 2. 波次状态总览

| Wave | 范围 | 状态 | Owner | 最后更新 | 验证状态 | 阻塞 |
|------|------|------|-------|----------|----------|------|
| 1 | foundation | `done` | controller + agent-team | 2026-04-12 | `cargo test --test config_defaults --test state_init --test memory_fs_io --test init_root` + `cargo test` 通过 | 无 |
| 2 | write/load | `done` | controller + agent-team | 2026-04-12 | `cargo test --test mutations_flow --test boot_load --test evidence_write --test cli_wave2` + `cargo test` 通过 | 无 |
| 3 | retrieval/index | `done` | controller | 2026-04-12 | `cargo test --test search_flow --test cli_e2e` + `cargo test` 通过 | 无 |
| 4 | feedback/dream/repair | `done` | controller | 2026-04-12 | `cargo test --test feedback_dream_repair --test cli_e2e` + `cargo test` 通过 | 无 |
| 5 | codex/e2e | `done` | controller | 2026-04-12 | `cargo test --test cli_wave2 --test cli_e2e` + `cargo test` 通过 | 无 |
| 6 | experiments | `done` | controller | 2026-04-12 | `cargo test --test experiments_run` + `cargo test` 通过 | 无 |

## 3. 集成关口

- [x] Gate A: `config + state + memory_fs + init bootstrap` 集成通过
- [x] Gate B: `mutations + boot + evidence + init/load/write-evidence` 集成通过
- [x] Gate C: `retrieval + qmd_adapter + search + bundle compile + qmd sync` 集成通过
- [x] Gate D: `feedback + dreaming + repair` 集成通过
- [x] Gate E: `Codex host flow + CLI E2E` 通过
- [x] Gate F: `experiment run` 与离线回归集通过

## 4. 当前已知前置约束

- 核心语言固定：`Rust`
- 状态层固定：`SQLite`
- 索引侧边车固定：`QMD + 本地小模型索引`
- 实施流程固定：模块内 TDD -> 波次集成 -> 系统 E2E

## 5. 更新约定

- 各 wave 的细节只在对应文件维护
- 主控只在波次状态发生变化时回写本文件
- 如果总览板和 wave 文件冲突，以 wave 文件的最新记录为准，随后主控负责同步

## 6. 更新日志

- `2026-04-12`: Wave 1 切换为 `in_progress`，已生成实现计划并开始 agent team 施工
- `2026-04-12`: Wave 1 已通过 `cargo test --test config_defaults --test state_init --test memory_fs_io --test init_root` 和 `cargo test`，当前进入审查阶段
- `2026-04-12`: Wave 1 规格偏差修正完成，Gate A 勾选，基础层切换为 `done`
- `2026-04-12`: Wave 2 完成并勾选 Gate B，当前进入 retrieval/index 与 feedback/dream/repair 阶段
- `2026-04-12`: Wave 3、4、5 已全部完成，Gate C/D/E 勾选，核心 E2E 跑通
- `2026-04-12`: Wave 6 完成并勾选 Gate F，MemFold V1 全部收口
- `2026-04-19`: 补充 `07-post-v1-hardening.zh-CN.md`，记录 V1 发货后的 retrieval/repair/deploy/verify 连续加固结果；当前真实全局部署态已重新同步并通过 end-to-end verify
