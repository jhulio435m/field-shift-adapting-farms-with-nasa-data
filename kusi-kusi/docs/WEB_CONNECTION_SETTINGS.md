# Configurar Plane, ERP CONEIMERA y Telegram desde la web

Entra con una cuenta **propietaria** y abre **Conexiones → Sistemas y canal del piloto**. No hace falta editar .env para guardar las credenciales de estos tres perfiles. La cuenta, base de datos y secretos de arranque del servidor siguen siendo configuración de infraestructura.

## Campos del piloto

| Perfil | Datos públicos | Credenciales de escritura |
| --- | --- | --- |
| Plane | Nombre, URL HTTPS de la instancia, workspace opcional | API Key |
| ERP CONEIMERA | Nombre, URL HTTPS del ERP | API Key y API Secret de Frappe |
| Telegram | Nombre, usuario del bot y URL pública del webhook opcionales | Token del bot; secreto de webhook opcional |

La URL de una instancia es el origen, sin /api, usuario/contraseña, query o fragmento. Los destinos con IP literal, puertos alternativos o HTTP se rechazan. Esta validación es sintáctica: no consulta DNS ni confirma que la credencial funcione. Telegram usa el destino oficial en sus adaptadores; el formulario no permite elegir una URL alternativa para enviar el token.

Los campos corresponden a los mecanismos oficiales de [Plane API Key](https://developers.plane.so/api-reference/v2/authentication), [Frappe API Key/API Secret](https://docs.frappe.io/framework/user/en/api/rest) y [Telegram Bot API y webhook secret](https://core.telegram.org/bots/api). La autenticación real depende además de versión, permisos y configuración del servicio destino; no se prueba al guardar.

## Guardar, editar y borrar

- **Configurar / Editar** abre un formulario con los datos públicos. Los secretos anteriores nunca se rellenan ni pueden recuperarse desde la API.
- **Guardar configuración** crea o actualiza el perfil de esta organización. Los campos secretos vacíos conservan su valor anterior. Introducir un valor nuevo lo reemplaza.
- Telegram genera un secreto de webhook en el servidor al guardar el primer token si no se suministró uno. Dejarlo vacío al editar conserva el actual.
- **Borrar credenciales** exige confirmación, elimina el ciphertext y conserva los datos públicos. No revoca el token en el proveedor ni cambia un canal heredado que esté activo.
- Si otra pestaña cambió la configuración, el guardado antiguo se rechaza con conflicto. Cierra, actualiza y vuelve a editar; los secretos introducidos se borran del formulario también cuando falla el guardado.

El servidor deriva el tenant y el usuario de la sesión HttpOnly. Editor, aprobador y lector no pueden listar, modificar ni borrar estos perfiles aunque llamen a la API directamente. Las mutaciones mantienen CSRF, comprobación de Origin y respuestas privadas no-store. La auditoría se confirma junto con cada cambio y solo registra operación/adaptador, actor y organización, sin URLs, etiquetas, credenciales ni cuerpos.

## Estado real de esta entrega

**Guardada** significa que los datos persistieron y las credenciales requeridas están presentes. **Sin verificar** significa que no se han autenticado contra el proveedor. Guardar no activa conexiones, no registra webhooks y no envía mensajes.

Estos perfiles locales no alimentan automáticamente `KUSI_SYSTEM_PROFILES`, `KUSI_CHANNELS` ni los archivos de staging del asistente. No hay un botón de activación real en esta entrega. La futura activación debe consumir el secreto exclusivamente en el servidor, validar destinos/host allowlist y permisos, y mantener las ACL y aprobaciones aplicables. El plano OIDC de producción y las sesiones del workspace local siguen separados: estos endpoints solo existen en el workspace local.

La configuración se probó con tokens sintéticos y servicios/base desechables. No se cambiaron credenciales ni canales existentes de Plane, ERP CONEIMERA o Telegram.

## Cifrado y recuperación

La migración 0019 crea `managed_connection_settings`. Cada organización tiene un perfil por adaptador. La revisión aumenta en cada cambio; un expected_revision obsoleto nunca sobrescribe datos recientes.

Se reutiliza XChaCha20-Poly1305 con nonce aleatorio y AAD que incluye organización, id de perfil, tipo y revisión. La clave maestra se genera al iniciar el almacenamiento local y se guarda en `KUSI_LOCAL_STORAGE/connection-vault.key` (por defecto `.local-data/connection-vault.key`), con permisos 0600 dentro de una carpeta 0700. No se imprime, no se envía al navegador y no se incluye en OpenAPI ni en una respuesta de configuración.

La clave persiste durante los reinicios. Si existen secretos cifrados y falta la clave, el servidor local no genera otra silenciosamente: falla cerrado. También rechaza archivos de clave con permisos públicos, symlinks, formato inválido o ciphertext alterado. Restaurar la clave original y la base correspondiente conserva las credenciales.

El dump PostgreSQL contiene ciphertext pero no la clave del archivo. El respaldo existente copia los activos referenciados, no esta clave. Conserva una copia privada y protegida de la clave por separado; restaurar solo el dump no recupera la capacidad de descifrar. La rotación de credenciales API sí está implementada desde la web. La rotación de la clave maestra y su recuperación operativa requieren un procedimiento específico; no se declara un gestor de secretos productivo.

## Reproducir la aceptación sin servicios reales

```sh
cargo build -p kusi-server --bins
npm run build --prefix web/admin
python3 scripts/verify_connection_settings.py --result /tmp/connection-settings-api.json
python3 scripts/verify_connection_settings_browser.py --artifacts /tmp/connection-settings-browser
```

Los runners no leen .env: crean PostgreSQL y servidores temporales, usan credenciales sintéticas, limitan el navegador a loopback y eliminan los procesos/datos de prueba al terminar. La prueba API incluye reinicio, pérdida/permisos de clave, tamper, revisión, cifrado y roles; el navegador cubre los tres formularios, rotación, conflicto entre pestañas y borrado confirmado en desktop/móvil.


## Operación local 010

**Simular verificación** comprueba la presencia de campos requeridos en la revisión guardada. No autentica el token ni prueba identidad o permisos remotos. El resultado muestra modo fixture, fecha y vencimiento de 15 minutos; rotar, editar o borrar credenciales lo invalida atómicamente. La verificación de proveedor y la habilitación real se rechazan en local antes de abrir transporte o descifrar para red. La API de estado es `/v1/local/connection-settings/{kind}/status`; el campo heredado `verification` del resumen 009 sigue indicando que el proveedor real no está verificado.

Los manifiestos versionados definen autenticación, operaciones, esquemas y límites. `/v1/local/managed-queries` ejecuta únicamente lecturas sintéticas de `fixture-a` mediante perfil vigente, principal de sesión, capacidad registrada, permiso, vínculo externo y concesión exacta de 007. Una autenticación o una verificación simulada no conceden acceso. Hay una composición opcional `managed_fixture` del asistente que utiliza el mismo perfil: exige ACL, fuentes manifest y un planificador literal en loopback, sin clave de proveedor. No cambia configuraciones de staging existentes.

En **Operación del piloto** se administran:

- Vínculos registrados sin mostrar chat/remitente: vencimiento, revisión y revocación. Revocar elimina contexto retenido; el trabajador vuelve a comprobar el vínculo antes de encolar una respuesta.
- Cola de respuestas propia: historial durable, cancelación y propuestas explícitamente pendientes de revisión. La vista privada de propuesta muestra contenido y destino exactos al actor autorizado; no se devuelven en listas/historial. La aprobación permanece deshabilitada hasta recuperar ese snapshot. Aprobar se vincula al digest de operación, conversación/destino, contenido y perfil/revisión/versión si existe. Editar invalida aprobación. Recuperar conserva la fila original; un efecto incierto no se reenvía. Cancelar una operación ya en curso solo registra una solicitud.
- Evaluación de 24 preguntas/casos ficticios, ocho por conector, con resultado esperado, obtenido y evidencia sintética. No prueba calidad de un modelo ni conectividad real.
- Memoria propia: consentimiento explícito, cifrado, máximo siete días, eliminación y reautorización de fuentes. La integración inicial de memoria utiliza los dos recursos de demostración de 002; no conserva automáticamente datos de Plane/Frappe. Revocar su concesión elimina el payload; caducidad y cambio de versión impiden recuperarlo. El borrado no regenera memoria automáticamente; la retención purga filas vencidas/eliminadas.
- Simulador de permisos de los recursos de demostración para usuarios de la organización, sin modificar concesiones. Cedar no se instaló; tampoco se añadió Rig ni otro motor externo.

La migración 0020 añade ciclo de conexión, 0021 extiende vínculos/outbox e historial, 0022 añade evaluación y memoria, y 0023 vincula revisiones de propuestas. No se crea una segunda cola. Las cuentas OIDC de producción siguen separadas y no se ha construido una activación web de proveedores reales en ese plano.

Aceptación y límites actuales: [010/acceptance.md](../specs/010-managed-integrations/acceptance.md).
