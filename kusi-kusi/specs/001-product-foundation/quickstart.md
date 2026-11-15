# Quickstart: Local academic demo

This validation uses only the repository's loopback workspace, a temporary/test organization, fictional fixture records, and signed webhook fixtures. It does not need WhatsApp credentials or public routing.

## Preconditions

- Rust workspace and Node/npm dependencies installed.
- A test PostgreSQL database or the isolated database setup used by `scripts/verify_local.py`.
- Do not point verification at a production database or put real student data into the fixture.

## Verify

1. Run Rust formatting, lint, and tests using the repository-required commands:

   ```sh
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

2. Run the isolated local API/workflow acceptance checks:

   ```sh
   python3 scripts/verify_local.py
   ```

3. Run admin formatting and production build checks:

   ```sh
   npm run format:check --prefix web/admin
   npm run build --prefix web/admin
   ```

4. With the local workspace running on loopback, run the browser acceptance checks:

   ```sh
   python3 scripts/verify_browser.py
   python3 scripts/verify_next.py
   ```

## Expected acceptance evidence

- An owner sees only the fixed fictional students and can link a same-tenant viewer to one.
- An owner can issue that viewer a short-lived channel code; it is stored hashed, shown once, and cannot bind another channel or tenant.
- A signed fixture event with the one-time code binds the sender; repeated event delivery does not create duplicate messages or link effects.
- A supported academic query returns values only for the viewer's linked fictional students and is marked simulated.
- An unlinked sender, disabled viewer, cross-tenant assignment, unknown student, unsupported request, and delivery-status event reveal no academic result.
- No actual provider send or external request occurs.
