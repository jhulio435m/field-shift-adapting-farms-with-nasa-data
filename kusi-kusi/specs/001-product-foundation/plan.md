# Implementation Plan: Fundamentos del producto Kusi Kusi

> Alcance histórico: este documento registra la entrega del piloto académico local.
> Sus tareas o pruebas completadas no certifican la autonomía del producto. La corrección
> de alcance está en spec.md y su aceptación transversal en
> [006-adaptive-environment](../006-adaptive-environment/spec.md). No se borran resultados
> históricos ni se marcan como realizadas las capacidades adaptables pendientes.


**Branch**: `001-product-foundation` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Product foundation and first local validation pilot: read-only academic queries for families through replayed WhatsApp events and fictional academic data.

## Summary

The repository already provides a generic capability contract, tenant authorization for service credentials, signed WhatsApp webhooks, a deduplicated inbox, local user/channel linking, RBAC, and a local-only admin workspace. The missing pilot path is a tenant- and user-scoped read of academic fixture records from the local conversation worker.

Add an isolated fictional academic fixture adapter, an application use case that requires a guardian-to-student authorization link, owner-only local APIs and UI for managing demo links, and deterministic message handling. Reuse existing channel linking and inbox recovery. Responses remain marked simulated and local; they are not sent to WhatsApp. Keep all current creative workflows and generic connector APIs intact.

## Technical Context

**Language/Version**: Rust 2024 workspace; TypeScript with Next.js App Router

**Primary Dependencies**: Existing Axum 0.8, Tokio, SQLx/PostgreSQL, serde/serde_json, Next.js, React, local shadcn/ui; no new dependency planned

**Storage**: PostgreSQL. Add one versioned migration for organization-scoped guardian-to-student demo links. Fictional academic records remain immutable source fixtures in the infrastructure adapter and are not customer data.

**Testing**: Existing Rust unit/integration suite and `scripts/verify_local.py`; add behavior coverage for tenant boundaries, viewer/owner authorization, identity linking, fixture reads, denial cases, idempotent replay, and no external send. Run the repository-required Rust and admin checks plus relevant local API/browser checks.

**Target Platform**: Loopback-only local workspace at `127.0.0.1:3000` (UI) and `127.0.0.1:8080` (API)

**Project Type**: Rust modular monolith with Next.js administration UI

**Performance Goals**: Local fixture response completes within the existing inbox-worker lease; no remote connector is called. No production latency claim is introduced.

**Constraints**: No real students, school systems, WhatsApp credentials, paid providers, outbound messages, production database, or public bind. Resolve tenant/user identity only from trusted channel configuration and existing one-time link codes. A guardian account receives only explicitly linked demo students. Existing root and nested `AGENTS.md` instructions govern.

**Scale/Scope**: One local organization, a small fixed fictional dataset, several demo guardian accounts, and replayed signed webhook fixtures. The product architecture and capability contracts remain system-independent.

## Constitution Check

| Principle | Gate | Plan evidence |
|---|---|---|
| Generic Integration Core | PASS | Academic fixtures stay in `crates/infrastructure`; do not add school concepts to generic domain/capability contracts. |
| Layered Architecture | PASS | Use case and ports in `crates/application`; persistence and fixture adapter in infrastructure; Axum routes in server; UI in Next.js. |
| Tenant Authorization and Privacy | PASS | Tenant derives from local session or registered channel; user derives from verified binding; queries filter by organization and user; use synthetic records only. |
| Safe and Explicit Automation | PASS | Deterministic read-only intent; no LLM, SQL generation, writes, external provider call, or actual channel send. |
| Fallible, Evolvable Interfaces | PASS | Explicit error mapping, additive API fields, migration, OpenAPI and docs updates, tests for behavior changes. |
| Evidence-Based Delivery | PASS | Required Rust, admin format/build, local API and browser acceptance checks are included in final phase. |

**Post-design gate**: PASS, provided the fixture adapter cannot be selected as a production academic connector and the local response stays labeled simulated.

## Decisions and Research

See [research.md](research.md) for repository-grounded decisions and rejected alternatives. No external provider behavior is needed to implement the local validation slice.

## Data Model

See [data-model.md](data-model.md). A migration adds organization-scoped links from local viewer accounts to fixed fictional student references. Existing `local_link_codes` and `local_channel_bindings` establish the channel sender identity; student authorization is checked separately for each organization/user.

## Contracts

See [contracts/local-academic-demo.md](contracts/local-academic-demo.md) for the owner-only fixture and guardian-link endpoints and simulated query behavior. Preserve the existing `/v1/local/link-codes` shape; an additive optional target user field can be used only by an owner and only for a same-tenant viewer account.

## Project Structure

```text
crates/application/src/academic.rs                  # guardian-scoped read use case and ports
crates/infrastructure/src/local/academic.rs          # fictional fixture reader and PostgreSQL link repository
crates/infrastructure/src/local/worker.rs            # deterministic read-only intent dispatch
crates/domain/src/local.rs                           # new intent only; no academic records in generic domain
apps/server/src/local.rs                             # owner-only fixture/link endpoints and target-user code issuance
web/admin/src/features/AcademicDemo.tsx              # local owner management for fictional links
web/admin/src/components/workspace.tsx               # feature routing and owner visibility
migrations/0011_local_academic_demo.sql               # tenant-bound guardian/student link records
contracts/openapi/openapi.yaml                        # local API contract
scripts/verify_local.py                               # isolated local API/workflow regression coverage
docs/LOCAL_WORKSPACE.md                               # setup, link, replay, and limitations
```

Keep the capability execution path, external HTTP connector configuration, existing creative flows, and frontend proxy/security boundary intact. Do not introduce production academic integration code in this feature.

## Delivery Phases

1. **Foundation**: implement the guardian-link application port/use case, fixed fixture adapter, tenant-scoped migration/repository, and focused Rust tests.
2. **US1 — Connectable system contract**: retain the generic capability API and add a contract regression proving the academic sample is an isolated fixture behind an adapter; do not make school data part of the product domain.
3. **US2 — Authorized family query**: expose safe demo student/link APIs, issue a short-lived code for an authorized same-tenant viewer, process a deterministic read-only query, deny missing links, and preserve simulated local outbox behavior.
4. **US3 — Admin and audit**: add an owner-only Next.js page for synthetic student assignments and visible verification state; do not accept actual student identity documents or real grades.
5. **Polish**: update OpenAPI and local-workspace docs; run required Rust, formatting, frontend build, local API, browser, and container checks where applicable.

## Risks and Mitigations

- **Demo phone number mistaken for identity**: keep the existing one-time channel code and verified account binding; never infer access from a sender number. Require an operator to confirm the relationship out of band before assignment.
- **Fixture leaks into real deployments**: name every endpoint and result as demo/simulated, include no provider secret or external URL, and add a guard so fixture records are not exposed outside local mode.
- **Cross-tenant link bug**: include organization in every lookup and foreign key; test a user/link from another organization and require a denial.
- **Real school data accidentally entered**: the UI uses fixed fictional fixture records and does not accept free-form grade/name data. Document that production identity and data handling require a separate approved feature.
- **Unexpected outbound message**: do not change the local worker's queued/simulated outbox policy; verify no external send is attempted.
- **Pre-existing checkout differs from docs**: inspect actual files before editing; preserve existing unrelated local data and retain loopback-only behavior.

## Validation Quickstart

See [quickstart.md](quickstart.md). All validation uses synthetic records, a local PostgreSQL database, and signed test webhook payloads; it must not contact WhatsApp or a school system.

## Complexity Tracking

No constitution exceptions or additional architectural layers are proposed.
