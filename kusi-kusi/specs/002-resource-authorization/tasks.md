# Tasks: Autorización de capacidades por recurso

**Input**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/local-resource-access.md`

## Phase 1: Foundations

- [x] T001 Add application DTOs, repository/reader/audit ports, and authorization use cases with deny-before-reader unit tests in `crates/application/src/resource_access.rs`; export module from `crates/application/src/lib.rs`.
- [x] T002 [P] Add tenant-scoped grants and payload-free audit migrations in `migrations/0012_local_resource_access.sql` and `migrations/0013_resource_audit_tenant_fk.sql`.
- [x] T003 [P] Add fixture resources and SQLx implementations in `crates/infrastructure/src/local/resource_access.rs`; export from `crates/infrastructure/src/local/mod.rs`.

## Phase 2: User Story 1 — Grant and revoke exact resource access (P1)

**Independent test**: owner grants one synthetic resource to an enabled same-tenant viewer; other identities/resources fail; revocation denies next read.

- [x] T004 [US1] Implement owner-only list/create/revoke handlers using `Extension<LocalUser>` and session organization in `apps/server/src/local.rs`.
- [x] T005 [US1] Add API integration coverage for grant scope, idempotency, role checks, disabled users, and revocation in `scripts/verify_local.py`.
- [x] T006 [US1] Publish the resource-access schemas and route security contract in `contracts/openapi/openapi.yaml`.

## Phase 3: User Story 2 — Execute an authorized simulated read (P1)

**Independent test**: session-derived viewer can read only an explicitly granted fixture; unauthorized and forged identity requests do not reach the fixture reader.

- [x] T007 [US2] Implement session-bound execution handler, strict request validation, fixed capability/output validation, and payload-free decisions in `apps/server/src/local.rs` and application tests.
- [x] T008 [US2] Add owner/viewer resource-access screens with simulated-result labeling in `web/admin/src/features/ResourceAccess.tsx`.
- [x] T009 [US2] Wire role-aware navigation and page rendering in `web/admin/src/navigation.ts` and `web/admin/src/components/workspace.tsx`.
- [x] T010 [US2] Extend desktop/mobile browser acceptance for granting, reading, denying, and simulation labels in `scripts/verify_browser.py`.

## Phase 4: User Story 3 — Review safe authorization decisions (P2)

**Independent test**: owner sees decision metadata while response arguments and values never appear in audit output.

- [x] T011 [US3] Add owner-only grant and access-decision activity views without request/response payloads in the API and `ResourceAccess.tsx`.
- [x] T012 [US3] Verify audit privacy and cross-tenant isolation with synthetic API cases in `scripts/verify_local.py`.

## Phase 5: Polish

- [x] T013 [P] Document setup, synthetic scope, and identity-provider exclusions in `docs/LOCAL_WORKSPACE.md` and `docs/ROADMAP.md`.
- [x] T014 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, admin format/build, and local/Next/browser checks; record evidence in `docs/VALIDATION.md`.
- [x] T015 Complete success-criteria evidence and checklist in `specs/002-resource-authorization/checklists/requirements.md`.

## Dependencies

- T001–T003 are foundational and precede handlers.
- US1 (T004–T006) and US2 (T007–T010) share foundational storage; execution depends on grants from US1.
- US3 (T011–T012) depends on the decision audit event created by US2.
- Polish starts after all three stories.
