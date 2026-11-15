# Research: Inicio de sesión del panel con OIDC

## Decisiones

1. **Usar Authorization Code con PKCE S256 y correlación de solicitud de un uso.** RFC 9700 recomienda PKCE también para clientes confidenciales y requiere ligar el challenge/nonce a la transacción y al navegador. El cliente verificará `state`, PKCE y `nonce`; una transacción vencida o repetida falla cerrada.
   - Fuente primaria: [RFC 9700, OAuth 2.0 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700.html).
   - Alternativa descartada: implicit flow y validación basada únicamente en la URL de retorno.

2. **Validar tokens conforme a OpenID Connect Core.** Rechazar tokens si `iss` no coincide exactamente con el emisor configurado, si `aud` no contiene el client ID esperado, si firma/tiempo/nonce no son válidos. Nunca confiar en `email` como identificador estable.
   - Fuente primaria: [OpenID Connect Core 1.0, ID Token Validation](https://openid.net/specs/openid-connect-core-1_0.html).
   - Alternativa descartada: decodificar JWT sin verificación o aceptar claves indicadas por el propio token.

3. **Mantener el modo local sin OIDC.** Las pruebas usarán un emisor simulado enlazado exclusivamente a loopback. El backend local no carga configuración real ni realiza discovery/exchange hacia Internet.
   - Razón: instrucción del repositorio de bloquear llamadas externas y conservar modo local aislado.

4. **Vincular por `(issuer, subject)` con cuenta local preexistente.** Un proveedor confirma autenticación; Kusi Kusi mantiene la autoridad de tenant, estado de cuenta y RBAC. Sin JIT provisioning ni coincidencia de correo.
   - Alternativas descartadas: usar `email` como clave, aceptar un tenant claim, crear usuarios automáticamente.

5. **Usar `openidconnect` 4.1.0 para el cliente de protocolo.** La biblioteca descubre metadatos, obtiene JWKS, intercambia el código y valida firma, issuer, audience, tiempo y nonce. La implementación restringe el algoritmo aceptado a RS256 y el cliente HTTP desactiva redirects.
   - Referencia del crate: [openidconnect en docs.rs](https://docs.rs/openidconnect/latest/openidconnect/).
   - Pendiente de aceptación: emisor sintético loopback con firmas válidas e inválidas; la compilación sola no demuestra la interoperabilidad.

## Decisión pendiente para operación real

El primer emisor corporativo, sus metadatos, client ID/secreto, callback HTTPS, política MFA, contrato de aprovisionamiento y soporte de rotación JWKS deben validarse antes de cualquier conexión real. No se crean apps ni secretos en este bloque. El diseño local permite completar una prueba de interoperabilidad sin una cuenta externa.
