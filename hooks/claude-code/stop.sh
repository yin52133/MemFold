#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

setup_from_stdin

run_memfold init >/dev/null
run_memfold hook capture \
  --event turn_end \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" \
  --session-id "${MEMFOLD_SESSION_ID}" \
  --source-kind decision \
  --summary "turn completed" \
  --origin-mode "${MEMFOLD_MODE}" \
  --state-changed "${MEMFOLD_STATE_CHANGED:-0}" \
  --promotable 0 >/dev/null 2>&1 || true
