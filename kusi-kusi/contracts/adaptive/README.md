# Contrato de composición adaptable v1

CLI: `adaptive_assistant <configuración-privada-registrada>`.
Una petición JSON por stdin, una respuesta JSON por stdout. La configuración y la
identidad son del transporte, nunca del mensaje/modelo. No es un endpoint Axum.

Consulta: `{"message":"Cuenta los registros disponibles","request_id":"id-estable"}`.
Evento: `{"event":{"resource":"fuente::recurso","event_id":"id-estable","kind":"created","values":{},"test":false}}`.
Ambas formas son excluyentes. La respuesta contiene `status`, `text` y `evidence`.
Estados: completed, clarification, blocked, unavailable, deferred, unknown,
notification o ignored. Evidence incluye fuente, recurso, versión, filtros,
filas observadas y resultados tipados. El texto no sustituye a esos datos.

Decisiones del intérprete (serde estricto): query(plan), clarify(question,draft),
subscribe(resource,event,filters), cancel(subscription), reset,
unsupported(capability,explanation,requires_approval). Un plan contiene
version y queries(resource,filters,analyses). No acepta SQL, código, URLs ni identidad.
Los identificadores de cancelación deben pertenecer al actor autenticado.

Callback de staging: POST `/adaptive/events/{source}`, firma HMAC-SHA256 base64
sobre bytes originales en `X-Frappe-Webhook-Signature`. Cuerpo de evento según el
contrato anterior sin el envoltorio `event`. Fuente y recurso deben coincidir con
la configuración. `test=true` etiqueta explícitamente el aviso de prueba; en
Frappe solo permite documentos sintéticos con el prefijo reservado y delegación
watch. No crea un registro de negocio.

HTTP: 200 procesado/ignorado, 400 formato, 401 firma, 403 delegación revocada,
404 fuente desconocida, 413 tamaño, 503 dependencia o resultado no confirmado.
El callback no reintenta envíos inciertos automáticamente.

RPC Frappe interno: discover, read, watch y authorize_event. watch recibe un namespace
calculado por infraestructura a partir de la organización y principal registrados;
no procede de la propuesta del modelo. El nombre idempotente del hook incluye ese
namespace, la fuente y el recurso. authorize_event comprueba lectura del documento.

## Contexto con ACL (008)

La configuración privada admite `access.database_url_env` y `access.resources`, un mapa de id analítico a `{adapter, capability, version, resource_ref}`. El principal es el hash de una credencial individual, resuelto desde el canal/configuración registrados. `KUSI_ADAPTIVE_REQUIRE_ACL=true` impide omitir la configuración. No se añaden campos tenant/principal al contrato de entrada. El Environment protegido exige política, permiso global, vínculo y concesión vigentes antes de leer o autorizar eventos. Una ACL sobre un recurso analítico es de conjunto completo, no de fila. Detalles y límites en docs/ADAPTIVE_RUNTIME.md.

La fuente puede declarar `event_secret_file` opcional y absoluto para validar eventos firmados, sin mezclar secretos de transporte con el manifiesto de datos.

## Fuente GraphQL loopback

`SourceConfig.kind = graphql_loopback` usa el mismo RPC `discover/read`, con catálogo versionado derivado del esquema y consultas paginadas delegadas. Los destinos, credenciales, recursos autorizados y categorías opcionales están en la configuración privada; nunca en el plan. Las mutaciones de negocio, watch y authorize_event se rechazan. No se agregan nombres comerciales al contrato.

La composición optativa del worker local requiere un perfil privado en `<storage>/adaptive-bindings/<organization>/<user>/<sha256(channel:sender)>.json`, correspondiente al usuario resuelto por la vinculación vigente. El perfil debe pasar `validate_loopback`: fuentes exclusivamente locales, sin suscripciones y modelo loopback sin clave. El servidor conserva el bloqueo de entregas y llamadas externas del modo local. Véase `docs/MUBARAK_ADAPTIVE.md`.
