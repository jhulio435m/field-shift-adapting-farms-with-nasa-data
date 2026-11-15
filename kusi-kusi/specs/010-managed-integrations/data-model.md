# Data Model — 010

Todos los IDs de organización provienen de contexto verificado. Las relaciones persistentes usan claves compuestas que impiden referencias entre organizaciones. Los nombres siguientes son lógicos; no sustituyen migraciones.

## ManagedConnection

Extiende el perfil 009 con adapter_version, enabled_revision nullable y disabled_at. Revisión: entero positivo, CAS obligatorio en mutaciones. Un perfil por organización/kind continúa vigente. No expone ciphertext ni referencia de clave. Cualquier cambio de perfil invalida habilitación y verificación de forma atómica.

## ConnectionVerification

id, organization_id, profile_id, profile_revision, adapter_version, mode (`fixture|provider`), status (`pending|verified|failed|stale`), checked_at, expires_at, identity_reference privada, scope_summary saneado, error_code nullable. Vigencia de verificación: 15 minutos, sin superar expiración de credencial conocida. Probe timeout: 10 segundos. Estado `verified` con mode fixture jamás habilita proveedor. Resultado tardío queda stale mediante CAS.

Activación requiere revisión actual, versión compatible, verificación provider vigente, propietario y política de entorno que permita destino. En local devuelve bloqueo sin descifrar para red. Revocación de sesión/ACL se comprueba en cada ejecución, no se congela con habilitación. Verificación vencida bloquea ejecuciones nuevas hasta renovarse.

## ConnectorManifest

kind, version, auth_fields, capabilities, input/output schemas, verification_probe, required_scopes, timeout/rate_limit policy. Registro estático versionado y saneado; no permite cargar código ni destinos desde prompts. No incluye valores de credenciales. Versiones incompatibles requieren nueva verificación.

## ChannelBinding / Execution

Extender entidades existentes, no duplicarlas. Binding: actor/organización/contexto registrado, expires_at, revoked_at, política por chat. Execution: identidad idempotente, actor, recurso, perfil/revisión, adapter_version, proposal_digest, estado y checkpoint/version existente. Estados proyectados: pending → authorized → running → completed/failed/awaiting_review/cancel_requested/uncertain; awaiting_review → authorized tras aprobación exacta; pending/authorized → cancelled. Un running no garantiza cancelación. uncertain solo sale mediante conciliación que determine efecto o ausencia comprobada; nunca reenvío ciego.

Aprobación vinculada a digest de argumentos normalizados, operación, recurso, perfil/revisión y versión. Reautorizar al reanudar y justo antes del efecto. Revocación bloquea envíos aún no iniciados; no promete retirar solicitudes ya transmitidas.

## EvaluationCase / EvaluationRun

ID, connector/version, fixture_version, resource_ref, pregunta sintética, decisión/evidencia esperada, referencias permitidas. Suite de 24 casos (8 por conector). Run conserva decisiones, referencias saneadas y métricas separadas de autorización y calidad; sin cuerpos remotos privados ni credenciales. Dataset versionado en specs/010-managed-integrations/fixtures/.

## AuthorizedMemory

id, actor_id, organization_id, source_refs, source_versions, contenido mínimo cifrado/referencia privada, created_at, expires_at, deleted_at. TTL máximo inicial 7 días, configurable solo hacia abajo en esta entrega. No memoria global; acceso solo del actor. Reautorizar todas las fuentes al recuperar y antes de responder; si cualquiera se revoca o cambia de versión, invalidar la memoria completa. Borrado lógico inmediato excluye recuperación; purga física mediante retención existente.

## PolicySimulation

Solicitud efímera con principal derivado/objetivo autorizado, operación y recurso accesible; devuelve allow/deny + código saneado. No persistir cuerpos, no mutar grants. Solo propietario para simulación administrativa; un recurso ajeno produce respuesta indistinguible de recurso inaccesible.


## Correspondencia con el esquema local

En local no se persiste una identidad remota verificada: scope_summary distingue presencia de campos sintéticos. El digest de identidad solo es resultado privado del parser provider y no se usa para conceder ACL. El perfil no recibe enabled_revision por la API local. `managed_execution_events` se refiere por FK compuesta a `local_outbox`; no es una cola adicional. Los snapshots opcionales de perfil en outbox forman parte del digest y se comprueban antes de acciones. Las memorias locales actuales tienen una fuente de demostración por fila y versión derivada de su representación canónica; varias fuentes requerirán ampliar ese contrato antes de usarse.
