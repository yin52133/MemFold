#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

setup_from_stdin

run_memfold init >/dev/null
run_memfold hook capture \
  --event session_end \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" \
  --session-id "${MEMFOLD_SESSION_ID}" \
  --source-kind decision \
  --summary "session ended" \
  --origin-mode "${MEMFOLD_MODE}" \
  --state-changed 1 \
  --promotable 0 >/dev/null 2>&1 || true

run_memfold summarize-history \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" \
  --session-id "${MEMFOLD_SESSION_ID}" \
  --trigger session_end >/dev/null 2>&1 || true

nohup "${MEMFOLD_BIN}" --root "${MEMFOLD_ROOT}" dream maybe-run \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" >/dev/null 2>&1 &
