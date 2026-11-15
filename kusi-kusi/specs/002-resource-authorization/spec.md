# Feature Specification: Autorización de capacidades por recurso

**Feature Branch**: `002-resource-authorization`

**Created**: 2026-11-05

**Status**: Implemented locally

**Input**: Siguiente bloque local de la hoja de ruta: comprobar identidad de usuario final y permisos por recurso antes de habilitar consultas genéricas a sistemas conectados.

## User Scenarios & Testing

### User Story 1 - Asignar acceso a un recurso (Priority: P1)

Un propietario concede a una cuenta lectora habilitada acceso de solo lectura a un recurso de demostración concreto, después de confirmar la autorización fuera del sistema. El acceso se limita a la organización, usuario, capacidad y recurso seleccionados.

**Why this priority**: La identidad de una organización o de un canal no determina por sí sola qué registros puede consultar una persona. Este límite debe existir antes de conectar capacidades a conversaciones.

**Independent Test**: Crear un acceso de prueba para un usuario y recurso, y confirmar que otro recurso, usuario u organización no hereda el permiso.

**Acceptance Scenarios**:

1. **Given** un propietario y una cuenta lectora habilitada de la misma organización, **When** el propietario confirma la validación manual y concede lectura a un recurso ficticio, **Then** queda activo únicamente ese acceso.
2. **Given** un usuario deshabilitado, privilegiado, de otra organización o un recurso desconocido, **When** se intenta conceder acceso, **Then** la solicitud se rechaza sin crear un permiso.
3. **Given** un acceso activo, **When** el propietario lo revoca, **Then** la siguiente consulta se deniega.

### User Story 2 - Consultar un recurso autorizado (Priority: P1)

Una persona autenticada solicita una capacidad genérica de lectura. Kusi Kusi obtiene su organización y usuario desde la sesión verificada, verifica el permiso para el recurso concreto, valida los argumentos y devuelve solo el resultado de demostración autorizado.

**Why this priority**: El producto necesita aplicar permisos de usuario y de recurso antes de invocar un adaptador, sin delegar la decisión a una instrucción o identificador enviado por el cliente.

**Independent Test**: Ejecutar la misma capacidad con un recurso concedido y otro no concedido; el adaptador debe invocarse solo para el primero y la respuesta debe marcarse como simulada.

**Acceptance Scenarios**:

1. **Given** una sesión válida y permiso vigente para el recurso solicitado, **When** se ejecuta una lectura con argumentos válidos, **Then** se devuelve el resultado ficticio de ese recurso con estado de simulación.
2. **Given** un recurso sin permiso, una sesión revocada o argumentos no válidos, **When** se solicita la lectura, **Then** se deniega antes de invocar el adaptador y no se revela ningún valor del recurso.
3. **Given** una solicitud que incluye tenant o usuario en su cuerpo, **When** se envía, **Then** se rechaza o se ignoran esos campos; nunca sustituyen la identidad de la sesión.

### User Story 3 - Revisar las decisiones de acceso (Priority: P2)

Un propietario revisa concesiones y resultados de autorización para entender quién pudo consultar una capacidad y si el adaptador fue invocado, sin acceder a cuerpos de solicitud, respuestas privadas ni credenciales.

**Why this priority**: La operación debe permitir detectar concesiones incorrectas sin convertir los registros de auditoría en una copia de los datos consultados.

**Independent Test**: Ejecutar consultas permitidas y denegadas, y revisar que cada decisión tenga actor, capacidad, resultado y marca de tiempo sin payload.

**Acceptance Scenarios**:

1. **Given** una lectura permitida o denegada, **When** el propietario abre la actividad, **Then** ve su resultado y su actor, pero no el contenido de argumentos ni el resultado.
2. **Given** una cuenta no propietaria, **When** intenta listar o cambiar concesiones, **Then** la API deniega la operación aunque la interfaz oculte los controles.

### Edge Cases

- El usuario pierde su sesión o es deshabilitado después de recibir un permiso.
- El permiso se revoca mientras el usuario mantiene abierta la página de consulta.
- El adaptador devuelve un resultado que no cumple el contrato de salida.
- La capacidad está deshabilitada o su versión cambió.
- Se repite la misma lectura; no debe crear efectos externos ni una publicación.
- Un error de base de datos impide verificar el permiso; el sistema debe fallar cerrado.

## Requirements

### Functional Requirements

- **FR-001**: La organización y el usuario que solicitan una capacidad MUST derivarse de una sesión local verificada; el cliente no puede elegirlos.
- **FR-002**: Cada concesión MUST limitarse a una organización, un usuario habilitado, una capacidad y un recurso identificable.
- **FR-003**: El sistema MUST denegar por defecto si no existe una concesión explícita y vigente.
- **FR-004**: Solo un propietario MUST poder conceder o revocar accesos y MUST confirmar que la autorización fue validada fuera del sistema.
- **FR-005**: El sistema MUST comprobar la autorización del recurso antes de llamar al adaptador y MUST fallar cerrado ante errores de autorización.
- **FR-006**: La entrada y salida MUST validarse contra los contratos de la capacidad antes de devolver datos.
- **FR-007**: Las capacidades de este bloque MUST ser exclusivamente de lectura y la interfaz MUST mostrar que los resultados locales son simulados.
- **FR-008**: La auditoría MUST registrar actor, capacidad, recurso opaco, decisión y tiempo, y MUST excluir credenciales, argumentos y resultados.
- **FR-009**: El modo local MUST permanecer en loopback, usar solo recursos ficticios y bloquear llamadas o envíos a proveedores externos.
- **FR-010**: El contrato de autorización MUST ser independiente de una escuela, ERP, canal o lenguaje de programación.
- **FR-011**: Las cuentas deshabilitadas o sesiones revocadas MUST perder el acceso incluso si existe una concesión previa.

### Key Entities

- **Concesión de recurso**: permiso de lectura ligado a organización, usuario, capacidad y recurso.
- **Principal autenticado**: identidad interna derivada de la sesión verificada; no es un identificador suministrado en la petición.
- **Capacidad**: contrato versionado de lectura con argumentos, resultado y adaptador.
- **Decisión de acceso**: evento de auditoría que registra si la consulta podía continuar y si el adaptador fue invocado.

## Success Criteria

### Measurable Outcomes

- **SC-001**: El 100% de las lecturas permitidas en la matriz sintética devuelve el recurso esperado y el 100% de las lecturas sin permiso es denegado antes de invocar el adaptador.
- **SC-002**: Ningún caso de la matriz permite acceso cruzado entre organizaciones, usuarios, capacidades o recursos.
- **SC-003**: Las solicitudes que incluyen una identidad o tenant falsos no alteran el principal autenticado.
- **SC-004**: Cada lectura produce una decisión auditable sin guardar argumentos, resultados ni credenciales.
- **SC-005**: La revisión del tráfico local confirma cero llamadas externas y todas las respuestas de la demo aparecen marcadas como simuladas.
- **SC-006**: Un propietario completa la concesión y revocación desde el panel; una cuenta lectora consulta únicamente recursos concedidos en escritorio y móvil.

## Assumptions

- La primera entrega valida el modelo de autorización con cuentas locales y un adaptador sintético; no afirma validar el login ni los permisos de un sistema de cliente real.
- La autorización externa, OIDC/SSO, el mapeo de identidades de cada cliente y la selección de un proveedor de identidad quedan para una fase posterior con decisiones y credenciales autorizadas.
- El acceso se concede manualmente en la demo; la confirmación es una salvaguarda del flujo local, no una verificación legal o automatizada.
- Las lecturas no ejecutan SQL generado por IA, no usan proveedores de IA y no cambian datos.
