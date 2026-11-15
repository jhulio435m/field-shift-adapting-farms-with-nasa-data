# Development

Requirements: stable Rust toolchain, Cargo, Docker Engine, and Docker Compose plugin.

```sh
cp .env.example .env
# Replace the sample values before starting.
docker compose up --build
```

For a host-run API, start PostgreSQL with Compose (published at `127.0.0.1:5433` by default), set `DATABASE_URL` and `KUSI_ADMIN_API_KEY`, then run `cargo run -p kusi-server` from the repository root. The server applies migrations on startup. `BIND_ADDRESS` defaults to `127.0.0.1:8080`.

Quality commands:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

Run `./scripts/verify.sh` to execute the same checks and, when `DATABASE_URL` is present in `.env`, the PostgreSQL adapter integration test.

Organization provisioning requires `x-admin-key`. The response contains the tenant API key once. Use that value as `Authorization: Bearer <key>` for the current-organization endpoint. Do not save real secrets in source control.

## Panel y modo local

Guía completa: [LOCAL_WORKSPACE.md](LOCAL_WORKSPACE.md). `scripts/run_local.sh` compila frontend/backend y activa únicamente los trabajadores simulados. `scripts/setup_local.py` crea el primer propietario usando la organización ya aprovisionada. No modifica usuarios existentes. Contrato API en `/openapi.yaml`, incluyendo rutas `/v1/local/*` y sesión `/local/auth/*`.

El plano administrativo OIDC es independiente y aparece en `/admin` solo fuera del workspace local. Queda desactivado si no se configuran `KUSI_ADMIN_OIDC_*`; el arranque falla si se intenta configurar OIDC con `KUSI_LOCAL_MODE=true`. Para una prueba de desarrollo usa solo un emisor loopback. Las identidades se preaprovisionan con `cargo run -p kusi-server --bin provision_admin_identity`; las variables `KUSI_ADMIN_PROVISION_ISSUER`, `KUSI_ADMIN_PROVISION_SUBJECT`, `KUSI_ADMIN_PROVISION_ORGANIZATION` y `KUSI_ADMIN_PROVISION_ROLE` deben corresponder a una organización ya existente. No configures un proveedor real hasta aprobar el piloto y su manejo de secretos. El contrato está en `contracts/openapi/openapi.yaml`.

Node >=22.12 y npm son necesarios para `npm ci --prefix web/admin` / `npm run build --prefix web/admin`. Los renders locales requieren FFmpeg, ffprobe y DejaVu Sans. Dockerfile y Dockerfile.web construyen Rust y Next.js por separado. El perfil Compose `panel` incorpora Next; para el recorrido autenticado local se usa el script del host con ambos orígenes loopback. Ver NEXT_MIGRATION.md.

Pruebas del producto local: `python3 scripts/verify_local.py` usa un esquema temporal y borra solo ese esquema. `python3 scripts/verify_browser.py` recorre la UI real y agrega datos ficticios identificados en el espacio local. El navegador no recibe claves administrativas. Las evidencias actuales están en VALIDATION.md.
