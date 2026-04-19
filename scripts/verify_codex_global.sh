#!/usr/bin/env bash
set -euo pipefail

CODEX_HOME="${HOME}/.codex"
MEMFOLD_HOME="${CODEX_HOME}/memfold"
MEMFOLD_BIN="${MEMFOLD_HOME}/bin/memfold"
MEMFOLD_DB="${MEMFOLD_HOME}/state/memfold.db"
CDX_MEMFOLD_BIN="${HOME}/.local/bin/cdx-memfold"
HOME_PLUGIN_DIR="${HOME}/plugins/memfold"
HOME_MARKETPLACE="${HOME}/.agents/plugins/marketplace.json"
VERIFY_REPAIR_NEEDED=42
VERIFY_SCOPE_TYPE="${MEMFOLD_SCOPE_TYPE:-project}"
VERIFY_SCOPE_ID="${MEMFOLD_SCOPE_ID:-$(basename "$(pwd)")}"
VERIFY_SUFFIX="$(date +%s)_$$"
LAUNCHER_SESSION_ID="verify_launcher_${VERIFY_SUFFIX}"
HOOK_SESSION_ID="verify_hook_${VERIFY_SUFFIX}"
MEMORY_SESSION_ID="verify_memory_raw_trace"
SEARCH_TOKEN="verify-token-${VERIFY_SUFFIX}"
HOOK_SUMMARY="MemFold hook verify ${SEARCH_TOKEN}"
LAUNCHER_SUMMARY="MemFold launcher verify ${VERIFY_SUFFIX}"
VERIFY_MEMORY_SUMMARY="用户要求默认中文"
VERIFY_MEMORY_RAW_TEXT="以后默认用中文回答，而且直接指出我哪里说错了。"
VERIFY_MEMORY_QUERY="默认用中文回答而且直接指出我哪里说错了"
VERIFY_MEMORY_CLAIM="cfp_verify_raw_trace"
if [[ "${VERIFY_SCOPE_TYPE}" == "project" ]]; then
  VERIFY_SCOPE_DIR_ID="$(printf '%s' "${VERIFY_SCOPE_ID}" | tr '[:upper:]' '[:lower:]')"
  VERIFY_SCOPE_FRAGMENT="repos/${VERIFY_SCOPE_DIR_ID}"
else
  VERIFY_SCOPE_FRAGMENT="user"
fi
VERIFY_SESSION_ROOT="${MEMFOLD_HOME}/memory/${VERIFY_SCOPE_FRAGMENT}/sessions"

fatal() {
  echo "[verify] fatal: $*" >&2
  exit 1
}

repair_needed() {
  echo "[verify] repair-needed: $*" >&2
  exit "${VERIFY_REPAIR_NEEDED}"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fatal "missing required command: $1"
}

require_file() {
  [[ -e "$1" ]] || fatal "missing required path: $1"
}

require_executable() {
  [[ -x "$1" ]] || fatal "required executable missing: $1"
}

ensure_marketplace_entry() {
  python3 - "$HOME_MARKETPLACE" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
if not path.exists():
    sys.exit(1)
data = json.loads(path.read_text())
for plugin in data.get("plugins", []):
    if plugin.get("name") == "memfold":
        sys.exit(0)
sys.exit(1)
PY
}

sqlite_single_value() {
  sqlite3 "${MEMFOLD_DB}" "$1"
}

require_command sqlite3
require_command python3
require_command cdx
require_file "${HOME_MARKETPLACE}"
require_executable "${MEMFOLD_BIN}"
require_executable "${CDX_MEMFOLD_BIN}"
require_executable "${MEMFOLD_HOME}/hooks/session_start.sh"
require_executable "${MEMFOLD_HOME}/hooks/turn_end.sh"
require_executable "${MEMFOLD_HOME}/hooks/session_end.sh"
require_file "${HOME_PLUGIN_DIR}"
ensure_marketplace_entry || fatal "marketplace is missing the memfold plugin entry"

"${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" init >/dev/null
"${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" load \
  --mode normal \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" \
  --intent startup \
  --budget 400

MEMFOLD_SESSION_ID="${LAUNCHER_SESSION_ID}" \
MEMFOLD_SCOPE_TYPE="${VERIFY_SCOPE_TYPE}" \
MEMFOLD_SCOPE_ID="${VERIFY_SCOPE_ID}" \
MEMFOLD_SESSION_SUMMARY="${LAUNCHER_SUMMARY}" \
"${CDX_MEMFOLD_BIN}" --help >/tmp/cdx_memfold_verify_help.txt

launcher_count="$(sqlite_single_value "SELECT COUNT(*) FROM session_log_entries WHERE session_id = '${LAUNCHER_SESSION_ID}' AND summary = '${LAUNCHER_SUMMARY}'")"
[[ "${launcher_count}" -ge 1 ]] || repair_needed "launcher smoke test did not persist the expected session_end summary"

MEMFOLD_ROOT="${MEMFOLD_HOME}" \
MEMFOLD_SCOPE_TYPE="${VERIFY_SCOPE_TYPE}" \
MEMFOLD_SCOPE_ID="${VERIFY_SCOPE_ID}" \
MEMFOLD_SESSION_ID="${HOOK_SESSION_ID}" \
MEMFOLD_SOURCE_KIND=decision \
MEMFOLD_TURN_SUMMARY="${HOOK_SUMMARY}" \
MEMFOLD_STATE_CHANGED=1 \
"${MEMFOLD_HOME}/hooks/turn_end.sh" >/dev/null

hook_count="$(sqlite_single_value "SELECT COUNT(*) FROM session_log_entries WHERE session_id = '${HOOK_SESSION_ID}' AND summary = '${HOOK_SUMMARY}'")"
[[ "${hook_count}" -ge 1 ]] || repair_needed "turn_end hook did not persist the expected session_log row"

if ! "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" qmd sync \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" >/tmp/memfold_qmd_sync_verify.json; then
  repair_needed "qmd sync failed during verify"
fi

if ! search_output="$("${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" search \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" \
  --intent continue \
  --query "${SEARCH_TOKEN}" \
  --budget 80)"; then
  repair_needed "search failed during verify"
fi

search_json="$(printf '%s\n' "${search_output}" | tail -n 1)"

if ! SEARCH_JSON="${search_json}" python3 - "${SEARCH_TOKEN}" <<'PY'
import json
import os
import sys

token = sys.argv[1]
payload = json.loads(os.environ["SEARCH_JSON"])
for item in payload.get("results", []):
    if token in item.get("summary", ""):
        sys.exit(0)
sys.exit(1)
PY
then
  repair_needed "search did not return the newly indexed session_log entry"
fi

if ! "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" write-evidence \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" \
  --session-id "${MEMORY_SESSION_ID}" \
  --source-kind user \
  --summary "${VERIFY_MEMORY_SUMMARY}" \
  --raw-text "${VERIFY_MEMORY_RAW_TEXT}" \
  --promotable 1 \
  --origin-mode normal \
  --claim-fingerprint "${VERIFY_MEMORY_CLAIM}" >/tmp/memfold_verify_memory.json; then
  repair_needed "write-evidence failed for raw_text verify path"
fi

if ! "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" dream run \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" \
  --trigger manual >/tmp/memfold_verify_dream.json; then
  repair_needed "dream run failed during raw_text verify path"
fi

if ! raw_search_output="$("${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" search \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" \
  --intent continue \
  --query "${VERIFY_MEMORY_QUERY}" \
  --budget 120)"; then
  repair_needed "raw_text search failed during verify"
fi

raw_search_json="$(printf '%s\n' "${raw_search_output}" | tail -n 1)"

if ! SEARCH_JSON="${raw_search_json}" python3 - "${VERIFY_MEMORY_SUMMARY}" <<'PY'
import json
import os
import sys

summary = sys.argv[1]
payload = json.loads(os.environ["SEARCH_JSON"])
for item in payload.get("results", []):
    if item.get("summary") == summary:
        sys.exit(0)
sys.exit(1)
PY
then
  repair_needed "raw_text search did not resolve back to the stored memory"
fi

if ! "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" feedback \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" \
  --claim-fingerprint "${VERIFY_MEMORY_CLAIM}" \
  --verdict rejected \
  --reason "verify cleanup" \
  --session-id "${MEMORY_SESSION_ID}" >/tmp/memfold_verify_feedback.json; then
  repair_needed "feedback cleanup failed for verify memory"
fi

rm -rf \
  "${VERIFY_SESSION_ROOT}"/verify_launcher_* \
  "${VERIFY_SESSION_ROOT}"/verify_hook_* \
  "${VERIFY_SESSION_ROOT}"/verify_memory_* \
  "${VERIFY_SESSION_ROOT}/${MEMORY_SESSION_ID}"

if ! "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" repair \
  --scope-type "${VERIFY_SCOPE_TYPE}" \
  --scope-id "${VERIFY_SCOPE_ID}" >/tmp/memfold_verify_cleanup_repair.json; then
  repair_needed "repair cleanup failed for verify memory"
fi

echo "[verify] ok: launcher, hooks, qmd sync, and search all passed"
