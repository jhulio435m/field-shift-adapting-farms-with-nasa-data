# Feature Specification: Autonomía y adaptación al entorno

**Feature Branch**: No se crea una rama; continuidad de la feature 006 existente.

**Created**: 2026-11-09

**Status**: Especificación corregida; implementación y aceptación pendientes.

**Input**: User description: "Kusi Kusi debe ser autónomo y adaptable a su entorno; no programar funciones por cada negocio o pregunta. Corregir lo definido con Spec Kit."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Conectar y comprender un entorno autorizado (Priority: P1)

Un responsable conecta un sistema existente y concede acceso a recursos y operaciones.
Kusi Kusi descubre qué puede consultar y con qué límites, explica qué está disponible
y detecta lo que falta para resolver solicitudes. La organización configura conexión,
identidad y permisos; no desarrolla un catálogo de preguntas ni programa cada negocio.

**Why this priority**: Sin descubrimiento reutilizable, cada integración seguiría siendo
una aplicación a medida aunque su interfaz aceptara texto libre.

**Independent Test**: Conectar dos entornos con recursos distintos y conceder sus accesos.
Comprobar que recursos y campos autorizados aparecen sin cambios específicos del negocio.

**Acceptance Scenarios**:

1. **Given** una fuente compatible y credenciales autorizadas, **When** se conecta,
   **Then** se descubren recursos, tipos de datos y operaciones accesibles con su procedencia.
2. **Given** dos negocios con nombres, campos y vocabulario distintos, **When** se conectan,
   **Then** ambos usan el mismo proceso de descubrimiento y resolución, sin funciones,
   instrucciones del intérprete ni reglas de código añadidas para cada negocio.
3. **Given** metadatos incompletos o un significado ambiguo, **When** hacen falta para decidir,
   **Then** se solicita la aclaración mínima y se conserva su interpretación aprobada por
   organización y versión; no se inventa ni exige una plantilla sectorial completa.
4. **Given** una fuente incompatible o sin metadatos suficientes, **When** se intenta conectar,
   **Then** se informa la limitación y el contrato que falta; no se afirma compatibilidad ni
   se exploran destinos ajenos a la conexión autorizada.

---

### User Story 2 - Resolver peticiones nuevas con capacidades reutilizables (Priority: P1)

Una persona escribe lo que necesita, sin menú: consultar, comparar, resumir o proyectar.
Kusi Kusi elige fuentes y operaciones disponibles, prepara un plan, comprueba permisos,
lo ejecuta y responde con evidencias. Una pregunta nueva no exige una nueva función
si puede resolverse combinando capacidades existentes.

**Why this priority**: La adaptación debe producir resultados útiles, no solo un inventario
ni una clasificación fija de frases del piloto.

**Independent Test**: Ejecutar preguntas equivalentes sobre inscripciones, pedidos y un
tercer recurso incorporado durante la prueba; comprobar resultados sin cambiar el núcleo.

**Acceptance Scenarios**:

1. **Given** datos y permisos suficientes, **When** se pide «resume cómo vamos», **Then** se
   identifica el recurso pertinente o se aclara cuál, se consultan datos actuales y se
   muestran el alcance, los resultados y su procedencia.
2. **Given** un histórico autorizado, **When** se pide una proyección a una fecha futura,
   **Then** se compone lectura y análisis reutilizable, se comprueba la suficiencia del
   histórico y se explican método, periodo y supuestos. No se rechaza por faltar una
   función que contenga el nombre del negocio ni se presenta una estimación como certeza.
3. **Given** una respuesta anterior, **When** se pregunta «¿y solo los VIP?» o una variante
   equivalente en otro negocio, **Then** se conserva el recurso y periodo anteriores salvo
   modificación explícita, se valida el nuevo filtro y se comprueban otra vez los permisos.
4. **Given** dos interpretaciones plausibles, **When** se plantea la petición, **Then** se
   hace una pregunta de desambiguación y se continúa al responder, sin imponer comandos.
5. **Given** una capacidad realmente ausente, **When** el plan la necesita, **Then** se explica
   qué parte puede resolverse, qué capacidad falta y qué requiere habilitarla; no se fabrica
   el resultado ni se sustituye la tarea por una negativa genérica.

---

### User Story 3 - Adaptarse a cambios y actuar bajo delegación (Priority: P1)

El entorno cambia: aparecen recursos, se renombran campos o se revocan permisos.
Kusi Kusi actualiza su comprensión, invalida planes obsoletos y continúa cuando existe
una alternativa autorizada. Automatiza lecturas y análisis dentro de la delegación,
con límites de consumo y sin pedir confirmación redundante para cada consulta permitida.

**Why this priority**: Un sistema adaptable no debe depender de una fotografía manual
permanente del entorno ni conservar privilegios que ya no existen.

**Independent Test**: Cambiar recursos y permisos durante una conversación y repetir
peticiones; comprobar adaptación o aclaración sin cambios de código por recurso.

**Acceptance Scenarios**:

1. **Given** un recurso nuevo accesible en una conexión compatible, **When** se redescubre
   el entorno, **Then** puede utilizarse sin nuevas funciones ni reinicio obligatorio del
   núcleo; si existe una sincronización pendiente, se informa su estado.
2. **Given** un campo cambiado o eliminado, **When** se retoma un plan anterior, **Then** se
   invalida o replantea antes de consultar; no se reutiliza a ciegas la interpretación vieja.
3. **Given** un permiso revocado, **When** se retoma una conversación o automatización,
   **Then** deja de acceder aunque exista memoria, un plan previo o una respuesta almacenada.
4. **Given** una operación con efectos o fuera de los límites delegados, **When** el modelo
   la propone, **Then** no se ejecuta automáticamente: se obtiene la aprobación exigida o
   se informa que la capacidad de ejecución todavía no está disponible.
5. **Given** un límite de coste, tiempo o llamadas, **When** un plan lo excede, **Then** se
   detiene o propone una alternativa acotada; no amplía el presupuesto por sí mismo.

---

### User Story 4 - Suscribirse a eventos con lenguaje natural (Priority: P2)

Una persona autorizada pide «avísame cuando ocurra X». Kusi Kusi reconoce los eventos
que expone la fuente, propone el alcance y establece una suscripción autorizada,
independiente de una conversación activa y sin nuevas funciones por cada tipo de negocio.

**Why this priority**: Los avisos deben surgir del entorno conectado y de una delegación
clara, no de temporizadores presentados como eventos ni de condiciones sectoriales fijas.

**Independent Test**: Suscribirse a creaciones de dos recursos diferentes, entregar cada
evento dos veces y comprobar un solo aviso al destinatario autorizado; cancelar y repetir.

**Acceptance Scenarios**:

1. **Given** eventos disponibles y autorización de suscripción, **When** se pide un aviso,
   **Then** quedan definidos fuente, evento, condición, destinatario y datos permitidos.
2. **Given** una suscripción activa, **When** llega un evento autenticado, **Then** se comprueba
   la autorización vigente, se aplica la condición y se entrega o registra el estado del aviso.
3. **Given** una fuente sin eventos disponibles, **When** se pide un aviso inmediato, **Then**
   se informa esa limitación; cualquier alternativa periódica requiere acuerdo y no se
   presenta como webhook.
4. **Given** una suscripción cancelada o un destinatario desvinculado, **When** llega un evento,
   **Then** no se envía el aviso. Reentregas o recuperaciones no duplican efectos conocidos.

### Edge Cases

- Ausencia de datos, histórico demasiado corto, periodos incompletos, datos truncados o
  muestras sesgadas: informar el alcance y no presentar cifras como completas.
- Dos fuentes llaman «venta» a hechos diferentes; «confirmado» no demuestra un pago:
  aclarar la semántica y conservar procedencia sin equiparar conceptos automáticamente.
- Instrucciones maliciosas en mensajes, descripciones, registros o metadatos descubiertos:
  tratarlas como datos no confiables, sin cambiar permisos, herramientas ni políticas.
- Un usuario cambia de organización, pierde acceso o retoma contexto de otra sesión:
  separar el contexto y comprobar de nuevo la identidad autorizada.
- Un evento llega antes de activar una suscripción, fuera de orden o durante una caída:
  respetar el punto de inicio y registrar pendientes, fallos o resultados inciertos.
- Una fuente solo expone parte de sus recursos o el proveedor de interpretación falla:
  comunicar el límite, sin afirmar que no existen datos ni inventar respuestas.
- El entorno local está en simulación: no convertir planes o suscripciones en llamadas
  externas, cargos o mensajes reales.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: El sistema MUST obtener identidad y organización de un contexto verificado,
  nunca de texto del usuario, metadatos externos o propuestas del modelo.
- **FR-002**: El sistema MUST descubrir recursos, campos, tipos, operaciones, restricciones
  y eventos de fuentes registradas dentro del acceso concedido, sin otorgar permisos nuevos.
- **FR-003**: La configuración inicial MUST limitarse a conexión, credenciales, delegación
  y políticas; un recurso o pregunta compatible nueva MUST NOT exigir funciones, ramas
  de código o instrucciones de interpretación específicas para cada negocio.
- **FR-004**: El sistema MUST conservar un catálogo con procedencia, versión, vigencia y
  alcance autorizado, y actualizarlo ante cambios o antes de reutilizar información obsoleta.
- **FR-005**: El sistema MUST interpretar lenguaje natural usando el catálogo y el contexto
  de la conversación, sin depender de comandos ni de una lista de frases sectoriales.
- **FR-006**: El sistema MUST componer un plan verificable con operaciones reutilizables,
  fuentes, recursos, argumentos, dependencias, límites y aprobaciones necesarias.
- **FR-007**: El sistema MUST validar permisos vigentes, campos, tipos, alcance, vigencia
  del catálogo y límites antes de cada lectura o acción; el modelo MUST NOT ejecutar por
  sí mismo consultas o comandos arbitrarios ni ampliar la autorización.
- **FR-008**: Las lecturas y análisis dentro de una delegación vigente MUST poder ejecutarse
  autónomamente; efectos fuera de esa delegación MUST requerir aprobación válida o quedar
  expresamente sin ejecutar. La autonomía MUST NOT significar privilegios ilimitados.
- **FR-009**: Aclaraciones de significado MUST quedar acotadas al usuario u organización
  pertinente, conservar su procedencia y poder corregirse o invalidarse; no son permisos.
- **FR-010**: Las respuestas MUST incluir datos verificables, filtros, periodos y procedencia;
  análisis y proyecciones MUST declarar método, suficiencia y supuestos sin inventar certeza.
- **FR-011**: El contexto MUST conservar referencias y decisiones útiles entre turnos,
  aislar organizaciones y usuarios, permitir reinicio/borrado y revalidarse tras revocaciones.
- **FR-012**: Ante ambigüedad o capacidad ausente, el sistema MUST distinguir falta de datos,
  permiso, conexión, significado u operación; MUST pedir la aclaración mínima o explicar
  el bloqueo específico, sin ocultarlo tras una negativa genérica.
- **FR-013**: El sistema MUST detectar cambios relevantes del entorno, invalidar planes
  incompatibles y replanificar únicamente dentro de capacidades autorizadas.
- **FR-014**: Las suscripciones MUST vincular fuente, evento, condición, alcance, destinatario,
  inicio y autorización; MUST poder cancelarse sin desarrollar un manejador por cada negocio.
- **FR-015**: Los avisos por eventos MUST originarse en eventos autenticados y deduplicados;
  cualquier consulta periódica alternativa MUST identificarse como tal y acordarse aparte.
- **FR-016**: Solicitudes, planes y suscripciones MUST conservar estados recuperables y
  resultados inciertos explícitos; reintentos MUST NOT repetir efectos ya confirmados.
- **FR-017**: El sistema MUST proteger secretos y datos personales, minimizar datos enviados
  al intérprete y auditar decisiones y procedencia sin registrar cuerpos innecesarios.
- **FR-018**: El sistema MUST imponer límites de coste, tiempo, llamadas y volumen; MUST NOT
  instalar integraciones, explorar destinos nuevos ni cambiar políticas por decisión del modelo.
- **FR-019**: El modo local MUST conservar aislamiento y simulación; conectar proveedores,
  publicar o enviar mensajes reales MUST requerir el contexto externo explícitamente autorizado.
- **FR-020**: La aceptación MUST incluir recursos y vocabulario no codificados para el piloto;
  pruebas de salud, autenticación o demostraciones específicas MUST NOT declararse como prueba
  suficiente de autonomía o funcionalidad completa del producto.

### Key Entities *(include if feature involves data)*

- **Entorno conectado**: fuente registrada con identidad de servicio, alcance y límites.
- **Catálogo autorizado**: descripción versionada de recursos, campos, operaciones y eventos.
- **Delegación**: permisos y límites vigentes de una persona u organización para actuar.
- **Contexto conversacional**: referencias y decisiones acotadas a una identidad verificada.
- **Interpretación semántica**: significado aclarado de un concepto y su vigencia en una fuente.
- **Plan**: composición de operaciones, argumentos, dependencias, límites y aprobaciones.
- **Evidencia**: resultado con procedencia, filtros, periodo, completitud y supuestos.
- **Suscripción**: acuerdo autorizado para recibir eventos filtrados en un destino concreto.
- **Estado de ejecución**: situación de un plan o entrega, incluida recuperación e incertidumbre.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: En dos entornos y tres recursos con nombres y campos distintos, 20 solicitudes
  equivalentes y 6 formulaciones no incluidas en la preparación resuelven el objetivo o piden
  la aclaración esperada, sin cambios de código ni instrucciones específicas por negocio.
- **SC-002**: En esas pruebas se completan al menos conteo, resumen agrupado, comparación
  temporal y proyección; cada resultado coincide con la referencia o con el cálculo
  reproducible declarado, y el 100 % identifica procedencia y límites.
- **SC-003**: Seis conversaciones de al menos tres turnos conservan el recurso y filtros
  correctos; ninguna mezcla usuarios u organizaciones y todas comprueban permisos vigentes.
- **SC-004**: Al añadir un recurso, cambiar un campo y revocar un permiso, las tres pruebas
  terminan respectivamente en descubrimiento utilizable, aclaración/replanificación y bloqueo,
  sin un cambio de código por recurso ni acceso posterior a la revocación.
- **SC-005**: Diez solicitudes con permisos insuficientes, campos excluidos o instrucciones
  maliciosas no acceden a datos fuera del alcance ni producen efectos no autorizados.
- **SC-006**: Dos suscripciones sobre recursos distintos entregan un aviso por cada evento
  confirmado pese a dos reentregas; tras cancelarlas o desvincular el destino, entregan cero.
- **SC-007**: En cinco pruebas de caída o fallo ambiguo, ningún resultado se presenta como
  éxito sin confirmación y se conserva un estado que permita revisar o recuperar la solicitud.
- **SC-008**: Antes de declarar esta feature aceptada, todas las pruebas anteriores tienen
  evidencia identificable, se distingue entorno simulado de real y no quedan criterios
  reemplazados únicamente por pruebas de conexión o por un caso sectorial.

## Assumptions

- El objetivo del producto es autonomía adaptable bajo delegación, no acceso ilimitado ni
  compatibilidad automática con sistemas sin interfaz utilizable o sin permisos.
- Hace falta configurar y autorizar la conexión inicial. Un protocolo nuevo puede requerir
  un adaptador reutilizable; un negocio o recurso nuevo del mismo protocolo no debe exigirlo.
- Los ejemplos de inscripciones, pedidos y entradas ilustran la aceptación; no forman parte
  de las reglas del núcleo ni autorizan datos reales de esos sectores.
- Esta primera entrega cubre descubrimiento, lecturas/análisis, contexto y suscripciones
  aprobadas. Preparar un plan con efectos no significa que ya esté implementada su ejecución;
  las escrituras seguirán un alcance y aprobación explícitos cuando se desarrollen.
- Los datos personales requieren políticas y permisos específicos; disponer de acceso técnico
  no autoriza automáticamente mostrarlos ni enviarlos al proveedor de interpretación.
- Los scripts y demostraciones actuales siguen siendo evidencia de componentes concretos;
  no satisfacen por sí solos esta especificación y no se marcan aquí como producto autónomo.
- La especificación define lo que debe construirse. No activa conexiones, cobros, permisos,
  servicios, publicación, suscripciones o despliegues adicionales.
