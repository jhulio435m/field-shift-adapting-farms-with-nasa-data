# Feature Specification: Inicio de sesión del panel con OIDC

**Feature Branch**: `004-panel-oidc`

**Created**: 2026-11-06

**Status**: Aceptación API loopback aprobada; pendiente revisión visual en navegador

**Input**: Siguiente bloque de la hoja de ruta: construir una primera entrada de administración de producción con OIDC, separada del workspace local y sin acoplar el núcleo a un proveedor.

## User Scenarios & Testing

### User Story 1 - Iniciar sesión con la identidad de la organización (Priority: P1)

Una persona entra al plano de administración usando la identidad corporativa configurada para la instancia. Solo las identidades y membresías provisionadas previamente pueden entrar; el plano de administración muestra su organización y rol vigentes.

**Why this priority**: El plano de administración necesita una opción de identidad corporativa antes de un piloto empresarial.

**Independent Test**: Con un emisor de prueba compatible y una identidad provisionada, iniciar sesión y comprobar que la persona recibe únicamente la organización y el rol registrados en el plano de administración.

**Acceptance Scenarios**:

1. **Given** OIDC configurado y una identidad con membresía provisionada y habilitada, **When** la persona completa el inicio de sesión, **Then** llega al plano de administración con la organización y el rol registrados localmente.
2. **Given** una identidad válida del proveedor que no tiene membresía provisionada, **When** intenta entrar, **Then** el acceso se deniega y no se crea una cuenta, membresía ni organización.
3. **Given** una instancia sin acceso OIDC configurado, **When** una persona intenta entrar al plano de administración, **Then** el panel no inicia una redirección a un proveedor desconocido.

### User Story 2 - Rechazar respuestas de identidad inválidas o reutilizadas (Priority: P1)

Una persona y el operador reciben un resultado claro cuando la respuesta de identidad está vencida, alterada, destinada a otro cliente, emitida por otro proveedor o vinculada a una solicitud previa ya consumida. Ninguna respuesta inválida crea una sesión.

**Why this priority**: La validación de identidad es el límite de seguridad del nuevo método de acceso.

**Independent Test**: En un proveedor simulado, alterar individualmente emisor, audiencia, vencimiento, nonce, código o estado; cada variante debe fallar y dejar intactas las sesiones existentes.

**Acceptance Scenarios**:

1. **Given** una respuesta con emisor, audiencia, firma, tiempo o nonce inválido, **When** regresa al panel, **Then** se rechaza sin emitir cookie de sesión.
2. **Given** un código o estado ya consumido o asociado a otra ventana de navegador, **When** se vuelve a presentar, **Then** se rechaza y no se inicia una sesión nueva.
3. **Given** que el proveedor o su configuración no está disponible, **When** la persona intenta autenticarse, **Then** ve un error genérico sin secretos, tokens ni detalles internos.

### User Story 3 - Mantener las sesiones y la operación local seguras (Priority: P2)

Una persona que inició sesión con OIDC dispone de cierre, expiración y revocación de sesión en el plano de administración. El workspace local continúa aislado, usa su autenticación de demostración y no contacta proveedores ni ofrece OIDC externo.

**Why this priority**: Añadir una opción de identidad no debe degradar las protecciones actuales ni habilitar llamadas externas durante el desarrollo local.

**Independent Test**: Ejecutar el panel con configuración local sin proveedor y comprobar que el flujo OIDC no realiza conexiones; en el entorno de prueba OIDC, cerrar la sesión e invalidar accesos administrativos posteriores.

**Acceptance Scenarios**:

1. **Given** una sesión de control creada mediante OIDC, **When** la persona la cierra o la revoca, **Then** las siguientes peticiones autenticadas con esa sesión fallan.
2. **Given** el modo local de desarrollo sin habilitación explícita, **When** una persona carga o usa el inicio de sesión, **Then** solo opera el método local existente y no se realizan llamadas externas.
3. **Given** una identidad o membresía administrativa deshabilitada, **When** intenta autenticarse, **Then** el panel deniega el acceso aunque el proveedor confirme la identidad.

### Edge Cases

- El proveedor responde con error, redirección no registrada o parámetros duplicados.
- Se vence el intento mientras la persona está en el proveedor, o la persona abre dos intentos en el mismo navegador.
- La cuenta vinculada cambia de nombre o correo en el proveedor; esos atributos no deben cambiar el identificador ni los permisos locales.
- Se deshabilita la identidad o membresía después de comenzar y antes de completar el flujo, o se elimina la organización asociada.
- El proveedor rota sus claves de firma o publica temporalmente metadatos incompletos.
- La cookie o el proxy de retorno pierde los atributos requeridos o no conserva la ruta de callback exacta.
- Una configuración intenta reutilizar un emisor, client ID o callback entre organizaciones de forma no permitida.

## Requirements

### Functional Requirements

- **FR-001**: El plano de administración MUST permitir autenticación mediante OpenID Connect Authorization Code Flow solo cuando el operador configure explícitamente un emisor.
- **FR-002**: El sistema MUST asociar la identidad federada por el par estable emisor/sujeto con membresías administrativas provisionadas previamente.
- **FR-003**: El sistema MUST derivar organización, persona y rol desde las membresías registradas; nunca aceptar esos valores desde el callback ni confiar solo en correo o nombre.
- **FR-004**: El sistema MUST validar emisor, audiencia, firma, vencimiento y correlación con el intento de autenticación antes de crear una sesión.
- **FR-005**: Cada intento MUST ser impredecible, de un solo uso, ligado al navegador que lo inició y tener vencimiento limitado.
- **FR-006**: El sistema MUST rechazar identidades o membresías sin provisión previa o deshabilitadas; no debe crear identidades, membresías u organizaciones automáticamente.
- **FR-007**: Las sesiones OIDC MUST usar cookies HttpOnly, CSRF, no-store y revocación en el plano de administración, separadas de las sesiones locales. Cada endpoint administrativo que gestione datos de organización MUST aplicar RBAC del lado servidor antes de habilitarse.
- **FR-008**: El sistema MUST mantener el inicio local disponible únicamente dentro del modo local aislado; ese modo no debe mostrar OIDC ni realizar llamadas a proveedores.
- **FR-009**: Configuraciones, secretos de cliente, códigos, tokens y errores sensibles MUST permanecer del lado servidor y no aparecer en logs, respuestas del panel ni almacenamiento del navegador.
- **FR-010**: El operador MUST poder desactivar el acceso OIDC de la instancia sin alterar membresías ni sesiones no relacionadas.
- **FR-011**: La integración MUST permanecer independiente de un proveedor particular; la selección y validación del proveedor del primer piloto se registran como decisión operativa antes de habilitarlo.

### Key Entities

- **Configuración de identidad**: Emisor registrado, parámetros del cliente y estado de habilitación; no visible para usuarios finales.
- **Identidad administrativa**: Par único emisor/sujeto, independiente de cuentas del workspace local.
- **Membresía administrativa**: Relación entre identidad, organización existente, rol asignado y estado habilitado.
- **Intento de autenticación**: Operación temporal y de un solo uso ligada a navegador, organización y configuración.
- **Sesión administrativa**: Sesión persistente del plano de administración, independiente del workspace local.

## Success Criteria

### Measurable Outcomes

- **SC-001**: El 100% de las identidades con membresía habilitada del entorno de prueba inicia sesión con la organización y rol correspondientes, sin poder alterarlos desde el navegador.
- **SC-002**: El 100% de callbacks con firma, emisor, audiencia, tiempo, correlación o cuenta inválidos falla sin crear una sesión.
- **SC-003**: Ninguna identidad sin membresía previa puede crear acceso u organización automáticamente.
- **SC-004**: El modo local de desarrollo realiza cero conexiones a proveedores de identidad y conserva el inicio local existente.
- **SC-005**: El cierre o revocación invalida la sesión en la siguiente petición protegida.

## Assumptions

- La primera entrega soporta OIDC estándar con un proveedor por instancia configurado por operador; no incluye SAML ni elección dinámica de proveedores por usuarios.
- La primera pantalla administrativa solo muestra membresías provisionadas y permite cerrar la sesión. No expone aún APIs de gestión de usuarios, organizaciones o configuración.
- Las identidades y membresías se provisionan previamente en el plano de administración; el proveedor autentica identidad, pero no asigna tenant ni permisos.
- La política de MFA depende del proveedor elegido y debe verificarse para el piloto; Kusi Kusi no declarará MFA si no comprueba una política equivalente.
- Las pruebas de protocolo pueden usar un proveedor simulado en loopback. No usan cuentas, secretos, correo ni endpoints de identidad reales. El workspace local no se convierte en cliente OIDC.
- El inicio de sesión local continúa siendo la opción del modo local; el método habilitado para despliegues reales se decidirá y configurará expresamente.
