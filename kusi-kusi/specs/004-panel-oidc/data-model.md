# Data Model: Plano administrativo OIDC

La configuración del proveedor vive solo en variables del servidor. No se guarda en PostgreSQL ni se acepta del navegador.

## `admin_identities`

- Clave primaria `(issuer, subject)`; `subject` tiene un máximo de 255 caracteres.
- `disabled_at` deshabilita la identidad federada.
- Es independiente de las cuentas y sesiones del workspace local.

## `admin_memberships`

- Clave `(issuer, subject, organization_id)`; una identidad puede tener varias organizaciones.
- `organization_id` refiere a una organización Kusi Kusi existente.
- `role` solo admite `owner`, `editor`, `approver` o `viewer`; `disabled_at` permite retirar el acceso sin borrar el vínculo.
- Ni rol ni organización proceden de claims OIDC o parámetros del navegador.

## `admin_oidc_attempts`

- `state_hash` identifica la transacción; `browser_hash` la liga a la cookie HttpOnly de corta duración.
- `nonce` y `code_verifier` se cifran con XChaCha20-Poly1305 usando una clave de estado del servidor.
- `expires_at` limita la transacción; el callback elimina y obtiene la fila en una sola operación atómica, así que un segundo uso no encuentra secretos.
- Un inicio nuevo elimina intentos vencidos y registros consumidos heredados; el índice de vencimiento acota la búsqueda.

## `admin_sessions`

- Guarda solo el hash del token opaco, la identidad asociada, vencimiento y revocación.
- La cookie es HttpOnly y SameSite Lax para permitir el retorno OIDC de nivel superior; usa Secure cuando el origen es HTTPS.
- La vista de sesión consulta las membresías activas en cada petición. Cerrar sesión valida Origin y CSRF, revoca en PostgreSQL y limpia la cookie.
- Las APIs de negocio del plano administrativo aún no están expuestas; cada endpoint futuro debe resolver esta sesión y aplicar RBAC en servidor.

## Configuración

- `KUSI_ADMIN_OIDC_ISSUER`, client ID, client secret y redirect URI deben configurarse juntos. Redirect URI debe coincidir exactamente con `{KUSI_ADMIN_PUBLIC_ORIGIN}/v1/admin/auth/oidc/callback`.
- `KUSI_ADMIN_OIDC_STATE_KEY` aporta 32 bytes en hexadecimal para cifrar material temporal.
- Todo OIDC se rechaza con `KUSI_LOCAL_MODE=true`; el proveedor permanece desactivado si la configuración está ausente.
