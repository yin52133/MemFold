#!/usr/bin/env bash
set -euo pipefail

MEMFOLD_HOME="${HOME}/.claude/memfold"
MEMFOLD_BIN="${MEMFOLD_HOME}/bin/memfold"
SKILLS_DIR="${HOME}/.claude/skills"
SETTINGS_FILE="${HOME}/.claude/settings.json"

PASS=0
FAIL=0
REPAIR_NEEDED=0

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

check_repair() {
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "  [PASS] ${desc}"
    PASS=$((PASS + 1))
  else
    echo "  [REPAIR] ${desc}"
    REPAIR_NEEDED=$((REPAIR_NEEDED + 1))
  fi
}

echo "=== MemFold Claude Code Verification ==="

echo ""
echo "--- Binary ---"
check "memfold binary exists" test -x "${MEMFOLD_BIN}"
check "memfold init" "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" init

echo ""
echo "--- Hooks ---"
check "session_start.sh exists" test -x "${MEMFOLD_HOME}/hooks/session_start.sh"
check "stop.sh exists" test -x "${MEMFOLD_HOME}/hooks/stop.sh"
check "session_end.sh exists" test -x "${MEMFOLD_HOME}/hooks/session_end.sh"
check "common.sh exists" test -x "${MEMFOLD_HOME}/hooks/common.sh"

echo ""
echo "--- Settings ---"
check "settings.json exists" test -f "${SETTINGS_FILE}"
check "SessionStart hook configured" python3 -c "
import json, sys
d = json.loads(open('${SETTINGS_FILE}').read())
hooks = d.get('hooks', {}).get('SessionStart', [])
assert any('session_start.sh' in json.dumps(h) for h in hooks)
"

echo ""
echo "--- Skills ---"
for skill in memfold-memory memfold-search memfold-remember memfold-forget memfold-dream memfold-qmd; do
  check "skill ${skill} linked" test -f "${SKILLS_DIR}/${skill}/SKILL.md"
done

echo ""
echo "--- Smoke Test ---"
SMOKE_SCOPE="verify-claude-smoke-$$"

check_repair "memfold load smoke" "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" load \
  --mode normal --scope-type project --scope-id "${SMOKE_SCOPE}" \
  --intent startup --budget 50

check_repair "memfold write-evidence smoke" "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" write-evidence \
  --scope-type project --scope-id "${SMOKE_SCOPE}" \
  --session-id "verify-sess-$$" \
  --source-kind decision \
  --summary "claude verification smoke test" \
  --promotable 0 --origin-mode normal

check_repair "memfold search smoke" "${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" search \
  --scope-type project --scope-id "${SMOKE_SCOPE}" \
  --intent continue --query "smoke test" --budget 50

"${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" feedback \
  --scope-type project --scope-id "${SMOKE_SCOPE}" \
  --claim-fingerprint "verify-claude-smoke" \
  --verdict rejected \
  --reason "cleanup after verification" >/dev/null 2>&1 || true

"${MEMFOLD_BIN}" --root "${MEMFOLD_HOME}" repair >/dev/null 2>&1 || true

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed, ${REPAIR_NEEDED} repairable ==="

if [[ "${FAIL}" -gt 0 ]]; then
  exit 1
elif [[ "${REPAIR_NEEDED}" -gt 0 ]]; then
  exit 42
fi
exit 0
