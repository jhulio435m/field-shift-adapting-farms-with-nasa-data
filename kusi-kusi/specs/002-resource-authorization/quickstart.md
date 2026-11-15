# Quickstart: Autorización local por recurso

## Prerequisites

- PostgreSQL loopback and existing Kusi Kusi development setup.
- No provider credentials, external URL, real user data, or real resource records.

## Verification

1. Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
2. Run `python3 scripts/verify_local.py`. It uses an isolated schema and signed-in synthetic users.
3. Run `npm run format:check --prefix web/admin`, `npm run typecheck --prefix web/admin`, and `npm run build --prefix web/admin`.
4. With loopback API and UI running, run `python3 scripts/verify_next.py` and `python3 scripts/verify_browser.py`.

## Acceptance evidence

- Owner grants one fictional resource to a same-tenant enabled viewer and revokes it.
- The viewer can read only the granted resource; missing, cross-tenant, disabled, or revoked access is denied before adapter invocation.
- Tenant/user fields in request bodies do not change the verified session identity.
- Response is labeled simulated; audit contains the decision but no arguments or result payload.
- No external connection or message send occurs.
