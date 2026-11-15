# ADR 0004: external creative providers with explicit approval

Status: accepted for the local creative integration delivery.

Image/video generation remains in external services. Kusi Kusi owns tenant-scoped recipes, immutable proposals, approval, a PostgreSQL queue and audit. Ports separate provider transport from business state. Inputs and price estimates are frozen and bound to the job UUID and organization; approval expires after thirty minutes.

A daily UTC approval ledger serializes reservations per tenant. It limits reviewed estimates, not actual provider invoices. A provider-side financial limit is still needed when strict billing enforcement is required.

Workers claim jobs with row locks and leases, commit before contacting providers and never automatically resubmit ambiguous generation POSTs. Interrupted submissions become unknown. GET status polling is bounded to ten consecutive failures. This sacrifices automatic recovery of uncertain work to avoid duplicate paid jobs. Human reconciliation is required for unknown state.

Generation and publication are separate approvals. The creative module cannot publish to social accounts. Assets are external references in this delivery; durable binary archival and messaging orchestration remain separate integrations.
