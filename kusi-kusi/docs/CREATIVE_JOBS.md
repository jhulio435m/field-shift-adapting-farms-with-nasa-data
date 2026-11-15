# Creative Jobs: generación externa con aprobación

Implementado como módulo del monolito: Kusi Kusi valida, solicita aprobación y coordina proveedores especializados. No entrena ni ejecuta modelos de imagen/video. El puerto `CreativeProvider` admite adaptadores Higgsfield, fal y uno simulado de desarrollo. Las recetas son configuración por organización, no nombres de presets inventados ni garantías sobre el catálogo de un proveedor.

## Qué está operativo

- Descubrimiento de recetas habilitadas y sus esquemas.
- Borradores persistentes con parámetros y precio estimado congelados.
- Idempotencia por organización: misma clave y mismos parámetros devuelve el mismo trabajo; un cambio devuelve 409.
- Aprobación ligada al UUID del trabajo, organización, proveedor, modelo, parámetros, receta y estimación mediante SHA-256; vence a los 30 minutos.
- Reserva atómica de presupuesto diario de aprobaciones, con bloqueo transaccional por tenant.
- Worker PostgreSQL con `FOR UPDATE SKIP LOCKED`, leases y eventos auditables.
- Envío asíncrono oficial y polling; normalización de imágenes, video y audio en URLs HTTPS.
- Cancelación local de borradores/aprobaciones antes del envío. No se presenta como cancelación remota.
- Recuperación conservadora: un envío interrumpido o ambiguo pasa a `unknown`, sin reenviarse automáticamente.

## Configurar

`KUSI_CREATIVE_RECIPES` contiene un objeto JSON `{ "tenant-uuid": { "recipe_id": { ... } } }`. Los ejemplos son [recetas externas desactivadas](../examples/creative-recipes.json) y [una receta simulada](../examples/creative-recipes-simulated.json). Reemplazar UUIDs antes de utilizar.

Cada receta declara proveedor (`higgsfield`, `fal` o `simulated`), endpoint de modelo relativo al origen oficial, referencia de credenciales `key_env`, esquema JSON, valores predeterminados, estimación en micro-USD y `enabled`. El esquema se valida sin resolver referencias remotas. Los parámetros del cliente se fusionan con los defaults y el resultado completo debe cumplir el esquema. Limitar `additionalProperties`, duración, resolución y número de salidas para que la estimación revisada sea válida.

No se consulta un catálogo ni se selecciona un modelo automáticamente. Verificar el endpoint y esquema en la ficha oficial del modelo y revisar el precio vigente antes de habilitar. La receta de ejemplo de Higgsfield usa el endpoint documentado de Genjutsu motion transfer; requiere video e imagen de referencia, y su fuente requiere video de al menos cuatro segundos. La receta fal usa el ejemplo documentado de FLUX schnell para imagen desde texto. Ninguna se anuncia como generador de 360° fiel de un producto.

Credenciales:
- Higgsfield: `HF_CREDENTIALS` con `api_key_id:api_key_secret`, enviado como `Authorization: Key ...`.
- fal: `FAL_KEY`, enviado como `Authorization: Key ...`.

Los secretos se resuelven en el servidor; nunca se guardan sus valores en el borrador. Los ejemplos externos están deshabilitados y con estimación cero: no habilitarlos hasta revisar el costo y configurar credenciales. Los envíos reales habilitados requieren una estimación positiva. La API de Higgsfield tiene facturación separada de la suscripción web.

`KUSI_CREATIVE_WORKER=false` por defecto. Al activarlo, los trabajos aprobados pueden generar gasto. `KUSI_CREATIVE_SIMULATION=true` permite recetas simuladas de costo cero; estas devuelven `simulated:true` y ningún archivo, no una generación real. No utilizar el esquema/DB de producción para pruebas. El worker solo arranca tras validar configuración y abrir correctamente el listener HTTP.

## API de operador

Todas las rutas requieren la clave administrativa y el bearer del tenant. No exponerlas como chatbot público ni interpretar el número de mensajería como identidad autorizada.

1. `GET /v1/creative/recipes`: obtener recetas y parámetros.
2. `POST /v1/creative/jobs`: `{ "recipe":"product_video", "input":{"prompt":"..."}, "idempotency_key":"UUID" }`. Esto guarda un borrador, no llama al proveedor.
3. Revisar `proposal.input`, proveedor, modelo, estimación, vencimiento y `approval_digest` en la respuesta.
4. Configurar el presupuesto del tenant con `POST /v1/creative/budget`: `{ "daily_limit_microusd": 5000000 }`. Este valor es un ejemplo de límite ($5), no precio de un modelo.
5. `POST /v1/creative/jobs/{id}/approve`: `{ "confirm":true, "approval_digest":"digest devuelto", "max_cost_microusd":50000 }`. El máximo por solicitud debe cubrir la estimación. El worker debe estar habilitado para despachar.
6. `GET /v1/creative/jobs/{id}` para estados y resultados; `GET /v1/creative/jobs` para los últimos 50 del tenant.
7. `POST /v1/creative/jobs/{id}/cancel` antes de `submitting`.

Estados: `awaiting_approval`, `approved`, `submitting`, `queued`, `processing`, `succeeded`, `failed`, `unknown`, `canceled`. Una aprobación repetida no reenvía el trabajo. Los defaults, modelo o estimaciones cambiados requieren un nuevo borrador; el anterior conserva su snapshot hasta cancelarse o completarse.

## Presupuesto y facturación

1 USD = 1 000 000 micro-USD. El presupuesto por defecto es cero; por tanto bloquea aprobaciones de estimaciones positivas. El máximo diario configurable es 1000 USD y una receta puede estimar hasta 100 USD por trabajo. Son límites internos, no precios del proveedor.

La reserva usa la fecha de aprobación en UTC e incluye trabajos aprobados, en marcha, completados e inciertos. Los fallidos/cancelados se excluyen. Es un libro de estimaciones aprobadas, no medición del gasto facturado: no controla cambios de precio, consumo real ni el día en que un proveedor cobra. Una aprobación de ayer puede ejecutarse hoy. Para un límite financiero externo estricto, configurar límites/saldo en el proveedor y verificar la tarifa del conjunto de parámetros permitidos. Reducir el presupuesto no revoca trabajos ya aprobados.

`creative_budget_changes` guarda los cambios de límite; `creative_events` registra creador/aprobador por fingerprint de la credencial y transiciones sin copiar prompts ni secretos al log. `creative_jobs` sí conserva el prompt y los parámetros necesarios, de modo que exige retención y acceso restringido a la base.

## Fallos, reinicios y seguridad

Las solicitudes salientes solo usan orígenes oficiales fijos, sin proxies de entorno ni redirecciones. Los URLs de estado y resultado devueltos por el proveedor deben conservar el mismo origen y contener el request id correspondiente; esto evita enviar credenciales a otra URL. Timeout 20 s y respuesta JSON máxima 2 MiB. Higgsfield recibe `Idempotency-Key` con el UUID del trabajo. No se reintenta automáticamente ningún POST de generación; fal no se trata como si ofreciera la misma garantía de idempotencia.

Los estados se consultan con GET. Tras diez fallos consecutivos de polling se requiere reconciliación (`unknown`). Si el proceso cae durante un POST, al vencer el lease de 120 s el trabajo pasa a `unknown`. No reutilizar su contenido para crear otro trabajo hasta confirmar en el proveedor si el primero fue aceptado. No hay endpoint de “reintentar generación incierta”.

## Archivos, comandos y límites de esta entrega

Las recetas pueden aceptar URLs HTTPS de imágenes/videos ya alojados por el cliente. Verificar propiedad/autorización de esos archivos antes de aprobar su envío a un tercero. No se implementa todavía la carga propia de archivos ni la transferencia a almacenamiento S3; el módulo conserva metadatos y URLs del resultado. Los archivos remotos pueden caducar y deben copiarse a almacenamiento propio para conservación prolongada. No hay promesa de 360° geométricamente fiel a partir de una sola foto.

El `recipe_id` es el comando estable para integraciones. Una conversación futura podrá traducir “crea un video de este producto” a un borrador de receta autorizada. Este módulo no conecta aún esos comandos a los mensajes de WhatsApp/Telegram ni ejecuta automáticamente productos/webhooks de un ERP. Eso depende de vinculación de identidad y del worker conversacional. No se habilitó publicación en redes, no hay panel de aprobación y no se ha efectuado ninguna generación de pago.

## Verificación repetible

`./scripts/verify.sh` ejecuta formato, Clippy, pruebas unitarias/integración y build. La prueba de PostgreSQL crea su propio esquema y comprueba aislamiento, doble worker, vencimiento, cancelación, reanudación conservadora e idempotencia.

`python3 scripts/verify_creative.py` inicia una API temporal en 18084, usa un esquema independiente, crea dos tenants de prueba, valida el flujo HTTP, reinicia la API y comprueba la auditoría. No llama a proveedores ni requiere GPU. Requiere `psql`, PostgreSQL local, un `DATABASE_URL` disponible y el binario compilado. El esquema se elimina al finalizar. Puede cambiarse el puerto con `KUSI_CREATIVE_TEST_PORT`.

Los contratos Higgsfield/fal fueron contrastados con documentación oficial y fixtures. La accesibilidad de modelos, facturación, generación real y calidad visual siguen sin verificarse en cuentas reales.

Fuentes oficiales consultadas:
- https://docs.higgsfield.ai/docs (autenticación, envío y catálogo por modelo).
- https://docs.higgsfield.ai/docs/concepts/requests (estados, idempotencia y resultados).
- https://docs.higgsfield.ai/docs/api-reference/requests/get-request-status (schema de polling).
- https://fal.ai/docs/documentation/model-apis/inference/queue (submit, status y response URLs).

## Actualización: recorrido local y archivo permanente

El espacio local incorpora comandos deterministas de conversación y recetas simuladas `local_product_video/image`, con archivos privados y permisos verificados. Tras aprobar, un archivador FFmpeg produce MP4/PNG rotulado; los metadatos y bytes sobreviven a reinicios. No representa generación de IA ni valida generaciones remotas. El resultado se puede usar en borradores de publicación con una segunda aprobación. Ver LOCAL_WORKSPACE.md.
