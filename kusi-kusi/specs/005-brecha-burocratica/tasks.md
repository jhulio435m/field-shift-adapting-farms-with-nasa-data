# Tasks: Plataforma demo de trazabilidad de pagos

**Input**: [spec.md](spec.md) and [plan.md](plan.md)

## MVP local

- [x] T001 Define payment states and response DTOs in `crates/domain/src/student_payment.rs`.
- [x] T002 Define the platform port and list/validate use cases in `crates/application/src/student_payments.rs`.
- [x] T003 Implement a populated local clone with two synthetic records and code aliases in `crates/infrastructure/src/student_payments.rs`.
- [x] T004 Add unauthenticated demo list/code endpoints gated by local loopback mode and same Origin in `apps/server/src/local.rs`.
- [x] T005 Add bounded in-memory OCR endpoint that returns only masked reference and matching record.
- [x] T006 Add signed WhatsApp text intents for “mis pagos” and payment-code lookup through the same application port.
- [x] T007 Add WhatsApp image webhook normalization and explicit acknowledgment that bytes are not yet validated.
- [x] T008 Build a simple no-login `/tramites` UI with populated history, code lookup and receipt-photo upload.
- [x] T009 Add a wholly synthetic receipt image for OCR testing; do not use the user-provided receipt.
- [x] T010 Document API operations, local-only behavior, OCR privacy and future platform connector seam.
- [x] T017 Add a fixed synthetic DNI lookup for the local demo and label it as non-authenticating test data.

## Verification and readiness

- [x] T011 Run Rust fmt, clippy and full workspace tests after the final backend changes.
- [x] T012 Run `scripts/verify_local.py` for unauthenticated synthetic API, fake-DNI lookup, OCR, WhatsApp text lookup, photo receipt acknowledgment, Origin and no external sends.
- [x] T013 Run Next format/build and `scripts/verify_next.py`.
- [x] T014 Review `/tramites` in Chromium at desktop and mobile sizes; verify loading, error, no-result and successful code/photo states.
- [ ] T015 Configure and verify WhatsApp Cloud API app, webhook secret, phone-number ID, access token, and media download/OCR path. Keep local mode external-call blocking intact.
- [ ] T016 Replace `LocalStudentPaymentsPlatform` with an adapter for the team's payment platform after its API contract, auth method, identity mapping and test endpoint exist.

## Completion criteria

The loopback MVP is ready to show when T011–T014 pass. T015 is required before claiming WhatsApp Cloud API connectivity or WhatsApp photo validation. T016 is required before claiming integration with the team's actual payment platform.

## Entrega staging WhatsApp

- [x] T018 Implementar binario staging separado, firma HMAC, destinatario autorizado y consultas sintéticas.
- [x] T019 Implementar descarga acotada de imágenes Meta y OCR sin persistir fotografías.
- [x] T020 Preparar lanzador privado y ledger de deduplicación sin mensajes ni credenciales.
- [ ] T021 Verificar callback HTTPS, suscripción Meta y respuesta real a texto y foto del número autorizado.

- [x] T022 Mejorar formato de WhatsApp y traducir estados de pago/trámite.
- [x] T023 Añadir notas de voz en staging mediante el puerto Transcriber y Groq opcional, sin persistir audio/transcripción.
- [ ] T024 Validar nota de voz real después de configurar GROQ_API_KEY y foto real contra el fixture sintético.

- [x] T025 Añadir estado explícito voucher_use a los registros de prueba y mostrar uso/revisión en WhatsApp; no incluye consumo ni bloqueo externo de trámites.

- [x] T026 Extraer códigos hablados con pruebas de dígitos, espacios y ambigüedad.
- [x] T027 Mejorar carga web de recibos con arrastre, preview, límites y uso del comprobante; aceptación escritorio/móvil.

- [x] T028 Compactar consulta web con selector de método, resultado/modal, historial por DNI/modal y pruebas de Escape, viewport, código y OCR.

- [x] T029 Distinguir DNI sintético sin pagos (70000002) de desconocido (70000003), en web y WhatsApp; probar ambos estados.

- [x] T030 Gateway público temporal acotado a la consulta sintética, bloqueo de administración y segundo remitente de WhatsApp autorizado.
