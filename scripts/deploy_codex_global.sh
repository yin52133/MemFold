#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX_HOME="${HOME}/.codex"
MEMFOLD_HOME="${CODEX_HOME}/memfold"
MEMFOLD_BIN_DIR="${MEMFOLD_HOME}/bin"
MEMFOLD_HOOK_DIR="${MEMFOLD_HOME}/hooks"
SKILL_DIR="${CODEX_HOME}/skills/memfold-codex-memory"

mkdir -p \
  "${MEMFOLD_BIN_DIR}" \
  "${MEMFOLD_HOOK_DIR}" \
  "${MEMFOLD_HOME}/docs" \
  "${MEMFOLD_HOME}/config" \
  "${MEMFOLD_HOME}/state" \
  "${MEMFOLD_HOME}/memory" \
  "${MEMFOLD_HOME}/qmd" \
  "${MEMFOLD_HOME}/runtime" \
  "${CODEX_HOME}/skills"

pushd "${REPO_ROOT}" >/dev/null
cargo build --release
popd >/dev/null

cp "${REPO_ROOT}/target/release/memfold" "${MEMFOLD_BIN_DIR}/memfold"
chmod +x "${MEMFOLD_BIN_DIR}/memfold"
ln -sf "${MEMFOLD_BIN_DIR}/memfold" "${HOME}/.local/bin/memfold"

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
rm -rf "${SKILL_DIR}"
ln -sfn "${REPO_ROOT}/skills/memfold-codex-memory" "${SKILL_DIR}"
cp "${REPO_ROOT}/README.md" "${MEMFOLD_HOME}/README.repo.zh-CN.md"
cp "${REPO_ROOT}/README.en.md" "${MEMFOLD_HOME}/README.repo.en.md"
cp "${REPO_ROOT}/hooks/local/README.zh-CN.md" "${MEMFOLD_HOME}/docs/hooks.local.zh-CN.md"
cp "${REPO_ROOT}/hooks/codex-global/README.zh-CN.md" "${MEMFOLD_HOME}/docs/hooks.codex-global.zh-CN.md"
cp "${REPO_ROOT}/tests/fixtures/experiments/passing.json" "${MEMFOLD_HOME}/docs/experiments.passing.json"
cp "${REPO_ROOT}/tests/fixtures/experiments/failing.json" "${MEMFOLD_HOME}/docs/experiments.failing.json"

"${MEMFOLD_BIN_DIR}/memfold" --root "${MEMFOLD_HOME}" init >/dev/null

echo "MemFold deployed to ${MEMFOLD_HOME}"
