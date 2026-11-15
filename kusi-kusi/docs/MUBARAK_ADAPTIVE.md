# Conexión de Mubarak al Kusi Kusi existente

Se conserva el mismo producto, motor adaptable, identidad registrada, proveedor y canales. No se creó un Kusi Kusi para Mubarak. La cuenta operadora existente recibió dos fuentes adicionales y conserva su fuente Frappe anterior. No se arrancó un nuevo servicio persistente ni se activaron entregas externas.

## Cuentas y conexiones

Vendure tiene dos cuentas dedicadas, separadas de las cuentas humanas de los vendedores, con roles `ReadCatalog` y `ReadOrder` limitados a su canal. No tienen permisos de escritura, administración global ni lectura de datos de otros vendedores. Las credenciales viven solo en archivos privados excluidos de Git.

- Aprovisionamiento idempotente: `npm run setup:kusi`, desde `../Projects/mubarak-vendure/apps/mubarak`.
- Registro aditivo: `python3 scripts/register_mubarak_sources.py`, desde este checkout.
- Runtime existente: `.local-data/adaptive/runtime.json`.
- Conexiones y secretos: `.local-data/adaptive/mubarak/`.
- Respaldo previo al registro: `.local-backups/adaptive-before-mubarak-*.json`.

El registro conserva el bloque `llm` (Groq en el runtime actual), organización, principal, vinculación y fuentes anteriores. No llama al modelo existente ni a los canales externos. Las fuentes se identifican como `mubarak_moda_ana` y `mubarak_calzados_luis`; esos nombres son configuración de conexión, no instrucciones o funciones del intérprete.

El runtime actual corresponde a su operador ya registrado. Para dar acceso a un vendedor por su propio chat se debe verificar y vincular esa identidad y delegar solamente su fuente. No se asignan fuentes por el nombre, teléfono o tenant que alguien escriba en un mensaje. No se reutilizan las credenciales del operador para los vendedores.

## Adaptación

`adaptive_graphql_bridge.py` implementa el protocolo genérico `discover/read`: introspección, descubrimiento de consultas paginadas y campos tipados, autorización con la cuenta actual, lectura acotada y catálogo versionado. No contiene nombres de productos, tiendas, preguntas comerciales ni reglas de frases. Las preguntas siguen pasando por `LanguagePlanner`, `Assistant`, validación y análisis genéricos ya existentes.

La conexión delega inicialmente `products`, `productVariants` y `orders`. Los campos numéricos, fechas, booleanos y enumeraciones se descubren del esquema; solo cadenas expresamente delegadas pueden ser categorías acotadas. Se excluyen identificadores sensibles y texto libre. Los importes `Money` conservan unidades menores de la moneda y sus etiquetas advierten que no representan necesariamente dinero cobrado. Las fechas se expresan en America/Lima.

Autorizar otro recurso compatible significa actualizar el alcance de conexión, no agregar funciones al núcleo. Se comprobó que `stockLocations` aparece sin cambiar código. La introspección sola no concede acceso: se prueban consultas con el usuario vigente, y Vendure aplica sus roles y canal. Cambiar el token de canal al de otro vendedor se rechaza.

El bridge acepta solo HTTP con IP literal 127.0.0.1, rechaza redirecciones y proxies, limita respuestas/plazo/paginación y genera las consultas a partir del esquema. El modelo no emite GraphQL ni SQL. No ejecuta mutaciones de negocio; login/logout son únicamente autenticación. No anuncia eventos que no estén implementados.

`adaptive_source_bridge.py` permite que el runtime existente use protocolos distintos sin perder su bridge Frappe. La composición optativa del worker local carga un perfil privado ligado a organización, usuario, canal y hash de remitente verificados. Solo admite fuentes GraphQL loopback y un modelo HTTP loopback sin claves de pago. No afloja los bloqueos globales de llamadas externas.

## Verificación

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace`: pasan.
- Pruebas Python de bridges Frappe, GraphQL y dispatcher: siete pasan.
- Descubrimiento real de los dos vendedores, recurso nuevo sin funciones y sustitución de canal rechazada: comprobados.
- `python3 scripts/verify_mubarak_adaptive.py`: pasa con dos vendedores y webhooks firmados locales de WhatsApp y Telegram; usa planes fixture para comprobar el motor, las lecturas reales, la persistencia y la deduplicación. Las respuestas quedan `queued` en outbox; no se llama a Meta ni Telegram.
- Informe privado: `.local-data/mubarak-adaptive/channel-acceptance.json`.

La prueba con Ollama/qwen2.5-coder:3b no terminó dentro del plazo; no se cuenta como aceptación de lenguaje natural. El proveedor del runtime existente se conservó y no se invocó. Los planes fixture NO prueban comprensión de preguntas nuevas ni equivalen a un chatbot desplegado.

## Límites actuales

El motor adaptable actual permite consulta, conteo, agrupación, sumas, medias y análisis temporales sobre recursos autorizados. No se habilitaron cambios de stock/precio, despacho, cobros, reembolsos ni acciones administrativas por chat. Exponer un esquema no vuelve ejecutables todas sus mutaciones: estas requerirían el contrato genérico de acciones, delegación y aprobación correspondiente.

WhatsApp/Telegram reales requieren las vinculaciones verificadas de los vendedores, credenciales oficiales y un callback autorizado. No se configuró ni publicó ninguno. No se instaló Chatwoot/Botpress ni se compró una licencia.
