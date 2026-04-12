# Codex Global Hooks

这一组脚本是 **全局 Codex 部署态** 的 canonical hook 源，受 Git 管理。

部署脚本不会再把 hook 内容复制到 `~/.codex/memfold/hooks/` 里长期手改，
而是通过符号链接把全局 hook 指向这里，保证：

- 仓库里的 hook 设计
- 实际部署到 Codex 的 hook

始终同步。

## 脚本

- `common.sh`
- `session_start.sh`
- `turn_end.sh`
- `session_end.sh`

## 行为

- `session_start`: `init + load + dream maybe-run + qmd sync`
- `turn_end`: 过滤后记录非 promotable evidence
- `session_end`: 记录 session 摘要，并在后台静默触发 `dream maybe-run`

## 调度规则

`dream maybe-run` 使用 Rust 核心门控逻辑：

- 距上次 dreaming `>= 24h`
- 新结束 session 数 `>= 5`

不满足时静默返回 `ran=false`，不打断宿主。

这仍然是 best-effort。

- 正常退出时，`session_end` 负责后台 nudging
- 如果上一次退出没触发成功，下一次 `session_start` 会补做一次 `dream maybe-run`
