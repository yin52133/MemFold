#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

run_memfold init >/dev/null
run_memfold load \
  --mode "${LOCAL_MODE}" \
  --scope-type "${LOCAL_SCOPE_TYPE}" \
  --scope-id "${LOCAL_SCOPE_ID}" \
  --intent "${LOCAL_INTENT}" \
  --budget "${MEMFOLD_BUDGET:-400}"
