# Contract: Local generic resource access

All routes are same-origin through the Next.js fixed-upstream proxy. Mutations require the existing CSRF token and Origin check. Organization and actor come from the verified local session.

## `GET /v1/local/resource-access/resources`

Returns the fixed fictional resource catalog and, for viewers, only the resources granted to the current user. Owners can see the two fixed resource labels for assignment. No customer data is returned.

## `GET /v1/local/resource-access/grants`

Owner-only list of active grants in the current organization. The response includes viewer display name, fictional resource label, capability, and creation time. It excludes users disabled or no longer viewers.

## `POST /v1/local/resource-access/grants`

Owner-only. Body:

```json
{
  "user_id": "<same-tenant viewer UUID>",
  "resource_ref": "demo-resource-001",
  "authorization_verified": true
}
```

The server rejects false confirmation, unknown resources, non-viewer/disabled/cross-tenant accounts, and unknown fields. Duplicate grants are idempotent.

## `DELETE /v1/local/resource-access/grants/{user_id}/{resource_ref}`

Owner-only revocation. The path is tenant-scoped by the session. Returns `204` on deletion and `404` if no grant exists.

## `POST /v1/local/resource-access/execute`

Session-bound read-only request. Body:

```json
{"resource_ref":"demo-resource-001"}
```

No tenant, user, principal, provider URL, capability choice, or credentials are accepted. The server selects the fixed `demo.resource.read` capability, requires the matching current viewer/resource grant, validates the fixed output schema, and calls only the in-process fictional adapter. Success returns the synthetic resource summary and `simulated: true`. Denied access returns `403`; an unavailable store returns `503`. All outcomes are audit logged without payloads.
