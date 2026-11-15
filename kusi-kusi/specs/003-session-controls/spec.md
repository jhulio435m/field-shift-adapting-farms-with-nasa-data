# Feature Specification: Control de sesiones locales

**Feature Branch**: `003-session-controls`

**Created**: 2026-11-06

**Status**: Implemented locally

**Input**: Siguiente bloque local de seguridad: permitir que cada usuario revise y cierre sesiones, y que el propietario pueda revocar sesiones de su organización sin exponer credenciales.

## User Scenarios & Testing

### User Story 1 - Revisar sesiones activas (Priority: P1)

Una persona autenticada revisa las sesiones activas de su cuenta para reconocer accesos vigentes. Un propietario también puede revisar las sesiones activas de cuentas de su organización. La lista identifica cuál es la sesión actual, pero no presenta tokens, hashes, direcciones IP ni datos del navegador.

**Why this priority**: Un usuario necesita detectar sesiones que ya no reconoce antes de poder cerrarlas.

**Independent Test**: Crear dos sesiones para un usuario y otra en una organización distinta. El usuario solo ve sus dos sesiones; el propietario ve las sesiones de su organización; ningún listado contiene secretos ni datos de red.

**Acceptance Scenarios**:

1. **Given** una cuenta con más de una sesión activa, **When** consulta sus sesiones, **Then** ve fechas de creación/vencimiento y una marca para la sesión actual.
2. **Given** una sesión de otra organización, **When** un propietario consulta el inventario, **Then** esa sesión no aparece.
3. **Given** una sesión vencida, revocada o asociada a una cuenta deshabilitada, **When** se lista el inventario, **Then** no se muestra como activa.

### User Story 2 - Revocar una sesión anterior (Priority: P1)

Una persona cierra una de sus otras sesiones. Un propietario puede cerrar cualquier otra sesión de su organización para responder a una cuenta comprometida. La sesión en uso no se puede revocar desde este flujo; la persona puede cerrar esa sesión mediante la acción de salir existente.

**Why this priority**: El cierre inmediato limita el acceso de una sesión sospechosa sin obligar a cambiar la contraseña de toda la organización.

**Independent Test**: Revocar una sesión secundaria, confirmar que su siguiente petición protegida devuelve no autenticado y verificar que las demás sesiones siguen activas.

**Acceptance Scenarios**:

1. **Given** una persona autenticada con varias sesiones, **When** revoca una sesión propia distinta de la actual, **Then** el token revocado deja de autenticar peticiones y desaparece del inventario activo.
2. **Given** un propietario y una sesión objetivo de su misma organización, **When** la revoca, **Then** deja de autenticar peticiones y se conserva la sesión actual del propietario.
3. **Given** una sesión actual, una sesión de otra organización o un ID desconocido, **When** se intenta revocar, **Then** la operación se rechaza sin cerrar otras sesiones.
4. **Given** una persona que salió de la aplicación, **When** verifica su cookie anterior, **Then** la sesión sigue revocada por la ruta de salida ya existente.

### User Story 3 - Mantener el cierre coherente ante cambios de cuenta (Priority: P2)

Cuando un propietario deshabilita una cuenta o restablece su contraseña, todas sus sesiones se revocan de forma persistente y desaparecen del inventario activo.

**Why this priority**: La cuenta no debe seguir apareciendo ni autenticando sesiones que ya perdieron autorización.

**Independent Test**: Crear sesiones, deshabilitar o restablecer la contraseña de la cuenta, y verificar el rechazo inmediato y la ausencia de sesiones activas.

## Edge Cases

- El propietario intenta revocar su sesión actual desde otra pestaña.
- Dos propietarios revocan la misma sesión simultáneamente.
- El ID de sesión es válido pero pertenece a otra organización.
- La base de datos falla durante la revocación.
- La sesión expiró entre la carga del inventario y la revocación.
- Se restablece una contraseña mientras la cuenta tiene sesiones activas.

## Requirements

### Functional Requirements

- **FR-001**: Cada sesión local MUST tener un identificador aleatorio no autenticador distinto del hash/token de credencial.
- **FR-002**: Una persona MUST poder listar únicamente sus sesiones activas; un propietario MUST poder listar sesiones activas de su organización.
- **FR-003**: El inventario MUST indicar la sesión actual y fechas de creación y vencimiento.
- **FR-004**: El inventario MUST excluir tokens, hashes de token, contraseñas, direcciones IP, agentes de usuario y datos de dispositivo.
- **FR-005**: Una persona MUST poder revocar una sesión propia distinta de la actual; un propietario MUST poder revocar una sesión ajena de la misma organización.
- **FR-006**: La sesión actual MUST rechazarse para revocación por esta operación; usar el cierre de sesión existente para salir.
- **FR-007**: La revocación MUST invalidar la fila de sesión y la credencial asociada en una transacción, y MUST bloquear la siguiente petición protegida.
- **FR-008**: Los errores de base de datos MUST fallar de forma cerrada y no informar éxito parcial.
- **FR-009**: Deshabilitar una cuenta o restablecer su contraseña MUST dejar sus sesiones persistentemente revocadas.
- **FR-010**: Los cambios MUST conservar cookies HttpOnly, CSRF, Origin, no-store, RBAC y el modo local loopback-only.
- **FR-011**: La auditoría MUST registrar actor, acción, ID opaco de sesión y hora, sin registrar token, hash, IP ni agente de usuario.

### Key Entities

- **Sesión local**: credencial de sesión opaca ligada a una cuenta y organización, con vencimiento, revocación e ID independiente para mostrar/administrar.
- **Sesión actual**: la sesión verificada que autentica la petición en curso.
- **Evento de revocación**: registro de actor, objetivo opaco, acción y momento; no contiene datos de transporte ni secretos.

## Success Criteria

- **SC-001**: El 100% de las cuentas de prueba solo ve sus sesiones; propietarios ven únicamente las sesiones de su propia organización.
- **SC-002**: El 100% de las sesiones revocadas o invalidadas falla en la siguiente petición autenticada.
- **SC-003**: El 100% de intentos de revocar la sesión actual, una sesión ajena por usuario no propietario, o una sesión de otro tenant se rechaza sin revocar otras sesiones.
- **SC-004**: Respuestas y auditoría no contienen tokens, hashes, contraseñas, IP ni User-Agent.
- **SC-005**: La lista y revocación funcionan en escritorio y móvil dentro del panel local.

## Assumptions

- La función aplica a sesiones locales existentes; no elige ni integra un proveedor OIDC/SSO.
- El ID de sesión usado en la ruta es una referencia de gestión, nunca un token de autenticación.
- No se añade recopilación de IP, User-Agent, ubicación ni huella de dispositivo.
- El cierre de la sesión actual sigue resolviéndose con el endpoint de logout actual.
- Rotación de claves de servicio y autenticación federada afectan integraciones/proveedores y permanecen fuera de este bloque.
