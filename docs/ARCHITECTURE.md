# Platonic Core Architecture

Platonic Core is the harness kernel for bounded agent execution.

## Boundary

In scope:

- run and turn identifiers
- typed context fragments and context packs
- model-facing messages
- tool-call proposals
- effect classes
- policy decisions
- structured tool results
- durable harness events

Out of scope for core:

- Discord/Slack/SMS gateways
- cron scheduler
- desktop UI
- model-provider implementations
- memory backends
- skill packages
- MCP server/client implementations
- filesystem/process/browser tools

Those belong in outer crates or apps.

## Core concepts

### Henad

A bounded agent unit. A henad is not a personality blob; it is an execution identity with policy, tools, and state scope.

### Run

A durable execution instance. A run is event-log-first. Transcripts, metrics, replay, and audit views are derived from events.

### ContextPack

A bounded context bundle assembled from lane-accounted fragments:

- `system_contract`
- `current_task`
- `tool_schemas`
- `recent_turns`
- `retrieved_context`
- `artifact_summary`
- `policy`

The assembler must validate budget before a model call. No hidden prompt growth.

### ToolCall

A model-proposed action with:

- stable call id
- registered tool name
- JSON input
- declared effect class

Inputs are validated by the tool registry outside this crate. Core tracks the boundary.

### EffectClass

Current default posture:

- `read_only` → allow
- `workspace_write` / `network` → require approval unless policy grants it
- `external_side_effect` / `secret_access` → deny by default

### ToolResult

A structured result with:

- short summary
- JSON data
- artifact refs
- model/user/both visibility

Large raw output should be stored as an artifact and summarized for the model.

### HarnessEvent

The durable ledger. Initial events include:

- `run_started`
- `context_built`
- `model_requested`
- `tool_call_proposed`
- `policy_evaluated`
- `tool_started`
- `tool_finished`
- `run_finished`
- `run_failed`

Future storage backends can persist these events to SQLite, Postgres, files, or an append-only log.

## Planned crate split

Possible workspace shape:

```text
platonic-core       typed kernel primitives
platonic-provider   model-provider adapter traits
platonic-tools      tool registry and schema validation
platonic-store      event store implementations
platonic-replay     replay/caching engine
plato-agent         CLI/product agent built on Platonic
```

Do not create these crates until the core contract needs them.
