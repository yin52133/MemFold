# MemFold Codex Integration

## Memory Tools

At the start of every session, load memory with:

```bash
memfold load --mode normal --scope-type project --scope-id <project-slug> --intent startup
```

Inject returned `items[].text` into context before answering.

Available memory commands:

- `memfold write-evidence --scope-type project --scope-id <project-slug> --session-id <session-id> --source-kind <kind> --summary "<safe-summary>" --promotable 0|1 --origin-mode normal|fresh|sterile`
- `memfold search --scope-type project --scope-id <project-slug> --intent continue|knowledge_lookup --query "<query>" --budget <tokens>`
- `memfold feedback --scope-type project --scope-id <project-slug> --claim-fingerprint <cfp> --verdict confirmed|disputed|rejected --reason "<reason>" [--session-id <session-id>]`
- `memfold dream run --scope-type project --scope-id <project-slug> --trigger manual`
- `memfold repair --scope-type project --scope-id <project-slug>`

## Codex Usage Rules

- Use `write-evidence --promotable 1` only when the user explicitly asks to remember something or when a stable preference/constraint is clear.
- Use `search` when the user references prior work, prior preferences, or background knowledge.
- Use `feedback --verdict rejected` when the user explicitly says an existing memory is wrong.
- Use `dream run` only when explicitly consolidating memory or at a controlled workflow checkpoint.
- In `sterile` mode, do not trigger memory search automatically.
