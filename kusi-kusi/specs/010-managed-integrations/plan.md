# Implementation Plan: Integraciones gestionadas y operación verificable

**Branch**: sin rama creada | **Date**: 2026-11-10 | **Spec**: [spec.md](spec.md)

**Input**: `specs/010-managed-integrations/spec.md`

## Summary

Extender 007–009 con resolución de conexiones versionadas, manifiestos propios, administración Telegram, timeline recuperable, evaluaciones sintéticas y memoria autorizada. Primera entrega vertical: US1; US2 estabiliza el catálogo compartido. Incorporar patrones de los referentes, manteniendo el monolito modular y los puertos existentes.

## Technical Context

**Language/Version**: Rust 1.95.0 y TypeScript/Next.js existentes; Python 3 para aceptación.

**Primary Dependencies**: Axum, SQLx/PostgreSQL, Tokio, cifrado existente XChaCha20-Poly1305; Next App Router y shadcn/ui locales. Sin nueva dependencia de motores de agentes.

**Storage**: extender PostgreSQL con migraciones nuevas posteriores a 0019; conservar bóveda privada de 009. Reutilizar inbox/outbox y auditoría; no crear una segunda cola.

**Testing**: Rust unitarias/integración; aceptación Python con PostgreSQL desechable y fixtures, navegador desktop/móvil y contenedores de dos imágenes.

**Target Platform**: Linux; desarrollo solo 127.0.0.1:3000 y 127.0.0.1:8080.

**Project Type**: monolito modular con frontend Next separado.

**Performance Goals**: validación con plazo máximo de 10 segundos por probe, una solicitud activa por perfil/revisión; timeline paginado de 50 elementos. Son presupuestos de diseño a comprobar, no mediciones actuales.

**Constraints**: local sin egress/pagos, credenciales servidoras, ACL vigente por recurso, sin SQL ni herramientas arbitrarias de LLM. Probes con allowlist de destino, sin redirecciones, bloqueo de IP privadas/link-local/metadata y revalidación DNS en conexión. Fixtures inyectados por transporte de prueba; no excepción de red accesible por formulario.

**Scale/Scope**: tres conectores, seis historias, 24 ejemplos; operación empresarial inicial de lectura. Telegram puede producir efectos de canal únicamente en entorno habilitado y mediante outbox autorizada.

## Constitution Check

Pre-diseño: PASS. Núcleo genérico y contratos propios; dominio sin frameworks; autorización en aplicación; adapters en infraestructura; HTTP en server. Local bloquea egress aun para probar conexiones. Sin excepciones.

Post-diseño: PASS. La verificación no concede ACL; revisión y activación están separadas; memoria/evidencia se reautorizan; secretos solo en infraestructura; migraciones, OpenAPI y aceptación forman parte de tareas. Guardar no activa el canal. Adopción de bibliotecas externas queda documentada, sin cambios de toolchain implícitos.

## Project Structure

### Documentation (this feature)

`spec.md`, `research.md`, `data-model.md`, `contracts/api.md`, `quickstart.md`, `tasks.md`, `checklists/requirements.md` y `acceptance.md`.

### Source Code (repository root)

- `crates/domain/src/managed_integrations.rs`: estados/versiones e invariantes sin dependencias de transporte.
- `crates/application/src/connection_settings.rs`: compatibilidad de DTOs de 009.
- `crates/application/src/managed_integrations.rs`: casos de uso y puertos de verificación/resolución.
- `crates/application/src/connector_manifest.rs`: contrato y validación de capacidades.
- `crates/application/src/channel_operations.rs`, `execution_review.rs`, `evaluation.rs`, `authorized_memory.rs`: responsabilidades de US3–US6.
- `crates/infrastructure/src/local/connection_settings.rs`: acceso privado a la bóveda existente.
- `crates/infrastructure/src/managed_integrations/`: estado persistente, resolver y probes por proveedor.
- `crates/infrastructure/src/adaptive/`: conectar las fuentes al resolver autorizado.
- `apps/server/src/local/managed_integrations.rs`: HTTP de configuración/verificación y bloqueo de habilitación local; `managed_operations.rs`: operación privada y errores comunes.
- `web/admin/src/features/ConnectionSettings.tsx`: ampliar pantalla; nuevas vistas de operación/evaluación con componentes locales.
- `migrations/0020_managed_connection_lifecycle.sql` y siguientes: esquema evolutivo; confirmar número libre antes de crear.
- `contracts/openapi/openapi.yaml`: contrato público vigente durante implementación.
- `scripts/verify_managed_integrations.py` y `verify_managed_integrations_browser.py`: aceptación sintética nueva.

**Structure Decision**: módulos por responsabilidad; reutilizar autorización y persistencia existentes. Las rutas/nombres nuevos son diseño propuesto, no archivos ya implementados.

## Phases

### Phase 0 — Research

Decisiones en [research.md](research.md). Los contratos remotos exactos Plane/Frappe se fijan con documentación oficial y fixtures antes de implementar probes; la disponibilidad de instalaciones reales es dependencia de aceptación remota, no requisito para desarrollo local. Sin decisiones técnicas abiertas que bloqueen el diseño local.

### Phase 1 — Design & Contracts

Modelo y transiciones en [data-model.md](data-model.md), contrato HTTP propuesto en [contracts/api.md](contracts/api.md), aceptación en [quickstart.md](quickstart.md). Conservar rutas y semántica de 009 y ampliar mediante endpoints nuevos.

### Incremental Delivery

US1 establece resolución y ciclo, usando un registro interno mínimo de proveedores. US2 expone manifiestos y valida compatibilidad. US3 reutiliza el canal registrado y outbox. US4 amplía estados durables existentes. US5 añade evaluación; US6 memoria y simulador sobre ACL actuales. Cada incremento entrega su UI y aceptación antes del siguiente.

## Complexity Tracking

Sin violaciones constitucionales ni motores nuevos. Rig y Cedar se evalúan mediante ADR, sin adopción obligatoria: Rig main exige toolchain superior; Cedar solo aporta valor si el simulador sobre ACL actuales resulta insuficiente.


## Diseño ejecutado

La composición opcional `managed_fixture` conecta Sources con el resolver antes de la autorización del recurso y exige planner loopback literal sin clave. Las rutas nuevas comparten límite de cuerpo de 32768 bytes sin reducir el de uploads existentes. Los endpoints de memoria usan el puerto `AuthorizedMemoryRepository` y los casos de uso de aplicación; la infraestructura realiza cifrado, SQL y bloqueos de consentimiento/concesión. La propuesta privada de outbox se recupera en `/managed-executions/{id}/proposal` solo cuando es revisable, por su actor autorizado; la UI muestra destino/contenido exactos y exige que el digest coincida antes de habilitar aprobación. No se devuelve el cuerpo en listas/timeline.
