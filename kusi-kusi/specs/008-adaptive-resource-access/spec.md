# Feature Specification: Consultas adaptativas con ACL de recurso

**Created**: 2026-11-10
**Status**: Verified locally
**Input**: Completar autónomamente los pendientes locales: integrar ACL con asistente/canal registrado, validar OIDC visualmente y corregir fugas SQLite.

## User Scenarios & Testing

### User Story 1 - Descubrir solo recursos concedidos (Priority: P1)
Una persona vinculada a un canal registrado ve solo recursos analíticos autorizados.
**Why this priority**: El conocimiento del catálogo no puede convertirse en autorización.
**Independent Test**: Una ACL parcial oculta recursos; una clave revocada impide incluso descubrir la fuente.
**Acceptance Scenarios**:
1. Given principal de una configuración privada y chat vinculado, When se descubre el entorno protegido, Then solo aparecen recursos con vínculo, ACL y permiso global vigentes.
2. Given falta de política o error de permisos, When se descubre, Then se falla cerrado sin lecturas de datos.

### User Story 2 - Consultar y recibir avisos tras autorización (Priority: P1)
El asistente vuelve a comprobar permisos antes de leer, suscribir y autorizar eventos.
**Why this priority**: Contexto y respuestas guardadas nunca conceden acceso.
**Independent Test**: Revocar tras una primera consulta impide nuevas lecturas, replay de evidencia y avisos.
**Acceptance Scenarios**:
1. Given recurso concedido, When el intérprete propone un cálculo tipado, Then el adaptador original ejecuta solo esa consulta tras comprobar la ACL.
2. Given revocación, When se repite un request o evento, Then no se devuelve evidencia anterior ni se invoca la lectura.
3. Given tenant, principal o recurso falsos, When se intenta sustituir el contexto, Then la operación se deniega.

### User Story 3 - Cerrar aceptación local y recursos privados (Priority: P2)
El operador prueba OIDC en escritorio/móvil y Telegram con transportes sintéticos sin proveedores.
**Why this priority**: Hace verificable el límite entre interfaz, sesión y autorización.
**Independent Test**: Navegador real recorre éxito/error/cierre sin credenciales frontend y SQLite cierra las conexiones.
**Acceptance Scenarios**:
1. Given emisor OIDC sintético, When se revisa login/error/sesión, Then la UI es usable en viewport real y la sesión solo vive en cookie HttpOnly.
2. Given procesamiento de mensajes/events, When finaliza o falla, Then las conexiones SQLite se cierran y la transacción confirma o revierte correctamente.

### Edge Cases
Revocación entre descubrimiento y lectura, permiso global ausente, política no protegida, catálogo/capacidad versionados, recurso sin binding, configuración ACL ausente cuando es obligatoria, fallo PostgreSQL, replay de recibos, error de callback, sesión revocada.

## Requirements
### Functional Requirements
- **FR-001**: Actor y tenant MUST proceder de configuración privada/canal registrado, nunca texto ni payload.
- **FR-002**: Cada recurso protegido MUST vincularse explícitamente a una capacidad/version/recurso de ACL; sin coincidencia exacta se deniega.
- **FR-003**: El permiso global, clave, vínculo y ACL MUST comprobarse sin caché antes de descubrimiento de fuente, lectura, suscripción y autorización de evento.
- **FR-004**: Una política ausente MUST denegar en este flujo protegido, aunque otras capacidades conserven su contrato de servicio.
- **FR-005**: Cambios en los recursos autorizados MUST invalidar el catálogo/recibos previos; el adaptador mantiene filtros y validaciones de origen.
- **FR-006**: El despliegue protegido MUST poder exigir ACL; configuración antigua no se migra ni activa silenciosamente.
- **FR-007**: La auditoría MUST excluir sujetos externos, mensajes, credenciales y datos leídos.
- **FR-008**: La ejecución local y pruebas MUST bloquear llamadas pagadas/reales; no activar proveedores, cambiar staging ni usar bases existentes.
- **FR-009**: La aceptación OIDC MUST usar emisor loopback, PostgreSQL desechable y navegador real con evidencia visual desktop/móvil.
- **FR-010**: Las conexiones SQLite MUST cerrarse incluso en excepciones sin perder commit/rollback.
### Key Entities
Vínculo de recurso analítico/capacidad; entorno autorizado; contexto de canal privado; catálogo filtrado/versionado; evidencia de aceptación.

## Success Criteria
### Measurable Outcomes
- **SC-001**: El 100% de denegaciones de la matriz impide lectura y no devuelve evidencia anterior.
- **SC-002**: Revocación de clave/vínculo/concesión afecta la siguiente operación y los eventos.
- **SC-003**: Dos organizaciones y recursos distintos no heredan permisos.
- **SC-004**: Cero conexiones SQLite sin cerrar en la suite Python con ResourceWarning tratada como fallo.
- **SC-005**: Login, error, sesión y logout OIDC se verifican en viewport desktop y móvil sin llamadas externas.

## Assumptions
Una concesión sobre un recurso analítico autoriza el conjunto completo de registros que ya permite ese adaptador; no sustituye ACL de filas. No se aplica una concesión de un registro a un conjunto sin filtrado de origen. Comprobación remota automática pendiente de protocolo/proveedor. Sin publicación, activación real, push ni reinicio de servicios existentes. El modo legado permanece explícitamente distinguido del modo ACL obligatorio.
