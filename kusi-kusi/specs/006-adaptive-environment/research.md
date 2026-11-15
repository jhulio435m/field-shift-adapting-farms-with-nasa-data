# Research

## Catálogo y permisos
Decision: describir recursos tipados y capacidades de cada fuente mediante puertos.
Rationale: los adaptadores actuales identidad/salud y los scripts por DocType no satisfacen
adaptación. Los permisos se renuevan durante descubrimiento y justo antes de leer.
Alternatives: lista sectorial de intents (rechazada); SQL libre del modelo (rechazado).

## Planes y análisis
Decision: planes acotados de lectura + análisis genérico, con campos y filtros tipados.
Rationale: componer primitivas compartidas permite preguntas nuevas sin funciones por negocio.
No se acepta código, URLs ni efectos arbitrarios en planes. Los análisis son deterministas.
Alternatives: un handler por pregunta (rechazado); agente con herramientas sin autorización (rechazado).

## Adaptadores
Decision: manifiestos dinámicos para aceptación y Frappe con descubrimiento de permisos/metadatos.
Rationale: dos fuentes distintas pueden ejercitar el mismo ejecutor, y Frappe permite usar
recursos nuevos sin instalar código por DocType. El puente usa destino/credenciales registrados.
Alternatives: dar System Manager al bot (rechazado); deducir semántica sin preguntar (rechazado).

## Contexto, estados y eventos
Decision: contexto estructurado por identidad registrada; formatos privados versionados,
claims exclusivos y reemplazo atómico. Suscripciones con ID estable, filtros y origen;
revalidación antes de entregar, cancelación y estado incierto ante fallo ambiguo.
Rationale: memoria no equivale a autorización; eventos y polling no se confunden.
Alternatives: transcripciones completas como permisos (rechazado); reintento ciego (rechazado).

## Evidencia
Investigación independiente de planificación revisó los puertos existentes e identificó
inyección desde metadatos, revocaciones, truncación y declaraciones de éxito prematuras
como riesgos. Fixtures prueban composición; pruebas reservadas con intérprete real y
catálogo son necesarias para demostrar comprensión, y deben distinguirse en el informe.
