# Quickstart: plano administrativo OIDC

## Workspace local

1. Deja sin configurar las variables `KUSI_ADMIN_OIDC_*` y ejecuta `./scripts/run_local.sh`.
2. Abre `/admin`; debe indicar que el workspace local no ofrece login corporativo.
3. Ejecuta `python3 scripts/verify_local.py`. El chequeo confirma `oidc_enabled=false`, `/v1/admin/auth/oidc/start` en 404 y la sesión sin autenticar en 401.
4. Ejecuta `python3 scripts/verify_next.py` para verificar forwarding fijo, cookies, CSRF, no-store y rutas directas.

## Prueba OIDC aislada

Ejecuta `cargo build -p kusi-server` y luego `python3 scripts/verify_oidc.py`. El script crea un esquema PostgreSQL temporal, enlaza el emisor sintético y la API a loopback, provisiona una identidad ficticia y cubre login/logout, identidad desconocida, firma/issuer/audience/expiración/nonce/PKCE inválidos, estado incorrecto y replay. No usa cuentas ni secretos reales.

No habilitar configuración OIDC real ni exponer la API a una red pública dentro de este bloque.
