# Data Model: Autorización de capacidades por recurso

## LocalResourceGrant

- `organization_id`: tenant derived from the local session; required.
- `user_id`: enabled local account with role `viewer`; required.
- `capability`: fixed local demo identifier `demo.resource.read`; required.
- `resource_ref`: one fixed opaque synthetic reference; required.
- `created_by`: same-tenant owner who confirmed authorization; required.
- `created_at`: server timestamp.
- Primary key: `(organization_id, user_id, capability, resource_ref)`; repeated grant is idempotent.
- Composite foreign keys bind both user and grantor to the same organization.

## DemoResource

Immutable infrastructure fixture with opaque `resource_ref`, fictional display label, and safe summary fields. The API exposes only resources in the fixed fixture; callers cannot add resource data.

## ResourceAccessAudit

Append-only execution decision with event ID, organization, local actor ID, capability/version, SHA-256 digest of the opaque resource reference, outcome, and timestamp. No request body, response body, external credential, or message content is stored.

## Authorization invariants

- A grant is valid only while the referenced local user exists, is enabled, and remains a viewer.
- Every lookup binds `organization_id` from the authenticated session.
- The application checks the grant before calling the fixture adapter.
- Revocation takes effect on the next request; there is no cached grant state.
