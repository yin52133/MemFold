# Session Lifecycle Compensation Design

## Goal

Make MemFold more reliable around session boundaries by compensating for missed exit work on the next session start, while keeping exit-time work best-effort and non-blocking.

## Design

- `session_start` remains the canonical startup hook, but now does more than `init + load`.
- Startup order becomes:
  1. `memfold init`
  2. `memfold load`
  3. `memfold dream maybe-run`
  4. `memfold qmd sync`
- The startup `dream maybe-run` call is a compensation pass. If the previous session missed its exit hook or the background dreaming process did not run, the next startup gets a chance to apply scheduled dreaming.
- `session_end` keeps using background `dream maybe-run` so normal exits still nudge consolidation without blocking the user.
- The launcher trap expands from `EXIT INT TERM` to `EXIT INT TERM HUP` to cover one more common non-normal termination path.
- `run_dream()` itself should sync QMD after writing stable memory and recompiling the bundle. That makes the implementation match the intended contract: if dreaming applies stable changes, the retrieval sidecar should be refreshed in the same flow.

## Boundaries

- This is still best-effort, not absolute guarantee. `SIGKILL`, kernel crashes, power loss, and any path that bypasses the launcher cannot be intercepted by shell traps.
- The design does not add a persistent dirty-bit queue yet. It uses startup compensation plus dream-time QMD sync to close the most obvious gaps with minimal surface change.

## Verification

- Script-level regression tests check that startup hooks and launcher traps contain the expected lifecycle calls.
- Dreaming tests verify that running a dream also refreshes QMD collections.
- Full `cargo test`, deploy, and verify runs confirm the end-to-end behavior.
