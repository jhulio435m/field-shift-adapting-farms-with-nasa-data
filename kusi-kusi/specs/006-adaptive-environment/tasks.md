# Tasks: Autonomía y adaptación al entorno

**Input**: spec.md, plan.md, research.md, data-model.md, contracts/adaptive.md.
**Tests**: requeridos por spec y AGENTS; pruebas por historia antes de integración.

## Phase 1: Setup
- [X] T001 Resolver plan y decisiones, contratos y guía en specs/006-adaptive-environment/.

## Phase 2: Foundation
- [X] T002 Definir catálogo, planes y análisis tipados con límites de 4 consultas/8 filtros/8 análisis/10 000 filas en crates/domain/src/adaptive/ (mod.rs y analysis.rs).
- [X] T003 Definir puertos/contexto autorizado y ejecución fallible en crates/application/src/adaptive/ (mod.rs y render.rs).

## Phase 3: US1 — Descubrimiento
- [X] T004 [US1] Probar descubrimiento dinámico y ausencia de campos privados en crates/application/tests/adaptive.rs.
- [X] T005 [US1] Implementar manifiestos y puente por permisos/metadatos Frappe en crates/infrastructure/src/adaptive/sources.rs y scripts/adaptive_frappe_bridge.py.

## Phase 4: US2 — Conversación y análisis
- [X] T006 [US2] Probar conteos, agrupación, comparación y proyección sobre nombres/esquemas distintos en crates/application/tests/adaptive.rs.
- [X] T007 [US2] Implementar intérprete de catálogo y análisis reproducible en crates/infrastructure/src/adaptive/planner.rs y crates/domain/src/adaptive/analysis.rs.
- [X] T008 [US2] Componer ejecutable y transporte de Telegram en apps/server/src/bin/adaptive_assistant.rs y scripts/telegram_adaptive.py.

## Phase 5: US3 — Contexto y cambios
- [X] T009 [US3] Probar tres turnos, revocación/cambio de campos, límites e interrupciones en crates/application/tests/adaptive.rs.
- [X] T010 [US3] Persistir contexto, claims y auditoría privados versionados en crates/infrastructure/src/adaptive/state.rs.

## Phase 6: US4 — Suscripciones
- [X] T011 [US4] Probar dos recursos, reentrega, cancelación y destinatario revocado en crates/application/tests/adaptive.rs.
- [X] T012 [US4] Implementar suscripciones/eventos autenticados genéricos en crates/application/src/adaptive/mod.rs, crates/infrastructure/src/adaptive/ y scripts/telegram_adaptive.py.

## Phase 7: Acceptance
- [X] T013 Publicar contrato e instrucciones reales en contracts/adaptive/ y docs/ADAPTIVE_RUNTIME.md.
- [X] T014 Ejecutar matriz SC-001–008 con evidencia en scripts/verify_adaptive.py y specs/006-adaptive-environment/acceptance.md.
- [X] T015 Ejecutar Rust fmt/clippy/tests y aceptación Python/CLI; mantener limitaciones verificables en docs/VALIDATION.md.

## Dependencies & Execution Order
T001 → T002/T003 → US1 → US2 → US3 → US4 → aceptación.
Las pruebas por historia se redactan primero; no se marcan completas sin evidencia.

## Parallel Examples
US1: comparar metadatos y fixtures mientras se revisan permisos del puente.
US2: casos de cálculo independientes de la adaptación LLM.
US3: fallos de contexto independientes de nuevas formulaciones.
US4: firma/deduplicación independientes de las plantillas de proveedor.
Son oportunidades por archivos; no se requiere delegación adicional.

## Implementation Strategy
Construir el núcleo compartido y probarlo con fixtures antes de reemplazar la ruta
sectorial de Telegram. Mantener sus avisos actuales hasta validar la transición.
No detenerse en un scaffold ni declarar completas las historias con pruebas simuladas.

## Phase 8: Convergence
- [X] T016 Persistir interpretaciones semánticas aprobadas por organización/versión e invalidarlas ante cambios de esquema, y manejar propuestas de efectos o capacidades ausentes sin ejecución automática en crates/domain/src/adaptive/, crates/application/src/adaptive/ y crates/infrastructure/src/adaptive/ per FR-008, FR-009, FR-012, US1/AC3, US2/AC5, US3/AC4 (partial)
- [X] T017 Registrar auditoría privada versionada de decisiones y procedencia sin cuerpos de mensajes y verificarla con pruebas en crates/application/src/adaptive/, crates/infrastructure/src/adaptive/, crates/application/tests/adaptive.rs y crates/infrastructure/tests/adaptive_state.rs per FR-017, T010 (partial)
- [X] T018 Eliminar el enrutamiento sectorial residual en scripts/run_telegram_staging.py y validar rutas en scripts/test_telegram_staging.py per FR-003, plan: scripts/run_telegram_staging.py (contradicts)
- [X] T019 Ejecutar una corrida completa de extremo a extremo de scripts/verify_adaptive.py (stage=full) y actualizar la evidencia verificable en specs/006-adaptive-environment/acceptance.md, contracts/adaptive/README.md, docs/ADAPTIVE_RUNTIME.md y docs/VALIDATION.md per SC-001, SC-008, T014, Constitution VI (partial)
