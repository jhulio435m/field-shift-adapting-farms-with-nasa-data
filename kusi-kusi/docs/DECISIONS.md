# Decisions

## ADR 0001: Rust modular monolith

Status: accepted for the initial product.

Use one Rust Cargo workspace with Axum and PostgreSQL. Integration is language independent because customer systems connect over published HTTP contracts; their implementation language does not constrain Kusi Kusi's backend. A modular monolith keeps the first release operationally small. Microservices are deferred until independent scaling or isolation is demonstrated.

## ADR 0002: API key-derived tenant identity for bootstrap

Status: accepted for the initial API slice only.

Provisioning uses a separately configured admin key. Each organization receives a high-entropy tenant key; only its hash is persisted and the key is returned once. This provides a concrete tenant boundary before OIDC and channel identity arrive. Production requires scoped key lifecycle management, rotation, and a stronger operational model.

## ADR 0003: OpenAPI maintained as a checked-in contract

Status: accepted for the initial API slice.

Keep the small initial contract in YAML and serve the same file from the API. This avoids adding code generation and macro dependencies before endpoint shape stabilizes. Revisit generated schemas as API surface grows.

## ADR 0008: espacio local explícito y simulación sin proveedores

Aceptado para desarrollo. El modo local exige loopback y origen exacto, y bloquea servicios externos; no se infiere autorización para activar pagos o publicar. La identidad local usa sesiones opacas/Argon2 y RBAC como alternativa verificable mientras se incorpora OIDC. El middleware puede componer el contexto administrativo para rutas existentes después de verificar la sesión/rol; siempre elimina cualquier encabezado de confianza aportado por el cliente.

Se reutilizan los trabajos creativos, contratos y cola existentes. FFmpeg produce vistas previas marcadas y archivos permanentes; no recrea modelos generativos ni promete 360. Publicaciones requieren otra aprobación sobre snapshot versionado y solo producen recibos simulados. PostgreSQL conserva inbox, debounce, leases, outbox y auditoría; no se añade Redis. El panel React/TypeScript consume la misma API y no almacena secretos de operador.

Los servicios verticales nuevos se agrupan en infraestructura local; las reglas puras y contratos permanecen en dominio/aplicación. La extracción de nuevos puertos se pospone hasta necesitar otro adaptador. Esta decisión no sustituye la orquestación LLM, autorización por recurso de un cliente ni publicación nativa de redes sociales.

## ADR 0009: Next.js App Router + Turbopack + shadcn/ui

Decisión explícita del usuario. Se sustituye Vite por Next.js y se mantienen Rust y sus límites hexagonales. Rutas reales, componentes shadcn locales, Tailwind 4 y Lucide. Next es la interfaz/proxy, no un segundo backend de negocio. Las cookies/CSRF se conservan y Rust autoriza cada operación. El runtime standalone requiere proceso Node; se mantienen dos imágenes y puertos distintos, sin introducir microservicios de negocio.
