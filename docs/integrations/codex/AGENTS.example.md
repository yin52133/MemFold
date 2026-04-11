# MemFold Codex Integration

## Memory Tools

At the start of every session, run:

```bash
memfold --root ~/.codex/memfold load --mode normal --scope-type project --scope-id <project-slug> --intent startup --budget 400
```

Inject returned `items[].text` into context before answering.

Available commands:

- `memfold --root ~/.codex/memfold write-evidence ...`
- `memfold --root ~/.codex/memfold search ...`
- `memfold --root ~/.codex/memfold feedback ...`
- `memfold --root ~/.codex/memfold dream run ...`
- `memfold --root ~/.codex/memfold repair ...`
