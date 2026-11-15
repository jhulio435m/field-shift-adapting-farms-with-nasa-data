# Tasks: Plano administrativo OIDC separado

## Completado en código (pendiente de gates de aceptación)

- [x] T001 Registrar `openidconnect` 4.1.0 y la política de issuer, audience, firma, nonce, PKCE y callback en `specs/004-panel-oidc/research.md`.
- [x] T002 Crear identidades, membresías, sesiones e intentos de un uso en `migrations/0015_admin_identity_sessions.sql`.
- [x] T003 Definir roles, identidad administrativa y puertos/casos de uso en `crates/domain/src/admin_identity.rs` y `crates/application/src/admin_identity.rs`.
- [x] T004 Implementar repositorio PostgreSQL y cifrado de datos temporales en `crates/infrastructure/src/admin_identity.rs`.
- [x] T005 Implementar cliente OIDC con redirects HTTP desactivados, HTTPS salvo loopback, RS256, Authorization Code y PKCE S256 en `crates/infrastructure/src/admin_oidc.rs`.
- [x] T006 Añadir inicio, callback, sesión y cierre en `apps/server/src/admin_auth.rs`; mantener las rutas separadas del workspace local.
- [x] T007 Añadir aprovisionamiento explícito de identidad/membresía en `apps/server/src/bin/provision_admin_identity.rs`.
- [x] T008 Añadir pantalla de entrada y membresías en `web/admin/src/app/admin/page.tsx` y `web/admin/src/features/AdminConsole.tsx`.
- [x] T009 Documentar endpoints y cookie en `contracts/openapi/openapi.yaml`; documentar configuración comentada en `.env.example`.
- [x] T010 Documentar los límites y el estado del flujo en `docs/SECURITY.md`, `docs/LOCAL_WORKSPACE.md` y `docs/ARCHITECTURE.md`.

## Aceptación pendiente

- [x] T011 Crear un emisor OIDC sintético loopback y ejercitar el camino válido en `scripts/oidc_test_issuer.py` y `scripts/verify_oidc.py`.
- [x] T012 Comprobar identidad desconocida, firma, issuer, audience, expiración, nonce, PKCE, estado incorrectos y replay sin emitir sesión.
- [x] T013 Comprobar por HTTP con cookie jar login, expiración, CSRF, logout, revocación y desactivación de membresía con el emisor loopback.
- [x] T014 Extender `scripts/verify_local.py` y `scripts/verify_next.py` para comprobar que modo local informa OIDC desactivado y rechaza las rutas; proxy fijo/cookies/no-store.
- [x] T015 Revisar fallos del callback y la interfaz con viewport real, y registrar evidencia actual en `docs/VALIDATION.md`. Aprobado en 008: Chromium desktop/móvil, callback inválido/reintento, membresía, revocación y logout.
- [x] T016 Ejecutar `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run format:check --prefix web/admin`, `npm run build --prefix web/admin`, `python3 scripts/verify_local.py`, `python3 scripts/verify_next.py` y `python3 scripts/verify_oidc.py`.

## Bloqueo operativo

- [ ] T017 Antes de activar un emisor real, acordar el proveedor, callback HTTPS, política MFA, entrega/rotación de secretos y piloto. No habilitar credenciales reales desde esta tarea.

## Alcance

El plano administrativo de producción es distinto del workspace local. En esta entrega solo se implementan la entrada OIDC, sesión, cierre y vista de membresías provisionadas. Las funciones futuras que cambien datos deberán autenticar sesión y aplicar RBAC en servidor. La autenticación OIDC permanece desactivada por defecto y no se configura en modo local.
