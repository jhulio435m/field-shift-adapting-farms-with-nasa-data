# Contract provisional: Inicio de sesión OIDC del panel

Las rutas son same-origin a través del proxy fijo de Next. No aceptan issuer, callback, tenant, rol ni URL de retorno arbitrarios del navegador.

## GET `/admin/auth/oidc/start` (ruta propuesta; depende del plano de control)

- Disponible solo cuando un proveedor para la instancia fue habilitado explícitamente por el operador.
- Crea transacción efímera ligada al navegador y redirige al authorization endpoint configurado.
- Parámetros de protocolo generados en servidor; usar Authorization Code + PKCE S256 y valores únicos correlacionados.
- Respuestas privadas con `Cache-Control: no-store`.

## GET `/admin/auth/oidc/callback` (ruta propuesta; depende del plano de control)

- Procesa solo la transacción pendiente que coincide con cookie y `state` de un uso.
- Verifica respuesta OIDC y vincula `(issuer, subject)` a una cuenta ya existente y activa.
- En éxito emite la cookie de sesión opaca actual, HttpOnly/SameSite y Secure en producción, junto al token CSRF en respuesta same-origin.
- En error no emite cookie; devuelve un mensaje genérico sin tokens, claims, códigos ni secretos.
- Idempotencia: una transacción/código aceptado no puede consumirse de nuevo.

## Modo local

- Las rutas OIDC de producción no se montan en el router del workspace local y no hacen requests de red a ningún emisor.
- El login de usuario/contraseña actual permanece sin cambios para el entorno local.

## Errores visibles

- `404/503`: login OIDC no habilitado.
- `400`: transacción ausente, vencida, repetida o mal correlacionada.
- `401`: identidad inválida, sin vínculo, deshabilitada o de organización no disponible.
- Error remoto de proveedor se normaliza a fallo genérico; nunca se devuelve respuesta cruda.
