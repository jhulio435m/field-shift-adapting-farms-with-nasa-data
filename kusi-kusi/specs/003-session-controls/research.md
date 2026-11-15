# Research: Control de sesiones locales

## Current boundaries

- `local_sessions` stores a hashed credential reference, user ID, created/expiry times, and revocation time.
- Local login creates an opaque credential with an eight-hour lifetime; logout revokes that credential.
- Disabling a user and resetting a password revoke linked API-key rows, so the session guard stops authenticating them, but the local session row is not currently marked revoked.
- There is no session inventory, and the current session token is available to protected handlers through the verified cookie/session middleware.

## Decisions

1. **Use a separate random UUID as a management handle.** Do not use or return the credential hash as a route identifier.
2. **Keep authentication unchanged.** The HttpOnly cookie/API-key credential remains the only authenticator; the UUID cannot authenticate.
3. **Scope list and revoke by the verified principal.** Non-owners see/revoke only their own other sessions. Owners can see/revoke sessions of accounts within their organization.
4. **Reject revocation of the current session.** Use the existing logout route for the active browser. Return conflict so the UI can explain why.
5. **Make state change and audit atomic.** In one transaction mark the session and associated API-key credential revoked, then insert a metadata-only audit event.
6. **Make password reset and account disable update both records.** Keep `local_sessions.revoked_at` consistent with the already-revoked API-key state.
7. **Avoid transport profiling.** Do not start collecting IP, User-Agent, geolocation or device fingerprints.

## Rejected alternatives

- Making the UUID a bearer credential would expose a second authentication mechanism and break the existing cookie security model.
- Allowing users to revoke the session backing the same request could leave ambiguous UI state and remove the only current recovery path; logout already handles this case.
- Showing IP addresses or browser fingerprints is unnecessary for the current local product and would introduce new personal data collection.
- Selecting an OIDC provider or rotating service API keys changes provider/deployment or integration behavior and is outside this local session-control slice.
