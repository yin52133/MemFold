#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX_HOME="${HOME}/.codex"
MEMFOLD_HOME="${CODEX_HOME}/memfold"
MEMFOLD_BIN_DIR="${MEMFOLD_HOME}/bin"
MEMFOLD_HOOK_DIR="${MEMFOLD_HOME}/hooks"
HOME_PLUGIN_DIR="${HOME}/plugins/memfold"
HOME_MARKETPLACE="${HOME}/.agents/plugins/marketplace.json"
PLUGIN_CACHE_DIR="${HOME}/.codex/plugins/cache/home-local/memfold"
PLUGIN_CACHE_STAMP="${CODEX_HOME}/.tmp/plugins.sha"
VERIFY_SCRIPT="${REPO_ROOT}/scripts/verify_codex_global.sh"
CURRENT_SCOPE_ID="$(basename "${REPO_ROOT}")"
VERIFY_REPAIR_NEEDED=42

clear_plugin_cache() {
  rm -rf "${PLUGIN_CACHE_DIR}"
  rm -f "${PLUGIN_CACHE_STAMP}"
}

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
  "${MEMFOLD_HOME}/scripts" \
  "${MEMFOLD_HOME}/docs" \
  "${MEMFOLD_HOME}/config" \
  "${MEMFOLD_HOME}/state" \
  "${MEMFOLD_HOME}/memory" \
  "${MEMFOLD_HOME}/qmd" \
  "${MEMFOLD_HOME}/runtime" \
  "${HOME}/plugins" \
  "$(dirname "${HOME_MARKETPLACE}")"

pushd "${REPO_ROOT}" >/dev/null
cargo build --release
popd >/dev/null

cp "${REPO_ROOT}/target/release/memfold" "${MEMFOLD_BIN_DIR}/memfold"
chmod +x "${MEMFOLD_BIN_DIR}/memfold"
ln -sf "${MEMFOLD_BIN_DIR}/memfold" "${HOME}/.local/bin/memfold"
cp "${REPO_ROOT}/scripts/codex/cdx_memfold.sh" "${MEMFOLD_HOME}/scripts/cdx-memfold"
chmod +x "${MEMFOLD_HOME}/scripts/cdx-memfold"
ln -sf "${MEMFOLD_HOME}/scripts/cdx-memfold" "${HOME}/.local/bin/cdx-memfold"

ln -sfn "${REPO_ROOT}/hooks/codex-global/common.sh" "${MEMFOLD_HOOK_DIR}/common.sh"
ln -sfn "${REPO_ROOT}/hooks/codex-global/session_start.sh" "${MEMFOLD_HOOK_DIR}/session_start.sh"
ln -sfn "${REPO_ROOT}/hooks/codex-global/turn_end.sh" "${MEMFOLD_HOOK_DIR}/turn_end.sh"
ln -sfn "${REPO_ROOT}/hooks/codex-global/session_end.sh" "${MEMFOLD_HOOK_DIR}/session_end.sh"
chmod +x "${REPO_ROOT}/hooks/codex-global/"*.sh

cp "${REPO_ROOT}/docs/integrations/codex/deployment.zh-CN.md" "${MEMFOLD_HOME}/docs/deployment.codex.zh-CN.md"
cp "${REPO_ROOT}/docs/integrations/codex/deployment.en.md" "${MEMFOLD_HOME}/docs/deployment.codex.en.md"
cp "${REPO_ROOT}/docs/integrations/codex/README.zh-CN.md" "${MEMFOLD_HOME}/docs/README.codex.zh-CN.md"
cp "${REPO_ROOT}/docs/integrations/codex/README.en.md" "${MEMFOLD_HOME}/docs/README.codex.en.md"
cp "${REPO_ROOT}/docs/integrations/codex/AGENTS.example.md" "${MEMFOLD_HOME}/docs/AGENTS.example.md"
rm -rf "${CODEX_HOME}/skills/memfold-codex-memory" "${CODEX_HOME}/skills/memfold-dream" "${CODEX_HOME}/skills/memfold-qmd" "${CODEX_HOME}/skills/memfold-search" "${CODEX_HOME}/skills/memfold-remember" "${CODEX_HOME}/skills/memfold-forget"
ln -sfn "${REPO_ROOT}/plugins/memfold" "${HOME_PLUGIN_DIR}"
clear_plugin_cache
cp "${REPO_ROOT}/README.md" "${MEMFOLD_HOME}/README.repo.zh-CN.md"
cp "${REPO_ROOT}/README.en.md" "${MEMFOLD_HOME}/README.repo.en.md"
cp "${REPO_ROOT}/hooks/local/README.zh-CN.md" "${MEMFOLD_HOME}/docs/hooks.local.zh-CN.md"
cp "${REPO_ROOT}/hooks/codex-global/README.zh-CN.md" "${MEMFOLD_HOME}/docs/hooks.codex-global.zh-CN.md"
cp "${REPO_ROOT}/tests/fixtures/experiments/passing.json" "${MEMFOLD_HOME}/docs/experiments.passing.json"
cp "${REPO_ROOT}/tests/fixtures/experiments/failing.json" "${MEMFOLD_HOME}/docs/experiments.failing.json"

python3 - <<'PY'
import json
from pathlib import Path

marketplace = Path.home() / ".agents" / "plugins" / "marketplace.json"
marketplace.parent.mkdir(parents=True, exist_ok=True)
if marketplace.exists():
    data = json.loads(marketplace.read_text())
else:
    data = {
        "name": "home-local",
        "interface": {"displayName": "Home Local Plugins"},
        "plugins": []
    }
plugins = data.setdefault("plugins", [])
entry = {
    "name": "memfold",
    "source": {
        "source": "local",
        "path": "./plugins/memfold"
    },
    "policy": {
        "installation": "INSTALLED_BY_DEFAULT",
        "authentication": "ON_USE"
    },
    "category": "Coding"
}
plugins = [p for p in plugins if p.get("name") != "memfold"]
plugins.append(entry)
data["plugins"] = plugins
marketplace.write_text(json.dumps(data, indent=2) + "\n")
PY

"${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" init >/dev/null
"${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" repair >/dev/null
if ! sync_known_scopes; then
  echo "[deploy] initial qmd sync failed; continuing to verify for repair gating" >&2
fi

if "${VERIFY_SCRIPT}"; then
  echo "MemFold deployed to ${MEMFOLD_HOME}"
  echo "MemFold plugin linked at ${HOME_PLUGIN_DIR}"
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

echo "MemFold deployed to ${MEMFOLD_HOME}"
echo "MemFold plugin linked at ${HOME_PLUGIN_DIR}"
