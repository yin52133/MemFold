#!/usr/bin/env bash
set -euo pipefail

CODEX_HOME="${HOME}/.codex"
MEMFOLD_HOME="${CODEX_HOME}/memfold"
MEMFOLD_BIN="${MEMFOLD_HOME}/bin/memfold"

"${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" init >/dev/null
"${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" load \
  --mode normal \
  --scope-type project \
  --scope-id MemFold \
  --intent startup \
  --budget 400

MEMFOLD_ROOT="${MEMFOLD_HOME}" \
MEMFOLD_SCOPE_TYPE=project \
MEMFOLD_SCOPE_ID=MemFold \
MEMFOLD_SESSION_ID=global_verify \
MEMFOLD_SOURCE_KIND=decision \
MEMFOLD_TURN_SUMMARY="完成全局 MemFold verify 脚本验证" \
MEMFOLD_STATE_CHANGED=1 \
"${MEMFOLD_HOME}/hooks/turn_end.sh"

cdx exec -C "${PWD}" \
  --dangerously-bypass-approvals-and-sandbox \
  --skip-git-repo-check \
  -o /tmp/cdx_memfold_verify.txt \
  "Run exactly this shell command and return only its stdout: memfold --root ~/.codex/memfold load --mode normal --scope-type project --scope-id MemFold --intent startup --budget 400"

cat /tmp/cdx_memfold_verify.txt
