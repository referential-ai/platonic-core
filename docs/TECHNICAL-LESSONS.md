# Technical Lessons from Rust Agent Harnesses

This doc captures what Platonic should learn from current Rust agent/harness projects without copying their sprawl.

## Goose

What to learn:

- Native CLI/desktop/API matters for adoption.
- Extension/provider flexibility is useful.
- A Rust core can carry a serious local agent UX.

What not to copy into core:

- Desktop, gateway, provider zoo, and extensions should not live inside the harness kernel.
- Product surface can be broad; core must stay narrow.

Platonic takeaway: keep core separate from app shells.

## Rig

What to learn:

- Provider abstraction and composable LLM calls are library problems.
- Agentic workflows should be built from small, typed Rust pieces.

What not to copy:

- Provider abstraction alone is not an execution harness.

Platonic takeaway: use provider adapters as replaceable edges; do not let provider mechanics define the kernel.

## AutoAgents / ADK-Rust

What to learn:

- Modular agents, models, tools, memory, and deployment are attractive to users.
- Type-safe tool boundaries are a strength of Rust.

What not to copy:

- “Everything framework” gravity. The module list can become the product and bury the harness contract.

Platonic takeaway: expose extension points, but keep the kernel focused on run orchestration and side-effect accounting.

## Chidori

What to learn:

- Durable, replayable, resumable execution is a first-class differentiator.
- Treat every LLM/tool/HTTP operation as a host call crossing a recorded boundary.
- Replay should cost zero model calls when inputs are cached.

Platonic takeaway: event log first. Transcript, replay, metrics, and audit views are projections.

## Cersei / pi_agent_rust / pie

What to learn:

- Single-binary Rust agents can be fast and comprehensible.
- Local sessions, resumability, MCP/tools, and low memory footprint are valuable.
- Stateful loops/cron-like automation should route findings through a triage surface, not spam users.

What not to copy:

- Coding-agent assumptions should not leak into the generic core.
- “Skills” and cron loops can become uncontrolled sprawl.

Platonic takeaway: build the generic harness first; coding-agent features live above it.

## C.A.D.I.S.

What to learn:

- One daemon owning orchestration, tool policy, and approval state is a strong anti-sprawl boundary.
- Local-first and policy-gated tools are the right instinct.

Platonic takeaway: policy must be central, explicit, and inspectable.

## Hermes

What to learn:

- Persistent agents need memory, session search, scheduled jobs, delegation, and platform adapters somewhere in the product stack.
- Tool batching, interruptibility, context compression, and session persistence are real operational needs.

What not to copy:

- One god-class agent owning prompt assembly, provider routing, tool execution, fallback, compression, memory flushing, callbacks, sessions, gateway concerns, and cron concerns.
- Self-improving skills as an unbounded accumulation surface.
- Silent provider fallback in unattended jobs.

Platonic takeaway: Hermes proves the demand; Platonic should be the smaller harness Hermes needed underneath.

## IronClaw / OpenFang-style projects

What to learn:

- Rust rewrites of agent OS concepts are active and visible.
- Privacy/security/local-first language resonates.

What not to copy:

- “Agent OS” marketing sprawl.
- Grand claims before a small contract is proven.

Platonic takeaway: do not sell an operating system. Ship a harness contract.

## Core design rules

1. Kernel owns only run orchestration, context packs, tool-call policy, event logs, and replay hooks.
2. Gateways, schedulers, platform adapters, memory stores, UI, and model providers are outer crates/apps.
3. Every side effect emits a structured event.
4. Context assembly is lane-budgeted before model calls.
5. Tool schemas are typed and selected, not dumped wholesale.
6. Tool results are summaries plus structured data plus artifact refs.
7. Fallbacks require policy and are recorded.
8. Unsafe code is forbidden in the core.
9. Benchmarks and telemetry are part of the contract, not an afterthought.
10. If a feature increases sprawl, it belongs outside `platonic-core` until proven otherwise.
