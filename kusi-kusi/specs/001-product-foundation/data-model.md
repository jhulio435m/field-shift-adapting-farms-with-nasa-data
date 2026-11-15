# Data Model: Fundamentos del producto Kusi Kusi

## Existing records reused

### Organization

The authenticated tenant owning users, channels, links, conversations, and audit records. Resolve from the local session or registered channel configuration. Never accept its ID from the message body.

### Local user

An organization-scoped account. The family account is a local `viewer`; privileged school operators are `owner` accounts. Disabled users cannot establish or use channel bindings.

### Channel binding

Existing relation between `(organization_id, channel_id, sender)` and a local user, established with an expiring, one-time code. The sender value is an identifier supplied by the authenticated provider event, not proof of school identity.

## New records

### Guardian/student demo link

| Field | Rule |
|---|---|
| `organization_id` | Required; references the owning organization. |
| `user_id` | Required; same-organization local viewer account. |
| `student_ref` | Required; one of the fixed fictional fixture IDs. |
| `created_by` | Required; owner who confirmed the demo relationship. |
| `created_at` | Server timestamp. |

Unique key: `(organization_id, user_id, student_ref)`. Every read filters by authenticated organization and linked user. Deleting a link immediately removes access without deleting the fixture record.

### Fictional academic record

Immutable adapter-owned fixture, not tenant data. Each record contains a stable synthetic `student_ref`, display label explicitly marked as fictitious, grade/year label, and a short fixed list of subject/value pairs. It contains no real names, phone numbers, addresses, government identifiers, or customer-provided grades.

### Academic query result

Derived on demand from the authorized guardian/student links and fixture adapter. Do not persist a separate academic cache. Existing local message history stores the response with metadata indicating `simulated: true`; existing retention controls apply.

## State and authorization

1. An owner creates a viewer account and a demo link after manual out-of-band verification.
2. An owner may issue that viewer a channel-bound one-time linking code. Only the hash is persisted; the plaintext code is displayed once and never logged.
3. The sender presents the code in a signed provider event. Existing worker logic consumes it once and binds the provider sender to the viewer.
4. A read request resolves organization from channel configuration and user from the active binding; the application use case loads only linked student references.
5. A missing user, missing student link, disabled account, unsupported intent, or unavailable fixture returns a denial/fallback and no academic values.
6. No writes are supported. A local response is stored as simulated and is never transmitted to a provider.
