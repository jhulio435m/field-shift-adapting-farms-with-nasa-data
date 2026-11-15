# Contrato: trazabilidad de pagos estudiantiles

## Demo local sin login

- `GET /v1/demo/payments` devuelve `{ platform: "clon_local", payments: StudentPayment[] }`.
- `POST /v1/demo/payments/by-dni` recibe únicamente el DNI sintético `70000001` en modo loopback; devuelve la identidad ficticia con DNI enmascarado y el historial. No es autenticación ni permite consultar datos reales.
- `POST /v1/demo/payments/validate` recibe `{ code: string }`; requiere Origin igual a `KUSI_LOCAL_ORIGIN` y devuelve un registro coincidente.
- `POST /v1/demo/payments/validate-receipt` recibe bytes PNG/JPEG/WebP (máximo 5 MiB); exige mismo Origin. Devuelve la referencia enmascarada y el pago coincidente, nunca OCR íntegro ni bytes.
- Estas rutas existen solo si `KUSI_LOCAL_MODE=true` y el API conserva el bind loopback validado. No reciben tenant ni estudiante del navegador.

## Modelo `StudentPayment`

```json
{
  "payment_code": "UNCP-EXP-004821",
  "procedure_code": "PA57002335",
  "concept": "Constancia de expedito para optar título profesional",
  "amount_cents": 19800,
  "currency": "PEN",
  "receipt_reference_masked": "V***-***4821",
  "paid_at": null,
  "payment_state": "confirmed",
  "process_state": "interrupted",
  "platform_name": "Clon local de pagos estudiantiles UNCP",
  "synthetic": true
}
```

## Aplicación

`StudentPaymentsPlatform` lista pagos y encuentra un pago para el estudiante resuelto por el actor confiable. `validate_payment` valida longitud/caracteres antes de llamar al puerto. `LocalStudentPaymentsPlatform` es el adaptador de demo sin red. El adaptador real futuro reemplaza esta implementación; debe recibir identidad verificada y mapear estados de su API sin inferirlos.

## WhatsApp

La firma del webhook y el contexto del canal resuelven organización y remitente. Tras vincular cuenta, “mis pagos” lista y un código busca el registro a través del mismo puerto. La respuesta local queda en outbox simulado. El webhook puede detectar `image.id`, pero el valor se descarta y solo se registra que llegó una foto; no se valida el contenido hasta habilitar descarga de medios autorizada.

`StudentPayment.voucher_use`: `available`, `used` o `review_required`. Se informa independientemente del estado del pago. En el fixture S/ 3,00: used; S/ 198,00 interrumpido: review_required. Es consulta de estado, sin operación de consumo ni bloqueo de un sistema externo.

DNI sintético 70000002: estudiante existente sin pagos, HTTP 200 con payments vacío. Otros DNI desconocidos: HTTP 404. La UI distingue historial vacío de documento no encontrado.
