# Acceptance — 010

Estado: **bloque local implementado y verificado; especificación completa parcialmente aceptada**. Corte: 2026-11-10 (Lima). T007 y T036 siguen abiertos por el resolver/activación real del plano OIDC y su aceptación. No se presentan como listos por aprobar fixtures.

## Evidencia ejecutada

| Comprobación | Resultado / evidencia |
| --- | --- |
| cargo fmt --check | aprobado |
| cargo clippy --workspace --all-targets -- -D warnings | aprobado |
| cargo test --workspace | 92 reportadas aprobadas, 0 fallidas, 3 opt-in omitidas por el comando general |
| Pruebas opt-in | las tres ejecutadas aparte con PostgreSQL desechable: ACL, HTTP y migraciones/races 010 |
| Python unittest discover | 14 aprobadas |
| Next format + build | aprobados; App Router/Turbopack y TypeScript |
| API 010 + asistente compilado | [api-acceptance.json](api-acceptance.json) |
| Regresiones 009 / 007 / 008 / Next | [settings](settings-regression.json), [external](external-regression.json), [adaptive](adaptive-regression.json), [Next](next-regression.json) |
| Chromium desktop/móvil | [browser-acceptance.json](qa/browser-acceptance.json), seis capturas de datos sintéticos |
| Detector UI | [design-detect.json](qa/design-detect.json): sin hallazgos |
| Dos imágenes Docker actuales | [container-acceptance.json](container-acceptance.json): PostgreSQL temporal, red interna, SSR/CSS, proxy, no-store, contrato 010 y separación local/admin |
| OpenAPI | YAML y referencias locales válidos; contrato actualizado |
| Snapshot de fuentes | [source-manifest.json](source-manifest.json); cotejado después de construir imágenes |

Resumen procesable: [validation-summary.json](validation-summary.json). Los runners usan procesos/bases temporales, no leen .env y no cambian servicios existentes.

## Historias y criterios

| Historia / criterios | Resultado local | Límite |
| --- | --- | --- |
| US1; FR-001–005, FR-016; SC-001–002 | perfiles versionados, simulación, CAS concurrente, expiración, FK tenant, lectura tipada y revocación en los tres conectores; composición real del binario con perfil fixture y planner loopback | US1 escenario 2 real no implementado ni probado: falta integrar resolver de credenciales y ciclo provider/enable en el plano OIDC |
| US2; FR-006–007; SC-002 | manifiestos, formularios, versiones/argumentos inválidos rechazados antes de transporte | no acredita contratos de instalaciones reales |
| US3; FR-008–009; SC-003 | trabajador compilado deduplica eventos repetidos/fuera de orden y no hereda contexto revocado/vencido; API/UI de revocación | no registra webhooks ni envía Telegram real |
| US4; FR-010–011; SC-003 | outbox existente con historial durable, snapshot privado revisable, digest/CAS, edición invalida aprobación, cancelación y solicitud de cancelación; reinicio conserva uncertain sin reenvío | revisión local propia; propuestas ligadas a proveedor se rechazan; no conciliación contra proveedor remoto |
| US5; FR-012, FR-015; SC-004 | 24 preguntas/casos sintéticos, decisión y evidencia esperadas, evaluación persistente y detección de esperado incorrecto en prueba | quality_evaluated=false; no mide calidad de un LLM ni permisos remotos |
| US6; FR-013–014; SC-005 | memoria opt-in cifrada por actor, TTL, borrado, fuente revocada/versionada y simulador de solo lectura | integración inicial con los dos recursos de demostración; no memoria remota Plane/Frappe |
| Transversal; SC-006 | flujos locales desktop/móvil, permisos actuales, sin secretos de conexión en HTML/localStorage; listas/timeline omiten cuerpos, propuesta privada solo para revisión autorizada | las capturas acreditan fixtures, no proveedor real |

## Pendientes concretos

1. **T007 parcial**: conectar resolución privada de credenciales y ciclo de verificación/habilitación provider al plano OIDC con identidad/membresía verificadas. La API local siempre bloquea provider y enable. El transporte de probe separado existe, con clasificación DNS, pinning, allowlist, límites y parsers probados sin red; no constituye esa integración administrativa.
2. **T036 abierto**: completar aceptación del escenario real US1.2 en un entorno explícitamente habilitado, con identidades esperadas, permisos de recurso y contratos de instalación comprobados. Registrar por separado autenticación, alcance y ejecución del conector. No simular aprobación de este criterio.

No se activaron conexiones reales, no se reiniciaron servicios de staging ni el túnel de Telegram, y no se publicaron cambios. No se instalaron motores externos ni se actualizó Rust para adoptar Rig/Cedar.
