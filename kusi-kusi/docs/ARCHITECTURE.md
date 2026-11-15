# Arquitectura

Monolito modular Rust en Cargo Workspace. `domain` contiene invariantes, estados e interpretación determinista, sin SQLx/Axum. `application` contiene puertos, contratos, preparación y autorización de capacidades y propuestas. `infrastructure` implementa PostgreSQL, HTTP restringido, IA, transcripción, proveedores creativos, OAuth y el espacio local. `apps/server` compone estos adaptadores y traduce HTTP. Next.js consume la API documentada sin claves administrativas.

El seguimiento de pagos usa el puerto `StudentPaymentsPlatform`. `LocalStudentPaymentsPlatform` entrega datos ficticios sin red; la interfaz `/tramites` consume la misma aplicación de consulta sin pedir login y solo está disponible con servidor loopback en modo local. El webhook autenticado puede consultar códigos a través del mismo puerto. La futura plataforma del hackatón tendrá un adaptador propio; la identidad de estudiante debe provenir de una sesión o vínculo verificado. OCR recibe bytes de imagen en memoria, devuelve solo una referencia coincidente y no persiste el documento ni el OCR completo.

Dependencias: servidor → aplicación/dominio y adaptadores; infraestructura → aplicación/dominio; aplicación → dominio. SQL permanece en infraestructura. Los servicios locales de persistencia y orquestación se agrupan en `infrastructure::local` como entrega vertical; pueden extraerse detrás de puertos más específicos cuando exista un segundo adaptador. No se crearon microservicios ni paquetes vacíos.

## Identidad y límites

La organización proviene de una clave verificada, sesión persistida o canal registrado. Nunca del prompt ni de un tenant enviado libremente. Las sesiones locales son opacas, duran ocho horas, se guardan como hashes y usan cookies HttpOnly/SameSite Strict. La autorización considera rol y relación con la conversación. El middleware aplica RBAC también si se presenta una sesión como Bearer; elimina siempre el encabezado de confianza del cliente. Solo la composición HTTP puede generar el contexto de operador interno.

El modo local exige bind y origen HTTP loopback; el origen del frontend puede usar otro puerto y se valida en cada petición. No es una configuración de producción ni reemplaza OIDC. Los canales firmados reciben eventos de forma independiente; un código de un uso vincula una identidad local, ligado al canal y organización. Los envíos externos se retienen; no se confunde una identidad de mensajería con autorización.

## Conversaciones y trabajos

PostgreSQL mantiene inbox, deduplicación por canal/evento, agrupación de tres segundos, UUID lógico estable, leases, intentos limitados y contexto de 24 horas. Un bloqueo por conversación evita procesadores simultáneos. Reintentar el mismo evento no crea otra acción; cambiar su contenido produce conflicto. Las acciones creativas y publicaciones reciben claves idempotentes estables. La respuesta, el estado de procesamiento y el outbox se confirman en una transacción. Caídas antes de confirmar recuperan la misma acción lógica.

El proveedor creativo conserva propuestas inmutables y digest de aprobación; las solicitudes externas ambiguas quedan inciertas, sin repetirse automáticamente. El modo local permite solo proveedores simulados. Un archivador separado usa FFmpeg con argumentos fijos, protocolo file/pipe y tiempo acotado; genera MP4/PNG rotulado y copia persistente. No desarrolla un modelo generativo.

Las publicaciones locales conservan snapshot, versión, digest, privacidad y programación; editar invalida aprobación. El trabajador verifica que el aprobador siga habilitado, y confirma un recibo simulado en una transacción. Nunca llama a una plataforma social.

## Integración universal

Las capacidades tienen esquema de entrada/salida, permiso, versión y destino registrado. `ExecuteCapability` autoriza y valida antes y después del adaptador. El cliente HTTPS limita DNS/destinos, deshabilita redirects/proxy y bloquea redes privadas. Cada sistema mantiene su lenguaje; solo implementa el contrato de la operación. No existe SQL libre generado por IA. En modo local la ejecución HTTP empresarial está bloqueada; el asistente usa exclusivamente reglas y operaciones locales.

Los proveedores de IA/transcripción implementan puertos independientes. Los adaptadores OAuth usan endpoints oficiales, estado de un uso, cookie de vinculación y cifrado autenticado por organización/cuenta. El inicio administrativo OIDC usa repositorios y sesiones propios, no las cuentas locales; la interfaz muestra membresías provisionadas y no permite autoprovisionar roles. Sus rutas se montan en el servidor, pero rechazan el modo local antes de iniciar protocolo. Ver SECURITY.md y DEVELOPMENT.md junto con AI_PROVIDERS.md, INTEGRATIONS.md, CREATIVE_JOBS.md y OAUTH.md.


## Integraciones gestionadas (010)

`domain::managed_integrations` expresa vigencia, revisión y estados sin frameworks. Application define puertos/casos de conexión, manifiestos, canal, revisión, evaluación y memoria. Infrastructure implementa estado PostgreSQL, cifrado, probes y resolución; HTTP vive en `apps/server/src/local`, y Next muestra configuración y operación sin secretos.

Los perfiles guardados habilitan pruebas fixture, nunca egress local. `Sources::with_managed_fixture` se compone dentro de `AuthorizedEnvironment`: versión y vigencia del perfil se vuelven a comprobar al descubrir/leer; ACL de 007–008 continúa siendo autoridad por recurso. La composición privada del asistente solo admite manifest y planificador HTTP en IP loopback literal, sin API key, para esta modalidad. La nueva lectura tipada HTTP aplica manifiesto y ACL antes de leer y antes de devolver evidencia.

El transporte de probe provider es una pieza separada, sin ruta de activación local: allowlist exacta, todos los resultados DNS públicos, pinning para conexión, proxies/redirecciones desactivados, plazo total de diez segundos y respuesta acotada. Sus parsers no convierten identidad en permisos de recurso. Su funcionamiento contra destinos reales permanece sin acreditar.

La outbox conserva identidad, revisión e historial de estados en PostgreSQL; los metadatos de revisión no contienen cuerpos. La memoria tiene consentimiento y TTL por actor/organización, cifrado ligado por AAD a ambos y al ID, y procedencia comprobada contra la fuente actual. Bloqueos compartidos de consentimiento/concesión durante creación serializan retirada de consentimiento y revocación. Una revocación borra payload para impedir resurrección después de volver a conceder acceso.
