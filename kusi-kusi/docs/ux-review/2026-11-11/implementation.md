# Mejoras del recorrido inicial

Inicio (/inicio) reúne preparación, acceso y consulta. La raíz abre Inicio. Navegación agrupada por trabajo, sistemas, contenido y administración; las rutas existentes siguen disponibles.

Consulta de demostración con permisos del servidor y resultado con fuente/evidencia. Los propietarios pueden solicitar lecturas sintéticas de Plane, Frappe y Telegram si hay configuración verificada y autorización existente; no se conceden permisos ni se crean vinculaciones automáticamente. El chat creativo enlaza al recorrido de consulta y explica su propósito.

Las secciones del piloto usan URL, admiten recarga y navegación atrás. Avisos y errores se limpian al cambiar destino. Evaluación agrupada por conector, fallos primero y detalles desplegables para móvil. Ejecuciones identificadas como respuestas locales con referencia corta; memoria con nombres comprensibles para recursos conocidos.

Validación: cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace, npm format:check, build Turbopack y verify_next_isolated pasaron. La aceptación Chromium ampliada prueba Inicio, lectura con viewer autorizado, aviso contextual y recarga de sección, además del recorrido previo. Consultar browser-acceptance.json para resultado. No se modificó la frontera frontend/runtime ni el contrato API. No se reconstruyeron contenedores para esta modificación de interfaz.

Limitaciones: el resolver privado y la activación real T007/T036 continúan pendientes. Este recorrido solo consulta fixtures; no acredita conectividad remota ni usabilidad con participantes reales. No se reiniciaron servicios persistentes.
