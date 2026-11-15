# Contract: Local session inventory and revocation

All routes are available only in local loopback mode through the Next fixed-upstream proxy. They require a verified local session; DELETE requires existing CSRF and exact Origin validation. Tenant and actor are never request fields.

## `GET /v1/local/sessions`

- An ordinary user receives only active sessions for their account.
- An owner receives active sessions for accounts in their organization.
- Each entry returns `id`, `user_id`, `display_name`, `username`, `created_at`, `expires_at`, and `current`.
- The response never includes a credential, key hash, password, IP address, User-Agent, or device details.

## `DELETE /v1/local/sessions/{id}`

- An ordinary user may revoke only their own active session.
- An owner may revoke any active session in their organization.
- The session identified by the current verified request is rejected with `409`; use `/local/auth/logout` to end the current session.
- Unknown, expired, revoked, or cross-tenant IDs return `404`.
- Success returns `204`. Session revocation, linked API-key revocation and audit append are transactional.

## Error behavior

- `401`: no valid local session.
- `403`: role or CSRF/Origin denied.
- `404`: target not found in the permitted scope.
- `409`: current session cannot be revoked by this operation.
- `503`: storage failure; no partial state is reported as successful.
