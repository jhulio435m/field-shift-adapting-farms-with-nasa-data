# Feature Specification: Fundamentos del producto Kusi Kusi

**Feature Branch**: `001-product-foundation`

**Created**: 2026-11-05

**Status**: Fundamento corregido el 2026-11-09; autonomía del producto pendiente de aceptación.

**Input**: User description: "Empezar desde cero a definir bien Kusi Kusi usando Spec Kit, partiendo de la idea principal."

## Clarifications

### Session 2026-11-05

- Q: ¿Qué primer caso de uso y tipo de organización debe guiar la versión inicial? → A: Usar como hipótesis de piloto consultas académicas de solo lectura para familias, por WhatsApp y contra un sistema académico de prueba. Se elige por ser una validación acotada de identidad, permisos y exactitud; no confirma un cliente, modelo comercial ni limita el producto genérico.

### Session 2026-11-06

- Q: ¿Qué situación concreta guiará el prototipo de la hackatón? → A: Un egresado de la UNCP pagó S/ 198 por la constancia de expedito para título profesional (PA57002335), interrumpió el proceso por salud y requiere saber cómo retomarlo y qué pasa con ese pago. El TUPA UNCP 2024 es la fuente inicial. Esta elección guía la hackatón y sustituye la hipótesis anterior para este prototipo, sin limitar el producto genérico ni confirmar que el pago sea reembolsable o reutilizable.

### Session 2026-11-09

- Q: ¿Qué significa que Kusi Kusi sea autónomo y adaptable? → A: Descubre recursos y
  capacidades autorizadas de un entorno conectado, interpreta peticiones nuevas, compone
  planes y ejecuta lecturas/análisis dentro de una delegación. Conserva contexto, se adapta
  a cambios y pide aclaración o aprobación cuando corresponda. No se programa cada negocio
  ni pregunta ni se amplían permisos automáticamente.
- Q: ¿Qué demuestran los pilotos anteriores? → A: Solo componentes y casos concretos.
  Autenticación, resúmenes de inscripciones y proyecciones añadidas a mano no prueban
  autonomía del producto. La aceptación transversal está definida en
  [006-adaptive-environment](../006-adaptive-environment/spec.md).

## Frontera y autonomía del producto

Kusi Kusi es una capa autónoma de interacción y orquestación que se adapta a sistemas
existentes bajo autorización. Requiere conexión, identidad y permisos iniciales, pero no
nuevas funciones ni instrucciones específicas por negocio cuando los metadatos y
capacidades disponibles permiten resolver la petición. Un protocolo nuevo puede exigir
un adaptador reutilizable. La falta de significado o acceso se aclara; no se inventa.

Las hipótesis académicas, la hackatón y CONEIMERA son escenarios de validación. No definen
el límite del producto. Los planes y tareas históricos de esta feature describen un piloto
acotado; su finalización no equivale a producto autónomo implementado.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Conectar y descubrir un sistema existente (Priority: P1)

Una organización conecta su sistema y concede un alcance de acceso. Kusi Kusi
identifica recursos, campos, operaciones y eventos utilizables y explica sus límites.
La conexión no exige reescribir el sistema ni programar cada pregunta del negocio.

**Why this priority**: La integración reutilizable y el descubrimiento autorizado
son la base de la adaptación, no una demostración sectorial concreta.

**Independent Test**: Conectar dos entornos compatibles con recursos diferentes y
completar solicitudes equivalentes sin cambios específicos del negocio en el núcleo.

**Acceptance Scenarios**:

1. **Given** una organización y una fuente registrada, **When** se concede acceso,
   **Then** se descubren únicamente recursos y operaciones autorizados, con procedencia.
2. **Given** fuentes con vocabulario y campos distintos, **When** se hace una solicitud
   equivalente, **Then** se usan el mismo proceso de interpretación, planificación y
   validación, sin añadir funciones ni instrucciones por negocio.
3. **Given** información insuficiente o una fuente incompatible, **When** se conecta,
   **Then** se explica qué falta y se pide la aclaración pertinente, sin inventar capacidad.

---

### User Story 2 - Resolver solicitudes mediante adaptación autónoma (Priority: P1)

Una persona conversa con sus sistemas autorizados sin aprender comandos ni esperar
que se programe cada pregunta. Kusi Kusi descubre recursos, interpreta la intención,
compone operaciones reutilizables, valida el plan y responde con evidencias. Retiene
contexto y se adapta cuando cambian el entorno o los permisos.

**Why this priority**: Aceptar lenguaje natural sobre funciones sectoriales fijas no
satisface la autonomía ni la adaptación que definen el producto.

**Independent Test**: Resolver solicitudes equivalentes sobre dos entornos y tres
recursos diferentes, incluido uno incorporado durante la prueba, sin nuevas funciones
ni instrucciones de interpretación específicas del negocio.

**Acceptance Scenarios**:

1. **Given** una identidad autorizada y una fuente conectada, **When** se solicita un resumen,
   **Then** se descubre el recurso pertinente o se aclara su significado, se consulta y se
   responde con procedencia, filtros y límites comprobables.
2. **Given** un histórico suficiente, **When** se solicita una proyección por fecha,
   **Then** se compone lectura y análisis genérico, declarando método y supuestos,
   sin añadir una función que contenga el nombre del negocio.
3. **Given** una conversación previa, **When** se cambia un filtro o se pregunta una
   continuación, **Then** se conserva el contexto correcto y se comprueban otra vez permisos.
4. **Given** metadatos o permisos cambiados, **When** se retoma un plan, **Then** se
   actualiza o invalida antes de acceder y nunca se conservan privilegios revocados.
5. **Given** una operación con efectos, **When** queda fuera de la delegación existente,
   **Then** se pide aprobación válida o se informa la capacidad pendiente; el modelo no
   obtiene privilegios ni ejecuta acciones arbitrarias.

---

### User Story 3 - Administrar acceso y revisar resultados (Priority: P2)

El equipo responsable configura usuarios, canales, integraciones y permisos; revisa qué ocurrió en cada solicitud y puede distinguir una operación real de una simulación o una operación fallida.

**Why this priority**: Las organizaciones necesitan controlar quién accede a sus sistemas y entender el resultado antes de confiar en la automatización.

**Independent Test**: Con varios roles y dos organizaciones de prueba, comprobar la configuración permitida, la separación de datos y el registro de resultados sin exponer credenciales ni contenido innecesario.

**Acceptance Scenarios**:

1. **Given** usuarios con roles distintos, **When** cada uno intenta administrar, aprobar o solo consultar, **Then** Kusi Kusi permite únicamente las operaciones asignadas a su rol.
2. **Given** una solicitud completada, denegada, simulada o fallida, **When** un operador autorizado revisa su estado, **Then** ve el estado correcto y una explicación suficiente para decidir el siguiente paso.
3. **Given** dos organizaciones, **When** una consulta usuarios, integraciones, conversaciones o resultados, **Then** solo obtiene datos pertenecientes a su propio contexto autorizado.

### Edge Cases

- La identidad del remitente no puede verificarse o no corresponde a una identidad registrada.
- La solicitud coincide con más de una capacidad, no cumple el formato esperado o pide datos fuera del permiso concedido.
- El sistema conectado está caído, tarda demasiado, devuelve datos contradictorios o responde con un formato inválido.
- Una aprobación queda obsoleta porque cambió el contenido o la acción después de aprobarse.
- Un canal reintenta el mismo evento, una operación se interrumpe o llega una respuesta tardía; el usuario no debe recibir ejecuciones duplicadas ni un estado falso de éxito.
- Una credencial, un identificador de organización o una instrucción maliciosa aparece en texto libre; no puede reemplazar el contexto de confianza ni ampliar permisos.
- La operación se ejecuta en modo local: debe quedar claro que está simulada y no producir llamadas pagadas, mensajes reales ni publicaciones externas.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Kusi Kusi MUST conectar sistemas existentes mediante contratos y adaptadores que permitan agregar sistemas sin hacer que uno de ellos defina el producto completo.
- **FR-002**: Una organización MUST poder habilitar capacidades específicas de una integración y asignar quién puede consultarlas o ejecutarlas.
- **FR-003**: Kusi Kusi MUST aceptar solicitudes desde canales registrados y asociarlas con una identidad y organización obtenidas de contexto verificado, nunca de un identificador de tenant escrito en la solicitud.
- **FR-004**: Antes de consultar datos o ejecutar una capacidad, Kusi Kusi MUST comprobar identidad, organización, permisos y validez de los argumentos.
- **FR-005**: Kusi Kusi MUST basar las respuestas sobre datos en resultados verificables de los sistemas autorizados y MUST indicar cuando carece de información suficiente.
- **FR-006**: Las operaciones de lectura y las operaciones que cambian datos MUST tener permisos diferenciables. Las operaciones con aprobación MUST requerir aprobación de una persona autorizada antes de ejecutarse.
- **FR-007**: Una aprobación MUST quedar vinculada al contenido y la versión exactos que se aprobaron. Cambiar esos elementos MUST invalidar la aprobación previa.
- **FR-008**: Kusi Kusi MUST identificar solicitudes simuladas, fallidas, pendientes y completadas de forma que el usuario no las confunda entre sí.
- **FR-009**: Reintentos de una misma solicitud MUST poder recuperarse sin duplicar efectos ni mostrar éxito antes de confirmarlo.
- **FR-010**: Operadores autorizados MUST poder revisar el estado, la procedencia funcional y el resultado de las solicitudes necesarias para administrar el servicio.
- **FR-011**: La información de una organización MUST permanecer aislada de otras organizaciones en consultas, configuración, solicitudes y resultados.
- **FR-012**: Kusi Kusi MUST proteger credenciales y datos personales, evitando exponer secretos o cuerpos de mensajes en registros innecesarios o en interfaces sin autorización.
- **FR-013**: El modo local MUST limitarse a loopback y bloquear llamadas pagadas o externas; las simulaciones MUST permanecer separadas de los envíos y publicaciones reales.
- **FR-014**: Los fallos de sistemas conectados MUST comunicarse como fallos o estados pendientes y MUST conservar información suficiente para recuperación autorizada, sin fabricar un resultado exitoso.
- **FR-015**: La experiencia MUST soportar una interfaz de administración en español y solicitudes conversacionales por canales habilitados.
- **FR-016**: Kusi Kusi MUST descubrir recursos, campos, operaciones y eventos accesibles en fuentes registradas, sin ampliar permisos ni imponer un sector como frontera del producto.
- **FR-017**: Kusi Kusi MUST distinguir falta de identidad, permiso, datos, significado o capacidad, y pedir la aclaración mínima o explicar el bloqueo concreto sin negativas genéricas.
- **FR-018**: Kusi Kusi MUST conservar contexto e interpretaciones aprobadas por identidad y organización, revalidarlas ante cambios y componer planes sin funciones específicas por cada negocio o pregunta.
- **FR-019**: Las simulaciones y los pilotos MUST permanecer identificados como tales; su éxito MUST NOT presentarse como aceptación de autonomía del producto.

### Key Entities *(include if feature involves data)*

- **Organización**: espacio de trabajo dueño de sus usuarios, integraciones, políticas y solicitudes.
- **Identidad**: persona o cuenta vinculada a una organización y verificada mediante un mecanismo de confianza.
- **Vínculo familiar**: relación verificada entre una madre, padre o tutor y uno o más estudiantes, con permisos explícitos para consultar sus datos.
- **Sistema conectado**: software externo con el que una organización intercambia información mediante una integración registrada.
- **Capacidad**: operación concreta disponible sobre un sistema, con argumentos, resultados y permisos definidos.
- **Canal**: medio registrado que entrega solicitudes y aporta contexto de identidad verificable.
- **Solicitud**: petición conversacional o administrativa con identidad, contexto autorizado, estado y resultado recuperable.
- **Aprobación**: autorización de una persona habilitada, vinculada a una operación y versión específicas.
- **Resultado**: dato o estado producido por un sistema conectado o por una simulación claramente identificada.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: En la matriz de aceptación del producto, todas las solicitudes permitidas devuelven el resultado correcto del sistema de prueba y todas las solicitudes denegadas quedan sin ejecutar.
- **SC-002**: En la matriz de aislamiento, ningún caso de prueba permite que una organización lea o modifique información de otra.
- **SC-003**: En todos los casos sin una respuesta verificable, Kusi Kusi informa que no tiene información suficiente y no presenta datos inventados como hechos.
- **SC-004**: En el modo local, una revisión de red confirma cero llamadas a proveedores pagados o servicios externos y cero envíos o publicaciones reales.
- **SC-005**: Para cada acción que requiere aprobación, ninguna ejecución ocurre antes de una aprobación válida de la misma versión; toda modificación posterior invalida esa aprobación.
- **SC-006**: En pruebas de interrupción y reintento, cada solicitud termina en un estado correcto y recuperable sin duplicar efectos ni informar éxito prematuro.

- **SC-007**: La aceptación de autonomía MUST cumplir la matriz transversal de la feature
  006 sobre varios entornos, recursos, conversaciones y cambios de permisos; pruebas de
  conexión o pilotos específicos no pueden sustituirla.

## Assumptions

- La idea base es una capa conversacional de integración que conecta canales y servicios especializados con sistemas existentes, sin sustituirlos.
- “Integrarse a cualquier sistema” significa que el producto conserva una vía genérica y extensible para incorporar sistemas; no implica que todas las integraciones comerciales ya vengan construidas.
- La hipótesis previa de consultas académicas familiares por WhatsApp permanece como un escenario histórico de validación. El caso de la hackatón es la titulación interrumpida en la UNCP descrita en `specs/005-brecha-burocratica/spec.md`.
- El comprador, el modelo comercial y el mecanismo concreto de identidad/vínculo familiar no están confirmados; deben resolverse antes de conectar datos reales.
- En el piloto académico histórico, la escuela valida manualmente la relación familiar fuera del sistema y asigna el vínculo a una cuenta de demostración; Kusi Kusi no verifica documentos oficiales ni debe recibir datos reales de menores.
- El trabajo actual define el producto y sus criterios; no activa proveedores, credenciales, envíos, publicación, acceso a producción ni servicios pagados.
