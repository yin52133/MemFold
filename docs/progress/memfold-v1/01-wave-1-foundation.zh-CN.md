# Wave 1 Foundation

- 状态：`done`
- Owner：`controller + agent-team`
- 最后更新：`2026-04-12`
- 依赖：`无`
- 退出条件：`基础 crate、配置、SQLite 初始化、memory_fs 和对应测试就绪`

## 1. 范围

这一波只负责最底层基础设施，不承担 dreaming、检索或宿主集成逻辑。

## 1.1 模块状态页

- `wave-1/01-domain-config.zh-CN.md`
- `wave-1/02-state.zh-CN.md`
- `wave-1/03-memory-fs.zh-CN.md`
- `wave-1/04-init-integration.zh-CN.md`

## 2. Checklist

- [x] 建立 Rust workspace / crate 结构
- [x] 建立统一错误码与结果类型
- [x] 建立核心 domain types（scope、mode、intent、status 等）
- [x] 配置读取与默认配置落盘
- [x] SQLite 初始化、建表、基础索引
- [x] `memory_fs`：Markdown / JSONL 读写
- [x] `memory_fs`：原子替换与安全路径解析
- [x] 模块级 unit tests 全绿
- [x] Wave 1 小集成 smoke tests 全绿

## 3. 验证记录

- `cargo test --test config_defaults --test state_init --test memory_fs_io --test init_root`
  - 结果：PASS
- `cargo test`
  - 结果：PASS

## 4. 阻塞记录

- 暂无

## 5. 更新日志

- `2026-04-12`: 初始化 Wave 1 进度文件
- `2026-04-12`: 切换到 `in_progress`，开始 Wave 1 实施；执行方式为模块内 TDD + agent team 并行施工
- `2026-04-12`: Wave 1 功能验证通过，等待规格与质量审查回写最终状态
- `2026-04-12`: 规格偏差已修正并完成复验；Wave 1 状态切换为 `done`
