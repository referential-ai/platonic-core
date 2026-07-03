# Agent Instructions for platonic-core

This repo is the small Rust harness kernel for the planned Platonic / Plato Agent stack under Referential.

## Rules

- Keep the core small. Do not add gateways, cron, UI, provider clients, memory stores, or tool implementations to this crate.
- Event log first. If behavior matters, model it as a typed event before building a view around it.
- Every side effect must cross a typed `ToolCall` + `PolicyDecision` boundary.
- Context must be lane-budgeted. Do not add unbounded prompt/memory injection.
- No silent provider fallback. Fallback is policy plus event log entry.
- No `unsafe` in the core crate.
- Prefer explicit types over stringly runtime behavior.
- If a feature feels like Hermes-style sprawl, put it in an outer crate or a doc until proven.

## Verification

Run before pushing changes:

```bash
cargo fmt --check
cargo test
```

For structural changes, update `docs/ARCHITECTURE.md` and `docs/TECHNICAL-LESSONS.md` if assumptions change.
