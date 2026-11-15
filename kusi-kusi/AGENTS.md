# Repository instructions

- Keep domain code independent of Axum, SQLx, channels, and AI providers.
- Add use cases to `crates/application`; implement ports in `crates/infrastructure`; keep HTTP concerns in `apps/server`.
- Tenant identity must come from verified credentials or registered channel context, never caller supplied tenant ids.
- Never log API keys, message bodies, credentials, or unnecessary personal data.
- Do not let an LLM issue SQL or invoke a capability without application authorization and argument validation.
- Add a migration for schema changes and tests for behavior changes.
- Use explicit fallible error handling in production paths. Do not use `unwrap` or `expect` there.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` before describing a change as verified.
- Update the OpenAPI contract and relevant docs with API or security changes.

- Local workspace must remain loopback-only and must block paid/external calls; never silently promote simulations to real sends.
- Local browser sessions require HttpOnly cookies, CSRF and server-side RBAC; never store the global admin key in frontend state or localStorage.
- Creative generation and publication are separate version-bound approvals. Preserve idempotency and durable recovery.
- Run `npm run format:check --prefix web/admin`, `npm run build --prefix web/admin` and relevant local API/browser acceptance checks for UI/workflow changes.
- `.local-data` and `.local-backups` contain private state; do not commit or print credentials or dumps.

- Frontend is Next.js App Router with explicit Turbopack builds and local shadcn/ui components. Do not reintroduce Vite or static frontend serving in Axum.
- UI runs on 127.0.0.1:3000, Rust on 127.0.0.1:8080. Preserve fixed-upstream proxying, Origin/CSRF/cookies, no-store and server-side authorization.
- Maintain `verify_next.py` plus browser and two-image container checks when changing the frontend/runtime boundary.
