# Seguridad y límites de la entrega

**Entorno de desarrollo; sin aprobación para producción.** No afirmar cumplimiento legal por tener controles técnicos. Roles, bases legales, datos de menores, derechos de titulares y transferencias requieren evaluación jurídica específica en Perú.

## Controles implementados

- Tenants derivados de claves/sesiones verificadas o canales registrados; consultas SQL parametrizadas y recursos filtrados por organización. Pruebas de acceso cruzado. RLS aún pendiente.
- Claves de servicio aleatorias y almacenadas como SHA-256; bootstrap administrativo separado. No guardar claves ni tokens en logs o código.
- Panel local: Argon2id con sal aleatoria, cookie opaca HttpOnly/SameSite Strict, ocho horas, CSRF y validación de origen; roles y relación usuario/conversación comprobados en servidor. Deshabilitar cuenta/restablecer contraseña revoca sesiones. Protección del último propietario y administrador de plataforma.
- Sesiones locales: inventario por cuenta y tenant, ID de gestión aleatorio separado del token, revocación transaccional de sesión/credencial/auditoría, bloqueo de la sesión actual en el inventario. No se registran IP, User-Agent ni datos de dispositivo.
- Límite de intentos de login y de peticiones por usuario local. Estos límites son específicos del modo local; no sustituyen protección distribuida del despliegue ni límites globales de las APIs de servicio.
- Local mode solo con bind loopback/origen exacto. Bloquea llamadas de IA, ejecución de conectores externos, acciones OAuth y envíos de canal. Los trabajos reales de proveedores creativos se rechazan incluso si quedaron pendientes previamente. No exponerlo mediante un túnel ni proxy público.
- Webhooks: firma Meta sobre cuerpo original y secreto Telegram por canal; recepción durable y deduplicada. La identidad del chat no concede acceso. Códigos de vinculación de un uso, organización/canal exactos y vencimiento de diez minutos.
- Capacidades autorizadas con esquemas limitados; validación antes/después de ejecutar; HTTPS con allowlist, DNS público fijado, sin redirects/proxy, tamaños y tiempos limitados. No se ejecuta SQL de modelos ni se aceptan herramientas sin permiso.
- Archivos privados por UUID; límites de tamaño/decodificación/dimensiones; reencodificación de imágenes sin metadatos. FFprobe/FFmpeg con tiempo acotado y protocolos restringidos. No es un antivirus ni aislamiento completo de decodificadores; procesar archivos no confiables en sandbox separado será necesario para producción.
- Aprobaciones por snapshot/digest, organización, operación y versión; editar invalida la confirmación de publicación. La ejecución simulada revisa que el aprobador siga habilitado. Estimaciones creativas reservadas en presupuesto UTC no son garantía del precio facturado externamente.
- OAuth social: estado de un uso y cookie segura; cifrado XChaCha20-Poly1305 con AAD de organización/cuenta, keyring con ids y refresh con leases. No repetir intercambios inciertos de credenciales rotatorias. Instagram puede requerir revocación remota manual; ver OAUTH.md.
- Plano administrativo OIDC separado del workspace local: identidad por emisor/sujeto provisionada, membresías locales, Authorization Code con PKCE S256, estado/nonce de un uso, intentos temporales cifrados, cookies HttpOnly y cierre con Origin/CSRF. La aceptación HTTP con emisor sintético loopback pasa; la revisión visual desktop/móvil pasó en 008 y la configuración real sigue desactivada.
- Auditoría de accesos/transiciones sin cuerpos ni secretos en logs; retención de mensajes bajo confirmación del propietario. Dumps y archivos son privados; respaldo probado mediante restauración y cotejo de hashes.

## Riesgos y trabajo de producción

OIDC del panel, recuperación de cuenta y MFA; rotación/revocación administrativa de claves de servicio; políticas por recurso de sistemas externos; RLS; gestor de secretos; TLS y proxy confiable; revisión de dependencias y decodificadores; antivirus/sandbox de archivos; límites de disco/uso y presupuestos de IA; retención integral de archivos/credenciales; derechos y eliminación de datos; alertas/observabilidad; pruebas de carga, pentest y recuperación operacional.

El dump puede contener hashes y credenciales cifradas: debe cifrarse/protegerse y conservarse con políticas apropiadas. El keyring y `.env` se respaldan separadamente. La verificación de un respaldo local no equivale a recuperación completa de un despliegue.

Los adaptadores oficiales están implementados con pruebas de contrato; no se han validado cuentas, consentimientos, calidad de resultados, cuotas ni publicación real de proveedores en esta entrega.

## Adaptadores de sistemas en el runtime

Los perfiles de Frappe, Plane y salud WPPConnect se resuelven solo en la
configuración privada del servidor y por tenant autenticado. Registro, permisos
y auditoría usan PostgreSQL antes/después de la ejecución; el adapter no recibe
URLs ni credenciales desde el request. Solo admite lecturas fijas de identidad
propia o salud y retorna estados normalizados sin nombres, correo, cookies ni
secretos. No concede acceso a registros comerciales ni envía mensajes.

Las credenciales residen en archivos regulares privados, limitados a 8 KiB;
rotación mediante reemplazo y reinicio. No se aceptan archivos symlink ni
legibles por grupo/otros. Los perfiles privados HTTP requieren opt-in y bases
IPv4 privadas/loopback literales con Host allowlisted; metadata/link-local,
destinos DNS HTTP, redirects y proxy heredado se rechazan. Los perfiles públicos
siguen HTTPS 443 con DNS fijado a IPs públicas. Esto no relaja el conector HTTPS
genérico. En modo local, perfiles configurados impiden el arranque y la ruta de
ejecución externa permanece bloqueada.

Logout upstream es de mejor esfuerzo ante fallos de transporte o timeout;
aplicar expiración y mínimo privilegio también en el sistema destino. No hay
reintentos automáticos. Los permisos de la cuenta de servicio del destino y
las autorizaciones por usuario/recurso de futuras operaciones deben
aprovisionarse explícitamente; no se deducen del login exitoso.

## Proxy del panel Next

Next conserva cookies, CSRF y Origin hacia el backend fijo configurado en servidor; no agrega credenciales administrativas. Las respuestas privadas usan no-store. El bind de Rust y el origen permitido de Next pueden tener puertos distintos pero ambos deben ser loopback. La autorización efectiva continúa en Rust. Las imágenes de contenedor están separadas y la web usa un usuario sin privilegios.

## Trámites ficticios de la UNCP

La pantalla demo sin login sirve solo registros sintéticos, se activa únicamente cuando el servidor local loopback está encendido y exige mismo Origin en consultas POST. No acepta organización ni identidad desde el navegador. OCR procesa imágenes limitadas a 5 MiB y 20 megapíxeles en memoria; solo retorna la coincidencia y su referencia enmascarada, sin guardar bytes ni el OCR íntegro. El canal WhatsApp requiere una configuración de canal y firma webhook; el entorno local no descarga medios de Meta ni envía respuestas reales. No hay conexión a ADESA ni a la futura plataforma de pagos.

## Vínculos y ACL externos (007)

[Contrato y operación](EXTERNAL_IDENTITIES_ACL.md): políticas por adaptador, comprobación manual explícita, vínculo por credencial individual y concesiones por capacidad/versión/recurso. Caducidad y revocación se consultan en PostgreSQL antes de cada transporte; no se reactivan por upsert. Auditoría sin sujetos ni payload. Sin política, una capacidad conserva su contrato de servicio: no acredita autorización personal. Verificación remota automática y conexión a canales pendientes. El modo local sigue bloqueando los conectores externos.

## Asistente con ACL y redirecciones OIDC (008)

El Environment protegido aplica las ACL de 007 al contexto privado del canal. No admite política ausente como permiso. Recomprueba cada lectura/aviso y elimina contexto de recursos revocados antes de invocar el intérprete. Una concesión de conjunto no sustituye ACL de filas ni permisos del origen. El modo legado queda distinguido; no se activó ACL ni un proveedor en staging.

El proxy Next reenvía `Location` del backend fijo además de cookies y cabeceras privadas: es necesario para inicio/callback OIDC. Mantiene redirects manuales hacia upstream, sin seguirlos desde el servidor, y conserva Origin/CSRF/no-store. La prueba de navegador usa emisor sintético, PostgreSQL nuevo y solo tráfico loopback. Las conexiones SQLite del transporte y fixtures usan cierre explícito después del contexto transaccional, con commit/rollback conservados.

## Configuración web privada (009)

Perfiles locales por organización para Plane/Frappe/Telegram, administrados únicamente por propietario autenticado. Secretos de escritura cifrados con XChaCha20-Poly1305/AAD por organización/perfil/tipo/revisión. La clave 0600 persiste en el almacenamiento privado y su ausencia con ciphertext existente bloquea arranque; no se regenera para ocultar pérdida de datos. No hay API de descifrado, vista de valores ni registro de cuerpos. Los errores de JSON de estas rutas se normalizan para no reflejar valores enviados.

Los cambios usan revisión optimista, lock por perfil, comprobación del propietario vigente en BD y auditoría en la misma transacción. Borrar credenciales no revoca tokens remotos. Modo local solo guarda configuración no verificada y no modifica staging/canales activos. No equivale a un gestor de secretos productivo ni conecta automáticamente los perfiles al runtime. Recuperación y límites en WEB_CONNECTION_SETTINGS.md.
