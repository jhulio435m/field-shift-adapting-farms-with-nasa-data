# Plan

Aplicación: DTOs de perfil/parche de secretos, validación de URL y campos por adaptador, caso de uso con propietario verificado y puerto de repositorio.
Infraestructura: migración 0019; implementación PostgreSQL en LocalStore y cifrado XChaCha20-Poly1305 reutilizando TokenVault. Clave maestra 0600, creación exclusiva y rechazo si se perdió con datos cifrados existentes. AAD por tenant/perfil/adaptador/revisión. Lock transaccional por perfil, control de revisión y auditoría atómica.
HTTP: rutas /v1/local/connection-settings y /{kind}, sesión HttpOnly y middleware existente; no endpoints de descifrado ni activación real.
UI: ampliar Conexiones conservando identidad visual y otros módulos; tres filas configurables, diálogo con campos públicos y secretos de escritura, borrar credenciales con confirmación y sin persistencia frontend.
Verificación: pruebas de validación/RBAC, API con PostgreSQL desechable, persistencia/cifrado/clave, navegador real desktop/móvil y regresión Next/OIDC. No cambian proxy ni frontera runtime; mantener verificadores existentes.
Constitución I–VI: puertos/adaptadores, tenant verificado, secretos cifrados, simulación explícita, migración/contrato y evidencia. No se usan credenciales reales ni se conectan servicios de clientes.
