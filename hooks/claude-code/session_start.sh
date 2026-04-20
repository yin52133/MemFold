#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

setup_from_stdin

run_memfold init >/dev/null
run_memfold load \
  --mode "${MEMFOLD_MODE}" \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" \
  --intent startup \
  --budget "${MEMFOLD_BUDGET:-400}"

run_memfold dream maybe-run \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" >/dev/null 2>&1 || true

run_memfold qmd sync \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" >/dev/null 2>&1 || true
