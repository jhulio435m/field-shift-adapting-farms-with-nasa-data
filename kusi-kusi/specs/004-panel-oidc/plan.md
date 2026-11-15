# Implementation Plan: Inicio de sesión del panel con OIDC

**Branch**: `004-panel-oidc` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-panel-oidc/spec.md`

## Summary

Añadir un camino OIDC al panel sin acoplar el dominio a un proveedor. El desarrollo y aceptación del protocolo pueden usar un emisor sintético en loopback. Sin embargo, la inspección de la composición actual detectó un límite previo: la UI existente es el espacio local de demostración, sus rutas administrativas pasan por `local::routes()`, y el almacenamiento/sesión `LocalStore` solo se crea con `KUSI_LOCAL_MODE=true`. El modo local exige loopback y bloquea conexiones externas. Por tanto, no es seguro ni suficiente agregar OIDC dentro de `apps/server/src/local.rs` y declarar habilitado el panel de producción.

La identidad federada debe vincularse por `(issuer, subject)` a una cuenta y organización administradas por el plano de control; correo, nombre, claims de tenant y datos del navegador no deben asignar rol. Se decidió avanzar con un plano de control de producción separado del workspace local. La primera entrega incluye inicio de sesión, provisionamiento previo y lectura de membresías/sesión; las pantallas de operación comercial y APIs administrativas completas quedan en bloques posteriores. El proveedor real sigue deshabilitado hasta su configuración operativa aprobada.

## Technical Context

**Language/Version**: Rust estable del workspace y TypeScript/Next.js App Router existentes.

**Primary Dependencies**: Axum, SQLx/PostgreSQL, fixed-upstream Next proxy y `openidconnect` 4.1.0. La biblioteca valida issuer, audience, firma/JWKS, expiración y nonce; el adaptador limita el algoritmo a RS256 y desactiva redirects HTTP.

**Storage**: PostgreSQL para vínculos `(issuer, subject)` y transacciones temporales de autenticación; issuer/client ID y referencia del secreto se inyectan desde configuración del servidor y no se escriben en base de datos. Los secretos no se guardan en frontend, responses ni logs.

**Testing**: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, pruebas aisladas de PostgreSQL, `verify_local.py`, `verify_next.py`, `verify_browser.py`, formato/build Next y un emisor OIDC simulado solo en loopback.

**Target Platform**: Linux; API `127.0.0.1:8080`, Next `127.0.0.1:3000` para desarrollo.

**Project Type**: Aplicación web multi-tenant con API Rust y panel Next.js.

**Performance Goals**: Un callback correcto completa dentro de los límites del timeout HTTP ya configurado, sin bloquear workers durante llamadas remotas. Medir por separado el tiempo del proveedor en el piloto.

**Constraints**: El modo local rechaza cualquier configuración OIDC y las rutas de autenticación responden 404. La configuración del plano administrativo exige HTTPS excepto en loopback de prueba. Mantener cookies HttpOnly, CSRF, Origin, no-store y revocación. Aplicar RBAC servidor antes de añadir endpoints que gestionen recursos; este bloque solo muestra membresías provisionadas. No registrar códigos, tokens, secretos, claims completos ni cuerpos de respuesta. No convertir correo en identificador de vínculo.

**Scale/Scope**: Un emisor registrado por instancia para el piloto inicial; múltiples organizaciones pueden coexistir mediante vínculos de cuenta explícitos. El esquema conserva `issuer` junto a `subject` para evitar colisiones y permitir evaluación posterior de varios emisores, sin incluir gestión de múltiples proveedores en esta fase.

## Constitution Check

- **Generic Integration Core**: Pasa; el puerto de identidad expresa claims verificados de forma neutral y los detalles OIDC viven en infraestructura.
- **Layered Architecture**: Pasa; casos de uso/puertos en `crates/application`, adaptador OIDC en `crates/infrastructure`, rutas Axum en `apps/server`, flujo de login en Next.
- **Tenant Authorization and Privacy**: Pasa con vínculo preexistente issuer/subject; tenant y rol salen de la cuenta local, nunca de claims o formularios.
- **Safe and Explicit Automation**: Pasa; sin modelos, capacidades, pagos ni proveedores externos desde el modo local; piloto externo requiere activación explícita.
- **Fallible, Evolvable Interfaces**: Pasa; errores tipados, migración, OpenAPI y documentación actualizada.
- **Evidence-Based Delivery**: Las puertas Rust, Next, API local y proxy Next pasan; el emisor sintético valida login, rechazos criptográficos, replay, CSRF y revocación. La revisión visual desde un navegador conectado sigue pendiente; no habilitar un proveedor real.

**Gate**: Aprobado para implementar el plano de control separado usando emisor sintético en loopback. No activar ni contactar un IdP real; local mode rechaza toda configuración OIDC.

## Project Structure

```text
crates/domain/src/admin_identity.rs                 # roles e identidad administrativa
crates/application/src/admin_identity.rs            # puertos y casos de uso
crates/infrastructure/src/admin_identity.rs         # PostgreSQL, sesiones e intentos cifrados
crates/infrastructure/src/admin_oidc.rs             # protocolo OIDC
apps/server/src/admin_auth.rs                       # rutas y cookies del plano administrativo
apps/server/src/bin/provision_admin_identity.rs     # aprovisionamiento explícito
migrations/0015_admin_identity_sessions.sql         # identidades, membresías y sesiones
web/admin/src/features/AdminConsole.tsx             # entrada y vista de membresías
contracts/openapi/openapi.yaml                      # contrato de autenticación
scripts/verify_local.py                             # modo local aislado
scripts/verify_oidc.py                              # aceptación HTTP con emisor sintético loopback
```

**Structure Decision**: La autenticación OIDC del plano de control no depende de `LocalStore` ni comparte las rutas `/v1/local/*`. Usa tablas y cookies propias. La primera UI solo lee sesión/membresías y cierra sesión; APIs administrativas adicionales quedan sujetas a autorización RBAC del servidor.

## Complexity Tracking

No hay violaciones de la constitución ni nuevos proyectos.
