# Naming

## Surface naming

- **Referential** — umbrella/company/product suite.
- **Platonic** — harness core and architecture: clean forms, bounded abstractions, disciplined execution.
- **Plato Agent** — planned usable agent product built on Platonic.

Preferred GitHub layout:

```text
referential-ai/platonic-core
referential-ai/plato-agent
```

## Deeper internal vocabulary

These names are available for architecture docs and module concepts, not public marketing unless they prove useful.

- **Henad** — one isolated agent unit.
- **Telesterion** — execution chamber/runtime context.
- **Anaktoron** — secure core/state vault.
- **Epopteia** — trace/replay/audit view; the act of seeing what happened.
- **Legomena** — model messages/prompts; things said.
- **Dromena** — tool actions/side effects; things done.
- **Deiknumena** — artifacts/observations/evidence; things shown.
- **Aporrheta** — secrets and policy-denied material; things not to disclose.
- **Prohodos** — forward execution/procession.
- **Epistrophe** — replay/return/correction loop.

Brand lives on packages (`platonic-core`, `plato-agent`); all code stays generic (`AgentId`, not `HenadId`; `Error`, not `PlatonicError`). Deep terms are documentation color only — never type names, fields, or serialized values.
