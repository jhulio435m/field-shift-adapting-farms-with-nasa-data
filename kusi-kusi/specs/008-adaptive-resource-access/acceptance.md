# Aceptación local 008 — 10 de noviembre de 2026

## Matriz

| Requisito | Evidencia |
| --- | --- |
| FR-001/002/004 | Configuración privada/canal vinculado, principal hash, adaptador/capacidad/version/recurso exactos; binario rechaza tenant en entrada y deniega política ausente. `adaptive-acceptance.json`. |
| FR-003/005 | Pruebas de aplicación con contadores, revocación previa a lectura/aviso/replay; limpieza de borradores/esquemas/preguntas/pistas revocados; PostgreSQL y binario reales con intérprete HTTP determinista. |
| FR-006 | Configuración sin access y KUSI_ADAPTIVE_REQUIRE_ACL=true bloqueada antes del intérprete; transporte conserva la exigencia. |
| FR-007/008 | Auditoría por metadatos, datos sintéticos, bases nuevas y procesos temporales; transportes reales no activados. |
| FR-009 | `qa/acceptance.json`, seis PNG desktop/móvil; revisión visual efectuada. Error de callback/reintento, sesión, membresía y revocación, cookie HttpOnly, sin clave global en frontend/localStorage, foco de teclado, logout CSRF. |
| FR-010 | Closing explícito con contexto transaccional, regresión de conexión cerrada y 14 pruebas Python sin ResourceWarning. |

## Fallo corregido en la frontera Next/Rust

El proxy no reenviaba Location y rompía la redirección OIDC del panel. Ahora preserva esa cabecera del backend fijo sin seguir redirects upstream. El navegador real recorre el flujo completo; verify_next.py pasa contra usuarios y API desechables, incluyendo cookie/Origin/CSRF/no-store/rutas directas/logout. La suite de protocolo OIDC con emisor sintético también pasa completa.

## Límites

Una ACL de recurso analítico autoriza un conjunto completo ya permitido por su origen; no implementa autorización por fila ni consulta remota automática de permisos. Staging permanece sin modificaciones. No se habilitaron cuentas OAuth, LLMs de pago, mensajes reales, publicación social ni proveedor OIDC real. Los modelos del test son intérpretes deterministas, no evidencia de calidad de lenguaje natural. RLS, observabilidad/despliegue/recuperación productivos y decisiones/contratos de proveedores permanecen fuera de esta aceptación.

Los PNG contienen solo cuentas y organizaciones sintéticas. La prueba 007 opt-in fue reejecutada con PostgreSQL desechable y aprobó; las dos pruebas no se consideran omitidas por el hecho de que estén marcadas ignored en el comando general de Cargo.

## Gates finales

Fmt y Clippy sin advertencias: aprobados. Cargo workspace: 80 resultados aprobados, cero fallos; las dos pruebas opt-in de PostgreSQL/HTTP también se ejecutaron explícitamente. Python: 14 pruebas aprobadas con ResourceWarning tratada como error, sin advertencias de conexiones abiertas. Formato admin y build Next Turbopack/TypeScript: aprobados.

Las imágenes locales kusi-kusi-api:qa008 y kusi-kusi-web:qa008 se construyeron y probaron mediante verify_container.py --isolated: PostgreSQL nuevo, migraciones/salud, FFmpeg/fuente, SSR/CSS, proxy fijo, no-store, sesión no autorizada y OpenAPI. El tráfico HTTP se comprobó desde el namespace del contenedor web porque la red Docker es interna, sin gateway externo. Los bindings publicados están limitados a 127.0.0.1. Los ids exactos están en container-acceptance.json. Las imágenes no se publicaron y no reemplazan los tags latest. La imagen API sirve kusi-server; la aceptación del binario adaptive_assistant se realiza por separado fuera de esa imagen.

La compilación Rust en Docker necesitó --network=host para descargar dependencias de construcción; el entorno de ejecución de la prueba usa red interna. No se interpretan esas descargas como llamadas a proveedores del producto. Los procesos, contenedores y red temporales se eliminaron; los servicios existentes permanecen.
