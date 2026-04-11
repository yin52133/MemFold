#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MEMFOLD_HOME="${HOME}/.codex/memfold"
MEMFOLD_BIN="${MEMFOLD_HOME}/bin/memfold"
MEMFOLD_ROOT="${MEMFOLD_ROOT:-${MEMFOLD_HOME}}"
MEMFOLD_SCOPE_TYPE="${MEMFOLD_SCOPE_TYPE:-project}"
MEMFOLD_SCOPE_ID="${MEMFOLD_SCOPE_ID:-unknown-project}"
MEMFOLD_MODE="${MEMFOLD_MODE:-normal}"
MEMFOLD_INTENT="${MEMFOLD_INTENT:-startup}"

run_memfold() {
  "${MEMFOLD_BIN}" --root "${MEMFOLD_ROOT}" "$@"
}
