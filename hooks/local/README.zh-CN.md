# Local Hook Rollout

这些脚本用于在当前仓库里先做本地 hook 自测，后续再迁到全局 Codex。

默认本地 memory 根目录：

```bash
./.memfold-local
```

## 脚本

- `hooks/local/session_start.sh`
- `hooks/local/turn_end.sh`
- `hooks/local/session_end.sh`

## 关键环境变量

- `MEMFOLD_SCOPE_TYPE`
- `MEMFOLD_SCOPE_ID`
- `MEMFOLD_SESSION_ID`
- `MEMFOLD_MODE`
- `MEMFOLD_INTENT`
- `MEMFOLD_TURN_SUMMARY`
- `MEMFOLD_SESSION_SUMMARY`
- `MEMFOLD_STATE_CHANGED`
- `MEMFOLD_SOURCE_KIND`

## 设计原则

- start hook 只做 `init + load`
- turn/session end hook 只在“有状态变化”且“摘要通过过滤”时落 evidence
- hook 默认写 `promotable=0`
- 长期候选仍然由 skill/tool 明确写入
