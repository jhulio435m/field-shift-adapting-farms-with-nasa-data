# Feature Specification: Integraciones gestionadas y operación verificable

**Feature Branch**: sin rama creada; directorio `010-managed-integrations`

**Created**: 2026-11-10

**Status**: Bloque local implementado y verificado; aceptación de la especificación parcial. T007/T036 pendientes para activación y aceptación reales.

**Input**: Usar Spec Kit para incorporar los aportes de la investigación GitHub al piloto Plane, ERP CONEIMERA y Telegram, con administración web.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Conexiones verificables desde la web (Priority: P1)

Como propietario quiero conocer qué identidad y alcance tiene cada conexión, y decidir cuándo queda disponible para consultas autorizadas.

**Why this priority**: Los perfiles guardados actualmente no son conexiones operativas verificadas.

**Independent Test**: Con tres destinos ficticios, guardar, verificar, habilitar, consultar y revocar; comprobar que ningún paso contacta servicios externos en el entorno local.

**Acceptance Scenarios**:

1. **Given** un perfil guardado, **When** se solicita su verificación local, **Then** aparece un resultado rotulado como simulación, ligado a su revisión y sin habilitación real.
2. **Given** un entorno que permite proveedores y un propietario autorizado, **When** verifica y habilita la revisión exacta, **Then** solo las consultas autorizadas utilizan esa conexión y los permisos comprobados quedan visibles.
3. **Given** una conexión habilitada, **When** se rota, cambia o revoca el perfil, **Then** queda invalidada su verificación y ninguna ejecución nueva usa la revisión anterior.
4. **Given** una conexión autenticada sin permiso sobre un recurso, **When** se consulta ese recurso, **Then** se deniega sin revelar su existencia ni contenido.

### User Story 2 - Catálogo coherente de conectores (Priority: P1)

Como administrador quiero entender qué autenticación, consultas y límites admite cada integración antes de configurarla.

**Why this priority**: Permite ampliar sistemas sin formularios y comportamientos contradictorios.

**Independent Test**: Consultar las fichas de Plane, Frappe y Telegram y contrastar formularios y operaciones contra sus definiciones.

**Acceptance Scenarios**:

1. **Given** un conector disponible, **When** se consulta su ficha, **Then** muestra versión, campos, operaciones, permisos y límites sin revelar credenciales.
2. **Given** una operación desconocida o argumentos inválidos, **When** se propone su ejecución, **Then** se rechaza antes de contactar el destino, incluso si la propone el asistente.

### User Story 3 - Administrar Telegram y sus conversaciones (Priority: P2)

Como propietario quiero revisar vinculaciones, revocarlas y conocer el estado del canal; como usuario quiero conocer el progreso y solicitar cancelación.

**Why this priority**: El canal necesita una operación comprensible además de guardar un token.

**Independent Test**: Vincular identidades ficticias, revocar una, reproducir eventos y cancelar una tarea pendiente sin envíos reales.

**Acceptance Scenarios**:

1. **Given** una vinculación registrada, **When** el propietario la revoca, **Then** dejan de autorizarse consultas nuevas desde ese contexto.
2. **Given** un evento repetido o una vinculación vencida, **When** llega al canal, **Then** no duplica efectos ni recupera acceso.
3. **Given** una tarea pendiente, **When** su actor pide cancelarla, **Then** aparece el resultado de cancelación; si un efecto ya ocurrió, se informa sin prometer deshacerlo.

### User Story 4 - Inspeccionar y recuperar ejecuciones (Priority: P2)

Como usuario autorizado quiero ver el estado y la evidencia de mis tareas, reanudar las pausadas y revisar propuestas que requieran aprobación.

**Why this priority**: Hace comprensibles los errores y evita repetir efectos al recuperar trabajos.

**Independent Test**: Interrumpir una ejecución ficticia, reiniciar el servicio, reanudarla y demostrar un solo efecto con la misma identidad de operación.

**Acceptance Scenarios**:

1. **Given** una ejecución interrumpida, **When** se recupera, **Then** conserva su estado y no reenvía automáticamente un efecto de resultado incierto.
2. **Given** una propuesta revisada, **When** cambian sus argumentos, destino o revisión de conexión, **Then** la aprobación anterior deja de servir.
3. **Given** un usuario de otra organización o sin acceso actual, **When** solicita historial o evidencia, **Then** se deniega el acceso.

### User Story 5 - Evaluar respuestas con fuentes autorizadas (Priority: P3)

Como administrador quiero probar preguntas y respuestas con ejemplos ficticios, identificar sus fuentes y detectar regresiones de permisos y calidad.

**Why this priority**: Aporta confianza medible antes de activar proveedores reales.

**Independent Test**: Ejecutar un conjunto de preguntas sintéticas con resultados esperados, incluyendo revocaciones y fuentes inaccesibles.

**Acceptance Scenarios**:

1. **Given** un ejemplo con evidencia permitida, **When** se ejecuta la evaluación, **Then** se muestran resultado esperado, resultado obtenido y fuentes accesibles.
2. **Given** acceso revocado después de una consulta, **When** se recupera su evidencia o una memoria derivada, **Then** no se revela contenido protegido.
3. **Given** pruebas locales, **When** se ejecutan, **Then** no consumen servicios pagos ni publican información.

### User Story 6 - Continuidad y permisos explicables (Priority: P3)

Como usuario quiero reutilizar contexto vigente y eliminarlo; como administrador quiero probar decisiones de permisos sin modificar concesiones reales.

**Why this priority**: Incorpora continuidad conversacional y facilita revisar reglas sin ampliar autoridad accidentalmente.

**Independent Test**: Crear memoria ficticia, vencerla o revocar su fuente; simular permisos y comprobar que no concede acceso.

**Acceptance Scenarios**:

1. **Given** memoria vinculada a un actor y fuentes, **When** expira, se borra o se revoca una fuente, **Then** no se usa en respuestas posteriores.
2. **Given** una prueba de política, **When** se ejecuta, **Then** devuelve una explicación saneada y no altera permisos ni revela recursos ajenos.

### Edge Cases

- Guardados y verificaciones concurrentes; resultado tardío de una revisión reemplazada.
- Credencial inválida, identidad distinta de la esperada, permisos parciales, tiempo agotado, límite de proveedor y destino inaccesible.
- Destinos que redirigen o resuelven a redes privadas; información sensible incluida en errores del proveedor.
- Revocación entre autorización y envío; recuperación de un envío con resultado incierto.
- Eventos fuera de orden, cancelación concurrente con ejecución y expiración durante recuperación.
- Fuente borrada, memoria obsoleta, cambio de esquema o versión de conector incompatible.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: El propietario DEBE poder guardar, verificar, habilitar y revocar conexiones como acciones diferenciadas; guardar no activa tráfico.
- **FR-002**: La verificación DEBE identificar revisión, modo real o simulado, fecha, identidad comprobada y alcance comprobado, sin equiparar autenticación con permiso de recurso.
- **FR-003**: Toda modificación relevante DEBE invalidar verificación y habilitación anteriores; resultados tardíos no pueden rehabilitar revisiones antiguas.
- **FR-004**: Toda consulta DEBE derivar organización y actor de identidad verificada o contexto registrado y comprobar sus permisos actuales sobre operación y recurso.
- **FR-005**: El entorno local DEBE bloquear conexiones externas y pagos incluso al verificar o habilitar; las simulaciones deben ser visibles y nunca habilitar producción.
- **FR-006**: El catálogo DEBE declarar versión, autenticación, operaciones, entradas válidas, permisos y límites; las operaciones desconocidas o incompatibles deben rechazarse.
- **FR-007**: Las credenciales DEBEN permanecer privadas; respuestas, prompts, historial y errores no deben revelarlas ni incluir cuerpos privados innecesarios.
- **FR-008**: El propietario DEBE poder inspeccionar y revocar vinculaciones Telegram y consultar salud saneada; una vinculación no concede permisos empresariales adicionales.
- **FR-009**: Los eventos repetidos DEBEN conservar idempotencia; el actor autorizado puede consultar estado y solicitar cancelación sin garantizar reversión de efectos consumados.
- **FR-010**: El historial DEBE mostrar estados, pausas y evidencia accesible; la recuperación debe conservar decisiones y conciliar efectos inciertos antes de reintentar.
- **FR-011**: Las aprobaciones DEBEN ligarse a la propuesta y revisión exactas; las aprobaciones creativas y de publicación existentes continúan separadas.
- **FR-012**: La evaluación DEBE incluir al menos 24 ejemplos sintéticos: ocho por conector del piloto, con éxito, denegación, revocación y fallos; debe mostrar evidencia y resultado esperado.
- **FR-013**: La recuperación de fuentes y memorias DEBE volver a comprobar permisos actuales; memoria debe requerir consentimiento explícito, limitarse a actor y organización, tener caducidad y permitir eliminación.
- **FR-014**: El simulador de políticas DEBE ser de solo lectura, explicar decisiones con motivos saneados y no alterar concesiones efectivas.
- **FR-015**: La entrega DEBE contar con pruebas de aislamiento, revisión concurrente, revocación, recuperación y bloqueo local; los resultados locales y remotos deben informarse por separado.
- **FR-016**: Un fallo de proveedor DEBE producir una categoría comprensible sin reenviar automáticamente operaciones de resultado incierto ni ocultar permisos parciales.

### Key Entities

- **Conexión gestionada**: perfil privado, organización, versión del conector, revisión y estado de disponibilidad.
- **Verificación**: revisión evaluada, modo, fecha, identidad y alcance comprobados, resultado saneado.
- **Definición de conector**: autenticación, operaciones, límites y versiones compatibles.
- **Vinculación de canal**: contexto registrado, actor, organización, vencimiento y revocación.
- **Ejecución**: actor, revisión de conexión, propuesta, estados, aprobación y evidencia autorizada.
- **Caso de evaluación**: datos ficticios, consulta, resultado esperado y fuentes permitidas.
- **Memoria**: actor, organización, procedencia, vigencia y estado de eliminación.
- **Simulación de permiso**: acción solicitada, decisión y motivos saneados sin autoridad adicional.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: En los tres conectores del piloto, el propietario puede distinguir guardado, simulación, verificación y habilitación sin consultar registros técnicos.
- **SC-002**: El 100% de los casos de acceso cruzado, revocación y revisión obsoleta de la suite se deniega sin revelar información protegida.
- **SC-003**: Las pruebas de reinicio y eventos duplicados producen como máximo un efecto confirmado por identidad de operación; los resultados inciertos quedan pendientes de conciliación.
- **SC-004**: Los 24 ejemplos de evaluación producen su decisión y evidencia esperadas; ningún ejemplo local realiza llamadas externas o pagas.
- **SC-005**: El 100% de las memorias vencidas, eliminadas o con fuente revocada deja de contribuir a respuestas y recuperación.
- **SC-006**: En revisión de escritorio y móvil, se completan las seis historias con datos ficticios y sin exponer credenciales.

## Assumptions

- Se reutilizan sesiones, roles, cifrado, ACL, contexto de canal y persistencia existentes. El piloto inicial de sistemas empresariales sigue limitado a lectura.
- Los aportes de GitHub son patrones y evaluaciones de bibliotecas; no implica instalar todos sus motores o copiar código con licencia restringida.
- La operación real exige un entorno separado habilitado explícitamente, contratos comprobados y credenciales válidas; su ausencia no se oculta con simulaciones.
- Búsqueda documental, transcripción, nuevos canales, publicación real, OAuth genérico y un editor visual completo de flujos quedan fuera de esta entrega. La evaluación utiliza evidencia de capacidades existentes.
- Se incluye una evaluación documentada de Rig y Cedar; su adopción requiere compatibilidad y beneficio demostrados. La memoria será explícita y acotada, no aprendizaje global automático.
