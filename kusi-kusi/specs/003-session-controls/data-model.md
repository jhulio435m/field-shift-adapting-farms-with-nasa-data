# Data Model: Control de sesiones locales

## LocalSession

- `id`: random UUID management reference; not an authenticator.
- `key_hash`: existing hash of the opaque credential; remains private and is not returned.
- `user_id`: authenticated account.
- `created_at`, `expires_at`, `revoked_at`: existing lifecycle timestamps.
- Active means session is unrevoked and unexpired, linked credential is unrevoked and unexpired, and account is enabled.
- Organization is derived by joining the session user; owners' list and revoke queries always bind to actor organization.

## SessionInventoryItem

- `id`, `user_id`, `display_name`, `username`, `created_at`, `expires_at`, `current`.
- `user_id` and account names appear only in the owner's same-tenant inventory; a non-owner sees only their own rows.
- No token, token hash, password, IP address, User-Agent, device data, or secret is included.

## SessionAuditEvent

- Existing `local_audit` row records actor ID, fixed action name, opaque session UUID target and server timestamp.
- No credential or transport metadata is stored.
- Session revocation, linked credential revocation and audit append commit atomically.

## Authorization invariants

- A session list always scopes organization from the verified `LocalUser`.
- A non-owner can operate only on rows with `user_id = actor.id`.
- The current session ID is derived from the verified token hash and cannot be a revocation target.
- A successful revoke invalidates the next protected request through the existing session guard.
- Disabled and password-reset accounts have both session rows and credentials marked revoked.
