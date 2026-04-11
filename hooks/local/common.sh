#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
LOCAL_MEMFOLD_ROOT="${MEMFOLD_ROOT:-${REPO_ROOT}/.memfold-local}"
LOCAL_SCOPE_TYPE="${MEMFOLD_SCOPE_TYPE:-project}"
LOCAL_SCOPE_ID="${MEMFOLD_SCOPE_ID:-$(basename "${REPO_ROOT}")}"
LOCAL_MODE="${MEMFOLD_MODE:-normal}"
LOCAL_INTENT="${MEMFOLD_INTENT:-startup}"

run_memfold() {
  if [[ -x "${REPO_ROOT}/target/debug/memfold" ]]; then
    "${REPO_ROOT}/target/debug/memfold" --root "${LOCAL_MEMFOLD_ROOT}" "$@"
  else
    cargo run --quiet --manifest-path "${REPO_ROOT}/Cargo.toml" -- --root "${LOCAL_MEMFOLD_ROOT}" "$@"
  fi
}
