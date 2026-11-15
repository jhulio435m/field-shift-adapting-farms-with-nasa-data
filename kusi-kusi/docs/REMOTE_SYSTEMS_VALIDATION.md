# Pruebas de servicios remotos: 8 de noviembre de 2026

La aceptación actual ya pasa por el runtime Axum y PostgreSQL de Kusi Kusi:
[SYSTEM_CONNECTORS_ACCEPTANCE.md](SYSTEM_CONNECTORS_ACCEPTANCE.md). Las fases
directas y en memoria descritas abajo se conservan como historial.

La segunda fase ya comprobó lecturas reales a través de `ExecuteCapability`:
[REMOTE_CAPABILITY_ACCEPTANCE.md](REMOTE_CAPABILITY_ACCEPTANCE.md). El resto
de este documento describe la primera fase de pruebas directas y creación de
cuentas.

Servidor autorizado por el usuario: `132.226.244.23`, acceso SSH como `opc`.
Se revisaron CRM Frappe, ERPNext, Plane y WPPConnect. Las comprobaciones se
ejecutaron mediante SSH y HTTP loopback en el servidor. No se activó tráfico
externo desde el workspace local de Kusi Kusi.

## Resultados observados

| Sistema | Comprobaciones realizadas | Resultado |
|---|---|---|
| CRM Frappe | Metadatos reales de User; cuenta de portal sin roles privilegiados; login HTTP; identidad propia; cookie HttpOnly; rechazo anónimo y de lectura de Administrator; logout y rechazo posterior | Aprobado |
| ERPNext | Mismas comprobaciones, en su instalación y base independientes | Aprobado |
| Plane | Cuenta activa sin staff, superusuario ni membresías; contraseña verificada; login HTTP; identidad propia; lista de workspaces vacía; cookie HttpOnly; CSRF obligatorio; logout y rechazo posterior | Aprobado |
| WPPConnect | `/healthz` HTTP 200 y mensaje OK; consulta de sesión ficticia sin token rechazada | Aprobado |

Plane devuelve HTTP 200 con una página HTML «CSRF Verification Failed» cuando
falta el token. La prueba comprobó también que la identidad continuaba
rechazada, y que el login con CSRF válido sí emitía sesión. No se interpretó
el HTTP 200 de la página de error como autenticación exitosa.

## Cuentas de prueba

Identificador sintético en cada aplicación: `kusi-qa-20261108@example.invalid`.
Hay tres cuentas independientes, con contraseñas distintas: CRM, ERPNext y
Plane. Las dos Frappe son Website User, sin roles comerciales ni
administrativos. Plane no tiene membresías en workspaces existentes.

Las cuentas permanecen creadas para futuras pruebas; no se configuró caducidad
automática. Las credenciales se conservaron únicamente en el archivo privado
local `.local-data/qa/remote-test-users-20261108.json`, con permisos 0600. No
se imprimieron contraseñas ni se copiaron secretos del servidor. En Frappe se
desactivó la bienvenida y se bloqueó `frappe.sendmail` durante la creación;
Plane se creó directamente mediante su ORM sin enviar invitaciones.

No se crearon sesiones de WhatsApp, no se vincularon teléfonos y no se
enviaron mensajes. WPPConnect utiliza sesiones y tokens, no una cuenta de
usuario equivalente a las otras aplicaciones. Vaultwarden y correo quedaron
fuera del alcance anunciado de esta prueba. No se cambiaron servicios,
imágenes, esquemas ni registros comerciales existentes.

## Límite de la primera fase

Estas son pruebas directas de las aplicaciones destino, no ejecuciones de
capacidades a través de Kusi Kusi. No prueban compatibilidad universal ni
lectura autorizada de registros comerciales. El conector HTTP genérico actual
usa POST JSON, destinos HTTPS públicos autorizados y no incorpora por sí solo
los flujos de autenticación y consulta de Frappe, Plane o WPPConnect.

Para afirmar una integración completa faltan adaptadores específicos,
credenciales de servicio con permisos mínimos, contratos de entrada/salida,
autorización por tenant y recurso, y pruebas de extremo a extremo a través de
los casos de uso de Kusi Kusi. Las protecciones de destinos privados y el
bloqueo de llamadas externas en modo local se mantuvieron intactos.

Evidencia detallada sin contraseñas: `.local-data/qa/remote-systems-20261108.json`
y `.local-data/qa/plane-http-result.json`. Esta ejecución no vuelve a validar
la suite Rust, el build de Next.js ni el runtime de Kusi Kusi en ARM64.
