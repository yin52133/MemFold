#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_DIR="${REPO_ROOT}/hooks/claude-code"

PASS=0
FAIL=0

check() {
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "  [PASS] ${desc}"
    PASS=$((PASS + 1))
  else
    echo "  [FAIL] ${desc}"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== Claude Code Hook Smoke Tests ==="

echo ""
echo "--- Script existence and permissions ---"
for script in common.sh session_start.sh stop.sh session_end.sh; do
  check "${script} exists" test -f "${HOOK_DIR}/${script}"
  check "${script} executable" test -x "${HOOK_DIR}/${script}"
done

echo ""
echo "--- common.sh sources correctly ---"
check "common.sh defines MEMFOLD_HOME" bash -c "
  source '${HOOK_DIR}/common.sh' <<< '{}'
  [[ -n \"\${MEMFOLD_HOME}\" ]]
"

check "common.sh defines run_memfold function" bash -c "
  source '${HOOK_DIR}/common.sh' <<< '{}'
  declare -f run_memfold >/dev/null
"

echo ""
echo "--- stdin JSON parsing ---"
check "extract session_id from JSON" bash -c "
  source '${HOOK_DIR}/common.sh' <<< '{\"session_id\":\"test-123\",\"cwd\":\"/tmp\",\"hook_event_name\":\"SessionStart\"}'
  setup_from_stdin
  [[ \"\${CC_SESSION_ID}\" == 'test-123' ]]
" <<< '{"session_id":"test-123","cwd":"/tmp","hook_event_name":"SessionStart"}'

check "extract cwd from JSON" bash -c "
  STDIN_JSON='{\"session_id\":\"s1\",\"cwd\":\"/home/user/myproject\",\"hook_event_name\":\"Stop\"}'
  source '${HOOK_DIR}/common.sh' <<< '{}'
  read_stdin_json <<< \"\${STDIN_JSON}\"
  [[ \"\$(extract_field cwd)\" == '/home/user/myproject' ]]
"

echo ""
echo "--- scope derivation ---"
check "derive_scope_id from git repo" bash -c "
  source '${HOOK_DIR}/common.sh' <<< '{}'
  result=\$(derive_scope_id '${REPO_ROOT}')
  [[ \"\${result}\" == 'MemFold' ]]
"

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ "${FAIL}" -eq 0 ]]
