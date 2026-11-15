# Implementation Plan: Control de sesiones locales

**Branch**: `003-session-controls` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

**Input**: Inspect and revoke active local browser sessions without exposing authenticators or transport metadata.

## Summary

Add a provider-independent session inventory for local accounts. Users see their own active sessions; owners can see active sessions in their organization. A user may revoke one of their other sessions, and an owner may revoke another account's session in the same organization. The current session cannot be revoked from this route; existing logout remains the explicit current-session exit. Session revocation, the linked API-key revocation, and audit entry commit atomically.

This does not select an OIDC provider or change service API-key rotation. The session inventory records no IP address, User-Agent, or device fingerprint.

## Technical Context

**Language/Version**: Rust 2024; TypeScript with Next.js App Router

**Primary Dependencies**: Existing Axum, SQLx/PostgreSQL, serde, Next.js, React, shadcn/ui; no new dependencies

**Storage**: Migration adds a random management UUID to existing `local_sessions` rows; token hashes and API keys remain private

**Testing**: Application port/use-case tests; isolated API checks for ownership, tenant separation, current-session protection and immediate revocation; browser checks on desktop/mobile; required Rust, Next and proxy checks

**Target Platform**: Local loopback workspace at 127.0.0.1:3000 and 127.0.0.1:8080

**Project Type**: Rust modular monolith with Next.js administration panel

**Constraints**: Current identity comes only from the verified session. Session IDs are non-authenticating management handles. The current session cannot be revoked from the inventory. Owner scope is tenant-bound. Each revocation and its audit event are transactional. No transport or secret metadata enters the response or audit.

## Constitution Check

| Principle | Gate | Evidence |
|---|---|---|
| Generic Integration Core | PASS | Session management is independent of provider and business domain. |
| Layered Architecture | PASS | Application owns session use cases/ports; infrastructure owns SQL; Axum owns routes; Next owns UI. |
| Tenant Authorization and Privacy | PASS | Organization and principal derive from verified session; owner scope is checked server-side; no token/IP/User-Agent is exposed. |
| Safe and Explicit Automation | PASS | Local only, loopback, no providers, no sends or generated SQL. |
| Fallible, Evolvable Interfaces | PASS | Additive migration, explicit errors, OpenAPI and docs updates. |
| Evidence-Based Delivery | PASS | Unit, isolated API, browser, Rust quality and Next checks are required. |

## Project Structure

```text
crates/application/src/session_management.rs           # session inventory/revocation port and use case
crates/application/src/lib.rs                          # export use case
crates/infrastructure/src/local/auth.rs                # SQL session implementation and transactional revocation
crates/infrastructure/src/local/mod.rs                 # module wiring
apps/server/src/local.rs                               # session API routes and current-session derivation
crates/domain/src/local.rs                              # route RBAC for self-session revocation
web/admin/src/features/Sessions.tsx                    # owner and personal session inventory
web/admin/src/navigation.ts                            # route registration
web/admin/src/types.ts                                 # sessions view
web/admin/src/components/workspace.tsx                 # page wiring
migrations/0014_local_session_management_ids.sql       # random management IDs for existing sessions
contracts/openapi/openapi.yaml                         # inventory and revoke contracts
scripts/verify_local.py                                # session authorization and revocation matrix
scripts/verify_browser.py                              # owner/user session management journey
docs/LOCAL_WORKSPACE.md                                 # local behavior and privacy notes
docs/SECURITY.md                                        # session controls and limits
docs/ROADMAP.md                                         # remaining provider decisions
```

## Data Model

See [data-model.md](data-model.md). Session identifiers are random UUID management references; authentication continues to use the existing secret cookie/API key whose hash is stored. Active listing joins the account and credential, filters expiry/revocation/disabled state, scopes tenant, and marks the current session by the hash derived from the verified cookie.

## Contract and Decisions

See [contracts/local-sessions.md](contracts/local-sessions.md) and [research.md](research.md). The API lists sessions visible to the current principal and revokes by UUID. Current-session revocation returns a conflict. Revocation of session row, linked credential, and audit row is one transaction.

## Delivery Phases

1. Add application session-management ports/use case and failure/authorization unit tests.
2. Add an additive migration and tenant-scoped SQL session repository.
3. Add authenticated list/revoke endpoints and OpenAPI contract.
4. Add role-aware Sessions UI and browser acceptance.
5. Update security docs and run required validation.

## Risks and Mitigations

- **Management UUID is mistaken for authentication**: it is never accepted as a credential; endpoints still require the existing verified session.
- **Cross-tenant revocation**: repository query requires session organization; owner role is checked from verified session.
- **User locks themselves out**: reject current-session target; preserve existing logout action for intentional exit.
- **Partial revocation**: update session, API-key credential and audit in one transaction.
- **Privacy leakage**: expose no credential hash, raw token, IP, User-Agent or device fingerprint.
- **Stale/disabled sessions**: active listing filters all revocation, expiration and account status conditions.

## Validation Quickstart

See [quickstart.md](quickstart.md). Use isolated local schema, synthetic accounts and loopback services only.

## Complexity Tracking

No constitution exceptions or new external dependencies.
