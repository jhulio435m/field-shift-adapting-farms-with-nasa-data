# Aceptación remota mediante el caso de uso de Kusi Kusi

**Fase anterior, conservada como historial.** La limitación de permisos y
auditoría en memoria de esta prueba ya se resolvió en el runtime Axum con
PostgreSQL: [SYSTEM_CONNECTORS_ACCEPTANCE.md](SYSTEM_CONNECTORS_ACCEPTANCE.md).
Ese documento y `scripts/verify_system_connectors.py` son la aceptación actual.

Verificada el 8 de noviembre de 2026 en el servidor ARM64 autorizado
`132.226.244.23`. Resultado: cuatro destinos aprobados, 16 rechazos previos al
transporte, 40 eventos de auditoría en memoria y clave inválida rechazada.
Evidencia estructurada: `.local-data/qa/remote-capabilities-result.json`.

| Destino real | Resultado de lectura mediante ExecuteCapability |
|---|---|
| CRM Frappe | Identidad sintética HTTP 200; Administrator HTTP 403; logout HTTP 200 |
| ERPNext | Identidad sintética HTTP 200; Administrator HTTP 403; logout HTTP 200 |
| Plane | Login HTTP 302; identidad HTTP 200; workspaces vacíos HTTP 200; logout HTTP 302 |
| WPPConnect | Salud HTTP 200; consulta de sesión sin token HTTP 401 |

El primer login de Plane desde Rust devolvió HTTP 500 sin `User-Agent`. El
adaptador de aceptación se corrigió para declarar `kusi-kusi-acceptance/0.1` y
el recorrido completo posterior pasó. No se modificó Plane.

También pasó `cargo test --workspace` del código Rust en un contenedor ARM64:
55 pruebas, cero fallos y cero filtros. `TEST_DATABASE_URL` apuntó a PostgreSQL
15.7 temporal con datos en tmpfs y sin red externa ni puertos publicados; las
tres pruebas PostgreSQL ejercitaron sus comprobaciones. Se ejecutaron las dos
pruebas de sockets que el sandbox local bloquea. `cargo fmt --check` y
`cargo clippy --workspace --all-targets -- -D warnings` pasaron localmente.

Al terminar se eliminaron los contenedores temporales, PostgreSQL de prueba,
fuentes y binarios remotos, copias temporales de credenciales e imagen del
compilador descargada para esta ejecución. Se comprobó que los cuatro servicios
originales seguían activos. Las cuentas sintéticas solicitadas permanecen.

El ejecutable `crates/infrastructure/examples/verify_remote_capabilities.rs`
ejecuta el caso de uso real `kusi_application::integrations::ExecuteCapability`
con adaptadores HTTP de aceptación para CRM Frappe, ERPNext, Plane y
WPPConnect. No es una simulación de sus respuestas. Las peticiones de los
adaptadores llegan a las aplicaciones existentes con cuentas sintéticas.

## Alcance del ejecutable

La identidad de organización se resuelve mediante `GetOrganizationByKey` con
una clave efímera y su digest; no se obtiene de argumentos de la capacidad.
El registro, los permisos y la auditoría de esta aceptación están en memoria.
Los esquemas JSON de entrada y salida se validan con `jsonschema`. Se usa el
código de dominio y aplicación del checkout, sin una reimplementación en Python.

Por cada sistema se comprueban cuatro rechazos previos al transporte: tenant
distinto, principal sin permiso, argumentos que intentan cambiar el destino y
versión inexistente. El contador de invocaciones debe permanecer igual. Después
se ejecuta una lectura real y se comprueba la salida contra el contrato.

- CRM y ERPNext: login de la cuenta sintética, consulta de su identidad,
  denegación al leer `User/Administrator` y logout.
- Plane: CSRF, login, identidad propia, lista de workspaces vacía y logout.
- WPPConnect: salud y rechazo de consulta de sesión sin token. No se autentica
  una sesión WhatsApp ni se envían mensajes.

La aceptación requiere 20 ejecuciones de capacidades (16 rechazos y 4 lecturas),
40 eventos de auditoría en memoria y rechazo de una clave inválida antes de
resolver organización. Solo se imprimen estados, nombres de sistemas y rutas
fijas, nunca claves, contraseñas, cookies ni cuerpos de respuesta.

## Ejecución explícita

Este es un ejecutable de pruebas optativo, no forma parte del servidor local
ni se ejecuta durante `cargo test`. Requiere:

```sh
KUSI_REMOTE_ACCEPTANCE=synthetic-read-only \
  cargo run -p kusi-infrastructure --example verify_remote_capabilities -- /ruta/privada/config.json
```

El archivo debe ser privado (0600), con un objeto `systems` y las cuatro claves
`crm`, `erp`, `plane`, `wppconnect`. Cada destino tiene `base`, `host`, `email`
y `password`; WPPConnect usa null para las dos credenciales. Solo admite URLs
HTTP con IPv4 privada o loopback, sin credenciales en URL, query ni fragmento.
Los correos de login deben ser exactamente
`kusi-qa-20261108@example.invalid`. No usar cuentas ni registros reales.

La ejecución remota se compila con Rust 1.95 para ARM64 en un contenedor
temporal limitado a 2 CPU y 2 GB de RAM. Las credenciales no se montan en el
contenedor de compilación. El ejecutable de aceptación corre con raíz de
filesystem de solo lectura, sin capacidades Linux, sin Docker socket, con
configuración de solo lectura y acceso a los destinos privados desde el host.
No se publican nuevos puertos ni rutas y no se sustituyen contenedores existentes.

## Límites

Pasar esta aceptación demuestra interoperabilidad de lectura a través del caso
de uso y de los adaptadores de aceptación. No valida aún la ruta Axum
`/v1/capabilities/execute`, la persistencia PostgreSQL de estos permisos y
eventos, ni conectores de producción con gestión durable de credenciales.
El conector HTTPS genérico de producción y sus protecciones contra destinos
privados permanecen intactos. Tampoco cambia el bloqueo de llamadas externas
del workspace local.

Las pruebas de regresión de `crates/application/src/integrations.rs` verifican
además que una denegación de permiso, un input inválido y una auditoría inicial
no disponible impiden invocar el conector.
