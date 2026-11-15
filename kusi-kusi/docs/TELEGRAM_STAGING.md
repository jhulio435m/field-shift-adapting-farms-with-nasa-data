> Actualización 006: la consulta activa usa el catálogo y el ejecutor genéricos
> descritos en [ADAPTIVE_RUNTIME.md](ADAPTIVE_RUNTIME.md). Las instrucciones
> sectoriales siguientes documentan el piloto anterior y su compatibilidad.
> La alerta activa ya usa `/adaptive/events/erp`; el webhook sectorial anterior
> quedó deshabilitado tras validar firma, entrega y dos reentregas sin duplicados.
> La evidencia se registra en la aceptación de la feature 006.

# Telegram staging de un chat privado

`python3 scripts/run_telegram_staging.py` ejecuta un receptor separado en
127.0.0.1:18197. Lee únicamente el token KUSI_CHANNEL_TOKEN configurado y
TELEGRAM_WEBHOOK_SECRET del .env privado. No usa la base del workspace ni su panel. La interpretación de texto usa Groq en este proceso staging separado, con lecturas agregadas restringidas de CONEIMERA. No es la ruta genérica
/webhooks/{id}: su endpoint separado es /webhook.

El webhook requiere el secreto de Telegram, limita el cuerpo a 64 KiB y acepta
solo texto de chats privados. El enlace de `.local-data/telegram-staging/start-link.txt`
contiene un código aleatorio para vincular un único chat. No compartirlo. Abrir
su URL en Telegram y pulsar Iniciar; luego enviar /estado o /ayuda.

El ledger SQLite privado conserva solamente hashes de update_id, estado de envío
y el chat vinculado; no guarda textos. Una marca durable previa al envío evita
repetir respuestas en reentregas. Fallos ambiguos quedan como uncertain sin
reintento automático. Los chats privados sin vincular reciben únicamente instrucciones de vinculación, sin revelar el código. No hay mensajes proactivos. Se responde únicamente a la
vinculación explícita y a mensajes posteriores de ese chat. El código de
vinculación no autentica a una persona de un sistema empresarial.

Servicios iniciados para la prueba:
- kusi-telegram-staging.service
- kusi-telegram-tunnel.service

`systemctl --user stop kusi-telegram-tunnel kusi-telegram-staging` detiene ambos.
El túnel temporal cambia de URL al reiniciarlo; debe actualizarse setWebhook.
No se habilitó arranque automático al iniciar sesión. Para desvincular el chat,
detener el receptor y gestionar su estado privado explícitamente.

Validado: test de autenticación/vinculación/aislamiento/dedupe con transporte
simulado (`python3 scripts/test_telegram_staging.py`); HTTP público de salud 200;
webhook sin secreto 401; setWebhook y getWebhookInfo reales aprobados para
SoryntelKusiBot. La entrega real al usuario queda pendiente de que pulse Iniciar.

Referencia: https://core.telegram.org/bots/api#setwebhook

## Prueba ERPNext CONEIMERA

El comando exacto `/erp`, solo después de vincular el chat, ejecuta
`scripts/telegram_erp_probe.py`: SSH al host fijo autorizado y HTTP interno
al backend ERPNext, con la cuenta sintética existente. Comprueba login,
identidad propia, denegación de lectura de Administrator y logout. No consulta
pagos ni registros comerciales y no usa credenciales administrativas.
Credenciales leídas de un archivo privado local, enviadas por stdin SSH;
no aparecen en argumentos, logs ni respuestas. No se aceptan URLs, comandos,
usuarios ni filtros desde Telegram. Este probe operativo es independiente de
la API de capacidades PostgreSQL: no representa todavía ese recorrido desde
Telegram ni autorización de un usuario externo a registros empresariales.

## Consultas naturales y avisos de inscripciones (9 de noviembre de 2026)

La entrada principal es lenguaje natural, sin menú: «¿cómo van las
inscripciones?», «¿cuántos VIP hay?» o «¿cuántos se inscribieron hoy?».
`scripts/telegram_inscripciones.py` usa Groq con el modelo configurado en
GROQ_CHAT_MODEL para proponer únicamente action/period/state/ticket. Los cuatro
campos y sus valores se validan antes de ejecutar una lectura fija. La IA no
recibe datos de inscritos, no genera SQL, no puede seleccionar URL, usuario,
DocType ni operaciones de escritura. El resumen se construye con agregados
reales, sin nombres, DNI, correo ni teléfono. El texto del usuario se envía a
Groq para interpretar la consulta y puede consumir cuota; no se persiste aquí.

Se agregó el rol `Kusi Inscripciones Lectura` a la cuenta de integración
existente; permisos read=1, create/write/delete=0 para CONEIMERA Inscripcion.
Se conservaron las reglas de las tres funciones existentes (API, Sales User y
Finanzas). La lectura comprueba contraseña, cuenta habilitada y permiso Frappe,
y utiliza get_list bajo ese usuario. No se consulta como Administrator.

ERPNext tiene el webhook `Kusi Telegram - Nuevas inscripciones`, after_insert,
firmado HMAC-SHA256 base64, con payload limitado a DocType/evento/id/estado/tipo
y marcador de prueba. Se recibe en `/events/coneimera-inscripciones` del túnel
actual. No hay polling, cron ni monitor que consulte cada minuto. Inscripciones
anteriores no generan avisos retroactivos; confirmar una inscripción existente
no equivale a crear una nueva y no dispara este webhook.

El receptor autentica la firma y el origen lógico fijo antes de obtener el chat
vinculado. Guarda únicamente hash del evento y estado de envío en erp_events.
Las reentregas no repiten el aviso. Los fallos ambiguos quedan como uncertain y
requieren revisión; no se garantiza exactamente una entrega ante caídas.
El secreto está en un archivo privado local y en el campo Password del Webhook
Frappe; no aparece en mensajes ni logs de la aplicación.

Validaciones: pruebas unitarias de enums, filtrado, firma, dedupe y aislamiento
aprobadas; consulta real interpretada de resumen: 104 DRAFT, 34 Standard,
27 VIP y 43 Platinum en el momento de la prueba; consulta VIP: 27; petición de
borrar inscripciones rechazada. Se probó el transporte firmado de Frappe hacia
el túnel dos veces con un documento no insertado y marcado de prueba: HTTP
200 en ambas, una única notificación aceptada por Telegram. No se creó ninguna
inscripción de prueba en producción. Falta observar un after_insert real para
confirmar también el recorrido de la cola de Frappe en una inscripción real.

El túnel y el proceso local deben seguir activos. Si cambia la URL del túnel,
actualizar tanto el webhook de Telegram como request_url del Webhook Frappe.
Este flujo de staging usa consultas SSH restringidas, no el Integration Hub
PostgreSQL; no debe describirse como un conector genérico completo.


## Proyecciones por fecha

Se corrigió el rechazo de «proyección al 25 de diciembre». El intérprete admite
forecast con target_date ISO validada (futura y a un máximo de 366 días). La
fecha relativa se resuelve con el día actual de America/Lima. Los filtros de
estado y entrada siguen validados. El lector añade únicamente conteos por fecha
de creación al agregado; no devuelve registros ni datos personales.

La proyección es determinista: ritmo medio de hasta 30 días completos desde la
primera inscripción observada, incluyendo ceros y excluyendo hoy, aplicado a
los días restantes. Escenarios de sensibilidad al 50 %, 100 % y 150 % del ritmo;
no son intervalos de confianza. Sin un día completo no se inventa un ritmo.
No modela campañas, cupos ni estacionalidad. No es un modelo predictivo validado.

Prueba real del 9 de noviembre: petición interpretada con target_date
2026-12-25; 104 inscripciones entre 19/10 y 08/11, 21 días completos, 4,95/día;
46 días restantes; central 332, sensibilidad 218–446. Estas cifras corresponden
al momento de la prueba; la consulta del bot vuelve a leer el ERP.
Cinco pruebas Python de interpretación restringida, firma/dedupe, aislamiento y
proyección aprobadas. El receptor se reinició conservando chat y dedupe.
