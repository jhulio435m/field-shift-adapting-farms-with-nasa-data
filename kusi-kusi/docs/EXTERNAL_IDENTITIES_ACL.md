# Identidades externas y ACL por recurso

Entrega backend local de la especificación 007. No configura un proveedor, no verifica sus permisos en vivo y no habilita envíos ni llamadas externas en modo local.

## Modelo

La clave Bearer vigente resuelve organización y principal en el servidor. Para representar una persona se necesita una credencial individual: una clave compartida no permite distinguir personas. La identidad externa se vincula mediante comprobación manual del operador y queda separada por organización, hash de credencial y adaptador registrado (sistema).

La política `external_resource_policies` protege todas las capacidades/versiones del mismo adaptador. A partir de su creación, cada ejecución exige además del permiso global un vínculo y una concesión vigentes para capacidad, versión y recurso exactos. La comprobación consulta PostgreSQL en cada lectura, sin caché. Los errores de almacenamiento fallan cerrado. Los adaptadores sin política conservan su autorización de servicio existente; no deben usarse para ofrecer lecturas personales.

El contrato protegido acepta únicamente `input: {"resource_ref":"record-a"}`. No admite filtros libres, subject, tenant, URLs ni argumentos adicionales. La salida del adaptador debe ser exactamente `{"resource_ref":"record-a","data":...}` y cumplir el esquema registrado. Esto requiere un adaptador que respete realmente el recurso solicitado; la etiqueta no demuestra por sí sola la procedencia de todos los datos internos. Los adaptadores incorporados de identidad/salud, que esperan `{}`, no implementan este contrato de lectura por recurso.

Los vínculos y permisos caducan en un máximo de 30 días al crearlos. Las concesiones nunca superan la vigencia del vínculo. Una concesión revocada o caducada no se renueva mediante upsert. Un vínculo existente tampoco se reemplaza: ante cambio de identidad o renovación se aprovisiona una nueva credencial individual y se conceden permisos nuevos, sin heredar los anteriores. Rotar la clave no transfiere ACL. Revocar la clave, el vínculo o la concesión afecta las siguientes comprobaciones; no cancela una lectura ya autorizada en curso.

## Operación local

El servidor aplica la migración `0018_external_resource_access.sql` en su ciclo normal. La CLI no aplica migraciones ni configura transportes. Usa únicamente DATABASE_URL de un PostgreSQL loopback de desarrollo y un archivo privado (0600) señalado por KUSI_EXTERNAL_ACCESS_REQUEST_FILE:

```sh
cargo run -p kusi-server --bin provision_external_access
```

No pongas claves ni sujetos en argumentos de shell o archivos versionados. El archivo contiene:

```json
{
  "organization": "00000000-0000-4000-8000-000000000001",
  "principal_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "adapter": "system-a",
  "action": {"operation": "protect"}
}
```

Los valores de arriba son ficticios. El hash debe corresponder a una clave vigente de esa organización, no es una nueva credencial. El tenant del archivo es contexto del operador de base de datos, nunca una selección aceptada por el endpoint de ejecución.

Acciones, en orden:

1. `protect`: protege el adaptador; se puede repetir. No existe acción de desprotección en la CLI.
2. `link`: añade `subject`, `ttl_seconds` y `confirmed: true` a action tras verificar la identidad fuera de banda.
3. `grant`: añade `capability`, `version`, `resource_ref`, `ttl_seconds` y `confirmed: true` tras comprobar el permiso externo. La capacidad debe estar habilitada y ligada al mismo adaptador. El permiso global sigue siendo necesario.
4. `revoke_grant`: incluye `capability`, `version` y `resource_ref`.
5. `revoke_link`: revoca todas las lecturas del vínculo.

Una operación rechazada no confirma cambios. Las mutaciones y su auditoría se confirman en la misma transacción. Auditoría: evento aleatorio, organización, operación estática y resultado; no incluye sujeto externo, hash de credencial, recurso ni cuerpos. La CLI emite errores genéricos.

## Verificación

Pruebas de aplicación con adaptador sintético: alcance exacto, argumentos adicionales, versión, revocación, almacenamiento no disponible y respuesta de otro recurso. No contactan proveedores.

Prueba PostgreSQL explícita, aislada en esquema aleatorio y restringida a loopback:

```sh
cargo test -p kusi-infrastructure --test external_access -- --ignored --nocapture
```

Requiere TEST_DATABASE_URL de una base de desarrollo desechable con permisos para crear/eliminar esquemas. Comprueba migración, comprobación real de ACL, dos adaptadores, caducidad, revocación, no reactivación y auditoría. No se ejecuta automáticamente contra una base real. El 10 de noviembre la prueba PostgreSQL y la aceptación HTTP loopback pasaron con una base desechable. La prueba HTTP usa el mismo manejador de autenticación/ejecución que producción y un conector sintético inyectado únicamente por el test; no activa ni modifica el transporte real. Ejecutar HTTP de forma independiente:

```sh
cargo test -p kusi-server --bin kusi-server http_external_acl_acceptance -- --ignored --nocapture
```

Para crear y eliminar automáticamente un PostgreSQL desechable, ejecutar ambas pruebas y comprobar el binario completo/CLI y el bloqueo del modo local:

```sh
python3 scripts/verify_external_access.py --result /tmp/kusi-external-acceptance.json
```

El runner escribe el JSON también si falla: código de salida `0` para éxito, `1` para fallo y `2` para bloqueo de permisos del sistema. Un resultado bloqueado lleva `verification_complete: false`; no sustituye una aceptación real. Los informes excluyen argumentos y detalles de excepciones.

Este runner no lee `.env`, descarta configuraciones de proveedores/base de datos heredadas y no usa bases existentes. Requiere initdb/pg_ctl, un usuario sin privilegios root y sockets locales. El runner integral pasó el 10 de noviembre con los permisos habilitados, incluyendo el binario completo y la CLI. La suite Rust y el build Next también pasaron mediante unidades temporales systemd de usuario. Los intentos EPERM anteriores quedan como históricos. El registro exacto y los comandos constan en VALIDATION.md y `specs/007-external-identities-acl/acceptance.json`. No hay aceptación de proveedor real.

## Siguiente integración

Para verificar automáticamente identidad y ACL remotas hace falta elegir el protocolo de evidencia, semántica de revocación, contrato del adaptador y entorno sintético del proveedor. No reutilizar una cuenta de servicio compartida como identidad personal. El asistente adaptativo y los canales no pasan a utilizar este vínculo automáticamente: su conexión requiere un contexto de canal registrado y pruebas propias.
