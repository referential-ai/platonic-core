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

Platonic takeaway: Hermes proves the demand; Platonic should be the smaller harness Hermes needed underneath. The god-class decomposes onto the kernel: prompt assembly → `context`, provider routing/fallback → `policy` plus host, tool execution → host-fulfilled commands, compression → recorded compaction events, sessions/search/callbacks → event-log projections outside core.

## IronClaw / OpenFang-style projects

What to learn:

- Rust rewrites of agent OS concepts are active and visible.
- Privacy/security/local-first language resonates.

What not to copy:

- “Agent OS” marketing sprawl.
- Grand claims before a small contract is proven.

Platonic takeaway: do not sell an operating system. Ship a harness contract.

## Security study: Codex, IronClaw, OpenFang, Goose (2026-07-12)

Source study of four Rust agents (openai/codex, nearai/ironclaw,
RightNow-AI/openfang, block/goose) for security layers and patterns. Clones are
read-only under `~/projects/_reference/` (outside the workspace). Full triage
lives on plato-agent#132; the shell-sandbox mechanism comparison is on
plato-agent#81. Claims below were spot-verified at the cited files.

### Codex — the OS-sandbox reference

- Linux: **bubblewrap is the default FS sandbox; Landlock is demoted to legacy
  fallback** because Landlock `ABI::V5` can't express restricted-read. Crates:
  `seccompiler` (in-process filter), `landlock` (legacy only). Two-stage exec: a
  `codex-linux-sandbox` helper builds the bwrap mount view, re-execs itself to
  apply seccomp + `PR_SET_NO_NEW_PRIVS`, then `execvp`s the command (seccomp
  must follow bwrap, which needs setuid). bwrap flags:
  `--unshare-user --unshare-pid [--unshare-net] --die-with-parent`, `--ro-bind /
  /` baseline, `--bind` writable roots, re-`--ro-bind` of `.git`.
- seccomp deny-set worth copying verbatim: `ptrace`, `process_vm_readv/writev`,
  `io_uring_*` always denied; a `SeccompCondition` on `socket()` arg0 allows only
  `AF_UNIX` (restricted) or `AF_INET/6` (proxy mode).
- macOS: Seatbelt via `/usr/bin/sandbox-exec` (hardcoded path defends against a
  PATH-injected `sandbox-exec`), `(deny default)` SBPL modeled on Chromium,
  writable roots passed as `-D` params not string-interpolated. Cheapest OS
  sandbox to bolt onto an exec wrapper — no helper binary.
- Approval UX: **run sandboxed without prompting; on a sandbox denial, offer an
  unsandboxed retry** — but only when there are no denied-read paths (else the
  retry silently grants those reads), gated by a denial heuristic (keyword +
  Linux `128+SIGSYS` exit). Grants persist per session as command-prefix keys.
- Discovery hardening: accept a `bwrap`/`sandbox-exec` only if it resolves
  outside cwd; probe `--help` for required flags before trusting it.
- Env: `env_clear()` then a derived allowlist, with default case-insensitive
  excludes `*KEY*`/`*SECRET*`/`*TOKEN*`. Refines our env-scrub.

### IronClaw — untrusted-content and egress patterns

- **`ironclaw_prompt_envelope`** (zero-dep leaf, ~430 LOC): one primitive
  `wrap_untrusted(source, trust, body)`. Content matching `INSTRUCTION_LIKE_MARKERS`
  ("ignore previous instructions", `<|im_start|>`, "system prompt", …) is
  **rejected, not scrubbed**; control chars stripped; byte-capped; word-boundary
  matching avoids `act as`⊂`impact` false positives. Directly droppable onto our
  Discord/memory/tool-output ingress.
- Tool-output pipeline: truncate → leak-scan (blocks on secret) → policy
  Block/Sanitize → injection sanitize, with `<tool_output>` delimiters and
  close-tag neutralization (boundary-injection defense).
- SSRF/rebinding egress: default-deny allowlist, block private/loopback/
  link-local/CGNAT-`100.64/10`/IPv4-mapped-IPv6, then **resolve DNS and pin the
  resolved IPs into the transport** (rebinding defense) + reject caller Host
  header. Self-contained; closes our network-egress gap.
- Env-hygiene primitives for exec: reject raw `*API_KEY*/*TOKEN*/*SECRET*`
  values, block dangerous entrypoint env names (`LD_PRELOAD`, `LD_LIBRARY_PATH`,
  `PATH`, `BASH_ENV`, `IFS`). ~100 LOC, hardens our allowlist immediately.
- Their Docker process-sandbox (`--cap-drop ALL`, `no-new-privileges`,
  `--network none`, `--read-only`, `--pids-limit`, exec+argv never a shell line,
  blocked-mount-prefix list, no host-env inheritance, fail-closed-without-broker)
  is a ready template for the container tier of our fast-follow.
- Confirms our own posture at scale: effect classes (`EffectKind` ×13 with
  `is_write()`), `Decision = Allow(ordered obligations)/Deny/RequireApproval`,
  event-sourced redacted audit log, and a Discord gate that checks owner first
  then out-of-band pairing — remote senders can never self-grant.

### OpenFang — validation-by-counterexample

- **Live remote-grant hole**: channel slash-commands are handled by an early
  return *before* the authorization gate, and the approval resolver has no owner
  check — so any channel user can `/approve` a pending Critical approval
  (`crates/openfang-api/src/channel_bridge.rs`, `crates/openfang-channels/src/bridge.rs`).
  This is exactly the "remote channel grants an approval" hole our gateway design
  (D5) forbids by construction. Their `DmPolicy::AllowedOnly` is a no-op comment;
  default authorize is fail-open.
- Worth stealing anyway: dumb-pipe `ChannelAdapter` trait with one
  `dispatch_message` chokepoint (40 adapters own zero semantics — validates our
  gateway boundary); `serde(deny_unknown_fields)` on routing/binding config so a
  typo fails closed; taint guards (secret-labeled data blocked from egress,
  external-tainted data from shell); a shell-wrapper bypass detector that unwraps
  `bash -c "curl evil"` and re-validates the inner command.
- Their `SECURITY.md` overstates controls the code doesn't implement (claimed
  universal zeroization is false). Reminder to verify at source, never trust the
  security doc.

### Goose — the anti-pattern on gateways

- **Gateway inherits the global permission mode with no per-tool approval**: in
  `Auto` mode a paired remote user gets auto-approved tool execution / RCE; tool
  requests surface only as typing indicators (`crates/goose/src/gateway/handler.rs`).
  Our owner-allowlist + notify-only design is categorically safer.
- Security inspectors mostly off-by-default and fail-open; the egress inspector
  extracts exfil destinations but **always returns Allow** (log-only, not
  enforcement). Do not mistake detection for a control.
- Steal the shape, not the defaults: read-only-hint → auto-approve with a cached
  LLM "permission judge" for unannotated tools; an OSV `MAL-*` gate before
  launching npx/uvx extensions (make it fail-*closed*); a pluggable
  `ToolInspector` chain whose merge rule is "restrictive always wins, Allow never
  overrides".

### Cross-cutting takeaways

1. **No serious project builds its primary Linux sandbox on Landlock** — Codex
   demoted it; bubblewrap+seccomp or a container is the real story. Our fast-
   follow should target bwrap+seccomp first, Landlock only as a fallback.
2. **Default-on, fail-closed** is the dividing line. Goose and OpenFang park
   most controls behind opt-in flags and fail open; for effect-gated tools the
   right default is on and closed.
3. **Untrusted-content handling should reject, not sanitize** (IronClaw), and
   should mark provenance with delimiter/close-tag neutralization. This is a
   card-worthy addition independent of OS sandboxing.
4. **Remote channels must never self-grant** — proven twice by counterexample
   (OpenFang command bypass, Goose mode inheritance). Our D5 notify-only stands
   validated.
5. **env_clear + derived allowlist with `*KEY*/*SECRET*/*TOKEN*` excludes** and
   dangerous-env-name blocking are cheap refinements every serious agent applies.

## Core design rules

1. Kernel owns only run orchestration, context packs, tool-call policy, event logs, and replay hooks.
2. Gateways, schedulers, platform adapters, memory stores, UI, and model providers are outer crates/apps.
3. Every side effect emits a structured event.
4. Context assembly is lane-labeled and budget-validated before model calls.
5. Tool schemas are typed and selected, not dumped wholesale.
6. Tool results are summaries plus structured data plus artifact refs.
7. Fallbacks require policy and are recorded.
8. Unsafe code is forbidden in the core.
9. Benchmarks and telemetry are part of the contract, not an afterthought.
10. If a feature increases sprawl, it belongs outside `platonic-core` until proven otherwise.
