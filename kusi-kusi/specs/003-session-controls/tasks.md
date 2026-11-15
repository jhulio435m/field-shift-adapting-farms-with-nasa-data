# Tasks: Control de sesiones locales

**Input**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/local-sessions.md`

## Phase 1: Foundations

- [x] T001 Add `SessionInventoryItem`, session repository port and list/revoke use cases with authorization/current-session unit tests in `crates/application/src/session_management.rs`; export from `crates/application/src/lib.rs`.
- [x] T002 [P] Add random management UUID to existing sessions in `migrations/0014_local_session_management_ids.sql`.
- [x] T003 Implement active session listing and atomic tenant-scoped revocation with audit in `crates/infrastructure/src/local/auth.rs` after T001 defines the repository port.

## Phase 2: User Story 1 — Review active sessions (P1)

**Independent test**: a user sees only their active sessions, an owner sees only the current organization's active sessions, and the current session is identified without secrets or transport metadata.

- [x] T004 [US1] Add authenticated session inventory endpoint and derive current session from verified cookie in `apps/server/src/local.rs`.
- [x] T005 [US1] Add inventory schema, privacy exclusions, and authentication contract to `contracts/openapi/openapi.yaml`.
- [x] T006 [US1] Add isolated API checks for owner/user scope, current-session marking, expiry, disabled users, and cross-tenant isolation in `scripts/verify_local.py`.
- [x] T007 [US1] Add the role-aware Sessions screen, navigation, and page wiring in `web/admin/src/features/Sessions.tsx`, `web/admin/src/navigation.ts`, `web/admin/src/types.ts`, and `web/admin/src/components/workspace.tsx`.

## Phase 3: User Story 2 — Revoke an older session (P1)

**Independent test**: a user revokes another own session or an owner revokes a same-tenant session; the next request using that credential fails; current and cross-tenant sessions remain untouched.

- [x] T008 [US2] Add session-ID DELETE endpoint with CSRF, RBAC, current-session conflict and atomic storage error mapping in `apps/server/src/local.rs` and `crates/domain/src/local.rs`.
- [x] T009 [US2] Add API coverage for self/owner revocation, current-session protection, cross-tenant denial, repeated revoke, and immediate credential rejection in `scripts/verify_local.py`.
- [x] T010 [US2] Add revoke controls and confirmation/result states in `web/admin/src/features/Sessions.tsx`; extend desktop/mobile browser acceptance in `scripts/verify_browser.py`.

## Phase 4: User Story 3 — Keep account security changes consistent (P2)

**Independent test**: disabling a user or resetting their password persistently revokes every local session and removes each from active inventory.

- [x] T011 [US3] Mark session rows revoked in the existing transactions for disable and password reset in `crates/infrastructure/src/local/auth.rs`.
- [x] T012 [US3] Add API regression cases for password-reset and disabled-account inventory/authentication behavior in `scripts/verify_local.py`.

## Phase 5: Polish

- [x] T013 [P] Document session inventory, current-session protection, and data exclusions in `docs/LOCAL_WORKSPACE.md`, `docs/SECURITY.md`, and `docs/ROADMAP.md`.
- [x] T014 Run Rust quality/tests, admin format/typecheck/build, isolated local API, Next proxy, and browser acceptance; record evidence in `docs/VALIDATION.md`.
- [x] T015 Complete success-criteria evidence in `specs/003-session-controls/checklists/requirements.md` and set the spec status based on verified results.

## Dependencies

- T001–T003 establish the use case, schema and repository before endpoints.
- US1 (T004–T007) establishes safe inventory; US2 (T008–T010) adds revocation controls.
- US3 (T011–T012) depends on inventory and lifecycle semantics but can be validated after the repository changes.
- Polish begins after all three user stories.
