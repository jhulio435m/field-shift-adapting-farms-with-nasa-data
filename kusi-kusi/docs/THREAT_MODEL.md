# Modelo básico de amenazas

Activos: identidades locales, claves de servicio, tokens OAuth cifrados, configuración, conversaciones, archivos, propuestas y decisiones de aprobación. Fronteras: navegador/API; API/PostgreSQL; trabajador/decodificador multimedia; webhooks/proveedor; conectores/sistemas de clientes; OAuth/credenciales externas.

| Amenaza | Control actual | Riesgo pendiente |
|---|---|---|
| Acceso de otro tenant | Identidad verificada, filtros de organización y pruebas cruzadas | RLS, auditoría independiente de todas las rutas |
| Escalada de rol o acceso directo por API | RBAC en middleware y servicios, revocación de sesiones | OIDC/MFA y permisos externos por recurso |
| CSRF o encabezado interno falsificado | SameSite, token de sesión, origen y borrado del encabezado entrante | TLS/proxy y revisión de cookies en despliegue |
| Repetición/caída durante procesamiento | Inbox único, UUID lógico, leases e idempotencia | Escala/carga y conciliación operativa |
| Reusar aprobación tras cambiar texto/archivo | Snapshot versionado, digest y vencimiento | Publicación real y conciliación por proveedor |
| Archivo malformado o agotamiento de recursos | Validación, límites, timeout y rutas internas | Sandbox/antivirus, cuotas de disco/CPU |
| SSRF o datos externos hostiles | Destino allowlisted, DNS fijado, esquema y separación de datos | Revisión de cada adaptador adicional |
| Prompt injection o datos inventados | Asistente local determinista, sin herramientas libres/SQL | Orquestación LLM autorizada y evaluaciones adversariales |
| Robo de token OAuth | Cifrado autenticado, keyring externo, metadatos sin token | Gestor de secretos, callbacks de eliminación y revisión real |
| Pérdida de datos | Dump, archivo inmutable, hashes y restauración temporal | Cifrado/offsite, RPO/RTO y simulacro completo |

Las limitaciones se detallan en SECURITY.md. Ningún control aislado demuestra cumplimiento de la Ley 29733 o seguridad de producción.
