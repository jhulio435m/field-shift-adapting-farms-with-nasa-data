# Implementation Plan: Autorización de capacidades por recurso

**Branch**: `002-resource-authorization` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Add local, generic end-user and resource authorization before a capability can invoke an adapter.

## Summary

The product currently authorizes capability execution with a tenant service key and permission grant. The local conversation worker separately authorizes two fictional academic records. This feature adds a generic local resource-access workflow: an owner grants a viewer read access to one of two neutral fixture resources; the viewer invokes a read-only capability using the current session; the server derives tenant and principal, checks the resource grant, validates data, calls only an in-process fixture adapter, and records a payload-free audit event.

This validates user-scoped authorization without claiming that local users are mapped to an external identity provider or that a real customer system has been connected. Existing tenant service-key execution remains compatible.

## Technical Context

**Language/Version**: Rust 2024; TypeScript with Next.js App Router

**Primary Dependencies**: Existing Axum, SQLx/PostgreSQL, serde, Next.js, React, shadcn/ui; no new package dependency

**Storage**: PostgreSQL migration for tenant-bound grants to fixed fictional resource references; fixture values remain immutable in infrastructure

**Testing**: Unit tests for authorization/use-case boundaries, isolated local API workflow tests, browser acceptance, Rust quality gates, admin format/typecheck/Turbopack build, and Next proxy checks

**Target Platform**: Loopback-only local workspace at 127.0.0.1:3000 and 127.0.0.1:8080

**Project Type**: Rust modular monolith with Next.js administration panel

**Performance Goals**: In-process fixture read completes within one request; no network adapter or provider is called

**Constraints**: Local identity comes only from the verified session. The API accepts no tenant ID, user ID, external principal, connector URL, or credential from a viewer. Only fixed fictional records and a fixed read capability are available. Owner grants require an explicit manual-confirmation field. Every denial happens before adapter invocation. The UI remains within the existing proxy, CSRF, Origin, no-store, and RBAC boundary.

**Scale/Scope**: One local organization, the existing local roles, two synthetic resources, one read-only capability, and no real external identity or connected system.

## Constitution Check

| Principle | Gate | Evidence |
|---|---|---|
| Generic Integration Core | PASS | Resource references, capability contract, and application use case stay vendor- and domain-neutral; the two records are infrastructure fixtures only. |
| Layered Architecture | PASS | Use case/ports in application; PostgreSQL and fixture adapter in infrastructure; Axum routes in server; Next UI in admin. |
| Tenant Authorization and Privacy | PASS | Tenant and user derive from the verified cookie session; organization is part of grant lookup and foreign keys; audit excludes payloads. |
| Safe and Explicit Automation | PASS | No LLM, SQL generation, remote connector, paid call, or actual message send participates in the local fixture. |
| Fallible, Evolvable Interfaces | PASS | Additive local routes, migration, OpenAPI, safe errors, and regression tests. |
| Evidence-Based Delivery | PASS | Isolated API and browser scenarios plus required Rust and frontend quality gates are in the plan. |

## Project Structure

```text
crates/application/src/resource_access.rs             # generic user/resource-scoped read use case and ports
crates/infrastructure/src/local/resource_access.rs    # grant repository and fixed synthetic adapter
crates/infrastructure/src/local/worker.rs              # unchanged; this feature is explicit UI/API invocation
apps/server/src/local.rs                               # owner grant and session-bound resource execution routes
web/admin/src/features/ResourceAccess.tsx              # owner grant and viewer read-only workflow
web/admin/src/navigation.ts                            # role-aware route
web/admin/src/components/workspace.tsx                 # page wiring
migrations/0012_local_resource_access.sql              # organization-scoped fixture grants and decision audit
migrations/0013_resource_audit_tenant_fk.sql             # composite tenant/actor integrity for audit rows
contracts/openapi/openapi.yaml                         # request/response/error contract
scripts/verify_local.py                                # isolated API/resource authorization coverage
scripts/verify_browser.py                              # owner and viewer browser journey
docs/LOCAL_WORKSPACE.md                                # local setup and scope limits
docs/ROADMAP.md                                         # mark local authorization slice complete
```

**Structure Decision**: Extend the existing Rust application/infrastructure/server layers and Next.js admin surface. Do not change the frontend/runtime proxy boundary or the generic `/v1/capabilities/execute` service-key contract.

## Decisions and Research

See [research.md](research.md). The local slice uses the verified local user ID as its principal and explicit resource grants. It does not pretend this ID is an identity in a customer system; external identity mapping and real resource enforcement remain a later provider-specific phase.

## Data Model

See [data-model.md](data-model.md). A grant is keyed by organization, viewer, capability, and resource. Composite foreign keys prevent cross-tenant ownership. The owner identity that created the grant is recorded. Grant listing and execution re-check that the target user remains enabled and in viewer role.

## Contracts

See [contracts/local-resource-access.md](contracts/local-resource-access.md) for routes and authorization behavior.

## Delivery Phases

1. Add application ports/use case and unit tests for deny-before-adapter behavior.
2. Add tenant-bound persistence and a fixed in-process fixture adapter.
3. Expose owner-only grant management and session-derived viewer execution; update OpenAPI.
4. Add the role-aware owner/viewer panel and browser acceptance.
5. Update local docs and run the full quality/acceptance suite.

## Risks and Mitigations

- **Caller substitutes another person or tenant**: reject unknown identity fields and derive both from session context.
- **A grant leaks to another resource or tenant**: composite grant key, same-tenant foreign keys, explicit resource check before adapter call, isolated negative tests.
- **Fixture becomes mistaken for a real connection**: label every value simulated and keep fixture adapter available only on the named local routes.
- **Audit stores sensitive payload**: record actor, capability, opaque resource digest, decision, and time only.
- **Current service-key API regresses**: retain its route and add regression coverage for unchanged behavior.

## Validation Quickstart

See [quickstart.md](quickstart.md). Use only the isolated local schema, fictional accounts/resources, signed or authenticated local requests, and loopback services.

## Complexity Tracking

No constitution exceptions or new architectural layer are proposed.
