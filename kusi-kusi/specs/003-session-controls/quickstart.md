# Quickstart: Control de sesiones locales

## Prerequisites

- PostgreSQL loopback and the local workspace.
- Synthetic local accounts only; no IdP credentials or external calls.

## Verification

1. Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
2. Run `python3 scripts/verify_local.py` for owner/user scope, current-session protection, cross-tenant denial, password reset, disable and immediate revocation.
3. Run `npm run format:check --prefix web/admin`, `npm run typecheck --prefix web/admin`, and `npm run build --prefix web/admin`.
4. With loopback services running, run `python3 scripts/verify_next.py` and `python3 scripts/verify_browser.py`.

## Acceptance evidence

- User sees own active sessions, owner sees only same-tenant sessions.
- Current session is marked and cannot be revoked through the inventory.
- A revoked session fails authentication on the next request.
- Response/audit contain no credentials or transport identifiers.
