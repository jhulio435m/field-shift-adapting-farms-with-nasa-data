# Data Model: Trazabilidad de pagos

## Payment platform port

The hackathon interface reads `StudentPayment` through `StudentPaymentsPlatform`; it does not persist payment records in Kusi Kusi. The local adapter contains two fixed synthetic rows and accepts a payment code, procedure code, or synthetic receipt alias. Identity for the no-login screen is a server-created fixed demo principal. WhatsApp lookup uses the verified channel-linked local user. No tenant or student identity is accepted from user input.

`StudentPayment` contains payment code, procedure code, concept, amount in cents, currency, masked receipt reference, optional payment date, payment state, process state, platform label and a `synthetic` marker. Raw receipt codes and OCR text are not returned or stored. OCR runs on a bounded image decoded and re-encoded in memory, returning only a receipt reference that is then resolved through the port.

The actual student payment platform does not exist yet. Its future adapter must map a verified student identity to the platform's records and map authoritative payment/process states without inference.

## Earlier broader case model (implemented but outside this demo's primary flow)

The case, source, request and consent entities below were designed for an earlier broader bureaucracy flow. They remain in the workspace but are not used by the current no-login payment-traceability screen.

Toda entidad persistida en esta feature queda asociada a la organización obtenida desde la sesión local verificada. Ninguna ruta recibe un tenant libre para seleccionar el espacio.

## `bureaucratic_cases`

- `id`, `organization_id`, `requester_user_id`, `assigned_staff_user_id?`, `workflow_key`, `status`, `created_at`, `updated_at`, `closed_at?`.
- Estado: `draft`, `needs_information`, `ready_for_review`, `awaiting_institution`, `answered`, `closed`.
- Caso de demo presembrado: trámite UNCP PA57002335, persona ficticia, total sintético S/ 198 con conceptos enmascarados. El tipo de interrupción se guarda únicamente como “interrumpido por motivos de salud”.
- El requester ve sus propios casos. El personal solo ve casos asignados en su organización según permisos del servidor.

## `bureaucratic_case_events`

- `id`, `case_id`, `actor_user_id?`, `actor_type`, `event_type`, `occurred_at`, `summary`, `source_id?`, `version`.
- Eventos registran hechos reportados, fuente añadida, solicitud redactada/aprobada, transferencia, pregunta administrativa y respuesta.
- El texto reportado por la persona, el texto derivado por IA y una respuesta institucional tienen etiquetas de procedencia distintas. No se escriben cuerpos ni datos de salud en logs técnicos.

## `bureaucratic_sources`

- `id`, `organization_id`, `source_key`, `title`, `official_url?`, `document_path?`, `document_sha256`, `resolution?`, `published_at?`, `version`, `verified_by_user_id?`, `verified_at?`, `validity_status`.
- Estado: `unverified`, `current`, `needs_confirmation`, `superseded`.
- El PDF TUPA 2024 se conserva como fuente versionada. Como no se ha comprobado si UNCP emitió modificatorias posteriores, la demo empieza con `needs_confirmation` hasta que un usuario administrativo ficticio la valide.
- Cada fragmento/cita usa `source_id`, página/sección y texto breve. Un resultado generado no puede inventar un ID o una cita.

## `bureaucratic_case_facts` y evidencia

- Hechos estructurados incluyen código PA57002335, monto total 198 PEN sintético, estado de pago y fechas ficticias, todos con procedencia y confianza.
- `evidence_ref` puede referir al recibo de fixture enmascarado. No se admiten recibos reales ni documentación clínica en el MVP de hackatón.
- La evidencia se presenta al personal solo tras consentimiento de la persona para ese caso/envío.

## Borrador y consentimiento

- `bureaucratic_requests`: `id`, `case_id`, `version`, `body`, `source_ids`, `evidence_refs`, `created_by`, `approved_by_requester_at?`, `shared_at?`, `delivery_status`.
- `bureaucratic_sharing_consents`: `id`, `case_id`, `request_version`, `scope`, `recipient_unit`, `consented_by`, `consented_at`, `revoked_at?`.
- Cualquier cambio del borrador o adjuntos crea una versión nueva e invalida la aprobación anterior.
- `delivery_status` solo puede usar `draft`, `approved`, `simulated_sent`, `received_confirmed`, `answered`; en demo no se permite `received_confirmed` sin comprobación institucional y el sistema solo crea `simulated_sent`.

## Asistencia IA

- El contexto de prompt se arma por aplicación con hechos y extractos aprobados. LLM no recibe claves, SQL, herramientas ni acceso al filesystem.
- Salida estructurada conserva IDs de fuente propuestos y texto de sugerencia; el backend elimina/rechaza cualquier referencia fuera de la lista autorizada y marca el borrador como generado para revisión.
- Un modelo local de loopback es opcional. El modo determinista no se presenta como inferencia generativa.
