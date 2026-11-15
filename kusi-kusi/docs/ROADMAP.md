# Estado y siguientes entregas

## Implementado localmente

- Rust/Axum/Tokio, PostgreSQL/migraciones, salud y OpenAPI.
- Registro genérico de capacidades y transporte HTTP autorizado, sin dependencia de un sistema académico.
- Adaptadores de IA y transcripción intercambiables, webhooks oficiales Meta/Telegram y OAuth social.
- Panel React/TypeScript, sesiones locales y cuatro roles, usuarios/organizaciones.
- Archivos privados, conversaciones con contexto y agrupación, cola persistente y recuperación.
- Borradores y aprobaciones creativas, vistas previas FFmpeg rotuladas y archivadas.
- Publicaciones simuladas, calendario, edición versionada, aprobación independiente y recibos locales.
- Autorización de lectura por recurso exacto, ligada a sesión y organización, con dos fixtures ficticios, revocación inmediata y auditoría sin payload.
- Inventario y revocación de sesiones locales por cuenta/organización, con protección de la sesión actual y sin datos de transporte.
- Auditoría, retención de mensajes y respaldo con restauración temporal y cotejo de archivos.
- Base del plano administrativo OIDC separada del workspace local: identidades/membresías provisionadas, sesiones revocables, rutas servidoras y pantalla para ver organizaciones. Permanece desactivada por defecto; protocolo HTTP y revisión visual desktop/móvil aprobados con emisor loopback y PostgreSQL desechable.

Las verificaciones están en VALIDATION.md. Adaptador implementado no significa conexión real validada.

## Piloto elegido y configuración web

El usuario eligió Plane y ERP CONEIMERA como sistemas, y Telegram como canal. La entrega 009 añade configuración web de sus URLs, espacios y credenciales desde Conexiones, con almacenamiento cifrado, revisión, rotación/borrado y permisos de propietario. Es configuración local guardada sin verificar: no activa los adaptadores heredados ni registra webhooks. Ver WEB_CONNECTION_SETTINGS.md. 010 ya consume estos perfiles en un runtime fixture autorizado. El siguiente paso real es integrar el ciclo provider con el plano administrativo habilitado y validar identidad, permisos y alcance contra los destinos del piloto.

## Entrega 010: operación local

[010-managed-integrations](../specs/010-managed-integrations/spec.md) incorpora manifiestos, verificación fixture ligada a revisión, consumo autorizado del perfil por lecturas sintéticas/asistente, vínculos revocables, historial/revisión de outbox, evaluación de 24 casos y memoria propia con simulación de permisos sobre recursos de demostración. Las pruebas y límites se registran en [acceptance.md](../specs/010-managed-integrations/acceptance.md). No configura ni habilita proveedores reales desde el plano OIDC; la activación y aceptación remotas continúan como siguiente integración.

## Próximo piloto con proveedores

1. Revisión visual de la pantalla completada localmente (004/T015, 008); elegir el primer proveedor corporativo y verificar MFA, TLS, ciclo de vida de claves y gestor de secretos. RLS/defensa adicional permanece pendiente.
2. Vinculación de identidades de sistemas externos y ACL verificables por recurso, antes de permitir consultas empresariales desde mensajería. Base backend 007 implementada con aprovisionamiento manual, políticas por adaptador y concesiones exactas; aceptación PostgreSQL/HTTP y runner integral aprobados con datos sintéticos; verificación remota automática pendiente (ver EXTERNAL_IDENTITIES_ACL.md). 008 conecta esas ACL al asistente mediante bindings privados por recurso analítico y al canal registrado, con pruebas de binario, PostgreSQL, intérprete HTTP sintético y avisos simulados. No se activó en staging ni acredita permisos remotos automáticos.
3. El asistente admite planes tipados y autorización por recurso (006–008). Falta ampliar la orquestación de herramientas y conectar transcripción al trabajador; presupuestos de IA y reglas de retención.
4. Validación real Meta/Telegram con credenciales y dominio HTTPS, envío de outbox, control de reintentos y conciliación de estados.
5. Cuentas OAuth reales; selección de páginas/perfiles; callbacks de desautorización/eliminación; validaciones oficiales y publicación nativa por plataforma.
6. Activación explícita de proveedores creativos, almacenamiento permanente de resultados remotos y pruebas de calidad/costos.
7. Observabilidad externa, alertas, CI real, despliegue reproducible probado y recuperación ante desastres.

## Expansión posterior

MCP con versión de especificación documentada, agente local, SDK según demanda y medición de carga. Redis/brokers solo con necesidad demostrada. Ninguna fase obliga a usar software académico ni a migrar los sistemas de los clientes.
# Próximo bloque: integrar la plataforma de pagos del hackatón

- Cambiar el adaptador de fixtures por el contrato/API del sistema de pagos que desarrolle el equipo; recibir esquema, autenticación y entorno de pruebas.
- Resolver la identidad del estudiante mediante vínculo verificado antes de consultar historial o validar referencias.
- Habilitar recuperación de foto desde WhatsApp Cloud API con token de medios y límites de descarga; reutilizar la extracción OCR local y confirmar la coincidencia contra la plataforma de pagos.
- Probar estados de pago y trámite con datos de prueba y mantener claramente etiquetada la fuente de cada resultado.
