#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

run_memfold init >/dev/null
run_memfold load \
  --mode "${MEMFOLD_MODE}" \
  --scope-type "${MEMFOLD_SCOPE_TYPE}" \
  --scope-id "${MEMFOLD_SCOPE_ID}" \
  --intent "${MEMFOLD_INTENT}" \
  --budget "${MEMFOLD_BUDGET:-400}"
