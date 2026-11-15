# Research: Acompañamiento de trámites interrumpidos

## Fuente institucional descargada

- Fuente de distribución: [sección TUPA de UNCP en gob.pe](https://www.gob.pe/institucion/uncp/informes-publicaciones/tipos/16-tupa), que publica “TUPA – UNCP 2024” con fecha 7 de diciembre de 2024.
- Aprobación: [Resolución de aprobación publicada en El Peruano](https://busquedas.elperuano.pe/dispositivo/NL/2362138-1). Señala que se aprueba el TUPA UNCP actualizado con 151 procedimientos y deja sin efecto el TUPA aprobado por Resolución 3958-CU-2015.
- Copia descargada desde el enlace de documentos de gestión de UNCP: `sources/TUPA_UNCP_2024_Res_3226-R-2024.pdf`.
- SHA-256 del archivo: `9235e6813006846338c46d6d5bbeeeb4668f833285d18c180461308975dbbdd7`.
- Revisión del archivo: 6 de noviembre de 2026. La página institucional consultada mantiene publicada la edición 2024; no se ha confirmado con UNCP si existen modificatorias posteriores. El prototipo debe mostrar la versión/fecha y mantener vigente como “pendiente de confirmación” hasta revisión del área responsable.

## Hallazgos relevantes para el caso

1. **PA57002335 - Constancia de expedito para optar título profesional.** El TUPA lista S/ 198, con desglose de constancia S/ 5, ficha estadística S/ 2, diploma de título profesional S/ 188 y trámite documentario S/ 3. El listado incluye requisitos y un plazo de cinco días hábiles. La persona indicó que su pago fue S/ 198, por lo que este es el caso que guía la demo.
2. **PA570004D8 - Otorgar el título profesional.** Es otro procedimiento con requisitos distintos, S/ 3 por trámite documentario y plazo de 30 días hábiles. No se debe confundir con la constancia de expedito.
3. **PA57002563 - Devolución de tasas educacionales.** El TUPA describe una solicitud dirigida al jefe de la Sub Unidad de Tesorería, el comprobante de pago original, un pago de S/ 3 por trámite documentario y plazo de atención de 15 días hábiles. El documento no confirma que el pago específico de PA57002335 sea elegible para devolución tras una interrupción por salud, ni que pueda reutilizarse. El asistente debe plantear esa consulta a Tesorería.

Estos datos se reportan como contenido del TUPA descargado, no como asesoría jurídica ni decisión sobre un expediente individual. Para una atención real, el área administrativa debe confirmar vigencia y aplicación.

## Decisiones de diseño

1. **La persona, no la IA, dirige el caso.** Kusi Kusi organiza hechos, identifica pasos y prepara preguntas; no convierte recomendaciones en decisiones administrativas.
2. **La fuente precede a la respuesta.** Cada regla tiene ID/versión/cita y estado de vigencia validado por personal. El modelo solo puede citar IDs incluidos por la aplicación; una cita ausente, vencida o no verificada se etiqueta como pregunta abierta.
3. **Inferencia exclusivamente local.** Reutilizar el puerto `LlmProvider` con herramientas deshabilitadas. Solo se admite un perfil loopback de modelo local como Ollama; si no está disponible, mostrar guía determinista basada en las fuentes y decir que la asistencia generativa no está disponible. No enviar contenido a IA/analítica externa.
4. **Caso demo sintético.** Una persona, comprobante enmascarado y cronología ficticios presembrados. No se aceptan archivos reales de recibos ni médicos en esta fase.
5. **Consentimiento por envío.** Antes de compartir se muestra exactamente el borrador y sus adjuntos, el área destinataria y se pide aprobación expresa. En demo cambia solo a estado “envío simulado”.
6. **Capas del workspace.** Reutilizar patrones de sesión, permisos y trazas locales, pero mantener entidades propias para caso, línea de tiempo y fuente. El demo académico familiar existente no representa el flujo de titulación.

## Referencias internas inspeccionadas

- Autorización/consulta académica de prueba: `crates/application/src/academic.rs`, `crates/infrastructure/src/local/academic.rs`, `web/admin/src/features/AcademicDemo.tsx`.
- Sesiones, recursos, archivos y chat: `crates/domain/src/local.rs`, `crates/infrastructure/src/local/resource_access.rs`, `crates/infrastructure/src/local/assets.rs`, `apps/server/src/local.rs`, `web/admin/src/features/Chat.tsx`.
- Perfiles de inferencia y validación de endpoints loopback: `crates/infrastructure/src/llm.rs` y `docs/AI_PROVIDERS.md`.
- El almacenamiento de archivos actual admite imágenes y video, no recibos PDF. Por eso el recibo de esta demo es un fixture sintético preinstalado y no una carga arbitraria.
