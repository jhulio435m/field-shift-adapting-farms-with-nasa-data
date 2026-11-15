# Research: Fundamentos del producto Kusi Kusi

## Decision: Extend the existing local channel workflow

The server already validates WhatsApp webhook signatures and stores normalized events in a deduplicated inbox. The local worker resolves a sender through a one-time code and persists an organization-scoped binding. Reuse those boundaries; do not create a second webhook stack or infer identity from the sender identifier.

**Rationale**: This minimizes changes to channel authentication and recovery while keeping tenant context in registered server configuration.

**Alternatives considered**: Add a new family-specific webhook; rejected because it duplicates the existing provider adapter and creates another authentication path.

## Decision: Keep academic data in a local fixture adapter

Create a fixed set of clearly fictional records behind an infrastructure adapter and an application read use case. Do not add academic classes, grades, or school naming to the shared generic capability domain.

**Rationale**: The pilot tests whether a generic authorization and connector foundation can support one concrete use case without turning that fixture into the product boundary.

**Alternatives considered**: Call arbitrary customer HTTP from the local worker; rejected because local mode intentionally blocks business connector execution. Call an AI provider; rejected because the answer must be deterministic, sourced, and usable without paid/external calls.

## Decision: Separate user-channel binding from student authorization

Use existing one-time channel link codes to bind a registered sender to a local viewer account. Store a distinct organization-scoped guardian-to-demo-student relation and check it before reading fixture data. An owner can issue a link code for a same-tenant viewer after the school has manually verified the demo relationship out of band.

**Rationale**: A WhatsApp sender identifier alone does not prove access to a student's records. Authentication of the webhook, linking of the account, and authorization for each student are separate checks.

**Alternatives considered**: Treat a phone number as sufficient proof; rejected. Ask the family to type a student ID and trust it; rejected. Collect identity documents in this local demo; rejected as unnecessary and data-risking.

## Decision: Use deterministic read-only intent handling

Recognize a small explicit set of academic-summary phrases and return fixture values through the application use case. Unknown or ambiguous requests receive a safe fallback or human-handoff guidance.

**Rationale**: The first validation is authorization and data accuracy, not open-ended language generation. Determinism makes acceptance evidence repeatable.

**Alternatives considered**: General LLM tool calling; deferred until tool authorization, resource-level validation, audit attribution, budget controls, and product value are separately established.

## Decision: Keep local responses simulated

Persist and display the generated response in the local conversation with explicit simulated metadata. Preserve the existing queued outbox for provider messages; do not call WhatsApp send APIs.

**Rationale**: The repository's local mode blocks paid/external calls, and no production credentials or send approval were provided.

**Alternatives considered**: Send an actual WhatsApp reply; out of scope and not authorized.
