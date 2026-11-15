# Spec-driven development with GitHub Spec Kit

Kusi Kusi includes the official GitHub Spec Kit scaffolding with its Codex integration. It gives substantial changes a reviewable sequence of specification, plan, tasks, and implementation. It does not replace repository instructions or authorize external actions.

## Start a feature

Run these skills in Codex from the project root:

1. `$speckit-specify` — describe the user outcome and acceptance criteria.
2. `$speckit-clarify` — resolve important ambiguity when needed.
3. `$speckit-plan` — map the change to the existing architecture and delivery constraints.
4. `$speckit-tasks` — create an ordered implementation checklist.
5. `$speckit-analyze` — check that the spec, plan, and tasks agree.
6. `$speckit-implement` — carry out the approved scope.

`$speckit-checklist` can add a requirements quality checklist. `$speckit-converge` assesses remaining work in the codebase; use it when that assessment is wanted, not as a substitute for scoping a feature.

The official project constitution is in `.specify/memory/constitution.md`; templates and shared scripts are in `.specify/`. Codex skills are in `.agents/skills/speckit-*`.

## Kusi Kusi-specific guardrails

- Read root and nested `AGENTS.md` files and inspect the current checkout before planning or editing.
- Keep the generic integration boundary: contracts and adapters should work across systems; do not make one academic fixture the product architecture.
- State tenant identity, authorization, validation, privacy, and failure behavior in specs whenever a feature touches data, capabilities, AI, or channels.
- Keep local work loopback-only and simulations non-sending. Specs and plans do not authorize paid calls, real messaging, social publication, production access, or deployment.
- For API or security changes, include migration/OpenAPI/docs work. For behavior changes, include tests and the required verification commands from `AGENTS.md`.
- Preserve version-bound approval, idempotency, and durable recovery for creative generation and publication workflows.
- Keep `.local-data` and `.local-backups` private; never include secrets or dumps in generated artifacts.

## Install and refresh

The Spec Kit CLI is not required for normal use once the scaffolding is present. To refresh official managed files, install/use `specify-cli` and check the CLI's current official documentation before running updates. Review the complete diff after any refresh; do not hand-edit generated Codex skill files.

For the installed integration, inspect it with `specify integration status`. The current integration is Codex and skills are invoked with the `$speckit-*` syntax.
