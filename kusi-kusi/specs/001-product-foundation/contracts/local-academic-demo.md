# Local Academic Demo Contract

All routes below are loopback-only local workspace routes, protected by an HttpOnly local session, CSRF on mutations, Origin validation, no-store headers, server-side RBAC, and tenant context from the session. They expose fixed synthetic fixtures only. They are not a production academic integration.

## List fictional students

`GET /v1/local/academic/demo-students`

- Owner only.
- Returns stable fixture identifiers and labels marked `Ficticio`.
- Returns no real academic system data or personal data.

## List guardian links

`GET /v1/local/academic/guardian-links`

- Owner only.
- Returns same-organization viewer account identifiers/display names and linked synthetic student references.
- Never returns channel secrets, message bodies, or other organizations' links.

## Create guardian link

`POST /v1/local/academic/guardian-links`

```json
{"user_id":"<same-organization-viewer-uuid>","student_ref":"demo-student-001","relationship_verified":true}
```

- Owner only; requires CSRF.
- The user must exist in the session organization, be enabled, and have viewer role.
- `student_ref` must exist in the fixed fixture set.
- Duplicate requests are idempotent. Cross-organization user, non-viewer, or unknown student is rejected.
- `relationship_verified` must be `true`; the owner attests that out-of-band relationship validation occurred. The request cannot upload identity documents.

## Remove guardian link

`DELETE /v1/local/academic/guardian-links/{user_id}/{student_ref}`

- Owner only; requires CSRF.
- Removes only the matching link from the current organization.
- Subsequent requests cannot read that student's fixture records.

## Issue a channel link code to a viewer

`POST /v1/local/link-codes`

Existing `{ "channel_id": "..." }` behavior remains supported for the current user. Add optional `user_id` for an owner to issue a code to an enabled viewer in the current organization. The server verifies the channel belongs to that organization. The code remains single use, channel bound, expires in ten minutes, is stored only as a hash, and is excluded from logs/audit details.

## Family query

The authenticated WhatsApp event is normalized into the existing inbox. A linked viewer asking a supported read-only academic-summary phrase receives fixture values only for linked students. The response is persisted/displayed in the local conversation as simulated. The local outbox remains non-sending; no WhatsApp API call is made.

Unknown requests, a user without a binding, a viewer with no linked students, or an unsupported field receive a safe response with no fixture values. Delivery-status events do not invoke the academic use case.
