# Espacio local de Kusi Kusi

Entrega utilizable sin proveedores externos. Es un entorno de desarrollo en loopback: no sustituye el despliegue ni la seguridad de producción. No emplea datos académicos ni exige un ERP concreto.

## Arranque

Requisitos: Rust estable, Node >=22.12, npm, Docker Compose, FFmpeg/ffprobe con libx264/drawtext, Tesseract con idioma español, fuente DejaVu Sans y cliente PostgreSQL (psql, pg_dump, pg_restore). Para pruebas de aceptación: Python con `cryptography`; para revisión visual: Playwright, Chromium y Pillow.

1. Copia `.env.example` a `.env`, configura secretos aleatorios y DATABASE_URL.
2. `docker compose up -d postgres`.
3. `./scripts/run_local.sh`: compila React y Rust, aplica migraciones, inicia API/panel y los trabajadores simulados.
4. Aprovisiona una organización con `POST /v1/organizations`, encabezado `x-admin-key` y JSON `name`/`slug`. Guarda la clave retornada en `.env` como `DEMO_ORGANIZATION_API_KEY`; no se vuelve a mostrar desde el servidor.
5. `python3 scripts/setup_local.py`. Crea solo el primer propietario y guarda la cuenta en `.local-data/login.json` (0600). No restablece usuarios ya existentes.
6. Abre http://127.0.0.1:3000 e inicia sesión con esa cuenta.

El script es reutilizable: no borra base de datos, archivos ni credenciales existentes. No ejecutes `docker compose down -v` para reiniciar: elimina el volumen.

## Recorrido

- Archivos → carga PNG/JPG/WebP o MP4/WebM de hasta 10 MiB. Imágenes: dimensiones máximas/decodificación limitada, reducción a 2400 px y PNG sin metadatos; videos: ffprobe, máximo 120 segundos y 3840×2160. Los archivos se guardan con UUID interno, hash y organización.
- Conversaciones → crea una solicitud y selecciona el archivo. Pide «Crea un video» o «Haz un diseño». Se agrupan mensajes durante tres segundos.
- Trabajos creativos → revisa snapshot, digest y costo; confirma la versión con un propietario/aprobador. El editor no puede aprobar mediante UI ni directamente por API.
- El trabajador archiva un MP4 de tres segundos o PNG con rótulo de simulación; lo notifica en la conversación. No crea nuevos ángulos del producto ni llama a modelos generativos.
- Continúa «Prepara para Instagram | texto elegido» o «Y para TikTok». Se conserva el archivo del contexto durante 24 horas.
- Publicaciones → revisa archivo, texto, destino, privacidad y horario. Editar crea otra versión y exige nueva aprobación. Una segunda confirmación autorizada habilita exclusivamente la publicación simulada.
- El calendario y el recibo `simulated:<uuid>` permiten revisar el proceso. Ningún contenido se envía a redes sociales.

También se pueden crear publicaciones manuales, administrar usuarios/organizaciones, generar códigos de vinculación para canales registrados y consultar operaciones. El panel muestra perfiles configurados sin secretos. Las capacidades empresariales se registran a través de los contratos existentes; el asistente local no las ejecuta ni inventa respuestas.

### Pagos y trazabilidad · demostración de hackatón

`http://127.0.0.1:3000/tramites` abre una pantalla local sin inicio de sesión. Muestra dos pagos sintéticos que sirven como clon de la plataforma estudiantil en desarrollo: uno de S/ 198 por PA57002335 y otro de S/ 3 para probar un segundo formato de recibo. Para simular la consulta de historial por DNI, usa solamente el valor ficticio `70000001`; no ingreses DNI reales. También se puede consultar por código o subir un comprobante de muestra; Tesseract extrae localmente una referencia, la contrasta con el clon y devuelve solo la referencia enmascarada y el registro encontrado. La foto y el texto OCR completo no se almacenan ni se muestran. El archivo `web/admin/public/demo/recibo-sintetico.png` es ficticio y no acredita un pago.

En el webhook WhatsApp se puede enviar `mis pagos` o un código sintético, por ejemplo `UNCP-EXP-004821`. La respuesta usa el mismo adaptador del clon y se etiqueta como simulada. Un evento de foto se recibe, pero actualmente no descarga el archivo multimedia de Meta; la validación OCR de la demo web muestra ese tramo de forma local. Para validar fotos enviadas por WhatsApp hace falta configurar descarga segura de medios en WhatsApp Cloud API.

La plataforma de pagos que el equipo desarrollará aún no está disponible. `StudentPaymentsPlatform` define el puerto de aplicación; `LocalStudentPaymentsPlatform` es el adaptador de prueba sin red. No se conecta a ADESA. Al existir la plataforma real, se reemplaza el adaptador por el contrato y credenciales de prueba que entregue el equipo.

### Demostración académica ficticia

En **Vínculos académicos**, el propietario puede asignar uno de los dos estudiantes ficticios a un usuario habilitado con rol Lector. Antes debe confirmar que verificó la relación fuera del sistema; esta casilla documenta el supuesto de la demo y no valida identidad real. Después puede emitir un código de canal de un solo uso, válido por diez minutos. El código se almacena como hash, se muestra una vez y el panel lo elimina al vencer.

El trabajador local reconoce únicamente frases exactas de consulta académica y devuelve calificaciones fijas del estudiante ficticio vinculado a esa cuenta. La respuesta queda etiquetada como simulada; el evento webhook firmado puede probarse en loopback. No se aceptan nombres, documentos ni datos académicos reales, no se conecta un sistema escolar y no se envían mensajes al proveedor. Los eventos de salida permanecen en la cola local.

### Autorización genérica por recurso

En **Accesos a recursos**, el propietario puede conceder y revocar a una cuenta Lector el acceso a uno de dos recursos ficticios. Cada lectura comprueba en PostgreSQL la organización y la cuenta de la sesión, el rol activo y la concesión del recurso antes de consultar el fixture en memoria. La respuesta se marca como simulada. La actividad registra la decisión y un hash del recurso, sin argumentos ni resultado. Esta prueba local no conecta un sistema de cliente ni acredita identidad externa; OIDC/SSO y ACL de proveedores quedan pendientes.

## Identidad y permisos

Sesiones opacas, hashes en PostgreSQL, ocho horas, cookie HttpOnly/SameSite Strict. CSRF entregado al cliente autenticado, almacenado solo en memoria y exigido en mutaciones; origen fijo. RBAC se verifica en cada petición incluso si una sesión se presenta como Bearer. Deshabilitar usuario o restablecer contraseña revoca sesiones.

Propietario administra; editor prepara; aprobador confirma/cancela; lector consulta. Las conversaciones se limitan a su usuario o al propietario. Usuarios y recursos se buscan dentro de la organización autenticada. No se acepta tenant desde prompts. La clave administrativa queda reservada al aprovisionamiento y APIs de servicio: no se entrega al navegador. Los códigos de vinculación son de un uso, diez minutos y restringidos a organización/canal.

En **Sesiones**, cada cuenta puede revisar sus sesiones activas y revocar las anteriores. El propietario puede revisar y cerrar sesiones de cuentas de su organización. La sesión actual se marca y se cierra con **Cerrar sesión**; no se puede revocar desde el inventario. Restablecer contraseña o deshabilitar una cuenta revoca sus sesiones y credenciales ligadas. No se recopilan IP, User-Agent ni huellas del dispositivo.

## Recuperación y retención

Inbox persistente, evento único, agrupación lógica estable y leases permiten reanudar tras un reinicio. Cada grupo conserva claves idempotentes para sus acciones. La respuesta y su outbox se confirman con el evento procesado. Tres intentos fallidos dejan un código de error visible en Operación; los envíos a canales externos permanecen retenidos.

Retención: el propietario puede previsualizar y confirmar limpieza de mensajes antiguos de su organización. Se conservan IDs de eventos para deduplicación; se elimina contenido de mensajes/contexto según antigüedad. No elimina automáticamente archivos o registros de aprobación.

## Respaldo verificado

`python3 scripts/local_backup.py --verify-restore`

El comando solo permite PostgreSQL loopback. Hace pg_dump, restaura a una base temporal con nombre aleatorio, lee el catálogo de archivos del dump, copia solo los archivos referenciados y valida sus SHA-256. Guarda dump, assets y manifest en `.local-backups/` con permisos privados; elimina la base temporal. Si falta un archivo o cambia durante el respaldo, falla sin declarar éxito. Una eliminación concurrente puede requerir repetirlo. Conserva `.env` y el keyring OAuth por un canal seguro aparte: no se incluyen en el respaldo.

El dump contiene hashes de usuarios y puede contener tokens OAuth cifrados: debe protegerse como información sensible. La recuperación real requiere restaurar el dump en una base vacía y copiar `assets/` a KUSI_LOCAL_STORAGE/assets, manteniendo el keyring externo original. No restaurar sobre una base activa.

## Pruebas

- `bash scripts/verify.sh`: fmt, clippy con warnings como errores, pruebas Rust/PostgreSQL y build.
- `npm ci --prefix web/admin && npm run build --prefix web/admin`.
- `python3 scripts/verify_local.py`: esquema temporal y almacenamiento aislado; roles, tenants, pagos demo sin login, OCR local del recibo sintético, lookup por WhatsApp, idempotencia, debounce, recuperación, FFmpeg y flujos locales. No modifica los datos del panel.
- `python3 scripts/verify_payment_demo_browser.py`: Chromium escritorio/móvil contra servicios y esquema temporales; historial sin login, DNI ficticio, estados de error, consulta por código y OCR de imagen sintética.
- `python3 scripts/verify_payment_demo_browser.py --serve`: levanta esa misma demo aislada en `http://127.0.0.1:18088/tramites`; Ctrl+C limpia el esquema temporal.
- `python3 scripts/verify_browser.py`: recorrido por Chromium contra el panel local, agrega únicamente datos ficticios rotulados y deja capturas en `.local-data/qa/`. Requiere la cuenta creada por setup_local.py.
- `docker compose build api && python3 scripts/verify_container.py`: build release y arranque temporal del modo API en otro puerto; no reemplaza el panel local.
- Scripts verify_oauth.py / verify_creative.py / verify_channels.py / verify_ai.py cubren adaptadores con fixtures. No prueban cuentas ni servicios reales.

## Límites

El inicio OIDC del plano administrativo se desarrolla por separado y permanece desactivado en este modo. No hay publicación nativa, envío automático del outbox externo, descarga/transcripción de audios de chats, modelo de conversación LLM conectado, mapeo de identidades de canales externos y ACL por recurso de sistemas de clientes, RLS, alertas externas ni evaluación independiente de seguridad. El slice local de permisos por recursos ficticios no valida esos controles de producción. Los adaptadores ya existentes para servicios externos son otro modo operativo, desactivado aquí. No activar proveedores reales ni cambiar a bind público como si este entorno fuera producción.
