# Configuración web del piloto 009 — 10 de noviembre de 2026

El piloto elegido es Plane + ERP CONEIMERA + Telegram. Se añadió **Conexiones → Sistemas y canal del piloto** para propietarios del workspace local. Las tareas T001–T006 de [009](../specs/009-web-connection-settings/tasks.md) están completadas; [guía](WEB_CONNECTION_SETTINGS.md) y [evidencia estructurada](../specs/009-web-connection-settings/acceptance.json).

## Cambios

- API de listado/guardado y borrado de credenciales por tenant derivado de sesión. Roles editor/aprobador/lector no pueden administrar estos perfiles. No hay tenant elegible en el cuerpo ni API de descifrado.
- Migración 0019 para perfiles Plane/Frappe/Telegram. Credenciales cifradas con XChaCha20-Poly1305 y AAD por tenant/id/tipo/revisión; clave maestra privada 0600 persistente. Si falta la clave con ciphertext existente, tiene permisos públicos o se altera el ciphertext, se falla cerrado.
- Guardado con revisión optimista, lock por perfil y propietario vigente en DB; auditoría atómica sin credenciales/cuerpos/URLs. Campos secretos vacíos conservan valores; nuevos valores rotan y borrado explícito elimina el ciphertext. JSON inválido produce un error genérico sin reflejar valores.
- Formularios web de Plane (URL/API Key/workspace), ERP (URL/API Key/API Secret) y Telegram (token/bot/webhook). Los secretos nunca se rellenan al editar ni se devuelven en respuestas; se borran del DOM al enviar incluso si el guardado falla.

## Verificación actual

| Control | Resultado |
| --- | --- |
| cargo fmt --check | Aprobado |
| cargo clippy --workspace --all-targets -- -D warnings | Aprobado |
| cargo test --workspace | 83 resultados aprobados, cero fallos; dos opt-in aprobadas explícitamente por el runner 007 |
| Python unittest con ResourceWarning como error | 14 aprobadas |
| Formato admin y build Next Turbopack/TypeScript | Aprobados |
| verify_connection_settings.py | PostgreSQL/migración reales desechables; 3 perfiles, CSRF/Origin/RBAC/tenant, cifrado, edición vacía, rotación, conflicto, tamper, borrado, reinicio y pérdida/permisos de clave: aprobado |
| verify_connection_settings_browser.py | Chromium desktop/móvil; guardar/recargar 3 perfiles, editar sin revelar secretos, rotación, conflicto entre pestañas, borrado confirmado, lector denegado y cero tráfico externo: aprobado |
| verify_next_isolated.py | Regresión de cookies, Origin/CSRF/no-store, proxy fijo, rutas directas/logout y OIDC local desactivado: aprobada |
| verify_external_access.py | Regresión integral 007 con PostgreSQL/HTTP, servidor/CLI y bloqueo local: aprobada |

Se inspeccionó el estado previo y las cuatro capturas finales en [qa](../specs/009-web-connection-settings/qa). Se corrigieron en una pasada la alineación de etiquetas y la distribución de acciones en móvil, y se confirmó el resultado. El detector Impeccable no reportó hallazgos deterministas. No se cambió la frontera de ejecución Next/Rust ni su proxy; no se reconstruyeron/reverificaron las imágenes de contenedor para esta entrega. La evidencia anterior de contenedores 008 se conserva como histórica.

## Límite de entrega

**Son perfiles locales guardados, sin verificar ni activar.** No se consumen aún desde `KUSI_SYSTEM_PROFILES`, `KUSI_CHANNELS` ni los archivos del asistente. Guardar no contacta APIs, no registra webhooks, no cambia staging y no envía mensajes. El plano OIDC productivo permanece separado de estos endpoints locales. La futura activación necesita un consumidor de perfiles autorizado y validación real de destinos/permisos; no se declara implementada.

Los runners usaron datos y credenciales sintéticos con PostgreSQL nuevo y servidores temporales; no se modificó ninguna base existente, credencial real o canal activo. Se conserva la clave privada separada del dump; la guía explica recuperación y límites de rotación de clave maestra. Algunas pruebas Rust preexistentes condicionadas a TEST_DATABASE_URL retornan temprano; la aceptación específica de configuración sí ejercitó PostgreSQL real y el servidor compilado.

# Entrega autónoma local 008 — 10 de noviembre de 2026

Se cerró el bloque local del asistente con ACL y la aceptación visual OIDC (004/T015). Las tareas T001–T007 de [008](../specs/008-adaptive-resource-access/tasks.md) están completadas; la evidencia consolidada está en [acceptance.json](../specs/008-adaptive-resource-access/acceptance.json).

## Cambios y comportamiento

- Environment de aplicación autorizado por principal/tenant privados, adaptador/capacidad/version/recurso exactos, permiso global, vínculo y ACL vigentes. Una política ausente no habilita el acceso. Auditoría fallible de la puerta identificada como `adaptive.resource.authorize`, distinta de ejecutar una capacidad.
- Catálogo filtrado y versionado por ámbito; revalidación antes de lecturas, suscripciones y eventos. Revocación bloquea replay de recibos y elimina borradores, esquemas, preguntas y pistas de suscripción de recursos retirados antes de llamar al intérprete.
- Configuración privada opcional `access` y exigencia explícita `KUSI_ADAPTIVE_REQUIRE_ACL=true`; el transporte registrado preserva esa exigencia y bloquea modo local antes del proceso. No se cambió el staging existente.
- Se corrigió el proxy Next: faltaba reenviar Location y el flujo OIDC se detenía. Ahora conserva redirecciones y cookies del backend fijo sin seguir redirects upstream. Origin/CSRF/no-store y autorización siguen en servidor.
- SQLite usa cierre explícito después del contexto transaccional en transporte y fixtures. No se observan conexiones sin cerrar en la suite con advertencias tratadas como error.

## Evidencia ejecutada

| Control | Resultado |
| --- | --- |
| cargo fmt --check | Aprobado |
| cargo clippy --workspace --all-targets -- -D warnings | Aprobado |
| cargo test --workspace | 80 resultados aprobados, cero fallos; dos opt-in aprobadas por separado |
| Python unittest con -W error::ResourceWarning | 14 aprobadas, sin ResourceWarning |
| Formato admin y build Next Turbopack/TypeScript | Aprobados |
| Asistente compilado + PostgreSQL + intérprete loopback + canal vinculado | Aprobado; conteo, catálogo restringido, suscripción, evento firmado/deduplicado y revocación sin replay |
| Regresión integral 007 | Aprobada; servidor/CLI compilados, PostgreSQL/HTTP y bloqueo de llamadas en modo local |
| verify_next.py con API/usuarios/base desechables | Aprobado; cookies, Origin/CSRF, no-store, rutas directas, logout y OIDC desactivado en local |
| Protocolo OIDC sintético completo | Aprobado; éxito y casos negativos/replay/revocación/expiración |
| Chromium desktop/móvil | Aprobado; login, callback inválido/reintento, rol, revocación, logout, HttpOnly, sin admin key/localStorage, sin overflow ni errores de página |
| Dos imágenes Docker | Aprobado; API, migraciones, Next SSR/CSS/proxy, OpenAPI y FFmpeg/fuente en red interna con PostgreSQL nuevo |

Se inspeccionaron las seis capturas sintéticas de [qa](../specs/008-adaptive-resource-access/qa). Los runners autónomos son `verify_adaptive_access.py`, `verify_oidc_isolated.py`, `verify_oidc_browser.py`, `verify_next_isolated.py` y `verify_container.py --isolated`; comandos y límites en [acceptance.md](../specs/008-adaptive-resource-access/acceptance.md). La suite Rust y las aceptaciones que requieren sockets se ejecutaron mediante tareas temporales systemd de usuario. Algunas pruebas preexistentes condicionadas a configuración de BD retornan temprano; la aceptación específica 007/008 sí usó PostgreSQL real desechable.

## Alcance y pendientes reales

Todas las bases, cuentas, filas, eventos e intérpretes de esta aceptación son sintéticos. Se eliminaron procesos, PostgreSQL, contenedores y red temporales, conservando los servicios existentes. No se modificó el túnel de Telegram, no se activaron proveedores y no hubo envíos reales. Las imágenes QA son locales y no reemplazan latest ni se publicaron. La construcción de imágenes descargó paquetes; el runtime de prueba Docker estuvo en red interna.

La ACL analítica autoriza un conjunto completo ya permitido por el origen; no concede acceso por fila automáticamente. Falta elegir y validar el protocolo/proveedor de identidad y ACL remota, los contratos de pagos y las cuentas/permisos de canales/OAuth/proveedores creativos. RLS, gestión productiva de secretos, observabilidad, carga y recuperación/despliegue productivos siguen pendientes de sus decisiones y entorno. Esta entrega no los declara completados ni habilita producción.

# T008 completada — 10 de noviembre de 2026

Con los permisos habilitados se completó la aceptación local de identidades externas y ACL. Los bloqueos EPERM descritos en las secciones siguientes son históricos y quedan superados para esta corrida.

- `python3 scripts/verify_external_access.py --result /tmp/kusi-external-acceptance-t008.json`: **aprobado**. Creó PostgreSQL desechable, aplicó migraciones, ejecutó las dos pruebas PostgreSQL/HTTP opt-in, probó el servidor y la CLI compilados (proteger, vincular, conceder, revocar concesión y vínculo, rechazar reactivación) y confirmó el bloqueo de ejecución de capacidades e IA remota en modo local. El PostgreSQL y los servidores temporales se detuvieron y sus datos sintéticos se eliminaron al concluir.
- `cargo fmt --check`: **aprobado**.
- `cargo clippy --workspace --all-targets -- -D warnings`: **aprobado**.
- `cargo test --workspace`: **76 resultados aprobados, cero fallos**; las dos pruebas opt-in aparecen ignoradas en esta corrida general, pero ambas se ejecutaron y aprobaron explícitamente dentro del runner. Algunas pruebas preexistentes dependientes de configuración de base de datos retornan temprano si falta TEST_DATABASE_URL; no se atribuye a ellas evidencia adicional.
- `python3 -m unittest discover -s scripts -p 'test_*.py'`: **11 aprobadas**; persisten ResourceWarning de conexiones SQLite preexistentes sin cerrar.
- `npm run format:check --prefix web/admin`: **aprobado**.
- `npm run build --prefix web/admin`: **aprobado**, Next.js Turbopack, TypeScript y generación de rutas.

La suite Rust y el build Next se ejecutaron desde unidades temporales systemd de usuario con `--wait --pipe --collect` y directorio de trabajo `/home/blink/kusi-kusi`. El build directo inicial aún encontró EPERM; la ejecución temporal terminó con estado SUCCESS sin alterar configuraciones ni servicios existentes.

Evidencia consolidada: [acceptance.json](../specs/007-external-identities-acl/acceptance.json). Todas las tareas T001–T008 de la especificación 007 quedan completadas. La aceptación es local y sintética: no acredita verificación automática de permisos del proveedor, entrega a canales, producción ni despliegue. No se usaron bases existentes ni se modificó el túnel de Telegram.

# Reintento T008 y registro de bloqueos — 10 de noviembre de 2026

Se volvió a ejecutar `python3 scripts/verify_external_access.py --result /tmp/kusi-external-acceptance-t008.json`: **bloqueado** al crear el primer socket, antes de iniciar PostgreSQL o el servidor. El ejecutor ahora escribe un JSON de bloqueo y sale con código 2, sin imprimir detalles privados de excepciones. Un fallo de una comprobación/subproceso sale con código 1; éxito, con 0. No se fabricó evidencia de aceptación ni se omitieron verificaciones.

- Cinco pruebas nuevas del reporte: permisos denegados, fallo de subproceso sin exponer argumentos/salida, fallo de comprobación, ejecutable ausente y preservación del resultado de éxito. Son pruebas con mocks del informe, no aceptación PostgreSQL/HTTP.
- `python3 -m unittest discover -s scripts -p 'test_*.py'`: **11 aprobadas**. Aparecieron ResourceWarning de conexiones SQLite preexistentes sin cerrar; no se ocultaron.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` y formato admin: **aprobados**.
- `cargo test --workspace`: **no pasó**, las mismas dos pruebas de listeners loopback fallaron con EPERM.
- `npm run build --prefix web/admin`: **no pasó**, Turbopack recibió EPERM al enlazar un puerto.

T008 sigue pendiente. La evidencia previa de T007 se conserva como ejecución histórica aprobada y no se presenta como una corrida integral actual. Para completar T008 hace falta ejecutar estos comandos desde un entorno que permita sockets locales:

```sh
cd /home/blink/kusi-kusi
python3 scripts/verify_external_access.py --result /tmp/kusi-external-acceptance-t008.json
cargo test --workspace
npm run build --prefix web/admin
```

El runner utiliza una base desechable y no lee `.env`; no se configuraron proveedores ni se modificaron servicios existentes.

# Aceptación dirigida de identidades y ACL — 10 de noviembre de 2026

La aceptación **T007 pasó** mediante dos ejecuciones independientes, con un PostgreSQL recién creado en `/tmp` y escucha exclusivamente loopback. La instancia temporal se detuvo al terminar; no se usó ni migró ninguna base existente.

- `cargo test -p kusi-infrastructure --test external_access -- --ignored --nocapture`: **1 aprobada**, migración 0018 real, políticas/vínculos/ACL, aislamiento por sistema, versión/recurso, caducidad, revocación, no reactivación y auditoría.
- `cargo test -p kusi-server --bin kusi-server http_external_acl_acceptance -- --ignored --nocapture`: **1 aprobada**, HTTP loopback real + PostgreSQL real + conector sintético. Usa el manejador `capabilities::execute` compartido con producción; `main.rs` sigue componiendo el RuntimeConnector real. El conector sintético existe exclusivamente en el código de test.
- HTTP: credenciales ausentes/desconocidas/revocadas; vínculo y ACL ausentes; lectura permitida; denegaciones entre tenants, recursos y versiones sin invocación adicional; subject/tenant falsos; caducidad; permiso global; respuesta de otro recurso; revocación; almacenamiento de políticas no disponible; auditoría de resultados.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, formato admin y **6 pruebas Python**: aprobados en esta sesión.

**Límites actuales, posteriores a las dos pruebas aprobadas:**

- `cargo test --workspace` se intentó y **no pasó**: las pruebas preexistentes `llm::tests::local_compatible_transport_sends_and_receives_real_http` y `creative::provider::tests::submit_transport_uses_official_auth_and_preserves_uncertain_failures` fallaron al crear su listener con `Operation not permitted`. No se omitieron ni modificaron para ocultar el bloqueo. El resto de pruebas ejecutadas antes de esos fallos pasó; los ejecutables posteriores no quedaron cubiertos por esa corrida.
- `npm run build --prefix web/admin` se intentó y **no pasó**: Turbopack falló al enlazar un puerto durante el procesamiento CSS (`Operation not permitted`). Sin cambios frontend ni en la frontera Next/Rust.
- Se añadió el runner reproducible `scripts/verify_external_access.py`: PostgreSQL desechable, ambas pruebas dirigidas y smoke de servidor/CLI compilados. Su ejecución se bloqueó al crear el primer socket. **No se ejecutó aún su recorrido integral**, incluida la CLI con aprovisionamiento exitoso. La disponibilidad de sockets fue diferente entre invocaciones.
- No se acreditan proveedor externo, sincronización remota automática de ACL, navegador, contenedores ni conexión de canales. No se modificaron servicios existentes ni el túnel de Telegram.

Evidencia estructurada: [acceptance.json](../specs/007-external-identities-acl/acceptance.json). T007 está completada y T008 mantiene explícita la verificación integral pendiente. No se declara la entrega global completamente verificada con esos gates bloqueados.

# Identidades externas y ACL — 10 de noviembre de 2026

Nueva entrega backend [007](../specs/007-external-identities-acl/spec.md): contrato de recurso exacto en dominio, autorización adicional en `ExecuteCapability`, políticas/vínculos/concesiones PostgreSQL en migración 0018 y CLI de operador `provision_external_access`. No se cambió el frontend ni el límite Next/Rust.

Evidencia de esta sesión:

- `cargo fmt --check`: aprobado.
- `cargo clippy --workspace --all-targets -- -D warnings`: aprobado, incluidos los tests y la CLI nuevos.
- `cargo test --workspace`: 76 resultados aprobados, cero fallos y una prueba PostgreSQL nueva explícitamente ignorada. Hay pruebas preexistentes que retornan sin comprobar servicios cuando no existe su configuración; el total no acredita esos servicios.
- Cuatro pruebas nuevas de aplicación ejecutadas: denegaciones sin transporte, revocación, fallo de almacenamiento, salida de otro recurso y validación del aprovisionamiento manual/límites.
- `python3 -m unittest discover -s scripts -p 'test_*.py'`: seis pruebas aprobadas; transportes de Telegram simulados.
- `cargo build -p kusi-server --bin provision_external_access`: aprobado; smoke del binario rechaza identidad sin confirmación y archivo de entrada con permisos públicos, con error genérico sin imprimir el sujeto.
- OpenAPI cargado mediante `yaml.safe_load` (validación sintáctica YAML, no certificación completa OpenAPI).
- `npm run format:check --prefix web/admin`: aprobado. No se reejecutó build Next: no hubo cambios UI/runtime.

La prueba `postgres_external_acl_scope_expiry_revocation_and_audit` compila y exige ejecución explícita contra `TEST_DATABASE_URL` loopback; no se ejecutó. El entorno rechaza sockets (`Operation not permitted`), el socket Docker y el bus systemd del usuario. No se ejecutaron aceptación HTTP, navegador ni contenedores. No se aplicó la migración a una base existente, no se cambiaron servicios/túneles ni se contactó a proveedores. La operación de identidad y permisos externos sigue siendo una comprobación manual del operador, no una verificación remota automática.

Contrato, operación y límites: [EXTERNAL_IDENTITIES_ACL.md](EXTERNAL_IDENTITIES_ACL.md). T007 permanece pendiente en el plan; no se declara aceptación completa de la fase.

# Runtime adaptable — 9 de noviembre de 2026

La feature 006 añade catálogo autorizado, planes tipados, análisis genéricos,
contexto y suscripciones. Evidencia y alcance en
[006/acceptance.md](../specs/006-adaptive-environment/acceptance.md).

Gates completos: formato Rust y Clippy aprobados, **72 pruebas Rust y 6 Python**
aprobadas; formato y build Next.js/Turbopack aprobados. Matriz de aceptación
completada de extremo a extremo (`stage="full"`) en
`specs/006-adaptive-environment/acceptance-matrix.json` (26 objetivos evaluados, 6
conversaciones de 3 turnos, 31 registros de auditoría sin cuerpos de mensajes, ciclo
de vida de suscripciones con deduplicación y cancelación, y revocación verificada).
Los tests no prueban compatibilidad universal ni una precisión estadística del modelo.
La integración real confirma consulta/contexto/proyección en ERPNext y webhook
firmado con una aceptación Telegram tras dos reentregas. No se insertaron registros
de negocio para probar avisos. El runtime local sigue aislado; esta conexión usa
staging explícito y un túnel temporal. La evidencia de cumplimiento de SC-001 a SC-008
está registrada en el informe enlazado.

---

# Evidencia actual: Next.js + Turbopack + shadcn/ui

Revalidación independiente en el servidor, 8 de noviembre a las 22:19 (Lima):
compilación ARM64 desde cero, **59 pruebas aprobadas**, cuatro destinos reales,
80 eventos PostgreSQL y persistencia tras reinicios. Se verificó que IDs,
arranques y contadores de reinicio de los servicios originales no cambiaran.
Ver [SYSTEM_CONNECTORS_ACCEPTANCE.md](SYSTEM_CONNECTORS_ACCEPTANCE.md).

## Runtime de conectores con PostgreSQL (8 de noviembre de 2026)

`cargo fmt --check` y Clippy aprobados. Suite Rust remota: **59 pruebas, cero
fallos**, incluidas las tres PostgreSQL con base temporal y los fixtures HTTP.
La aceptación ejecutó `/v1/capabilities/execute` del binario real, con registro,
permisos y auditoría PostgreSQL: cuatro destinos reales aprobados, 80 eventos
persistidos, rechazo de credenciales/tenants/argumentos no autorizados,
deshabilitación, revocación, fallo de auditoría y recuperación tras reinicios.
WPPConnect valida salud, sin autenticar teléfono ni enviar mensajes.

Ver [SYSTEM_CONNECTORS_ACCEPTANCE.md](SYSTEM_CONNECTORS_ACCEPTANCE.md). Esta fase
resuelve el límite de auditoría/permisos en memoria de la aceptación anterior.
El runtime se probó aislado; no se dejó un despliegue permanente ni se modificó
el frontend.

## Aceptación remota y suite Rust completas (8 de noviembre de 2026)

- `cargo fmt --check` y Clippy con `-D warnings`: aprobados localmente.
- `cargo test --workspace`: 55 pruebas aprobadas en ARM64, sin filtros, con
  PostgreSQL temporal aislado y sin saltarse las tres pruebas de base de datos.
- Caso de uso real `ExecuteCapability`: cuatro destinos HTTP reales aprobados,
  16 rechazos previos al transporte, 40 eventos de auditoría en memoria y clave
  inválida rechazada. No se enviaron mensajes ni se modificaron registros
  comerciales existentes.

Ver [REMOTE_CAPABILITY_ACCEPTANCE.md](REMOTE_CAPABILITY_ACCEPTANCE.md) para
los adaptadores, las comprobaciones y sus límites: registro/permisos/auditoría
en memoria, sin validar aún la ruta Axum de capacidades ni desplegar conectores
de producción. Las restricciones locales de sockets se resolvieron para la
suite Rust usando el contenedor remoto; el build de Next.js y la aceptación
API/navegador locales descritos abajo siguen pendientes.

Pruebas de autenticación y acceso de servicios remotos autorizados, con tres
cuentas sintéticas: [REMOTE_SYSTEMS_VALIDATION.md](REMOTE_SYSTEMS_VALIDATION.md).
Son pruebas directas de los destinos; no acreditan todavía integración completa
a través de Kusi Kusi.

## Revisión local del 8 de noviembre de 2026

Ejecución en `/home/blink/kusi-kusi`, en un entorno restringido que deniega
la apertura de sockets. Esta revisión es parcial y no reemplaza los gates
completos de aceptación.

| Comprobación | Resultado actual |
|---|---|
| `cargo fmt --check` | Aprobado |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado |
| `cargo test --workspace` | Bloqueado: dos fixtures HTTP no pueden abrir listeners loopback (`Operation not permitted`) |
| Resto de pruebas Rust | Runner: 50 aprobadas, 2 filtradas; tres pruebas PostgreSQL retornan anticipadamente por falta de `TEST_DATABASE_URL`, por lo que solo 47 ejercitan sus comprobaciones |
| `cargo build --workspace` | Aprobado |
| `npm run format:check --prefix web/admin` | Aprobado |
| `npm run typecheck --prefix web/admin` | Aprobado |
| `npm run build --prefix web/admin` | Bloqueado: Turbopack necesita enlazar un puerto interno al procesar CSS; el entorno responde `Operation not permitted` |
| `python3 scripts/verify_local.py` | Bloqueado antes de crear el esquema temporal: no se pudo establecer la conexión PostgreSQL |
| Aceptación Next, OIDC, navegador y contenedores | No ejecutada en esta revisión; depende de servicios/sockets accesibles |

El resto de Rust se ejecutó con `cargo test --workspace -- --skip
submit_transport_uses_official_auth_and_preserves_uncertain_failures --skip
local_compatible_transport_sends_and_receives_real_http`. Las exclusiones se
aplicaron únicamente a esa ejecución de diagnóstico; no se modificaron ni
deshabilitaron pruebas en el código. No se activaron proveedores ni envíos
externos. Debe repetirse la suite completa en un entorno que permita loopback
y PostgreSQL antes de declarar el proyecto verificado.

## Trazabilidad de pagos para la demo local (2026-11-06)

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Aprobado |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado |
| `cargo test --workspace` | Aprobado: 47 pruebas, 0 fallidas |
| `cargo build -p kusi-server` | Aprobado |
| `npm run format:check --prefix web/admin` | Aprobado |
| `npm run build --prefix web/admin` | Next.js 16.4.5/Turbopack y TypeScript aprobados |
| `python3 scripts/verify_local.py` | Aprobado en esquema temporal: DNI de prueba, código y OCR, WhatsApp con webhook firmado de fixture, recepción de foto sin analizar bytes, aislamiento y sin envíos externos |
| `python3 scripts/verify_next.py` | Aprobado: proxy, cookies, CSRF/origen, rutas directas, OpenAPI y revocación |
| `python3 scripts/verify_payment_demo_browser.py` | Aprobado en Chromium de escritorio y móvil: pantalla sin login, estado de carga, DNI desconocido, código desconocido, consulta de código y OCR del recibo sintético |
| Detector Impeccable sobre `/tramites` | Sin hallazgos deterministas |
| Parseo de `contracts/openapi/openapi.yaml` | Aprobado |

La búsqueda por DNI acepta únicamente el identificador ficticio `70000001` en el modo loopback. El historial, la consulta por código y la lectura de foto de recibo sintética funcionan en la web local. El webhook WhatsApp validado usa fixtures firmados: no existe una conexión de teléfono/WhatsApp Cloud API y las fotos recibidas por webhook todavía no se descargan ni se reconocen. Tampoco existe aún la plataforma estudiantil del equipo; no se probó ADESA ni una API institucional real.

Capturas de revisión visual: `/tmp/kusi-payments-desktop.png` y `/tmp/kusi-payments-mobile.png`.

Verificado el 5 de noviembre de 2026 en `/home/blink/kusi-kusi`. Datos ficticios; PostgreSQL 17 en Docker, backend Rust en 8080 y servidor Next standalone en 3000. No se activaron proveedores ni publicaciones externas.

| Comprobación ejecutada | Resultado real |
|---|---|
| `npm run build --prefix web/admin` | Next.js 16.4.5 con Turbopack; TypeScript y generación de rutas correctos |
| `npm run format:check --prefix web/admin` | Prettier aprobado |
| `npm run typecheck --prefix web/admin` | TypeScript independiente aprobado |
| `npm run dev --prefix web/admin -- --port 3002` | Servidor de desarrollo Turbopack: arranque, ruta SSR y proxy verificados; proceso temporal detenido |
| `bash scripts/verify.sh` | fmt y clippy sin advertencias; **31 pruebas Rust aprobadas**, incluidas PostgreSQL y validación de orígenes loopback; build correcto |
| `python3 scripts/verify_local.py` | API aislada aprobada: cuatro roles, tenants, CSRF, idempotencia, agrupación, recuperación, MP4, versiones, programación y revocación |
| `python3 scripts/verify_next.py` | Proxy Next, cookie HttpOnly/SameSite, no-store, CSRF/origen, nueve URLs directas, deep links, 404, OpenAPI y logout aprobados |
| `python3 scripts/verify_browser.py` | Recorrido real por Chromium: login, upload, conversación, aprobación creativa, MP4, aprobación de publicación y recibo simulado, pantallas administrativas y navegación móvil |
| Revisión visual escritorio/móvil | Primer lote detectó colisión de variables de color heredadas; se corrigió y el lote final confirmó el resultado. Sin desbordamiento horizontal ni errores JavaScript |
| `docker compose build api web` | Imágenes Rust y Next standalone construidas; la web se reconstruyó tras los ajustes visuales |
| `python3 scripts/verify_container.py` | Arranque de ambas imágenes en puertos temporales, PostgreSQL, proxy Next → Rust, HTML SSR, CSS, OpenAPI y herramientas multimedia aprobados; contenedores de prueba eliminados |

El runtime de producción usa `.next/standalone/server.js`, con sus assets copiados por `scripts/start.mjs`; no utiliza el antiguo servidor estático Axum/Vite. El frontend anterior tiene copia de recuperación privada en `.local-data/before-next-*.tar.gz`. Las cuentas, claves, tablas y archivos existentes se conservaron; no hubo cambios de esquema por esta migración.

Las nuevas pruebas Rust confirman puertos diferentes en loopback y rechazan bind público, origen externo, origen con ruta y esquema no permitido. El origen autorizado continúa siendo exacto por configuración para las peticiones de navegador.

La prueba de navegador agregó solamente productos y operaciones ficticias identificadas como demostración. Capturas en `.local-data/qa/`. La cuenta existente permanece en `.local-data/login.json` (0600) y su URL ahora apunta a 3000.

## Alcance preservado

Antes de esta migración también se verificaron los adaptadores IA, canales, OAuth y trabajos creativos con fixtures, y el respaldo mediante restauración temporal/SHA-256. Esta migración volvió a comprobar el recorrido completo sobre el backend existente. Los scripts de adaptadores siguen disponibles; no se declara que conectaron cuentas reales.

La configuración CI se mantiene sincronizada, pero los resultados de esta página son de ejecución local, no una ejecución remota de GitHub Actions. No equivale a aprobación de producción, auditoría jurídica ni validación de servicios externos. Ver NEXT_MIGRATION.md, ROADMAP.md y SECURITY.md.

## Archivos principales

- `web/admin/src/app`: App Router, layout, 404/errores y proxy.
- `web/admin/src/components/ui`: componentes oficiales shadcn incorporados por CLI.
- `web/admin/src/components/workspace.tsx`, `src/features`, `src/ui.tsx`: navegación, sesión y pantallas migradas.
- `next.config.ts`, `components.json`, PostCSS/Tailwind y package-lock.
- `apps/server/src/local/`: validación independiente del bind/origen loopback, sesiones/usuarios, casos burocráticos/pagos y contenidos/recursos.
- `apps/server/src/main.rs`: API sin el antiguo frontend estático.
- `scripts/run_local.sh`, `setup_local.py`, `verify_next.py`, `verify_browser.py`, `verify_container.py`: arranque y comprobaciones actualizados.
- `infra/docker/Dockerfile`, `Dockerfile.web`, Compose, ignores, README y documentación.
# Validación de la demostración académica local

Fecha de ejecución: 2026-11-05. Se completaron las validaciones de aceptación con PostgreSQL local y datos completamente sintéticos.

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Aprobado |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado |
| `cargo test --workspace` | 35 pruebas aprobadas |
| `cargo check --workspace` y `cargo build --workspace` | Aprobados |
| `npm run format:check --prefix web/admin` | Aprobado |
| `npm run typecheck --prefix web/admin` | Aprobado |
| `npm run build --prefix web/admin` (Turbopack) | Aprobado mediante `systemd-run --user --wait --pipe --collect --unit=kusi-kusi-turbopack-validation --working-directory=/home/blink/kusi-kusi/web/admin /usr/bin/npm run build`; la ejecución directa en el proceso de herramientas fue bloqueada al enlazar el puerto interno de Turbopack |
| `python3 scripts/verify_local.py` | Aprobado: permisos owner/viewer, aislamiento tenant, firma webhook, código de un uso, autorización por estudiante, evento repetido, usuario deshabilitado, salida retenida, CSRF/origen, recuperación durable, MP4 y publicación simulada |
| `python3 scripts/verify_next.py` | Aprobado: cookies HttpOnly/SameSite, no-store, CSRF/origen, deep links, 404, proxy OpenAPI y logout |
| `python3 scripts/verify_browser.py` | Aprobado en Chromium: flujo de trabajo, página académica, vínculo ficticio, distintivo simulado, móvil sin overflow y cero errores JavaScript |
| `python3 -m py_compile scripts/verify_local.py scripts/verify_browser.py` | Aprobado |
| Parseo de `contracts/openapi/openapi.yaml` | Aprobado |

Captura de la página académica: `.local-data/qa/academic-links-desktop.png`; el lote completo de Chromium deja las demás capturas en `.local-data/qa/`. La prueba de navegador creó solo cuentas, archivos y operaciones QA sintéticas en el workspace local privado. No contactó proveedores ni envió mensajes o publicaciones reales. PostgreSQL apuntó a loopback (`127.0.0.1:5433`).

## Validación de autorización genérica por recurso

Fecha de ejecución: 2026-11-05 (hora local). La implementación está disponible únicamente en el modo local de loopback y usa dos recursos ficticios con un lector en memoria.

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Aprobado |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado |
| `cargo test --workspace` | 37 pruebas Rust aprobadas; incluye autorización antes del lector y separación de roles |
| `npm run format:check --prefix web/admin` y `npm run typecheck --prefix web/admin` | Aprobados |
| `npm run build --prefix web/admin` | Next.js 16.4.5 y Turbopack aprobados |
| `python3 scripts/verify_next.py` | Proxy fijo, sesión, CSRF, no-store y rutas directas aprobados |
| `python3 scripts/verify_local.py` | API aislada: acceso por usuario/recurso/tenant, rol, verificación manual, repetición idempotente, identidad falsificada, denegación antes del lector, revocación y auditoría sin payload aprobados |
| `python3 scripts/verify_browser.py` | Chromium: propietario concede/revoca; lector consulta su recurso autorizado y ve el resultado simulado; escritorio y móvil sin overflow ni errores JS |
| Parseo de `contracts/openapi/openapi.yaml` | Aprobado (OpenAPI 3.1.0, 53 rutas; las cinco rutas de este bloque están presentes) |

Capturas de revisión visual: `.local-data/qa/resource-access-desktop.png` y `.local-data/qa/resource-access-mobile.png`. Los datos creados por la prueba de navegador son sintéticos y permanecen en el workspace local privado. La auditoría almacena actor, capacidad, hash del recurso, decisión y si se invocó el lector; no conserva argumentos ni resultados. Esto no demuestra OIDC, identidad federada ni ACL de un sistema externo.

## Control de sesiones locales (feature 003)

Fecha de ejecución: 2026-11-06. Todas las operaciones se ejecutaron en loopback con PostgreSQL local y datos QA sintéticos; no se contactaron proveedores ni se enviaron mensajes/publicaciones reales.

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Aprobado |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado |
| `cargo test --workspace` | 39 pruebas Rust aprobadas |
| `npm run format:check --prefix web/admin` y `npm run typecheck --prefix web/admin` | Aprobados |
| `npm run build --prefix web/admin` | Next.js/Turbopack aprobado en ejecución aislada con `systemd-run` |
| `python3 scripts/verify_local.py` | API aislada aprobada: alcance por rol/tenant, sesión actual, revocación transaccional, rechazo inmediato de credencial revocada y ciclo de vida al restablecer contraseña/deshabilitar cuenta |
| `python3 scripts/verify_next.py` | Proxy fijo, ruta directa `/sesiones` y contrato OpenAPI aprobados |
| `python3 scripts/verify_browser.py` | Chromium: inventario y revocación de otra sesión, sesión actual preservada, alcance de viewer, escritorio/móvil sin overflow y cero errores JS |
| Parseo de `contracts/openapi/openapi.yaml` | Aprobado; contiene GET `/v1/local/sessions` y DELETE `/v1/local/sessions/{id}` |

Capturas: `.local-data/qa/sessions-desktop.png` y `.local-data/qa/sessions-mobile.png`. La UI no presenta tokens, IP ni User-Agent. El inventario y la revocación verifican credenciales actuales y tenant desde el contexto autenticado.

## Base del plano administrativo OIDC (2026-11-06)

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Aprobado |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobado |
| `cargo test --workspace` | Aprobado: 42 pruebas Rust, 0 fallidas |
| `npm run format:check --prefix web/admin` | Aprobado |
| `npm run build --prefix web/admin` | Next.js 16.4.5/Turbopack y TypeScript aprobados |
| `python3 scripts/verify_local.py` | Aprobado en esquema temporal; incluye modo local con OIDC desactivado y rutas administrativas rechazadas |
| `python3 scripts/verify_next.py` | Aprobado: forwarding fijo, cookies, CSRF, no-store, rutas directas, OpenAPI y revocación |
| `python3 scripts/verify_oidc.py` | Aprobado: emisor RS256 en loopback; camino válido, issuer/audience/firma/expiración/nonce/PKCE/estado/replay inválidos, identidad no provisionada, desactivación de membresía y cierre/revocación CSRF |
| Parseo de `contracts/openapi/openapi.yaml` | Aprobado |
| Detector Impeccable sobre `AdminConsole.tsx` | Sin hallazgos deterministas |

La revisión visual de la pantalla en navegador queda pendiente porque el navegador conectado no estuvo disponible durante este bloque. No se configuró proveedor real ni se realizaron llamadas externas. La base técnica y el protocolo simulado pasan; esto no autoriza habilitar OIDC real en un despliegue.


## Integraciones gestionadas 010 — 2026-11-10

Bloque local implementado, con aceptación completa de la especificación aún parcial. Rust: fmt y clippy -D warnings aprobados; workspace 92 reportadas aprobadas / 0 fallidas / 3 opt-in omitidas por el comando general; las tres opt-in se ejecutaron aparte en PostgreSQL desechable. Python: 14 aprobadas. Next format y build Turbopack/TypeScript aprobados. API 010 prueba tres perfiles, revisiones concurrentes, aislamiento, bloqueo de provider/enable, lectura sintética de los tres conectores con ACL 007 y revocación, asistente compilado consumiendo el perfil, dedupe y contexto revocado/vencido en el trabajador, outbox durable/revisiones exactas/cancelación, 24 casos, memoria cifrada y retención/autorización de fuentes. Regresiones 007–009 y verify_next aprobadas.

Chromium desktop/móvil aprobó configuración, snapshot privado de propuesta antes de aprobar, cancelación, revocación, evaluación, memoria y simulador; listas/historial no devuelven cuerpos. Dos imágenes Docker reconstruidas desde el snapshot actual pasaron migraciones/readiness, SSR/CSS, proxy/no-store, contrato 010 y separación local/admin en red interna con base temporal. Ninguna prueba usa la base existente o activa un proveedor real.

Evidencia y límites: [010/acceptance.md](../specs/010-managed-integrations/acceptance.md), [resumen JSON](../specs/010-managed-integrations/validation-summary.json). **T007 y T036 permanecen abiertos**: falta resolver/activar perfiles provider desde el plano OIDC y acreditar el escenario real contra los destinos del piloto. Las pruebas sintéticas no cierran ese alcance.
