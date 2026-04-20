#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLAUDE_HOME="${HOME}/.claude"
MEMFOLD_HOME="${CLAUDE_HOME}/memfold"
MEMFOLD_BIN_DIR="${MEMFOLD_HOME}/bin"
MEMFOLD_HOOK_DIR="${MEMFOLD_HOME}/hooks"
SKILLS_DIR="${CLAUDE_HOME}/skills"
SETTINGS_FILE="${CLAUDE_HOME}/settings.json"
VERIFY_SCRIPT="${REPO_ROOT}/scripts/verify_claude_global.sh"
CURRENT_SCOPE_ID="$(basename "${REPO_ROOT}")"
VERIFY_REPAIR_NEEDED=42

sync_scope() {
  local scope_type="$1"
  local scope_id="$2"
  "${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" qmd sync \
    --scope-type "${scope_type}" \
    --scope-id "${scope_id}" >/dev/null
}

sync_known_scopes() {
  sync_scope user default
  sync_scope project "${CURRENT_SCOPE_ID}"

  local repos_dir="${MEMFOLD_HOME}/memory/repos"
  if [[ ! -d "${repos_dir}" ]]; then
    return 0
  fi

  while IFS= read -r repo_name; do
    [[ -n "${repo_name}" ]] || continue
    sync_scope project "${repo_name}"
  done < <(find "${repos_dir}" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort)
}

mkdir -p \
  "${MEMFOLD_BIN_DIR}" \
  "${MEMFOLD_HOOK_DIR}" \
  "${MEMFOLD_HOME}/state" \
  "${MEMFOLD_HOME}/memory" \
  "${MEMFOLD_HOME}/qmd" \
  "${MEMFOLD_HOME}/runtime" \
  "${MEMFOLD_HOME}/config" \
  "${MEMFOLD_HOME}/docs" \
  "${SKILLS_DIR}"

pushd "${REPO_ROOT}" >/dev/null
cargo build --release
popd >/dev/null

cp "${REPO_ROOT}/target/release/memfold" "${MEMFOLD_BIN_DIR}/memfold"
chmod +x "${MEMFOLD_BIN_DIR}/memfold"

ln -sfn "${REPO_ROOT}/hooks/claude-code/common.sh" "${MEMFOLD_HOOK_DIR}/common.sh"
ln -sfn "${REPO_ROOT}/hooks/claude-code/session_start.sh" "${MEMFOLD_HOOK_DIR}/session_start.sh"
ln -sfn "${REPO_ROOT}/hooks/claude-code/stop.sh" "${MEMFOLD_HOOK_DIR}/stop.sh"
ln -sfn "${REPO_ROOT}/hooks/claude-code/session_end.sh" "${MEMFOLD_HOOK_DIR}/session_end.sh"
chmod +x "${REPO_ROOT}/hooks/claude-code/"*.sh

for skill_dir in "${REPO_ROOT}/skills/claude-code/"*/; do
  skill_name="$(basename "${skill_dir}")"
  ln -sfn "${skill_dir}" "${SKILLS_DIR}/${skill_name}"
done

cp "${REPO_ROOT}/docs/integrations/claude-code/deployment.zh-CN.md" "${MEMFOLD_HOME}/docs/deployment.claude-code.zh-CN.md" 2>/dev/null || true
cp "${REPO_ROOT}/docs/integrations/claude-code/deployment.en.md" "${MEMFOLD_HOME}/docs/deployment.claude-code.en.md" 2>/dev/null || true
cp "${REPO_ROOT}/docs/integrations/claude-code/README.zh-CN.md" "${MEMFOLD_HOME}/docs/README.claude-code.zh-CN.md" 2>/dev/null || true
cp "${REPO_ROOT}/docs/integrations/claude-code/README.en.md" "${MEMFOLD_HOME}/docs/README.claude-code.en.md" 2>/dev/null || true

python3 - <<'PY'
import json
from pathlib import Path

settings_path = Path.home() / ".claude" / "settings.json"

if settings_path.exists():
    data = json.loads(settings_path.read_text())
else:
    data = {}

hooks = data.setdefault("hooks", {})

memfold_hook_dir = str(Path.home() / ".claude" / "memfold" / "hooks")

desired = {
    "SessionStart": [{
        "matcher": "startup",
        "hooks": [{
            "type": "command",
            "command": f"{memfold_hook_dir}/session_start.sh",
            "timeout": 15
        }]
    }],
    "Stop": [{
        "hooks": [{
            "type": "command",
            "command": f"{memfold_hook_dir}/stop.sh",
            "timeout": 10
        }]
    }],
    "SessionEnd": [{
        "hooks": [{
            "type": "command",
            "command": f"{memfold_hook_dir}/session_end.sh",
            "timeout": 15
        }]
    }]
}

for event, entries in desired.items():
    existing = hooks.get(event, [])
    for entry in entries:
        cmd = entry.get("hooks", [{}])[0].get("command", "")
        already = any(
            cmd in json.dumps(e)
            for e in existing
        )
        if not already:
            existing.extend(entries)
            break
    hooks[event] = existing

data["hooks"] = hooks
settings_path.write_text(json.dumps(data, indent=2) + "\n")
PY

"${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" init >/dev/null
"${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" repair >/dev/null
if ! sync_known_scopes; then
  echo "[deploy] initial qmd sync failed; continuing to verify" >&2
fi

if "${VERIFY_SCRIPT}"; then
  echo "MemFold (Claude Code) deployed to ${MEMFOLD_HOME}"
  echo "MemFold skills linked at ${SKILLS_DIR}/memfold-*"
  exit 0
else
  verify_status=$?
fi

if [[ "${verify_status}" -ne "${VERIFY_REPAIR_NEEDED}" ]]; then
  exit "${verify_status}"
fi

echo "[deploy] verify reported repairable drift; running memfold repair" >&2
"${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" repair >/dev/null
sync_known_scopes
"${VERIFY_SCRIPT}"

echo "MemFold (Claude Code) deployed to ${MEMFOLD_HOME}"
echo "MemFold skills linked at ${SKILLS_DIR}/memfold-*"
