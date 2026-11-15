# Kusi Kusi Constitution

These principles govern features planned with Spec Kit. Repository-level `AGENTS.md` instructions remain authoritative and apply to every change.

## Core Principles

### I. Generic Integration Core
Keep the product language-independent and usable with different existing business systems. Define stable contracts and ports in the domain/application layers; keep vendor- or channel-specific behavior behind infrastructure adapters. Treat academic systems and demos as fixtures, not as the product boundary.

### II. Layered Architecture
Keep domain code independent of Axum, SQLx, channels, and AI providers. Put use cases and ports in `crates/application`, adapters in `crates/infrastructure`, and HTTP concerns in `apps/server`. Keep the administration UI in the Next.js App Router application; do not reintroduce Vite or static frontend serving from Axum.

### III. Tenant Authorization and Privacy
Derive tenant identity only from verified credentials or registered channel context, never from caller-supplied tenant IDs. Enforce server-side authorization for every protected operation. Never log API keys, credentials, message bodies, or unnecessary personal data. Never expose the global admin key to frontend state or localStorage.

### IV. Safe and Explicit Automation
An LLM must not issue SQL or invoke a capability without application authorization and argument validation. Local workspace behavior must remain loopback-only and block paid or external calls. A simulation must never silently become a real send. Creative generation and publication require separate approvals bound to the exact version; preserve idempotency and durable recovery.

### V. Fallible, Evolvable Interfaces
Use explicit fallible error handling in production paths; do not use `unwrap` or `expect` there. Add a migration for schema changes. Update OpenAPI and relevant documentation for API or security changes. Preserve compatibility deliberately and document breaking changes.

### VI. Evidence-Based Delivery
Add tests for behavior changes and run the repository-mandated quality gates before describing a change as verified. For frontend or workflow changes, run the required admin format/build checks and relevant local API/browser acceptance checks. Keep unverified browser, container, provider, and deployment boundaries explicit.

## Technical Constraints

- Rust workspace: `crates/domain`, `crates/application`, `crates/infrastructure`, and `apps/server` retain their architectural boundaries.
- Frontend: Next.js App Router, explicit Turbopack builds, local shadcn/ui components, and fixed-upstream proxying. UI binds to `127.0.0.1:3000`; Rust binds to `127.0.0.1:8080`.
- Browser sessions require HttpOnly cookies, CSRF protection, and server-side RBAC. Preserve Origin checks, no-store behavior, and authorization at the server boundary.
- `.local-data` and `.local-backups` contain private state. Do not commit or print credentials or dumps.
- Do not connect local development to production or real databases unless the task explicitly authorizes it.

## Spec Workflow

For a feature, create the specification before the plan, then generate tasks and implement from those reviewed artifacts. Specs describe user-visible behavior, authorization, failure cases, acceptance evidence, and any migration or contract impact. Use clarification when unresolved decisions materially change scope or risk. Do not treat generated tasks as authorization to make external calls, publish, or deploy.

The repository's `AGENTS.md` and applicable nested instructions take precedence over generated templates and plans. If an artifact conflicts with them, revise the artifact before implementation.

## Governance

Changes to these principles must be deliberate and reflected in this file. Every feature plan must state how it complies with the constitution or explain a concrete, approved exception. Security, privacy, tenant isolation, and the local-only boundary cannot be waived by a feature spec.

**Version**: 1.0.0 | **Ratified**: 2026-11-05 | **Last Amended**: 2026-11-05
