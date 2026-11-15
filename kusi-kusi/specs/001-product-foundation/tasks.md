# Tasks: Fundamentos del producto Kusi Kusi

> Alcance histórico: este documento registra la entrega del piloto académico local.
> Sus tareas o pruebas completadas no certifican la autonomía del producto. La corrección
> de alcance está en spec.md y su aceptación transversal en
> [006-adaptive-environment](../006-adaptive-environment/spec.md). No se borran resultados
> históricos ni se marcan como realizadas las capacidades adaptables pendientes.


**Input**: Design documents from `/specs/001-product-foundation/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/local-academic-demo.md`, `quickstart.md`

**Tests**: Required by repository instructions for behavior changes. Keep all tests synthetic, isolated, and loopback-only.

**Organization**: Tasks follow the three user stories and the shared application foundation. Existing code is reused where it already satisfies a requirement.

## Phase 1: Setup

**Purpose**: Preserve current product behavior and prepare a focused regression surface.

- [x] T001 [P] Add synthetic academic fixture scenarios and acceptance assertions to `scripts/verify_local.py` without using `.local-data` or external services.

---

## Phase 2: Foundational Work

**Purpose**: Add the application boundary, tenant-scoped storage, and local fixture adapter required by the family query.

- [x] T002 Add migration `migrations/0011_local_academic_demo.sql` for organization-scoped links from enabled local viewer accounts to fixed synthetic student references, including same-tenant foreign keys and duplicate prevention.
- [x] T003 [P] Define the guardian-link repository port, fixture reader port, query result, and fallible read-only use case in `crates/application/src/academic.rs`; require both verified organization and linked user context.
- [x] T004 Implement the PostgreSQL guardian-link repository and fixed fictional academic fixture adapter in `crates/infrastructure/src/local/academic.rs`; return no customer-provided or real student data.

**Checkpoint**: The use case can return fixture results only after the caller is linked to those fixture records.

---

## Phase 3: User Story 1 — Conectar un sistema existente (Priority: P1)

**Goal**: Preserve proof that capabilities remain independent of a particular customer system while keeping the academic data set as an isolated fixture.

**Independent Test**: The existing capability use case accepts different connector adapters under the same contract, and no academic concepts are added to the generic domain or OpenAPI capability schema.

- [x] T005 [P] [US1] Add a connector substitution regression test in `crates/application/src/integrations.rs` showing the same capability use case can use two independent in-memory connector adapters.
- [x] T006 [US1] Update `docs/INTEGRATIONS.md` to state that the academic demo is a local fixture and does not extend or specialize the generic capability contract.

**Checkpoint**: Generic integration tests pass and existing capability registration/execution behavior remains unchanged.

---

## Phase 4: User Story 2 — Resolver una consulta familiar autorizada (Priority: P1) 🎯 MVP

**Goal**: A linked family viewer can request a read-only academic summary through a signed test channel event and see a clearly simulated response in the local workspace.

**Independent Test**: A linked viewer receives only linked synthetic records; an unlinked, disabled, or cross-tenant viewer gets no academic values; replaying the same signed event creates no duplicate result and no external send.

- [x] T007 [P] [US2] Add a deterministic `AcademicSummary` intent and supported phrase tests in `crates/domain/src/local.rs`; unknown or ambiguous requests must not be interpreted as academic queries.
- [x] T008 [US2] Extend owner-only code issuance in `apps/server/src/local.rs` so an owner may issue an existing channel-bound one-time code to an enabled same-tenant viewer, while preserving current-user issuance and never returning a code for a cross-tenant or privileged account.
- [x] T009 [US2] Add owner-only list/create/remove handlers for fictional students and guardian links in `apps/server/src/local.rs`, derive organization from the verified local session, validate CSRF/Origin through the existing middleware, audit link changes without message bodies/secrets, and return safe fallible errors.
- [x] T010 [US2] Dispatch supported academic intents through the application use case in `crates/infrastructure/src/local/worker.rs` after existing channel identity binding; keep the response simulated, read-only, and idempotent, and provide no-data/human-handoff responses for denied requests.
- [x] T011 [US2] Document the demo-student, guardian-link, and optional target-user link-code schemas and owner/CSRF/error behavior in `contracts/openapi/openapi.yaml`.
- [x] T012 [US2] Extend `scripts/verify_local.py` to cover owner-only link management, viewer linking, one-time code behavior, fixture query authorization, tenant isolation, unsupported requests, disabled users, event replay, and the absence of outbound delivery.

**Checkpoint**: All query results are synthetic, authorized per linked student, marked simulated, and never sent externally.

---

## Phase 5: User Story 3 — Administrar vínculos y revisar resultados (Priority: P2)

**Goal**: A school operator can create fictional guardian links, issue a channel-link code, and review the local simulated conversation without seeing provider secrets.

**Independent Test**: The owner-only page lists the fixed fictional roster, creates/removes a viewer link, and displays a one-time code once; non-owner accounts cannot access or mutate the page/API.

- [x] T013 [P] [US3] Build `web/admin/src/features/AcademicDemo.tsx` with the fictional roster, same-tenant viewer selection, link management, one-time channel-code issuance, loading/empty/error states, and no collection of real student details.
- [x] T014 [US3] Add the owner-only navigation entry and route wiring in `web/admin/src/navigation.ts` and `web/admin/src/components/workspace.tsx`, following `web/admin/AGENTS.md` and existing role restrictions.
- [x] T015 [US3] Show the local simulated badge for academic assistant replies in `web/admin/src/features/Chat.tsx` using response metadata; do not label real provider states as simulated.
- [x] T016 [US3] Add browser acceptance coverage in `scripts/verify_browser.py` for owner-only visibility, fictional links, simulated response labeling, and no external send.

**Checkpoint**: The workflow is usable from the local owner panel and all other roles remain denied.

---

## Phase 6: Polish and Cross-Cutting Validation

**Purpose**: Make the pilot behavior understandable and verify the whole local boundary.

- [x] T017 [P] Update `docs/LOCAL_WORKSPACE.md` with the fictional family-query flow, out-of-band demo-link assumption, exact local-only behavior, and explicit no-real-student/no-send boundary.
- [x] T018 [P] Update `README.md` to link the local academic demo guide without presenting the pilot as a validated customer or production integration.
- [x] T019 Run the repository-required Rust, admin formatting/build, local API/browser, and Next.js runtime checks documented in `specs/001-product-foundation/quickstart.md`; record exact results and any unavailable boundary in `docs/VALIDATION.md`.
- [x] T020 Review `specs/001-product-foundation/spec.md`, `plan.md`, and `tasks.md` against `.specify/memory/constitution.md`; resolve any mismatch before closing the feature.

## Dependencies & Execution Order

### Phase Dependencies

- Setup T001 can start immediately.
- Foundational tasks T002–T004 block the academic query path; T003 may proceed independently of the schema.
- US1 regression/documentation tasks do not depend on academic database work.
- US2 handlers depend on T002; the worker path depends on T003/T004/T007.
- US3 UI depends on the US2 APIs and on the existing owner/session boundary.
- Polish follows completed implementation and acceptance checks.

### User Story Dependencies

- **US1 (P1)**: Preserves the generic connector boundary and is independently verifiable against the existing application service.
- **US2 (P1)**: Depends on foundational application ports and tenant-scoped link storage.
- **US3 (P2)**: Depends on US2 endpoints; it is not needed to prove the server-side query authorization.

### Parallel Opportunities

- T001, T003, T005, and T007 touch separate files and can begin independently.
- After T002/T003, the fixture adapter (T004), UI shell (T013), and generic regression (T005) can proceed in parallel if their contracts are stable.
- API schema/docs and local acceptance harness work can proceed alongside server implementation after the request shapes are agreed.

## Implementation Strategy

### MVP First

Complete the application foundation and US2 first. Validate the query, denial, tenant isolation, idempotency, and local-only behavior with synthetic signed events. Then add the management UI in US3.

### Incremental Delivery

1. Preserve generic capability contracts (US1).
2. Deliver the backend local family-query path (US2).
3. Add the owner experience (US3).
4. Run the complete repository gates and document any unverified browser/container boundary.

No task authorizes production credentials, public routing, real student data, actual WhatsApp sends, paid services, or deployment.
