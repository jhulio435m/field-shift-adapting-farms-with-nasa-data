# Plan

1. Crear en aplicación un Environment protegido por CapabilityRegistry + AuthorizationService + AuditRepository. Cada recurso tiene binding privado a capacidad/version/resource_ref; exigir política protegida y permisos antes de llamar a la fuente. Catálogo filtrado y versión ligada a ámbito; conservar catálogo original para la validación del adaptador subyacente.
2. Componer con Sources, FileState y LanguagePlanner en adaptive_assistant solo cuando la configuración privada incluya ACL. Variable KUSI_ADAPTIVE_REQUIRE_ACL=true rechaza omisión; no cambiar staging. Conexión PostgreSQL desde variable explícita, sin cargar .env en el binario. No cambia esquema: reutiliza 0018.
3. Asegurar que el transporte registrado conserve la exigencia ACL y no borre el bloqueo KUSI_LOCAL_MODE. Probar sin envíos reales.
4. Usar contextlib.closing para SQLite además del contexto transaccional; arreglar fixtures y verificar ResourceWarning.
5. Runner OIDC visual: servidor Rust y Next en puertos loopback temporales, emisor sintético, PostgreSQL propio, Playwright Chromium. Revisar PNGs desktop/móvil. Sin cambios UI salvo defectos demostrados.
6. Actualizar contratos de configuración/seguridad y roadmap; ejecutar gates y aceptación real local.

Constitución I–VI: núcleo genérico, puertos en aplicación/adaptadores en infraestructura, identidad privada, comprobación y auditoría antes de ejecutar, sin cambios remotos, pruebas/migración solo si cambia esquema. El wrapper representa una política de autorización real, no una capa de reenvío sin responsabilidad.
