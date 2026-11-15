# Evidencia de aceptación 006 — 9 de noviembre de 2026

Estado: implementación y matriz de aceptación completas y aprobadas.

## Evidencia local independiente del proveedor

- `cargo fmt --all --check`: aprobado.
- `cargo clippy --workspace --all-targets -- -D warnings`: aprobado.
- `cargo test --workspace`: 72 pruebas aprobadas, cero fallos. Suite completa
  ejecutada sin exclusiones (incluye persistencia de interpretaciones semánticas e
  invalidación por cambio de esquema, propuestas `unsupported` y auditoría privada).
- `python3 -m unittest discover -s scripts -p 'test_*.py'`: 6 aprobadas (tras eliminar
  los scripts sectoriales obsoletos y verificar que la ruta `/events/coneimera-inscripciones`
  responde `404`).
- `npm run format:check --prefix web/admin`: aprobado.
- `npm run build --prefix web/admin`: Next.js 16.4.5/Turbopack y TypeScript aprobados.
- Intentos del ejecutable sin delegación y con modo local activo: bloqueados.
- Migración SQLite v1 aplicada dos veces en el ledger real: idempotente y sin
  perder la vinculación. Los formatos JSON son nuevos y versionados; PostgreSQL
  no requiere migración en esta feature.

Los tests de aplicación usan propuestas programadas; verifican cálculo y permisos,
no comprensión de lenguaje natural. El test de persistencia reabre FileState y
comprueba claims, recibos, aislamiento de organizaciones y permisos de archivos.
Las pruebas Python del puente usan una API Frappe simulada para verificar exclusión
de texto, secretos, adjuntos, campos ocultos/no permitidos y catálogo revocado.

## Integración real autorizada

Servidor `132.226.244.23`, ERPNext ya conectado, cuenta de servicio de lectura.
Sin elevar sus permisos de negocio ni insertar registros de prueba.

- Descubrimiento por roles explícitos y campos permitidos: un recurso de negocio,
  nueve campos aptos; nombres obtenidos de metadatos, sin despacho sectorial.
- Consulta natural: 104 registros; continuación sobre VIP: 27.
- Proyección por el transporte activo: 332 registros al 2026-12-25, a partir de
  104 registros en 21 días completos y ritmo 104/21 por día. Escenarios de
  sensibilidad 218–446; no son un intervalo de confianza ni una garantía.
- Suscripción creada por petición natural y webhook genérico instalado.
- Firma y payload generados con las funciones reales de Frappe sobre documento
  **no guardado**, prefijo reservado de prueba. Tres POST (evento + dos reentregas)
  devolvieron 200; un claim durable y una aceptación API Telegram. No demuestra
  lectura del destinatario. El aviso identifica explícitamente que es una prueba.
- Webhook vinculado a namespace de organización/principal. Hook anterior propio
  deshabilitado; webhook sectorial anterior también deshabilitado. Permanece un
  solo hook genérico activo para esta fuente/recurso.
- Receptor y túnel activos. El túnel es temporal; la URL debe mantenerse registrada.

## Ajustes detectados durante la aceptación

Las ejecuciones previas no se cuentan como matrices aprobadas: revelaron ambigüedad
entre cantidad e importe, pérdida de recurso al contestar una aclaración y fallos
al interpretar proyecciones. Se precisó la semántica genérica de las operaciones,
la conservación de recurso/filtros y los límites de ventana temporal; aumentó el
presupuesto de salida del perfil de staging. Una respuesta inválida del proveedor
se presenta como dependencia no disponible, no como una negativa al objetivo.
No se relajó la autorización ni se añadieron funciones por negocio.

El script admite una aclaración y exige que su respuesta conserve el objetivo y
produzca el cálculo de referencia. No acepta una negativa ni una cifra distinta.
Las seis formulaciones reservadas finales se renovaron después de los ajustes.

La matriz completa de aceptación (`acceptance-matrix.json`, ejecutada en una sola
corrida de extremo a extremo con `stage="full"`) consolidó los 26 objetivos de
consulta (20 estándar y 6 formulaciones de reserva, 29 llamadas al proveedor), las 6
conversaciones de 3 turnos, el ciclo de vida de suscripciones sintéticas con
deduplicación y cancelación, 31 registros de auditoría (`audits_recorded: 31`) sin
cuerpos de mensajes y la revocación de acceso.

## Matriz de criterios

| Criterio | Evidencia | Estado |
|---|---|---|
| SC-001 | verify_adaptive.py: dos fuentes, tres recursos, 20 objetivos y seis formulaciones reservadas con proveedor real | Aprobado (26/26 objetivos completados en acceptance-matrix.json) |
| SC-002 | Referencias deterministas de conteo/grupo/suma/media/comparación/proyección; ERP real con evidencia | Aprobado (motor determinista y validación con ERP real) |
| SC-003 | Seis conversaciones de tres turnos; aislamiento y revocación en tests Rust | Aprobado (6 conversaciones multi-turno completadas en acceptance-matrix.json y suite Rust) |
| SC-004 | new_resource_and_schema_changes_are_observed; revoked_permission_and_actor_isolation_override_memory; tercer recurso añadido durante aceptación | Aprobado (incorporación dinámica de Reservas y pruebas de aislamiento) |
| SC-005 | ten_invalid_permissions_fields_and_versions_never_read; JSON estricto; política de campos Frappe y columnas rechazadas sin lecturas | Aprobado en fixtures y pruebas unitarias |
| SC-006 | two_generic_subscriptions_dedupe_and_cancel; test transporte firma/destino; webhook Frappe real con dos reentregas | Aprobado (suscripción doble, deduplicación, cancelación verificada) |
| SC-007 | Truncación, fallo de claim, claim interrumpido, fallo de commit, plan inválido, fallo de watch pendiente y recuperación; envío incierto registra unknown | Aprobado en fixtures |
| SC-008 | Este informe, gates completos y separación de evidencia | Aprobado |

## Límites del alcance

Autonomía para descubrir y componer lecturas/análisis y suscripciones dentro de una
conexión delegada. Adaptadores disponibles: manifiestos y Frappe por SSH. Un protocolo
nuevo requiere adaptador y autorización inicial. No hay ejecución de escrituras
arbitrarias, SQL del modelo, autoexploración de destinos ni compatibilidad universal
probada. La matriz no constituye una medida estadística de fiabilidad del modelo.
Estado de staging privado en archivos; no se ha probado carga concurrente multiusuario,
entrega exactamente una vez bajo caídas ni recuperación automática de envíos inciertos.
