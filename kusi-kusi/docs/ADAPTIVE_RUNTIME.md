# Runtime adaptable (006)

El ejecutable `adaptive_assistant` comparte el dominio y los casos de uso Rust.
La conversación produce un plan JSON acotado; la aplicación valida permisos,
versión, campos, tipos, fechas y volumen antes de ejecutar cada lectura.
El modelo recibe metadatos, contexto estructurado y la petición, no las filas.
Los cálculos y la respuesta con evidencia se generan de forma determinista.

Se admiten conteo, agrupación categórica, suma, media, comparación de periodos y
proyección de ritmo diario constante. La proyección usa días completos, incluye
los días sin actividad y declara supuestos; su sensibilidad no es un intervalo
estadístico de confianza. No supone que un importe sea dinero cobrado.

## Conexión y límites

Los adaptadores registrados son manifiestos JSON privados y Frappe por SSH.
Un recurso nuevo del mismo protocolo no necesita funciones específicas por negocio.
Un protocolo nuevo requiere un adaptador y una delegación inicial. No se descubre
la red ni se instalan integraciones por instrucciones del modelo.

El puente Frappe autentica una cuenta de servicio, descubre roles explícitos,
campos permitidos y permisos vigentes. Solo expone categorías, números y fechas;
excluye texto libre, contraseñas, enlaces y adjuntos. Las lecturas respetan permisos
de registro. Los webhooks comprueban firma y vuelven a autorizar el registro antes
de avisar. La instalación de hooks exige `allow_watch` en la fuente registrada. Sus nombres
estables incluyen un hash de organización y principal; dos actores no comparten
accidentalmente la configuración de callback.

Máximos: 4 consultas, 8 filtros y 8 análisis por consulta, 16 campos por lectura,
10 000 filas por petición, 80 recursos por catálogo, 32 suscripciones activas o
pendientes y 1 llamada al modelo por turno. Una lectura truncada no es un total.
El contexto caduca tras 24 horas; cambios de esquema y permisos lo revalidan.

## Ejecución aislada

La aplicación local mantiene su bloqueo de proveedores externos. Este ejecutable
es un punto de composición de staging separado: requiere
`KUSI_ADAPTIVE_STAGING_ACK=authorized-environment` y rechaza `KUSI_LOCAL_MODE=true`.
La configuración privada se registra fuera del mensaje en `.local-data/adaptive/`.
El token del proveedor llega por la variable de entorno indicada en el perfil.
No introducir secretos en argv, fixtures ni control de versiones.

La identidad del chat debe coincidir con el hash registrado. El cliente no puede
pasar organización, usuario, rutas, destinos ni credenciales en su plan. No se ha
añadido un endpoint al servidor HTTP principal ni cambiado Next.js.

## Estado y recuperación

Formato de persistencia v1: documentos `{format:1,data:...}`, archivos 0600,
directorios 0700, reemplazo atómico y sincronización. La partición usa el hash de
organización y principal. Incluye contexto estructurado (con interpretaciones
semánticas aprobadas y revalidadas por versión), recibos, auditoría de decisiones
y procedencia (`audit-*.json`, sin almacenar cuerpos de mensajes), suscripciones y
claims exclusivos. No almacena transcripciones. No cambia el esquema PostgreSQL. El ledger SQLite del transporte añade una tabla
de entregas mediante `scripts/migrations/001_adaptive_deliveries.sql`, migración
aditiva e idempotente registrada por versión que conserva vinculación e historial.

Una petición con el mismo identificador y catálogo recibe su recibo. Un claim sin
recibo queda `unknown` y no se reejecuta automáticamente. Una suscripción se guarda
pendiente antes de instalar un hook idempotente y activa después de confirmarlo.
Cancelar impide entregas posteriores; el hook compartido puede permanecer instalado.

El claim de entrega precede al envío. Telegram registra aceptación o resultado
incierto por separado: aceptación API no demuestra lectura del destinatario. No
se promete entrega exactamente una vez ante una caída entre claim y envío. Revisar
el claim y el ledger antes de recuperar manualmente; no borrar claims para forzar
reintentos. El receptor de staging serializa peticiones; no se ha validado un
transporte concurrente multiusuario ni migración automática entre formatos.

## Verificación

`cargo test -p kusi-application --test adaptive` ejecuta cálculos, límites, cambios
de esquema, permisos, aislamiento y eventos sin proveedores. El planificador de
esas pruebas es programado y no constituye aceptación de lenguaje natural.

`python3 scripts/verify_adaptive.py --allow-provider --result <archivo-privado>`
usa el modelo configurado con datos sintéticos: dos fuentes, tres recursos,
conversaciones, formulaciones reservadas y suscripciones. Es staging con llamadas
externas explícitas; no envía mensajes Telegram. Sus resultados y posibles fallos
se registran en `specs/006-adaptive-environment/acceptance.md`.

El túnel temporal debe seguir activo y su URL registrada en Telegram y Frappe.
Este runtime no convierte un túnel temporal en un despliegue permanente.

## ACL de recursos analíticos (008)

La composición puede proteger el Environment con el registro/permiso global, el vínculo externo y las ACL PostgreSQL de 007. El principal es el hash de una credencial individual vigente, registrado en configuración privada y vinculado al chat; no es un sujeto o tenant enviado en un mensaje. Cada binding incluye adaptador esperado, capacidad, versión y recurso exacto de ACL. Un cambio de adaptador de la capacidad no transfiere autoridad a la fuente anterior.

Ejemplo ficticio del bloque adicional `access` en runtime.json (0600):

```json
{
  "access": {
    "database_url_env": "KUSI_ADAPTIVE_DATABASE_URL",
    "resources": {
      "alpha::orders": {
        "adapter": "system-a",
        "capability": "analytics.read",
        "version": "1",
        "resource_ref": "authorized-orders-dataset"
      }
    }
  }
}
```

Los otros campos de runtime.json se conservan. `principal` debe ser un digest SHA-256 hexadecimal minúsculo de 64 caracteres de una clave vigente del mismo tenant. La URL PostgreSQL se obtiene de la variable nombrada, nunca del mensaje. Este flujo no aplica migraciones: el servidor debe haber aplicado 0018. La cuenta que aprovisiona debe verificar fuera de banda tanto el vínculo como la concesión.

Para los despliegues que exijan esta protección se configura `KUSI_ADAPTIVE_REQUIRE_ACL=true`. La ausencia de `access` se rechaza antes del intérprete. El transporte de Telegram conserva la exigencia y solo pasa al proceso las variables de credenciales seleccionadas por la configuración privada. También bloquea `KUSI_LOCAL_MODE=true` antes de cargar configuración o invocar el proceso. No se cambió la configuración del staging existente: el modo legado sin `access` continúa explícitamente separado y no acredita ACL externas.

El catálogo entregado al intérprete contiene solo recursos concedidos y usa una versión ligada a ese ámbito. El adaptador conserva su catálogo original para validar la lectura. Se comprueban clave, permiso global, vínculo y ACL en cada descubrimiento, lectura, suscripción y evento. Si todos los recursos se deniegan, no se descubre la fuente ni se llama al intérprete. Si una fuente contiene otros recursos, sus metadatos pueden recuperarse en el descubrimiento de esa fuente registrada, pero se eliminan del catálogo mostrado y no se leen sus filas. Fallos de almacenamiento o auditoría fallan cerrado. La auditoría de la puerta usa `adaptive.resource.authorize` para distinguir una autorización aprobada de la ejecución de una capacidad; los resultados de la conversación siguen en su auditoría privada. Una política ausente se deniega también en este flujo, sin adoptar la excepción de capacidades de servicio.

Después de una revocación no se reutiliza una respuesta guardada ni se incluyen borradores, esquemas o pistas de suscripción de recursos revocados en el contexto del intérprete. Los avisos vuelven a comprobar permisos y la firma del evento. `event_secret_file` puede declararse en la fuente registrada, separado del manifiesto; el formato previo que guarda esa ruta dentro de la configuración del puente sigue admitido.

**Ámbito:** una concesión sobre un recurso analítico autoriza el conjunto completo que su adaptador ya permite. No convierte una ACL de una fila en acceso a una colección ni implementa filtrado por fila automático. El permiso remoto de la cuenta del adaptador y la validación tipada siguen siendo necesarios. No hay consulta automática al proveedor de identidades/ACL ni activación de envíos reales.

Aceptación sin proveedor:

```sh
cargo build -p kusi-server --bins
python3 scripts/verify_adaptive_access.py --result /tmp/adaptive-access.json
```

El runner crea PostgreSQL desechable, un intérprete determinista HTTP loopback y un canal vinculado sintético. Ejecuta el binario real, calcula un conteo, crea una suscripción, valida un evento firmado y captura la entrega en memoria. Comprueba duplicación, caducidad, revocación, replay de recibos, omisión de ACL obligatoria y cambio de chat. No usa .env, staging ni bases existentes. No acredita interpretación con un modelo real ni lectura de un proveedor real.
