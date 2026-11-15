# Feature Specification: Identidades externas y ACL por recurso

**Feature Branch**: sin rama (checkout sin metadatos Git completos)
**Created**: 2026-11-10
**Status**: Implementation
**Input**: Continuar con identidades y ACL externas.

## User Scenarios & Testing

### User Story 1 - Vincular identidad verificada (Priority: P1)
Un operador vincula una credencial interna individual con un sujeto de un sistema externo después de comprobar su identidad y autorización fuera de banda.
**Why this priority**: Una credencial de organización no demuestra identidad personal.
**Independent Test**: Una vinculación solo resuelve para su organización, credencial y sistema.
**Acceptance Scenarios**:
1. Given una credencial vigente, When se confirma la comprobación manual, Then se crea un vínculo explícito con caducidad.
2. Given un vínculo revocado, caducado o de otro tenant, When se consulta, Then se deniega.

### User Story 2 - Leer solo un recurso concedido (Priority: P1)
Un principal autenticado consulta un recurso exacto mediante una capacidad registrada y una ACL vigente.
**Why this priority**: Evitar que permisos de servicio se conviertan en acceso general a registros.
**Independent Test**: Lectura permitida invoca una vez el adaptador; recurso, versión, sistema o principal distinto nunca lo invocan.
**Acceptance Scenarios**:
1. Given vínculo y ACL vigentes, When se pide el recurso exacto, Then se valida el contrato y se devuelve su resultado.
2. Given permiso ausente, almacenamiento no disponible o argumentos adicionales, When se consulta, Then se falla cerrado antes del transporte.
3. Given un resultado etiquetado con otro recurso, When vuelve el adaptador, Then se rechaza.

### User Story 3 - Revocar (Priority: P2)
Un operador revoca vínculos o ACL sin reiniciar el proceso.
**Why this priority**: Cambios de acceso deben afectar la próxima lectura.
**Independent Test**: Repetir una lectura después de revocar produce denegación.
**Acceptance Scenarios**:
1. Given un vínculo revocado, When se reprovisiona por accidente, Then no se reactiva ni hereda concesiones antiguas.

### Edge Cases
Credencial revocada, identidad externa distinta, ACL caducada, cambio de versión, error de almacenamiento, resultado de otro recurso, cambio de adaptador y auditoría no disponible.

## Requirements
### Functional Requirements
- **FR-001**: Tenant y principal MUST proceder de credenciales verificadas; ningún campo de entrada los sustituye.
- **FR-002**: Cada sistema protegido MUST exigir vínculo y ACL por organización, principal, sistema, capacidad, versión y recurso.
- **FR-003**: Vínculos y concesiones MUST tener caducidad y revocación; las lecturas no usan caché de autorización.
- **FR-004**: Un operador MUST confirmar comprobación manual; no se afirma verificar permisos remotos automáticamente.
- **FR-005**: Permiso global, argumentos, ACL y contrato de salida MUST validarse; el recurso de salida debe coincidir con el solicitado.
- **FR-006**: El modo local MUST seguir bloqueando llamadas externas y pagadas; las pruebas usan dobles sintéticos.
- **FR-007**: Auditoría y errores MUST excluir sujetos externos, credenciales, argumentos y resultados.
- **FR-008**: Capacidades sin política de recurso conservan su contrato de servicio; las políticas de sistemas protegidos no pueden eludirse usando otra versión del mismo adaptador.
### Key Entities
Vínculo de identidad con comprobación manual; política por sistema/adaptador; concesión de recurso/version; principal autenticado.

## Success Criteria
### Measurable Outcomes
- **SC-001**: 100% de casos sin vínculo/ACL exactos deniega antes del transporte.
- **SC-002**: Revocaciones y caducidad afectan la siguiente lectura.
- **SC-003**: Cero credenciales, sujetos o cuerpos en los eventos de auditoría añadidos.
- **SC-004**: Dos sistemas sintéticos independientes no comparten permisos.

## Assumptions
Primera entrega backend, sin UI ni nuevo login. El principal es una credencial individual ya autenticada, nunca una clave compartida para representar personas distintas. El operador confirma identidad y permiso externos manualmente. Sin selección de proveedor, sincronización automática de ACL, SSO, mensajería ni cambios en staging. Los adaptadores de lectura deben respetar el contrato de recurso exacto.
