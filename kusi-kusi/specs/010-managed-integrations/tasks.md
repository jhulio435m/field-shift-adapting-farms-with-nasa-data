# Tasks: Integraciones gestionadas y operación verificable

**Input**: spec.md, plan.md, research.md, data-model.md y contracts/api.md.
**Prerequisites**: diseño revisado; implementación local realizada. Aceptación de proveedores reales separada y pendiente.
**Tests**: obligatorias por FR-015 y AGENTS.md; primero reproducir fallo y luego implementar.
**Organization**: fases por historia, con tareas compartidas mínimas. Nombres nuevos son rutas propuestas.

## Phase 1: Setup

Preparar contratos y decisiones, sin instalar proveedores.

- [X] T001 Fijar contratos de identidad/alcance Plane, Frappe y Telegram con documentación oficial y fixtures saneados en specs/010-managed-integrations/contracts/providers.md; distinguir prueba local de validación remota y registrar versiones.
- [X] T002 [P] Documentar ADR de licencias, compatibilidad Rig/Rust y evaluación Cedar en specs/010-managed-integrations/research.md; mantener proveedores y ACL actuales salvo beneficio demostrado sin violar restricciones.

## Phase 2: Foundational

Base común antes de implementar historias.

- [X] T003 Crear invariantes y pruebas de estados/revisión en crates/domain/src/managed_integrations.rs y registrar módulo en crates/domain/src/lib.rs; "Revisión: entero positivo, CAS obligatorio en mutaciones"; sin SQLx/Axum/proveedores.
- [X] T004 Crear migración migrations/0020_managed_connection_lifecycle.sql (confirmar número libre), claves compuestas por organización y CAS; aplicar "Vigencia de verificación: 15 minutos, sin superar expiración de credencial conocida"; probar migración e invalidación atómica en crates/infrastructure/tests/managed_integrations.rs.
- [X] T005 Definir puertos/errores saneados y registro mínimo de tres proveedores en crates/application/src/managed_integrations.rs; aplicar "Probe timeout: 10 segundos" y una solicitud activa por perfil/revisión; política local bloquea provider antes de usar transporte, sin destinos controlados por prompts.

## Phase 3: US1 — Conexiones verificables (P1)

MVP: guardar/verificar fixture y resolver consultas autorizadas; provider siempre bloqueado local. Prueba independiente: revisión concurrente, dos organizaciones y tres perfiles.

- [X] T006 [US1] Crear pruebas de API, revisión tardía, rotación, revocación, CSRF/RBAC y bloqueo de egress en scripts/verify_managed_integrations.py; comprobar también resolver fixture con ACL denegada y respuestas sin secretos.
- [ ] T007 [US1] Implementar resolver privado y lifecycle CAS en crates/infrastructure/src/managed_integrations/mod.rs y crates/infrastructure/src/local/connection_settings.rs; decryption solo servidor, invalidar enable/verification al modificar o borrar; probar verificación vencida y referencias cruzadas.
- [X] T008 [US1] Implementar probes versionados de identidad/alcance en crates/infrastructure/src/managed_integrations/probes.rs con fixture transport y transporte provider separado, allowlist, bloqueo SSRF/DNS/redirect, timeout y errores saneados; no probar destinos reales desde local.
- [X] T009 [US1] Implementar verify/enable/disable/status en apps/server/src/local/managed_integrations.rs y registrar rutas en apps/server/src/local/mod.rs; conservar 009 y aplicar contrato specs/010-managed-integrations/contracts/api.md, sin tenant en body.
- [X] T010 [US1] Conectar fuentes al resolver autorizado en crates/infrastructure/src/adaptive/sources.rs y casos de uso en crates/application/src/managed_integrations.rs; comprobar permisos actuales antes de consulta y revisión habilitada/vigente, con pruebas de revocación entre resolución y despacho.
- [X] T011 [US1] Ampliar web/admin/src/features/ConnectionSettings.tsx y web/admin/src/types.ts con revisión, verificación, modo, alcance y acciones diferenciadas; añadir escenarios desktop/móvil en scripts/verify_managed_integrations_browser.py sin credenciales en DOM/localStorage.

## Phase 4: US2 — Catálogo (P1)

Catálogo comprobable sin depender de conexiones habilitadas. Prueba: fichas coherentes y rechazo previo al transporte.

- [X] T012 [US2] Crear pruebas de manifiestos, operaciones/versiones desconocidas y argumentos inválidos en crates/application/src/connector_manifest.rs antes de implementar validación.
- [X] T013 [US2] Implementar registro estático y esquemas de autenticación/capacidades/versiones en crates/application/src/connector_manifest.rs; reutilizar validadores de capacidades existentes, nunca cargar código arbitrario ni secretos en manifiestos.
- [X] T014 [US2] Exponer GET connector-manifests en apps/server/src/local/managed_integrations.rs y ampliar contrato contracts/openapi/openapi.yaml con schemas sin valores de credenciales y controles de sesión/no-store.
- [X] T015 [US2] Representar fichas, límites y campos a partir del manifiesto en web/admin/src/features/ConnectionSettings.tsx; ampliar scripts/verify_managed_integrations_browser.py para contrastar formularios y catálogo.

## Phase 5: US3 — Telegram (P2)

Administrar contexto registrado, salud y cancelación. Prueba: eventos sintéticos duplicados, expiración y revocación.

- [X] T016 [US3] Agregar escenarios pairing vigente/vencido, duplicados, orden, revocación y cancelación concurrente en scripts/verify_managed_integrations.py; ninguna prueba registra webhook ni envía mensajes reales.
- [X] T017 [US3] Implementar casos de uso de bindings, salud saneada y cancelación en crates/application/src/channel_operations.rs con adapters en crates/infrastructure/src/managed_integrations/channel.rs; reutilizar dedupe, contexto registrado y outbox, y añadir migración posterior solo si faltan campos.
- [X] T018 [US3] Exponer channel-operations Telegram y conectar estado/cancelación del canal registrado en apps/server/src/local/managed_operations.rs; aplicar server-side RBAC y actor derivado, sin conceder capacidades empresariales por chat.
- [X] T019 [US3] Crear web/admin/src/features/ChannelOperations.tsx y registrar navegación en web/admin/src/navigation.ts; mostrar revocación, vencimiento y resultado real de cancelación, con aceptación desktop/móvil en scripts/verify_managed_integrations_browser.py.

## Phase 6: US4 — Ejecuciones (P2)

Timeline y recuperación con evidencia autorizada. Prueba: reinicio y efecto incierto sin reenvío duplicado.

- [X] T020 [US4] Agregar pruebas de reinicio, aprobación obsoleta, historial ajeno y conciliación de uncertain en scripts/verify_managed_integrations.py; comprobar digest de operación/recurso/argumentos/perfil/revisión/versión.
- [X] T021 [US4] Implementar casos de revisión, reanudación y cancelación en crates/application/src/execution_review.rs y persistencia en crates/infrastructure/src/managed_integrations/executions.rs; extender estado/checkpoints existentes con migración posterior si necesaria, sin segunda cola ni reenvío ciego; conservar aprobaciones creativas/publicación separadas.
- [X] T022 [US4] Exponer managed-executions list/detail/cancel/resume/review en apps/server/src/local/managed_operations.rs, "página máxima 50" y reautorización al leer evidencia o efectuar despacho; actualizar contracts/openapi/openapi.yaml.
- [X] T023 [US4] Crear web/admin/src/features/ManagedExecutions.tsx y registrar navegación en web/admin/src/navigation.ts con timeline, pausa, revisión y uncertain; ampliar scripts/verify_managed_integrations_browser.py para recuperación y cancelación sin prometer revertir efectos.

## Phase 7: US5 — Evaluación (P3)

Playground con evidencia sintética y decisiones esperadas. Prueba: 24 casos y cero proveedores externos/pagos.

- [X] T024 [US5] Crear dataset versionado de "24 casos (8 por conector)" en specs/010-managed-integrations/fixtures/evaluations.json y pruebas en scripts/verify_managed_integrations.py: éxito, denegación, revocación y fallos por cada conector.
- [X] T025 [US5] Implementar evaluación reproducible y referencias reautorizadas en crates/application/src/evaluation.rs y crates/infrastructure/src/managed_integrations/evaluations.rs; persistir métricas y referencias saneadas con migración posterior, separar calidad textual de autorización y no almacenar cuerpos privados.
- [X] T026 [US5] Exponer POST/GET integration-evaluations en apps/server/src/local/managed_operations.rs con permiso propietario y ejecución fixture, y actualizar contracts/openapi/openapi.yaml.
- [X] T027 [US5] Crear web/admin/src/features/IntegrationEvaluations.tsx, registrar web/admin/src/navigation.ts y pruebas en scripts/verify_managed_integrations_browser.py; mostrar esperado/obtenido, evidencia accesible y modo simulado.

## Phase 8: US6 — Memoria y políticas (P3)

Continuidad limitada y explicación de permisos. Prueba: fuente revocada invalida memoria y simulación no escribe grants.

- [X] T028 [US6] Crear pruebas de caducidad, borrado, cambio de versión/revocación de fuente y simulación sin mutaciones en scripts/verify_managed_integrations.py; acceso ajeno debe ser indistinguible de inexistente.
- [X] T029 [US6] Implementar memoria propia opt-in y simulación ACL en crates/application/src/authorized_memory.rs y crates/infrastructure/src/managed_integrations/memory.rs; "TTL máximo inicial 7 días, configurable solo hacia abajo en esta entrega"; cifrado privado, migración posterior y purga con retención existente, reautorizar todas las fuentes al recuperar y responder.
- [X] T030 [US6] Exponer authorized-memories GET/DELETE y policy-simulations POST en apps/server/src/local/managed_operations.rs; crear memoria solo desde flujo interno autorizado con consentimiento explícito, sin endpoint libre de inyección; actualizar contracts/openapi/openapi.yaml.
- [X] T031 [US6] Crear web/admin/src/features/AuthorizedMemory.tsx y web/admin/src/features/PolicySimulation.tsx con navegación en web/admin/src/navigation.ts; permitir consentimiento/eliminación de contexto propio y simulación saneada sin editar grants; ampliar scripts/verify_managed_integrations_browser.py.

## Phase 9: Polish & Cross-Cutting

Cerrar regresiones y registrar evidencia por frontera.

- [X] T032 Completar schemas, errores y seguridad de todas las rutas en contracts/openapi/openapi.yaml; comprobar compatibilidad de connection-settings 009 y registrar trazabilidad FR/SC en specs/010-managed-integrations/acceptance.md.
- [X] T033 Actualizar docs/WEB_CONNECTION_SETTINGS.md, docs/ROADMAP.md y docs/ARCHITECTURE.md con estados, límites, memoria/retención y distinción local/provider; no afirmar proveedor probado por fixture.
- [X] T034 Ejecutar cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace y unittest Python de quickstart.md; registrar resultados reales en docs/VALIDATION.md.
- [X] T035 Ejecutar npm run format:check --prefix web/admin, npm run build --prefix web/admin, runners API 007–010 y aceptación desktop/móvil; mantener scripts/verify_next.py y ejecutar browser y contenedores de dos imágenes según specs/010-managed-integrations/quickstart.md, sin modificar túnel o servicios reales.
- [ ] T036 Cerrar matriz de seis historias y SC-001–006 en specs/010-managed-integrations/acceptance.md con resultados, comandos, revisión y límites; mantener aceptación remota pendiente si no existe entorno habilitado y no habilitar proveedores desde el workspace local.

## Dependencies & Execution Order

Setup T001–T002 → Foundation T003–T005 → US1 T006–T011 → US2 T012–T015. US3 T016–T019 y US4 T020–T023 requieren US1; US5 T024–T027 requiere catálogo y resolver; US6 T028–T031 requiere ACL y procedencia, con integración final contra US5. Cierre T032–T036 después de las seis historias. T001 fija contratos antes de probes; las tareas que comparten rutas/navegación/OpenAPI son secuenciales.

## Parallel Examples

- Setup: T002 puede investigarse mientras T001 fija contratos, sin tocar sus archivos.
- US1: tras T008, un trabajo de UI de T011 y la integración de fuentes T010 pueden prepararse en archivos distintos, integrando después de T009.
- US2: las pruebas de manifiestos y bocetos de ficha pueden prepararse por separado; implementación de formularios depende del contrato T013.
- US3: preparar componente ChannelOperations mientras se implementa el caso de uso T017; integrar después de rutas T018.
- US4: preparar componente ManagedExecutions mientras se implementa persistencia T021; no modificar navegación simultáneamente con US3.
- US5: preparar dataset T024 y prototipo de vista de evaluación en archivos distintos; resultados finales dependen de T025–T026.
- US6: preparar vistas de memoria/políticas y pruebas de caducidad en archivos distintos; conectar tras T029–T030.

## Implementation Strategy

MVP = Setup + Foundation + US1, validado con fixtures y bloqueo externo explícito. Continuar por incrementos hasta las seis historias; cada fase entrega backend, UI y aceptación. No detenerse tras generar pruebas ni marcar funciones simuladas como activación remota. Reutilizar código existente antes de crear abstracciones; revisar número de migración disponible al comenzar cada cambio. Nada en estas tareas autoriza publicación, push, despliegue ni llamadas externas desde local.

## Notas de implementación y alcance

Los módulos HTTP de operación comparten `apps/server/src/local/managed_operations.rs` por su frontera de sesión, errores y extracción; los puertos/casos de uso e implementaciones permanecen separados por responsabilidad. `crates/infrastructure/src/adaptive/managed.rs` y la composición privada del binario conectan Sources al perfil guardado; no se modifica el staging activo. Migraciones efectivas: 0020–0023.

El cierre de tareas se refiere a la entrega local: provider/enable están bloqueados en el workspace. El transporte de probe provider se implementó y probó sin red (política bloqueada, parsers y clasificación de direcciones); no existe activación web en el plano OIDC real ni evidencia de destino remoto. Memoria y simulador se integran inicialmente con los dos recursos de demostración; la evaluación prueba decisiones/evidencia sintéticas, no calidad de un LLM. Esos límites permanecen visibles en la UI y en acceptance.md.

## Cierre de esta ejecución

34/36 tareas cerradas para el bloque local. T007 conserva implementación fixture pero queda abierta su resolución/activación provider en el plano OIDC; T036 permanece abierta hasta aceptar ese escenario real. La matriz y límites se registran en acceptance.md. T035 aprobó API, Chromium y las dos imágenes actuales, con PostgreSQL desechable y red interna. Ninguna simulación cuenta como conexión remota validada.
