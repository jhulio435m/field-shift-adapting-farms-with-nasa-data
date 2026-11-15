# Implementation Plan: Trazabilidad de pagos estudiantiles

**Branch**: `005-brecha-burocratica` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

## Summary

Kusi Kusi consults a student-payment platform through an application port. Until the team's platform exists, an in-process local adapter supplies synthetic payment records. A no-login screen is limited to local loopback and fixed fixtures. Users can search by code or upload a receipt image; OCR runs locally in memory and its detected reference is looked up against the adapter. Registered WhatsApp text messages use the same port. Photo webhooks are acknowledged without claiming media validation.

## Technical Context

- **Stack**: Rust workspace, Axum, SQLx/PostgreSQL, Next.js App Router and shadcn/ui.
- **Application**: `StudentPaymentsPlatform` port plus list and validate use cases.
- **Infrastructure**: `LocalStudentPaymentsPlatform` deterministic clone; OCR helper invokes local Tesseract and returns only a voucher reference.
- **HTTP**: `/v1/demo/payments*` works only when local mode is enabled. Existing local bind/origin validation guarantees loopback. POST requires same Origin; no tenant, student ID, credentials or raw OCR text comes from/returns to the browser.
- **WhatsApp**: signed inbound text is normalized by the existing adapter and processed only after account linking. Payment list/code messages call the same application port. Local mode queues only simulated responses. Image events are acknowledged but Meta media is not downloaded.
- **UI**: `/` and `/tramites` lead to the no-login synthetic payment demo. Authenticated workspace routes keep existing session, CSRF and RBAC requirements.
- **Testing**: Rust fmt, clippy and tests; isolated `verify_local.py` includes no-login lookup/OCR and signed WhatsApp flows; `verify_next.py`; Next format/build; visual browser review when a browser is available.

## Safety and scope

No ADESA or live payment platform connection exists. No external calls or real payment records are used. The API clone returns explicit synthetic labels. The photo provided by the user is excluded from the repository; the OCR fixture is independently generated. A future connector must map identity from a verified local session or registered channel context, never from request JSON.

## Project structure

```text
crates/domain/src/student_payment.rs
crates/application/src/student_payments.rs
crates/infrastructure/src/student_payments.rs
apps/server/src/local.rs
apps/server/src/channels.rs
web/admin/src/features/StudentPaymentsDemo.tsx
web/admin/src/app/tramites/page.tsx
web/admin/public/demo/recibo-sintetico.png
contracts/openapi/openapi.yaml
```

## Constitution check

- Domain remains independent of HTTP, SQLx and OCR providers.
- Future platform integration swaps the infrastructure adapter behind the application port.
- No-login path contains only fixed synthetic data and exists only with local loopback mode.
- OCR is bounded, local, in-memory and minimizes its output.
- WhatsApp photos are not represented as validated until Meta media retrieval and OCR are implemented.

## Entrega staging WhatsApp autorizada

Proceso separado `whatsapp_demo` con webhook único, credenciales META privadas, destinatario de prueba autorizado y contexto del actor sintético fijo. Reutiliza los casos de uso de pagos y OCR. Expone solo /webhook mediante túnel temporal, sin exponer el router local. Ledger de hashes de mensajes evita reenvíos automáticos ambiguos; los datos del recibo no se guardan. Ver docs/WHATSAPP_DEMO.md.
