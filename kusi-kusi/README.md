# Kusi Kusi

Kusi Kusi is a language independent conversational integration layer for existing business systems. This repository contains a Rust modular monolith, a Next.js/Turbopack administration panel with shadcn/ui, authorized HTTP integrations and an end-to-end local simulation workspace.

## Panel Next.js + shadcn/ui

Migración y comandos: [NEXT_MIGRATION.md](docs/NEXT_MIGRATION.md). Desarrollo y build usan Turbopack; el panel se abre en **http://127.0.0.1:3000** y la API Rust permanece en 8080.

## Entrega local completa

Panel Next.js + TypeScript + shadcn/ui en español; usuarios y sesiones locales; roles de propietario, editor, aprobador y lectura; archivos privados; conversaciones con contexto, agrupación e inbox persistente; trabajos creativos con aprobación; vistas previas FFmpeg archivadas; calendario y publicaciones simuladas con aprobación separada; auditoría, retención y respaldo con restauración verificada.

El núcleo conserva los adaptadores de capacidades HTTP, proveedores de IA, transcripción, WhatsApp Cloud API, Telegram y OAuth social. El modo local bloquea ejecución de conectores externos, IA remota, envíos de mensajería y acciones OAuth. No necesita credenciales ni consume servicios de pago. Las vistas previas **no son generación de IA ni videos 360 reconstruidos**, y las publicaciones **no se envían a redes sociales**.

**No está aprobado para producción.** OIDC, publicación social real, revisiones oficiales de aplicaciones, gestión de secretos y evaluación de seguridad siguen siendo requisitos separados. Alcance y comandos: [LOCAL_WORKSPACE.md](docs/LOCAL_WORKSPACE.md). Evidencia: [VALIDATION.md](docs/VALIDATION.md).

### Abrir el panel local

Con PostgreSQL y `.env` configurados:

```sh
docker compose up -d postgres
./scripts/run_local.sh
# En otra terminal, una sola vez:
python3 scripts/setup_local.py
```

Abre http://127.0.0.1:3000. La cuenta inicial queda en `.local-data/login.json` con permisos 0600. El asistente usa `DEMO_ORGANIZATION_API_KEY` para aprovisionar el primer propietario de esa organización; no modifica cuentas existentes. Si aún no existe una organización, créala con el endpoint administrativo descrito abajo y guarda su clave en esa variable. Requiere Node >=22.12, FFmpeg/ffprobe, PostgreSQL client y una fuente DejaVu Sans, además de Rust y Docker.

## Configurar el piloto desde la web

En **Conexiones**, una cuenta propietaria puede guardar Plane (URL/clave API), ERP CONEIMERA (URL/API Key/API Secret) y Telegram (token y webhook), rotar y borrar credenciales. Se almacenan cifradas y nunca se devuelven al navegador. El estado es **guardado, sin verificar**: no activa llamadas ni envíos reales. [Guía y límites](docs/WEB_CONNECTION_SETTINGS.md).

## Run locally

1. Copy `.env.example` to `.env` and set unique secrets. `KUSI_ADMIN_API_KEY` must have at least 32 characters. Keep the password URL-safe or URL-encode it in `DATABASE_URL`.
2. Run `docker compose up -d postgres`.
3. In another terminal run `cargo run -p kusi-server`; it applies migrations and binds to `127.0.0.1:8080`.
4. Check `http://127.0.0.1:8080/health/live` and `/health/ready`.
5. Create an organization with `POST /v1/organizations` and `x-admin-key`. Save the returned `api_key` securely; it is shown once.
6. Use that key as a Bearer token with `GET /v1/organizations/current`.

To query the pre-provisioned local demo tenant, load the ignored `.env` and call:

```sh
set -a
source .env
set +a
curl -fsS http://127.0.0.1:8080/v1/organizations/current \
  -H "Authorization: Bearer $DEMO_ORGANIZATION_API_KEY"
```

The Compose API is bound to localhost. Do not reuse development secrets in a deployed environment.

## Workspace map

- `crates/domain`: framework independent business entities and validation.
- `crates/application`: use cases and repository ports.
- `crates/infrastructure`: PostgreSQL adapter.
- `apps/server`: HTTP adapter and explicit application composition.
- `migrations`: versioned PostgreSQL schema.
- `contracts/openapi`: external HTTP contract.
- `docs`: architecture, security, development, roadmap, and decisions.

See [DEVELOPMENT.md](docs/DEVELOPMENT.md), [ARCHITECTURE.md](docs/ARCHITECTURE.md), and [ROADMAP.md](docs/ROADMAP.md).

For substantial feature work, use the local GitHub Spec Kit workflow for Codex. See [Spec-driven development](docs/SPEC_DRIVEN_DEVELOPMENT.md) and the project constitution at `.specify/memory/constitution.md`.

The local workspace includes a **fictional academic demo** for validating per-user authorization and simulated channel replies. It is not a customer integration or a production feature; see [Local workspace](docs/LOCAL_WORKSPACE.md#demostración-académica-ficticia).

Official WhatsApp Cloud API and Telegram webhook adapters are documented in [CHANNELS.md](docs/CHANNELS.md). Incoming events persist in a deduplicated inbox; conversational processing and live credential validation remain pending.

Multi-provider inference and independent Groq/OpenAI/local transcription profiles are documented in [AI_PROVIDERS.md](docs/AI_PROVIDERS.md), with configuration templates in `examples/`. No paid calls or models are enabled by default. Social publishing remains a proposed module: [SOCIAL_PUBLISHING.md](docs/SOCIAL_PUBLISHING.md).

Creative work can now be coordinated through external providers: [Creative Jobs](docs/CREATIVE_JOBS.md). The module includes tenant recipes, approval-bound immutable jobs, daily approval budgets, a persistent worker and Higgsfield/fal adapters. It is disabled by default. To verify the complete local API workflow without credits or models, build and run `python3 scripts/verify_creative.py`.

Social account linking: [OAuth configuration and limits](docs/OAUTH.md). Facebook Login, Instagram Login and TikTok web adapters, encrypted token storage, browser-bound callback validation and renewal/disconnect lifecycle are implemented. No real apps or accounts have been connected.
