#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${HOME}/.codex/memfold"
CLAUDE_ROOT="${HOME}/.claude/memfold"

MEMORY_LAYERS=(
  "stable"
  "sessions"
  "history"
)

usage() {
  cat <<'EOF'
Usage: migrate_memory.sh <command> [options]

Commands:
  codex-to-claude   Migrate memory from Codex to Claude Code
  claude-to-codex   Migrate memory from Claude Code to Codex
  sync              Bidirectional sync (merge both directions)

Options:
  --scope-id <id>   Migrate only the specified scope (default: all scopes)
  --dry-run         Show what would be copied without copying
  -h, --help        Show this help

Examples:
  ./scripts/migrate_memory.sh codex-to-claude
  ./scripts/migrate_memory.sh codex-to-claude --scope-id my-project
  ./scripts/migrate_memory.sh sync --scope-id MemFold
  ./scripts/migrate_memory.sh claude-to-codex --dry-run
EOF
  exit 0
}

prompt_conflict() {
  local src="$1"
  local dst="$2"
  local rel="$3"

  echo ""
  echo "=== Conflict: ${rel} ==="
  if command -v diff >/dev/null 2>&1; then
    diff --color=auto -u "${dst}" "${src}" || true
  else
    echo "  source: ${src}"
    echo "  target: ${dst}"
  fi
  echo ""

  while true; do
    read -rp "  [s]kip / [o]verwrite / [k]eep-both (backup target)? " choice
    case "${choice}" in
      s|S|skip)   echo "skip"; return ;;
      o|O|overwrite) echo "overwrite"; return ;;
      k|K|keep-both) echo "keep-both"; return ;;
      *) echo "  Please enter s, o, or k" ;;
    esac
  done
}

copy_file() {
  local src="$1"
  local dst="$2"
  local rel="$3"
  local dry_run="$4"

  if [[ "${dry_run}" == "true" ]]; then
    echo "  [DRY] ${rel}"
    return
  fi

  if [[ -f "${dst}" ]]; then
    if diff -q "${src}" "${dst}" >/dev/null 2>&1; then
      echo "  [SAME] ${rel}"
      return
    fi

    resolution="$(prompt_conflict "${src}" "${dst}" "${rel}")"
    case "${resolution}" in
      skip)
        echo "  [SKIP] ${rel}"
        return
        ;;
      overwrite)
        echo "  [OVERWRITE] ${rel}"
        ;;
      keep-both)
        mv "${dst}" "${dst}.bak"
        echo "  [BACKUP] ${dst}.bak"
        ;;
    esac
  else
    echo "  [COPY] ${rel}"
  fi

  mkdir -p "$(dirname "${dst}")"
  cp -a "${src}" "${dst}"
}

collect_scopes() {
  local root="$1"
  local memory_dir="${root}/memory"
  local scopes=()

  if [[ -d "${memory_dir}/user" ]]; then
    scopes+=("user/")
  fi

  if [[ -d "${memory_dir}/repos" ]]; then
    for repo_dir in "${memory_dir}/repos/"*/; do
      [[ -d "${repo_dir}" ]] || continue
      scopes+=("repos/$(basename "${repo_dir}")/")
    done
  fi

  printf '%s\n' "${scopes[@]}"
}

migrate_scope() {
  local src_root="$1"
  local dst_root="$2"
  local scope_path="$3"
  local dry_run="$4"
  local src_mem="${src_root}/memory/${scope_path}"
  local dst_mem="${dst_root}/memory/${scope_path}"

  if [[ ! -d "${src_mem}" ]]; then
    echo "  Source scope not found: ${src_mem}"
    return
  fi

  for layer in "${MEMORY_LAYERS[@]}"; do
    local layer_dir="${src_mem}${layer}"
    [[ -d "${layer_dir}" ]] || continue

    while IFS= read -r -d '' src_file; do
      local rel="${src_file#"${src_root}/memory/"}"
      local dst_file="${dst_root}/memory/${rel}"
      copy_file "${src_file}" "${dst_file}" "${rel}" "${dry_run}"
    done < <(find "${layer_dir}" -type f -print0 | sort -z)
  done
}

post_migrate() {
  local root="$1"
  local scope_id="$2"
  local bin="${root}/bin/memfold"

  if [[ ! -x "${bin}" ]]; then
    echo "[WARN] memfold binary not found at ${bin}, skipping post-migration steps"
    return
  fi

  echo ""
  echo "--- Post-migration: repair + bundle + qmd sync ---"

  "${bin}" --root "${root}" repair >/dev/null 2>&1 || true

  if [[ -n "${scope_id}" ]]; then
    "${bin}" --root "${root}" bundle compile \
      --scope-type project --scope-id "${scope_id}" >/dev/null 2>&1 || true
    "${bin}" --root "${root}" qmd sync \
      --scope-type project --scope-id "${scope_id}" >/dev/null 2>&1 || true
  fi

  "${bin}" --root "${root}" bundle compile \
    --scope-type user --scope-id default >/dev/null 2>&1 || true
  "${bin}" --root "${root}" qmd sync \
    --scope-type user --scope-id default >/dev/null 2>&1 || true

  echo "  Done."
}

do_migrate() {
  local src_root="$1"
  local dst_root="$2"
  local scope_id="$3"
  local dry_run="$4"
  local src_label="$5"
  local dst_label="$6"

  echo "=== Migrating: ${src_label} -> ${dst_label} ==="

  if [[ ! -d "${src_root}/memory" ]]; then
    echo "Source memory directory not found: ${src_root}/memory"
    exit 1
  fi

  mkdir -p "${dst_root}/memory"

  if [[ -n "${scope_id}" ]]; then
    echo "Scope: ${scope_id}"
    migrate_scope "${src_root}" "${dst_root}" "user/" "${dry_run}"
    migrate_scope "${src_root}" "${dst_root}" "repos/${scope_id}/" "${dry_run}"
  else
    echo "Scope: all"
    while IFS= read -r scope_path; do
      [[ -n "${scope_path}" ]] || continue
      echo ""
      echo "--- ${scope_path} ---"
      migrate_scope "${src_root}" "${dst_root}" "${scope_path}" "${dry_run}"
    done < <(collect_scopes "${src_root}")
  fi

  if [[ "${dry_run}" != "true" ]]; then
    post_migrate "${dst_root}" "${scope_id}"
  fi
}

COMMAND=""
SCOPE_ID=""
DRY_RUN="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    codex-to-claude|claude-to-codex|sync)
      COMMAND="$1"; shift ;;
    --scope-id)
      SCOPE_ID="$2"; shift 2 ;;
    --dry-run)
      DRY_RUN="true"; shift ;;
    -h|--help)
      usage ;;
    *)
      echo "Unknown argument: $1"; usage ;;
  esac
done

if [[ -z "${COMMAND}" ]]; then
  usage
fi

case "${COMMAND}" in
  codex-to-claude)
    do_migrate "${CODEX_ROOT}" "${CLAUDE_ROOT}" "${SCOPE_ID}" "${DRY_RUN}" "Codex" "Claude Code"
    ;;
  claude-to-codex)
    do_migrate "${CLAUDE_ROOT}" "${CODEX_ROOT}" "${SCOPE_ID}" "${DRY_RUN}" "Claude Code" "Codex"
    ;;
  sync)
    echo "=== Bidirectional sync ==="
    echo ""
    echo "--- Phase 1: Codex -> Claude Code ---"
    do_migrate "${CODEX_ROOT}" "${CLAUDE_ROOT}" "${SCOPE_ID}" "${DRY_RUN}" "Codex" "Claude Code"
    echo ""
    echo "--- Phase 2: Claude Code -> Codex ---"
    do_migrate "${CLAUDE_ROOT}" "${CODEX_ROOT}" "${SCOPE_ID}" "${DRY_RUN}" "Claude Code" "Codex"
    echo ""
    echo "=== Sync complete ==="
    ;;
esac
