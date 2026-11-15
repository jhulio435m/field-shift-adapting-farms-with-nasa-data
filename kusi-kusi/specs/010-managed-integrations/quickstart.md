# Quickstart de validación local

Usar PostgreSQL desechable y datos ficticios. No cargar `.local-data`, `.local-backups` ni credenciales reales. UI 127.0.0.1:3000 y Rust 127.0.0.1:8080; seguir arranque documentado en README. Los runners existentes crean su propio entorno desechable; comprobar sus opciones con `--help` antes de ejecutarlos.

## Gates existentes

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 -m unittest discover -s scripts -p 'test_*.py'
npm run format:check --prefix web/admin
npm run build --prefix web/admin
python3 scripts/verify_connection_settings.py --result specs/010-managed-integrations/settings-regression.json
python3 scripts/verify_external_access.py --result specs/010-managed-integrations/external-regression.json
python3 scripts/verify_adaptive_access.py --result specs/010-managed-integrations/adaptive-regression.json
python3 scripts/verify_next.py
```

## Aceptación nueva prevista

Los runners nuevos funcionan sin secretos y sin flags que permitan egress:

```bash
python3 scripts/verify_managed_integrations.py
python3 scripts/verify_managed_integrations_browser.py --artifacts specs/010-managed-integrations/qa
```

1. US1: dos organizaciones sintéticas, tres perfiles; guardar/verificar fixture, probar bloqueo provider/enable local y aislamiento. Cambiar revisión durante probe, comprobar stale y ausencia de habilitación. Probar resolver con transporte fixture inyectado y ACL denegada/revocada.
2. US2: contrastar manifiestos y formularios; operación/versión desconocida y argumentos inválidos se rechazan antes del transporte.
3. US3: pairing sintético vigente/vencido, evento duplicado y fuera de orden, revocación y cancelación concurrente.
4. US4: reinicio con tareas paused/uncertain; aprobación obsoleta inválida y conciliación sin segundo efecto. Historial ajeno inaccesible.
5. US5: ejecutar los 24 casos versionados; probar fuente revocada y confirmar cero solicitudes externas/pagas.
6. US6: caducidad/borrado/revocación de memoria y simulación sin escrituras en grants.
7. Navegador desktop/móvil: completar historias sin secretos en DOM, estado frontend, respuestas ni localStorage. Revisar cookies/CSRF/no-store, errores y estados vacíos.
8. Ejecutar checks de navegador y contenedores de dos imágenes siguiendo `scripts/verify_browser.py`, `scripts/verify_container.py` y runners aislados existentes. Mantener `verify_next.py` para límite frontend/runtime.

Registrar comandos, fechas, resultados y límites en acceptance.md y docs/VALIDATION.md. Los escenarios provider reales se mantienen pendientes hasta disponer de entorno explícitamente habilitado; no cuentan como aprobados por fixture.


Los runners que declaran `--result` requieren una ruta de salida; consultar `--help` antes de invocarlos. La aceptación 010 también ejecuta la prueba Rust opt-in de migraciones en esquema aislado y el asistente compilado con un planificador HTTP sintético. Compilar todos los binarios con `cargo build -p kusi-server --bins` antes de usarla. Contenedores: `python3 scripts/verify_container_isolated.py --api-image kusi-kusi-api:qa010 --web-image kusi-kusi-web:qa010 --require-managed --result specs/010-managed-integrations/container-acceptance.json` tras construir ambas imágenes desde el checkout actual.
