# MemFold V1 Post-Wave Hardening

状态：`in_progress`

## 1. 目标

这一页记录 Wave 6 之后针对“真实可用性”做的连续加固，不再是基础能力补齐，而是把 MemFold 从“测试通过”推到“真实部署可用、宿主接入不自污染、检索不误召回”。

## 2. 已落地的关键修复

- 统一 `project scope` 到 canonical lowercase，修复 `MemFold / memfold` 分裂导致的空载问题
- `hook / history / qmd` 统一过滤 launcher / verify 噪音
- `repair` 会合并 alias scope、清理脏记忆、修复 `sessions.ended_at`，并避免 tombstoned stable resurrect
- `write_evidence` 不再给 user 条目伪造 `raw_text`
- `dream` 对 user-origin promotion 强制要求显式 raw trace
- `search` 支持 `session_log.raw_text` 检索，并在同 claim 已晋升时优先返回 `stable`
- `search` 对 exact token / hyphenated query 做 exact-first / exact-only 收敛，解决 installed `turn_end -> qmd -> search` verify blocker
- startup `boot bundle` 在单 scope 与跨 `user/project` scope 两层都做正文去重
- `feedback` / `tombstone` / `qmd` / `repair` 的派生层一致性已补齐：被 reject 的 claim 不再通过 sidecar 或 repair 复活
- `verify_codex_global.sh` 现在是幂等 smoke：raw-trace path、verify cleanup、session/history cleanup 都已打通

## 3. 真实环境验证结果

- `cargo test` 全绿
- `./scripts/deploy_codex_global.sh` 可在当前分支上反复通过
- `~/.codex/memfold` 当前 startup load 只保留 canonical `clean_truth` memory
- verify 产生的 smoke memory / smoke session / smoke history / smoke feedback 审计残留已被清到可控范围
- 重复 deploy/verify 不再持续增长 verify raw-trace evidence、verify session 目录或 verify cleanup feedback rows

最新实测快照：

- startup load item count: `1`
- verify session rows / verify session dirs: `0 / 0`
- `feedback_events where reason='verify cleanup'`: `0`
- `tombstones where claim_fingerprint='cfp_verify_raw_trace'`: `1`

## 4. 当前结论

- MemFold 已经从“概念可跑”进入“真实部署可用”
- 当前剩余问题更偏低优先级打磨，不再是主链 correctness 缺口
- 后续如继续迭代，优先考虑：
  - 检索排序与 explainability
  - 宿主集成更多真实工作流
  - 文档继续跟进 shipped behavior
