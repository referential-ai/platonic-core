# Platonic Core

Core Rust harness primitives for disciplined, replayable agent execution.

Platonic Core is **not** a chatbot, personal assistant, workflow SaaS, or “Agent OS.” It is the small kernel underneath those things: typed context assembly, tool-call boundaries, policy decisions, event logs, and replay/audit surfaces.

Part of the Referential stack.

## Naming

- **Referential** — company/product umbrella.
- **Platonic** — harness core: clean forms, typed boundaries, no sprawl.
- **Plato Agent** — planned usable agent product built on Platonic.

## Design stance

1. Every side effect is typed.
2. Every context byte has a lane and a budget.
3. Every run is an event log first; transcript is a derived view.
4. Provider fallback is a policy event, not a silent retry.
5. Tool output is structured data plus a short summary; raw logs become artifacts.
6. Memory is scoped, typed, source-backed, and budgeted — not a junk drawer.
7. The core stays small. Gateways, dashboards, cron, skills, voice, and platform adapters belong outside the kernel.

## Current crate contents

The crate is intentionally modular inside the kernel:

```text
ids      identifier newtypes
message  model-facing message primitives
context  lane-budgeted context packs
policy   effect classes and policy decisions
tool     tool-call and tool-result boundaries
event    durable harness event ledger
error    shared error types
```

The first contract defines serializable primitives:

- `RunId`, `TurnId`, `HenadId`, `ToolCallId`, `ArtifactId`
- `Message`, `ContextPack`, `ContextFragment`, `ContextLane`
- `EffectClass`, `PolicyDecision`
- `ToolCall`, `ToolResult`, `ResultVisibility`
- `HarnessEvent`

## Verify

```bash
cargo test
```

## Status

Seed repo. Architecture first; implementation should stay narrow until the harness contract is sharp.
