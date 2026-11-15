# Adaptadores en la API real y PostgreSQL

Verificado el 8 de noviembre de 2026 en `132.226.244.23` (ARM64), con un
runtime de aceptación separado de los servicios existentes.

## Revalidación independiente: 8 de noviembre, 22:19 (Lima)

Se volvió a compilar desde cero el código actual en ARM64 con `Cargo.lock`
bloqueado y se repitió `scripts/verify_system_connectors.py` contra una base
PostgreSQL nueva. Resultado: **59 pruebas aprobadas, cero fallos**, cuatro
destinos reales aprobados mediante Axum, ocho permisos y 80 eventos de auditoría
PostgreSQL. Se repitieron los rechazos y comprobaciones de reinicio descritos
abajo. WPPConnect valida salud, sin autenticar teléfono ni enviar mensajes.

Se compararon IDs, fechas de arranque y contadores de reinicios de CRM,
ERPNext, Plane y WPPConnect antes y después: los cuatro seguían activos y sin
cambios. Formato, Clippy y compilación del script Python aprobados localmente.
No se hicieron cambios de código durante esta repetición.

Las 80 fuentes/contratos/migraciones subidas coinciden con el checkout local:
SHA-256 `47185536db5a26eb4aa80dea87fe7dd7694bde4d94badcd5267106a24a40e1f9`.
Binario ARM64:
`743055b5684d05e253d225b557b061af2a96426dd8ac5ebd3c13b49ef77d590b`.
Evidencia versionada con fechas UTC e imagen del compilador:
`.local-data/qa/system-runtime-revalidation-20261109T031536Z-a71267.json`.

## Resultado

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Aprobado localmente |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado localmente |
| `cargo test --workspace` | 59 pruebas aprobadas; suite remota con PostgreSQL temporal y sockets, sin filtros |
| Contrato OpenAPI | YAML parseado; envelope estricto y respuestas documentadas |
| API Axum + fixtures HTTP con contadores | Cuatro adaptadores aprobados |
| Tenant distinto, falta de permiso, argumentos, versión, capacidad deshabilitada | Rechazados antes de contactar el destino |
| Clave inválida o revocada | HTTP 401 |
| Tenant ID añadido al envelope | HTTP 422 |
| Input contra esquema y contra el adapter con esquema permisivo | HTTP 400; sin llamada al destino |
| Output que no cumple el esquema registrado | HTTP 400 después de la lectura; no se devuelve como éxito |
| Auditoría inicial no disponible | HTTP 503 y contador de llamadas sin cambios |
| Reinicios de API | Capacidades, permisos y auditoría persisten; lecturas posteriores aprobadas |
| CRM y ERPNext reales | Cuenta sintética autenticada a través de la API de Kusi Kusi |
| Plane real | Identidad sintética autenticada, cero workspaces |
| WPPConnect real | Salud aprobada; `authenticated: false`, sin sesión de WhatsApp |

La base de aceptación conservó **80 eventos**, agrupados en 40 ejecuciones con
inicio y resultado terminal: 12 `succeeded`, 16 `forbidden`, 12 `invalid_data`.
Se comprobaron ocho permisos persistidos para dos tenants. Los fallos de
autenticación del tenant se rechazan antes de iniciar una ejecución.

Evidencia estructurada: `.local-data/qa/system-runtime-result.json`.

## Qué cambió

`apps/server/src/main.rs` conecta `RuntimeConnector` con el caso de uso existente,
el repositorio real PostgreSQL, la autorización y la auditoría. Los adaptadores
viven en `crates/infrastructure/src/system_connectors.rs`; el dominio y la capa
de aplicación no conocen Frappe, Plane, reqwest ni las credenciales.

Se reutilizan las tablas existentes `capabilities`, `capability_grants` y
`execution_audit`; no hay cambios de esquema ni migraciones pendientes para
esta entrega. El runtime ya no depende del registro/permisos/auditoría en
memoria del ejecutable de aceptación de la fase anterior.

Los perfiles siguen desactivados por defecto. Para configurarlos, ver
[INTEGRATIONS.md](INTEGRATIONS.md#runtime-platform-adapters) y `.env.example`.
Las únicas operaciones implementadas aquí son identidad propia de la cuenta
configurada y salud; no hay lectura arbitraria de registros comerciales,
escrituras, publicación o envío de mensajes.

## Reproducir

El script crea su propia base PostgreSQL temporal, sin leer `.env` ni reutilizar
una base de producción. Requiere Docker y sockets loopback disponibles:

```sh
cargo build -p kusi-server
python3 scripts/verify_system_connectors.py \
  --result .local-data/qa/system-runtime-fixtures.json
```

Para el servidor ARM64 se compiló el checkout en `rust:1.95-slim`, limitado a
2 CPU y 2 GiB. El script también admite `--runtime-image rust:1.95-slim`
y `--binary /ruta/target/debug/kusi-server`, para ejecutar esa API compilada
en un contenedor con raíz de solo lectura y sin capacidades Linux. El runner
HTTP usa únicamente loopback; su PostgreSQL nuevo escucha en un puerto
loopback aleatorio con datos en tmpfs y se elimina al terminar.

La opción `--rust-workspace /ruta/al/workspace` ejecuta además la suite Rust
completa dentro de esa imagen contra la base temporal. Las lecturas de los
servicios reales requieren explícitamente `--live-credentials /ruta/privada.json`
y `--allow-live-reads`; ese archivo corresponde a las cuentas sintéticas ya
creadas, sin permisos administrativos. No activa envíos ni vincula teléfonos.

## Alcance operativo

No se habilitó permanentemente Kusi Kusi en el servidor ni se alteraron los
servicios productivos. Se modificó el checkout real y se probó su runtime
compilado de manera aislada. Las cuentas solicitadas permanecen disponibles;
las credenciales originales siguen en el archivo privado local documentado.
Se retiraron el runtime, la base, los archivos y credenciales temporales y la
imagen del compilador de aceptación. Se verificó que no quedaran contenedores
QA y que los cuatro servicios originales continuaran activos.

No se cambió el frontend, el proxy Next, los puertos habituales ni las reglas
de cookies/CSRF/RBAC. Esta entrega no vuelve a certificar UI, OIDC real ni
integraciones de pagos institucionales. El modo local mantiene el bloqueo de
llamadas externas; la conectividad de estos adaptadores es optativa y separada.
