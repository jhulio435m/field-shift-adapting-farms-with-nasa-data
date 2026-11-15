# Contrato: Casos institucionales locales

Todas las rutas requieren la sesión del workspace local y mantienen cookies HttpOnly, Origin, CSRF, no-store y RBAC existente. El tenant deriva de la sesión; no se aceptan `organization_id` en body, query ni path. Las rutas son solo para la demo en loopback.

## Lectura del caso

- `GET /v1/local/cases`: lista solo casos propios o asignados al personal autenticado.
- `GET /v1/local/cases/{case_id}`: devuelve estado, cronología, fuentes, evidencia sintética enmascarada y siguiente acción; un caso de otro tenant o fuera del permiso responde como no encontrado.
- `GET /v1/local/case-sources`: lista fuentes con título, resolución/fecha, estado de verificación y páginas pertinentes.

## Orientación

- `POST /v1/local/cases/{case_id}/guidance` recibe solo la pregunta actual de la persona; la aplicación construye contexto con hechos autorizados y extractos de fuentes permitidas.
- Respuesta: `summary`, `next_steps[]`, `open_questions[]`, `citations[]`, `assistant_mode` (`local_model` o `deterministic_demo`). Cada cita contiene `source_id`, `version`, `page_or_section`, `quote` y `validity_status`.
- No se habilitan herramientas para el modelo. Cualquier cita cuyo ID/version no esté en el contexto server-side se rechaza. La guía muestra claramente que no decide vigencia del pago o reembolso.

## Solicitud y consentimiento

- `POST /v1/local/cases/{case_id}/requests` crea un borrador versionado de consulta con área destinataria, preguntas pendientes y refs de evidencia.
- `POST /v1/local/cases/{case_id}/requests/{request_id}/approve` requiere confirmación expresa, versión/digest esperado, destinatario y alcance de datos que se compartirían.
- `POST /v1/local/cases/{case_id}/requests/{request_id}/simulate-send` requiere aprobación vigente y consentimiento ligado a esa versión; solo registra `simulated_sent` en PostgreSQL. No realiza HTTP, correo, WhatsApp ni llamada a IA externa.
- Editar el borrador o cambiar la evidencia invalida la aprobación y requiere consentimiento nuevo.

## Respuesta administrativa

- `POST /v1/local/cases/{case_id}/responses` solo se permite a personal con permiso asignado para el caso.
- Recibe `response_type` (`request_information`, `guidance`, `referral`, `institutional_decision`), texto y fuentes de soporte.
- `institutional_decision` solo se guarda como atribuida a la persona administrativa autorizada. La IA no puede crearla.

## Seguridad y fallos

- JSON rechaza campos desconocidos. Límites de texto por campo y número de citas/evidencias se documentan en OpenAPI antes de implementar.
- `400`: entrada inválida; `401`: sesión ausente; `403`: rol, CSRF, consentimiento o Origin inválidos; `404`: caso no visible; `409`: versión/digest cambió; `503`: repositorio o modelo local no disponible.
- Mensajes, recibos, salud y contenido del caso nunca aparecen en logs técnicos. Proveedores y analítica externos no se invocan.
