# Generic integration API

## Runtime platform adapters

`/v1/capabilities/execute` now dispatches through
`crates/infrastructure/src/system_connectors.rs`, after the existing PostgreSQL
registry, exact permission grant and application schema checks. The registry,
grants and start/terminal audit use the existing PostgreSQL tables; there is no
schema change or in-memory replacement in the API runtime.

Server-owned `KUSI_SYSTEM_PROFILES` maps tenant UUIDs to named profiles:

```json
{
  "tenant-uuid": {
    "crm": {
      "kind": "frappe_identity",
      "base_url": "https://crm.example.com",
      "host": "crm.example.com",
      "credential_file": "/run/secrets/tenant-crm.json"
    }
  }
}
```

Set `KUSI_SYSTEM_ALLOWED_HOSTS=crm.example.com` separately. The credential file
is a regular private file (0600 or stricter), at most 8 KiB, containing only
`username` and `password`. Symlink files and group/world-readable files are
rejected. Credentials load once at startup; rotate by replacing the secret file
and restarting the API. Neither registration nor execution accepts credentials
or secret file paths. Keep secret files outside version control.

Supported fixed reads:

| Kind | Operation | Normalized output |
|---|---|---|
| `frappe_identity` | Login, read the configured account's own identity, logout; works with Frappe CRM and ERPNext | `kind`, `reachable: true`, `authenticated: true` |
| `plane_identity` | CSRF, login, own identity/workspace count, logout | Same status fields plus `workspace_count` |
| `wppconnect_health` | GET `/healthz`; no credentials or WhatsApp session required | `kind`, `reachable: true`, `authenticated: false` |

Each accepts only `{}` as capability input, even if an operator registers a
permissive input schema. Fixed paths and configuration select the operation;
the caller cannot select another user, URL, record or business write. Outputs
omit account identity fields, cookies and response bodies. Register and grant
these capabilities through the existing admin-plus-tenant endpoints before
execution with the tenant service key. Adapter names must not overlap the
generic HTTP adapter names in the same tenant.

HTTPS public profiles keep DNS validation/pinning, port 443, exact host matching,
no redirects, no environment proxy, bounded time and a 64 KiB JSON response
limit. For an explicitly trusted internal deployment only,
`KUSI_SYSTEM_PRIVATE_HTTP=true` permits literal RFC1918/loopback IPv4 HTTP bases
with an allowlisted Host header. It rejects DNS HTTP targets, link-local/metadata,
public HTTP, IPv6, URL credentials, paths, queries and fragments. This separate
profile policy does not relax the generic HTTPS connector. All system profiles
are refused at startup in local mode, whose execution route remains blocked.

The adapters create short-lived upstream login sessions and attempt logout
before returning, including identity-validation failures. A hard timeout or
transport failure can prevent logout; the upstream session expiry remains the
fallback. There are no automatic retries or business writes/messages. This
does not establish end-user mapping or record-level access to business data;
these reads authenticate only the configured service account itself.

`scripts/verify_system_connectors.py` exercises the real Axum runtime with a
disposable PostgreSQL database, loopback fixtures and request counters. Optional
live reads require both `--live-credentials` and `--allow-live-reads`; they use
only the synthetic accounts created on the authorized server. It tests input
and output schemas, cross-tenant refusal, missing grants, disabled capabilities,
revoked keys, audit failure before transport, and persistence across API restarts.
See [SYSTEM_CONNECTORS_ACCEPTANCE.md](SYSTEM_CONNECTORS_ACCEPTANCE.md).

## Generic HTTPS JSON connector

Capabilities are tenant-owned contracts, independent of customer implementation language. Register them through POST /v1/capabilities with admin plus tenant credentials. Specify identifier, version, description, adapter, required_permission, enabled, timeout_ms, input_schema and output_schema. JSON Schema references are rejected and schemas are limited to 16 KiB.

Grant the exact required_permission to the authenticated tenant service key through POST /v1/capabilities/grants with both credentials. Execute through POST /v1/capabilities/execute with tenant bearer authentication and {identifier, version, input}. The service derives organization identity from the verified key, checks grants and capability state, validates input and output, then returns structured data. This endpoint authenticates service keys, not end users or parents.

Configure KUSI_HTTP_DESTINATIONS as a server-owned JSON mapping of organization UUIDs to objects mapping adapter identifiers to HTTPS URLs. Configure KUSI_HTTP_ALLOWED_HOSTS as exact comma-separated hostnames. Defaults deny all network execution. Never register endpoints that mutate state in this release: confirmation and write idempotency are pending. No connector credentials are accepted in capability payloads or execution requests.

Transport policy: HTTPS port 443 only; no URL credentials or fragment; host allowlist; public IPv4 only; rejected private, metadata, multicast and special networks; DNS results pinned to the connection; no redirects or environment proxy; 1 MiB maximum JSON response; configured timeout capped at 30 seconds including DNS. IPv6 is currently rejected. No retries are performed.

Verified locally: metadata persistence, authenticated listing, permission grant, forbidden execution without a grant, input schema rejection, and refusal of unconfigured destinations. Five existing automated tests and fmt/clippy/build pass. Live HTTPS echo execution was verified with fictional data. Execution start/result events are persisted without input/output payloads. Credential lifecycle, per-resource end-user authorization, writes, messaging and production security remain pending.

## Tenant-scoped server configuration

`KUSI_HTTP_DESTINATIONS` has shape `{"organization-uuid":{"adapter-id":"https://approved.example/query"}}`. Invalid JSON or invalid UUIDs stop startup. A tenant cannot resolve adapters configured only for another tenant. `operation` currently accepts only `read` (the default); registering `write` is rejected. Read-only behavior of the customer endpoint must also be confirmed by the integrator: an HTTP POST cannot prove business semantics.

The application records an execution event before contacting a connector and a terminal event afterwards. Database failure blocks execution or successful return. An interrupted execution may retain only a started event; reconciliation and durable retries are not implemented. Audit currently records event id, tenant, capability/version, outcome and server timestamp; attribution to individual end users is pending. No inputs, outputs, API keys or customer data are stored in this audit table.

Verification: real HTTPS POST to httpbin.org/post with fictional query data returned 200 and matched the expected structured output; write registration returned 422. PostgreSQL contained matching started/succeeded events. Automated format, Clippy, five tests (including PostgreSQL), and build passed.
# Demostración académica local

La demostración académica es un fixture local de solo lectura. Usa referencias y calificaciones ficticias, y no amplía ni especializa el contrato genérico de capacidades. Las integraciones reales se conectan mediante adaptadores independientes y contratos del Integration Hub; este flujo sirve para revisar identidad, permisos por registro y respuestas fundamentadas sin conectar un sistema escolar.

