# Plan: identidades externas y ACL

Dominio: validación de lectura exacta y sobre de respuesta independiente de transportes.
Aplicación: ampliar el puerto AuthorizationService y aplicar autorización de recurso antes del conector en ExecuteCapability. La ausencia de política mantiene contratos de servicio existentes.
Infraestructura: migración 0018 con políticas, vínculos y concesiones; PostgreSQL comprueba clave, vínculo y ACL activos usando tiempo del servidor en cada solicitud. El adaptador coincide con el sistema registrado.
Composición: CLI de operador para proteger un adaptador, vincular, conceder y revocar mediante variables de entorno; sin endpoint administrativo nuevo. Reutilizar POST /v1/capabilities/execute y su identidad verificada.
Contrato protegido: entrada únicamente resource_ref; respuesta con resource_ref idéntico y data. Sin filtros libres, URLs o sujetos elegidos por cliente.
Seguridad: política por adaptador para cubrir todas sus versiones; prueba manual explícita, caducidad obligatoria y no reactivación implícita. No caché ni llamadas de comprobación remotas. Auditoría existente solo metadatos de capacidad y resultado.
Constitución I–VI: contrato genérico, puertos/adaptadores, tenant autenticado, simulación local intacta, migración/OpenAPI/docs, pruebas y límites documentados.
Verificación: pruebas de aplicación con contador de invocaciones y repositorio PostgreSQL condicionado a TEST_DATABASE_URL loopback; fmt/clippy/test y Python. Sin cambio frontend/runtime.

## Aceptación HTTP añadida

`apps/server/src/capabilities.rs` concentra deserialización, credencial verificada, ejecución y mapeo de errores. `main.rs` conserva la composición del RuntimeConnector real por organización y el middleware local. El test inyecta un ConnectorExecutor sintético exclusivamente en su propio router loopback; no existe variable de configuración para habilitarlo en producción. No se relajaron HTTPS, allowlists ni SSRF para probar respuestas exitosas. Runner desechable en scripts/verify_external_access.py.
