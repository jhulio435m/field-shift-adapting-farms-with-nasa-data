# Research: Autorización de capacidades por recurso

## Current boundaries

- `/v1/capabilities/execute` is authorized by tenant service API key and `capability_grants`; the caller is not an end user.
- `ExecuteCapability` already separates registry, authorization, schema validation, connector, and audit ports.
- The academic local demo has per-viewer records, but its fixture and authorization are deliberately academic-specific.
- Local mode blocks remote provider execution and channel delivery.

## Decisions

1. **Use a separate session-bound use case for local resource reads.** Do not overload service-key identity or weaken existing capability authentication.
2. **Derive the principal from the validated local session.** Request bodies contain only capability arguments; tenant and user are not accepted.
3. **Model grants as organization + viewer + capability + resource.** This makes resource scope explicit and independently revocable.
4. **Use an in-process fictional adapter.** It tests the application port without network calls and does not imply that a customer system is connected.
5. **Keep connector-side identity mapping out of this slice.** Real SSO claims, customer identity federation, and external ACL checks require provider-specific contracts and are not represented by local account IDs.
6. **Audit decisions, not data.** Store the local actor, capability, a digest of the fixture resource ref, outcome, and time; do not store input or output JSON.
7. **Keep invocation explicit in the admin panel.** No LLM chooses a capability or resource, and no conversation worker runs a tool based on free text.

## Rejected alternatives

- Accepting `organization_id`, `user_id`, or an external principal in the request: caller-controlled identity would defeat the trust boundary.
- Reusing the academic fixture: it would make the generic authorization slice depend on a school domain.
- Calling an HTTPS connector from local mode: forbidden by the local-only policy and unnecessary for authorization acceptance.
- Claiming the local UUID is a customer-system identity: this is not validated and would misstate the integration boundary.
