#!/usr/bin/env bash
set -euo pipefail

MEMFOLD_HOME="${HOME}/.claude/memfold"
MEMFOLD_BIN="${MEMFOLD_HOME}/bin/memfold"
MEMFOLD_ROOT="${MEMFOLD_ROOT:-${MEMFOLD_HOME}}"

read_stdin_json() {
  STDIN_JSON="$(cat)"
}

extract_field() {
  local field="$1"
  echo "${STDIN_JSON}" | jq -r ".${field} // empty"
}

derive_scope_id() {
  local cwd="$1"
  local git_root
  git_root="$(git -C "${cwd}" rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -n "${git_root}" ]]; then
    basename "${git_root}"
    return
  fi
  basename "${cwd}"
}

setup_from_stdin() {
  read_stdin_json
  CC_SESSION_ID="$(extract_field session_id)"
  CC_CWD="$(extract_field cwd)"
  CC_HOOK_EVENT="$(extract_field hook_event_name)"
  CC_TOOL_NAME="$(extract_field tool_name)"

  MEMFOLD_SCOPE_TYPE="${MEMFOLD_SCOPE_TYPE:-project}"
  MEMFOLD_SCOPE_ID="${MEMFOLD_SCOPE_ID:-$(derive_scope_id "${CC_CWD:-$(pwd)}")}"
  MEMFOLD_MODE="${MEMFOLD_MODE:-normal}"
  MEMFOLD_SESSION_ID="${CC_SESSION_ID:-sess_$(date +%s)_$$}"
}

run_memfold() {
  "${MEMFOLD_BIN}" --root "${MEMFOLD_ROOT}" "$@"
}
