# Feature Specification: Trazabilidad de pagos estudiantiles

**Feature Branch**: `005-brecha-burocratica`  
**Created**: 2026-11-06  
**Status**: In progress  
**Input**: A hackathon prototype where Kusi Kusi connects to a student payment platform being developed by the team. A student can see their payment history and validate a payment code or receipt photo; Kusi returns the platform's payment and process status through a simple local screen or WhatsApp.

## Context

The example record is the UNCP procedure PA57002335, “Constancia de expedito para optar título profesional,” listed at S/ 198 in the 2024 TUPA. The payment-platform system does not exist yet. The prototype therefore uses an in-process local clone with synthetic records and an explicit `StudentPaymentsPlatform` application port. It does not connect to ADESA or an institutional system.

A user-provided receipt photo is only a visual format reference and contains personal data. It is not a fixture and its identifying fields are not copied. The demo receipt is independently generated, clearly marked fictitious, and has a synthetic reference. The example receipt in the photo is a different payment concept and amount from the S/ 198 titling payment.

## User Scenarios & Testing

### User Story 1 - See the payment history (P1)

As a student, I can see the payment records returned by the payment-platform adapter, including concept, amount, masked receipt reference, payment state and process state. The local demo locates its single synthetic student with a fixed fake DNI.

**Acceptance scenarios**

1. Given the loopback demo is running, when I open `/tramites`, then the page opens without login and lists only the fixed synthetic student records.
2. Given a payment has been confirmed but its process is interrupted, when the record is shown, then those two states remain distinct and are attributed to the local platform clone.
3. Given local mode is disabled, when the page requests demo data, then the API returns not found.
4. Given the fixed synthetic DNI `70000001`, when submitted on the loopback page, then the page shows the synthetic student's payment history. Unknown or malformed DNI values are rejected.

### User Story 2 - Validate a payment code or receipt photo (P1)

As a student, I can enter a payment/receipt code or upload a photo so Kusi Kusi can find the matching record in the platform clone.

**Acceptance scenarios**

1. Given a known synthetic code, when submitted, then the matching payment and process states are returned.
2. Given an unknown code, when submitted, then the response says the clone has no matching payment and invents no result.
3. Given a clear synthetic receipt photo, when uploaded, then local OCR extracts only its receipt reference, the clone is queried, and the API returns only the matched record and masked reference.
4. Given an invalid image, unsupported media, oversized image or unreadable reference, when uploaded, then the request fails safely without saving the image or full OCR text.

### User Story 3 - Query through WhatsApp (P1)

As a student with an already linked WhatsApp identity, I can send “mis pagos” or a payment code and receive the same synthetic platform data. A WhatsApp photo event is acknowledged, but its media bytes are not yet downloaded or OCR-validated.

**Acceptance scenarios**

1. Given a registered, signed webhook and linked demo user, when “mis pagos” or a known code arrives, then the worker calls the same payment-platform port and stores a simulated response.
2. Given a photo webhook without media download capability, when it arrives, then the response says the photo was received but not content-validated and requests a code.
3. Given an unlinked sender, when a payment code arrives, then no payment data is returned.

## Requirements

- **FR-001**: Kusi Kusi MUST read payment history and validation results through `StudentPaymentsPlatform`; status must come from that port, not from an LLM or hard-coded inference in a handler.
- **FR-002**: A payment lookup MUST accept only a bounded payment or receipt code and MUST return a matching record scoped to the verified/demo principal.
- **FR-003**: Payment status and process status MUST be displayed separately with the source platform identified.
- **FR-004**: The local clone MUST provide populated, synthetic payment records, including PA57002335 at S/ 198 and a second small payment fixture based on a different receipt layout. It MUST contain no real student identifiers.
- **FR-005**: The no-login demo screen and its endpoints MUST run only in local mode with loopback bind/origin; endpoints MUST return fixed synthetic records and reject foreign Origin on POST.
- **FR-006**: Receipt OCR MUST validate image type, byte size and decoded dimensions, execute locally, return only the detected receipt reference and matching payment, and never persist the image or full OCR text.
- **FR-007**: WhatsApp code lookup MUST use a registered channel and verified linked identity; the sender or message MUST NOT supply tenant/student identity.
- **FR-008**: WhatsApp photo webhook receipt MUST NOT be described as photo validation until authorized media download and OCR are implemented.
- **FR-009**: The future payment platform MUST be replaceable behind the application port; no real credentials or live integration are included in this feature.
- **FR-010**: The shared receipt image from the user MUST NOT be stored in the repository, test fixtures or logs; only synthetic examples may be used.
- **FR-011**: The loopback demo MAY accept only its fixed synthetic DNI `70000001` to demonstrate student payment-history lookup; it MUST mask the identifier in the response and reject all other or malformed values. This is not an identity-verification design for real student data.

## Success Criteria

- **SC-001**: The local no-login page lists at least two populated synthetic records and validates one code successfully.
- **SC-002**: The generated synthetic receipt is recognized by local OCR and matched to the corresponding fixture without returning OCR text.
- **SC-003**: Unknown code, foreign Origin, malformed image, oversized image and non-local mode are rejected.
- **SC-004**: A signed WhatsApp text webhook for “mis pagos” and a known payment code returns data from the same application port with a simulation label.
- **SC-005**: The full Rust, local API acceptance, Next format/build, Next proxy checks and UI review pass before declaring the demo ready.

## Out of scope

Live ADESA or payment-platform integration; WhatsApp Cloud API credentials; downloading WhatsApp media; claims that a payment receipt is legally valid; changes to student records; real student data; diagnosis or health records; automated refunds or process decisions.
