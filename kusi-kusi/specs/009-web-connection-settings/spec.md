# Feature Specification: Configuración web de Plane, ERP y Telegram

**Created**: 2026-11-10
**Status**: Verified locally
**Input**: Los sistemas del piloto serán Plane y ERP CONEIMERA; canal Telegram. Configurar APIs y tokens desde la web.

## User Scenarios & Testing

### User Story 1 - Guardar configuración desde el panel (Priority: P1)
Un propietario configura Plane (URL/clave API/workspace), ERP CONEIMERA (URL/API Key/API Secret) y Telegram (token/nombre del bot/webhook) sin editar archivos del servidor.
**Why this priority**: El operador necesita gestionar la configuración desde su herramienta habitual.
**Independent Test**: Guardar, recargar y editar los tres perfiles conserva los datos públicos y no devuelve secretos.
**Acceptance Scenarios**:
1. Given una sesión propietaria válida, When se guardan datos válidos, Then persisten por organización con credenciales cifradas y estado guardado sin verificar.
2. Given un formulario de edición, When las credenciales se dejan vacías, Then se conservan las existentes; los campos nunca se rellenan con valores secretos anteriores.
3. Given otra organización o cuenta sin rol propietario, When se intenta leer o cambiar la configuración, Then se deniega o no se encuentran datos ajenos.

### User Story 2 - Rotar y borrar credenciales (Priority: P1)
El propietario reemplaza credenciales o las borra expresamente conservando el perfil público.
**Why this priority**: La operación segura requiere corrección y revocación local sin filtrar claves.
**Independent Test**: Rotación aumenta revisión; edición obsoleta produce conflicto; borrado elimina material cifrado y marca credenciales pendientes.
**Acceptance Scenarios**:
1. Given edición simultánea, When se guarda una revisión antigua, Then se rechaza sin sobrescribir cambios nuevos.
2. Given borrado confirmado, When se recarga el perfil, Then ya no contiene credenciales y el runtime existente no se modifica.

### User Story 3 - Comprender estado y límites (Priority: P2)
El panel distingue configuración almacenada de conexión autenticada/activa.
**Why this priority**: Guardar un token no demuestra que funcione ni autoriza envíos.
**Independent Test**: La UI y API indican que modo local no contacta proveedores ni activa webhooks/envíos.
**Acceptance Scenarios**:
1. Given modo local, When se guarda cualquier perfil, Then no hay llamadas externas ni activación de canales.
2. Given reinicio, When se carga el almacenamiento con la clave original, Then se conservan credenciales cifradas; si falta la clave y existen secretos se falla cerrado.

### Edge Cases
URLs con credenciales/query/fragment/IP local; salto de línea en token; campos no permitidos; edición concurrente; secreto ausente; clave maestra perdida/tamper; rol revocado; sesión inválida; Origin ajeno; tenant en payload; error de almacenamiento.

## Requirements
### Functional Requirements
- **FR-001**: Tenant y actor MUST derivarse de sesión verificada; solo propietarios habilitados administran perfiles.
- **FR-002**: El panel MUST permitir crear/editar Plane, ERP y Telegram con validación específica y estados claros.
- **FR-003**: Secretos MUST cifrarse con clave privada del servidor, nonce aleatorio y autenticación ligada a tenant/perfil/revisión; nunca retornan en API, logs, auditoría, HTML o localStorage.
- **FR-004**: Edición vacía MUST conservar credenciales; reemplazo y borrado explícitos MUST invalidar la revisión anterior.
- **FR-005**: Las mutaciones MUST aplicar CSRF/Origin/RBAC, revisión optimista y auditoría atómica sin cuerpos ni secretos.
- **FR-006**: Guardar MUST permanecer separado de probar/activar. Modo local bloquea llamadas, registro de webhook y envíos; no cambia configuración de staging.
- **FR-007**: La clave de cifrado MUST persistir privada y no regenerarse silenciosamente ante datos cifrados existentes.
- **FR-008**: La UI MUST ser usable en escritorio/móvil con estados vacíos/carga/error/guardado y campos secretos vacíos al editar.
- **FR-009**: Añadir migración, OpenAPI y documentación, con pruebas Rust/API/navegador y gates requeridos.
### Key Entities
Perfil por organización y adaptador; parche de credenciales de escritura; resumen sin secretos; revisión; clave privada del workspace.

## Success Criteria
### Measurable Outcomes
- **SC-001**: Tres perfiles pueden guardarse y recargarse desde navegador sin modificar .env.
- **SC-002**: El 100% de intentos sin sesión/CSRF/rol o con tenant falso se deniega.
- **SC-003**: Cero credenciales en respuestas/listados/auditoría y cero plaintext en columnas cifradas.
- **SC-004**: Conflictos y fallos de clave no sobrescriben credenciales.
- **SC-005**: Desktop/móvil completan configuración, rotación y borrado con cero llamadas reales.

## Assumptions
Primera entrega local de administración de configuración persistente; no activa los perfiles heredados del runtime ni sustituye por sí sola permisos/ACL por recurso. La activación real con OIDC/servicios y pruebas de proveedor es un paso posterior separado del guardado. La UI usa las sesiones locales existentes. Clave maestra privada local independiente de claves de API; rotación de credenciales soportada, rotación de clave maestra requiere operación posterior documentada. El producto conserva su núcleo genérico; ERP CONEIMERA es la etiqueta de una instancia Frappe.
