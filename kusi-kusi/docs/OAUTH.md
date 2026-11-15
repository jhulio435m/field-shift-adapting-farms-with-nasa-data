# OAuth social

Backend implementado para vincular cuentas mediante Facebook Login, Instagram Login y TikTok Login Kit web. Este módulo autoriza cuentas externas; no sustituye el OIDC pendiente del panel ni publica contenido. Las rutas de operador requieren clave administrativa y bearer del tenant.

## Flujo

1. `POST /v1/oauth/{provider}/start` determina el tenant por la credencial verificada y selecciona su perfil configurado. Devuelve la URL de autorización y fija una cookie `__Host-kusi_oauth` con Secure, HttpOnly, SameSite=Lax, Path=/ y vigencia de diez minutos.
2. El navegador visita la URL oficial y el usuario presta o deniega consentimiento.
3. El proveedor vuelve a `GET /v1/oauth/{provider}/callback`. La aplicación compara proveedor, hash del estado, hash de la cookie y vencimiento en una operación transaccional. El estado se consume una sola vez antes del intercambio externo.
4. El backend intercambia el código, comprueba los permisos realmente concedidos y guarda tokens cifrados. La respuesta contiene únicamente id de conexión, proveedor, cuenta externa, scopes, estado y expiración.

El navegador que inicia el flujo debe ser el que recibe el callback. Un `curl` que obtiene la URL no coloca la cookie en otro navegador: copiar solo el enlace no es un flujo válido. La UI futura debe iniciar mediante una petición autenticada del mismo origen. No colocar claves de Kusi Kusi en enlaces ni parámetros OAuth.

## Configuración

`KUSI_OAUTH_PROFILES` contiene `{ "tenant-uuid": { "facebook": {...}, "instagram": {...}, "tiktok": {...} } }`. Ver [ejemplo](../examples/oauth-profiles.json). Reemplazar UUID, App IDs/client key, dominio, versión Graph API y scopes con los habilitados en cada app. `client_secret_env` referencia una variable protegida; no contiene el secreto. Una configuración inválida o una referencia de secreto ausente impide iniciar el servicio.

Registrar exactamente estos callbacks HTTPS en las consolas correspondientes:

- `https://dominio/v1/oauth/facebook/callback`
- `https://dominio/v1/oauth/instagram/callback`
- `https://dominio/v1/oauth/tiktok/callback`

No se admiten query strings, fragmentos ni credenciales en la redirect URI. Los endpoints de autorización y tokens son fijos, no URLs proporcionadas por un usuario. La configuración no prueba que la app tenga permisos aprobados o que el usuario sea elegible.

La versión Graph API del ejemplo es explícita y debe contrastarse con la app; no es una selección automática de “última versión”. Instagram usa Instagram Login y sus App ID/Secret propios, que no deben confundirse con los de WhatsApp o con otro flujo de Facebook Login.

## Protección de credenciales

Definir `KUSI_OAUTH_KEYRING` como un JSON de identificador de clave a nombre de variable de entorno, por ejemplo `{"v1":"OAUTH_TOKEN_KEY_V1"}`. `KUSI_OAUTH_ACTIVE_KEY=v1` elige la clave de escritura. `OAUTH_TOKEN_KEY_V1` debe contener 32 bytes aleatorios codificados como 64 caracteres hexadecimales. Guardar la clave fuera del código y conservarla de forma recuperable; perderla impide descifrar conexiones existentes.

Se utiliza XChaCha20-Poly1305 de RustCrypto con nonce generado por la biblioteca. Los datos asociados vinculan el cifrado al tenant, UUID de conexión y propósito. Los tokens de acceso y refresh se cifran juntos. El verificador PKCE, cuando existe, se guarda cifrado y se elimina tras consumir la sesión. El estado y la cookie solo se persisten como hashes. No se almacena el código de autorización.

Rotación: añadir la clave nueva al keyring, mantener la anterior y cambiar `KUSI_OAUTH_ACTIVE_KEY`. Nuevas vinculaciones y renovaciones usan la nueva clave; las conexiones antiguas siguen descifrándose con su key id. No retirar una clave antigua hasta haber migrado todas sus conexiones. No hay aún un endpoint de reencrypt masivo. Las copias de seguridad necesitan tanto la base como las claves, conservadas de manera separada.

Las respuestas y eventos no incluyen tokens, client secrets, códigos OAuth ni verificadores. La instrumentación entrante omite query strings. Los GET oficiales de Meta pueden requerir secretos en query al intercambiar tokens; no habilitar captura de URLs/payloads del cliente HTTP, proxy o APM para esos endpoints. Aplicar TLS y una política equivalente de logs en el reverse proxy público.

## Protocolos y permisos

- Facebook: autorización con versión Graph explícita; intercambio de código y de token corto por largo; consulta server-side de `/me` y `/me/permissions`. Se comprueban todos los scopes requeridos. La conexión representa al usuario de Facebook autorizado, no a una Página seleccionada. Descubrimiento/selección de Páginas y sus tokens para publicar pertenecen al siguiente módulo.
- Instagram: autorización independiente; intercambio multipart en api.instagram.com; se acepta el envelope `data[]` documentado; se comprueban `permissions` y se intercambia por token largo en graph.instagram.com.
- TikTok: autorización web y POST form a `/v2/oauth/token/`; `open_id`, scopes y vigencias se toman de la respuesta. No se presupone que consentir unos scopes concede todos los solicitados.

PKCE: la utilidad S256 y la persistencia segura están implementadas y probadas con el vector RFC 7636. Solo se habilita en el proveedor simulado en esta entrega. Los adaptadores nativos son clientes confidenciales web con client secret, estado y cookie de navegador; no se envían parámetros PKCE no confirmados. TikTok documenta `code_verifier` como obligatorio para móvil/escritorio; esos tipos de cliente no están implementados. Configurar `pkce:true` en los perfiles web nativos se rechaza.

Una combinación proveedor/app/cuenta externa solo puede pertenecer a un tenant. Un callback de otro tenant no reemplaza sus tokens. La transferencia explícita de propiedad entre organizaciones no está implementada.

## Renovación

`POST /v1/oauth/connections/{id}/refresh` protege la operación con un lease de 120 segundos y compare-and-swap al persistir el resultado. TikTok reemplaza atómicamente access y refresh tokens. Instagram solo se renueva si sigue vigente y tiene al menos 24 horas de antigüedad. Facebook requiere volver a autorizar, no se presenta su intercambio de token largo como un refresh token indefinido.

`KUSI_OAUTH_REFRESH_WORKER=true` habilita mantenimiento cada minuto, con hasta veinte conexiones por pasada y anticipación de diez minutos. Solo renueva TikTok/Instagram para tenants y proveedores configurados, y respeta los mismos controles. Está desactivado por defecto. No renueva Facebook.

Una renovación fallida o incierta queda en `needs_reauth`: no se reintenta automáticamente con el refresh token potencialmente rotado. Una caída durante la operación se detecta por el lease vencido; el mantenimiento o una nueva solicitud la reconcilia. El cliente debe iniciar otra autorización. No hay garantía de disponibilidad de los proveedores.

## Desconexión

`POST /v1/oauth/connections/{id}/disconnect` deshabilita primero el acceso local (`revoking`). TikTok y Facebook tienen solicitud de revocación implementada. Cuando se confirma, se borra el ciphertext y el estado pasa a `disconnected`.

Si no se confirma, devuelve 202 y `revoke_pending`, manteniendo el cifrado para reconciliación y sin habilitar uso local. El contrato remoto de revocación de Instagram no pudo verificarse de forma fiable: en esta entrega su desconexión devuelve `revoke_pending` y requiere revocar el acceso desde los controles oficiales de la cuenta. No se afirma que eliminar acceso local revoque el permiso en Meta. Tampoco se marca como exitosa una revocación incierta tras un fallo de red.

## Estado de implementación y pruebas

`GET /v1/oauth/connections` devuelve hasta cien conexiones del tenant sin secretos. Los cambios quedan en `oauth_events`. `oauth_sessions` y los perfiles almacenan referencias de configuración y metadatos de autorización; se necesita una política de retención/eliminación en producción. No hay todavía endpoints de deauthorization callbacks del proveedor ni solicitudes de borrado de datos para revisión de apps.

Ejecutar `./scripts/verify.sh` y `python3 scripts/verify_oauth.py`. La prueba de API usa una base/esquema temporal, dos tenants, un proveedor simulado y claves de cifrado aleatorias. Comprueba cookie, estado, replay, vencimiento, proveedor incorrecto, propiedad de cuenta, cifrado, rotación de claves, renovación interrumpida y desconexión. No contacta cuentas reales. Los flujos de Meta/TikTok necesitan validación real con las apps y HTTPS registrados; el simulador no acredita que sus permisos hayan sido aprobados.

No se implementó la UI de vinculación, la publicación social, selección de Páginas de Facebook, OAuth móvil/escritorio ni OIDC administrativo.

## Fuentes oficiales consultadas

- Facebook manual flow: https://developers.facebook.com/docs/facebook-login/guides/advanced/manual-flow/
- Facebook long-lived tokens: https://developers.facebook.com/docs/facebook-login/guides/access-tokens/get-long-lived/
- Instagram Business Login y renovación: https://developers.facebook.com/docs/instagram-platform/instagram-api-with-instagram-login/business-login/
- TikTok web: https://developers.tiktok.com/docs/en/login-kit-web
- TikTok token exchange: https://developers.tiktok.com/docs/en/oauth-get-a-user-access-token
- TikTok rotation: https://developers.tiktok.com/docs/en/oauth-refresh-a-user-access-token
- TikTok revocation: https://developers.tiktok.com/docs/en/oauth-revoke-a-user-access-token
- RustCrypto AEAD: https://docs.rs/chacha20poly1305/0.11.0/chacha20poly1305/

Las páginas Meta devolvieron 429 por el navegador de investigación; sus contenidos oficiales pudieron leerse mediante HTTP desde el entorno. Se utilizaron esos contenidos para los contratos implementados, sin validar acceso con una cuenta real.
