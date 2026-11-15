# Tasks: Identidades externas y ACL

## Phase 1: Contrato
- [x] T001 Definir especificación, alcance manual, plan y criterios.
## Phase 2: Autorización
- [x] T002 Añadir contrato de recurso exacto y puerto/caso de uso, con pruebas negativas antes del transporte.
- [x] T003 Migración y repositorio PostgreSQL con ámbito, expiración y revocación.
## Phase 3: Operación
- [x] T004 CLI explícita para política, vínculo, concesión y revocación sin mostrar secretos.
- [x] T005 Actualizar OpenAPI, seguridad y roadmap.
## Phase 4: Validación
- [x] T006 Ejecutar gates y registrar límites de PostgreSQL/HTTP sin afirmar aceptación externa.
- [x] T007 Ejecutar la prueba PostgreSQL aislada y aceptación HTTP del endpoint con un adaptador sintético de recurso exacto; requiere un entorno con sockets habilitados. No activar proveedor real.

- [x] T008 Ejecutar el runner integral `scripts/verify_external_access.py`, revalidar la suite Rust completa y build Next en un contexto con sockets habilitados. Aprobados el 10 de noviembre de 2026; suite y build mediante unidades temporales systemd de usuario. No se omitieron ni alteraron pruebas para ocultar fallos.

T007 y T008 completadas. PostgreSQL desechable, HTTP loopback con conector sintético, CLI y servidor compilados, revocación sin reactivación y bloqueo de llamadas en modo local: aprobados. Gates Rust, Python y Next: aprobados. Evidencia: `acceptance.json`. Los intentos EPERM anteriores se conservan como históricos. No hay aceptación de proveedor externo ni conexión automática a canales.
