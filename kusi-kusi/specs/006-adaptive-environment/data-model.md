# Data model

- Actor: organización, principal y fuente obtenidos del contexto privado registrado.
- Catalog: fuente, versión/digest, recursos y campos autorizados; nuevo descubrimiento por solicitud.
- Resource: id, etiqueta, campos y eventos soportados; no contiene valores de registros.
- Field: id/etiqueta, tipo temporal/numérico/categórico, opciones permitidas.
- Plan: versión de catálogo y hasta 4 consultas; cada consulta hasta 8 filtros y 8 análisis.
- Filter: campo existente, operador registrado, valor compatible; sin expresiones ejecutables.
- Analysis: conteo, agrupación, suma/media, comparación temporal o proyección por fecha.
- Evidence: fuente/recurso/versión, filas consultadas, filtros, resultados, método y límites.
- Interpretation: término aclarado, significado estructurado, recurso y versión del catálogo.
- Context v1: último plan, aclaración pendiente e interpretaciones semánticas aprobadas, separado por actor; nunca otorga permisos.
- Subscription v1: recurso/evento/filtros, destinatario obtenido del actor registrado, estado y versión.
- Execution v1: claim exclusivo, recibo con estado de respuesta y auditoría versionada (`AuditEntry`) de decisiones y procedencia sin almacenar el mensaje original.

Límites: hasta 10 000 filas por ejecución; proyección de 1 a 366 días; fechas válidas;
no resultados completos sobre páginas truncadas. Estados de suscripción: pending/active/cancelled.
Entregas: claimed/accepted/unknown. Un resultado ambiguo no se reintenta ciegamente.
Los archivos se crean privados y los reemplazos son atómicos. Un cambio incompatible
requiere migración explícita de su formato versionado; no se toca el PostgreSQL existente.
