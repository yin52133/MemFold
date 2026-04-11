#!/usr/bin/env bash
set -euo pipefail

CODEX_HOME="${HOME}/.codex"
MEMFOLD_HOME="${CODEX_HOME}/memfold"
MEMFOLD_BIN="${MEMFOLD_HOME}/bin/memfold"
CODEX_BIN="$(command -v cdx)"

if [[ -z "${CODEX_BIN}" ]]; then
  echo "cdx not found in PATH" >&2
  exit 127
fi

if [[ ! -x "${MEMFOLD_BIN}" ]]; then
  echo "memfold binary not found at ${MEMFOLD_BIN}" >&2
  exit 127
fi

WORKDIR="${PWD}"
SCOPE_TYPE="${MEMFOLD_SCOPE_TYPE:-project}"
SCOPE_ID="${MEMFOLD_SCOPE_ID:-$(basename "${WORKDIR}")}"
MODE="${MEMFOLD_MODE:-normal}"
INTENT="${MEMFOLD_INTENT:-startup}"
SESSION_ID="${MEMFOLD_SESSION_ID:-sess_$(date +%s)_$$}"
SOURCE_KIND="${MEMFOLD_SOURCE_KIND:-decision}"
MEMFOLD_ROOT="${MEMFOLD_ROOT:-${MEMFOLD_HOME}}"

export MEMFOLD_ROOT MEMFOLD_SCOPE_TYPE="${SCOPE_TYPE}" MEMFOLD_SCOPE_ID="${SCOPE_ID}" \
  MEMFOLD_MODE="${MODE}" MEMFOLD_INTENT="${INTENT}" MEMFOLD_SESSION_ID="${SESSION_ID}"

"${MEMFOLD_BIN}" --root "${MEMFOLD_ROOT}" init >/dev/null
"${MEMFOLD_HOME}/hooks/session_start.sh" >/dev/null || true

cleanup() {
  local summary="${MEMFOLD_SESSION_SUMMARY:-session exited via launcher trap}"
  MEMFOLD_SOURCE_KIND="${SOURCE_KIND}" \
  MEMFOLD_SESSION_SUMMARY="${summary}" \
  MEMFOLD_STATE_CHANGED="${MEMFOLD_STATE_CHANGED:-1}" \
  "${MEMFOLD_HOME}/hooks/session_end.sh" >/dev/null 2>&1 || true
}

trap cleanup EXIT INT TERM

"${CODEX_BIN}" "$@"
