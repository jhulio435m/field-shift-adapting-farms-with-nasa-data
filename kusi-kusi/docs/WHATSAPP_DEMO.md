# WhatsApp staging de la hackatón

`python3 scripts/run_whatsapp_staging.py` inicia exclusivamente `whatsapp_demo` en 127.0.0.1:18195. Es un proceso staging explícito, independiente del router local, sin panel, cookies ni API administrativa. Conserva bloqueos de red del workspace local. Lee únicamente variables META del .env privado; no conecta la base local ni publica datos reales.

Variables: META_APP_SECRET, META_ACCESS_TOKEN, META_PHONE_NUMBER_ID, META_VERIFY_TOKEN, META_TEST_RECIPIENT y META_GRAPH_VERSION. El lanzador expresa el consentimiento de staging para respuestas reales al único número de prueba configurado. Solo se responde a mensajes entrantes con firma Meta válida y phone-number ID coincidente. No se envían mensajes de inicio ni plantillas. El número autorizado se asigna exclusivamente al actor sintético de la plataforma ficticia; no autentica estudiantes reales.

Después de confirmar el servicio, ejecutar `cloudflared tunnel --url http://127.0.0.1:18195 --no-autoupdate --protocol http2`. Registrar URL pública + `/webhook` en Meta y el valor META_VERIFY_TOKEN; suscribir el campo `messages`. La URL temporal caduca al detener el túnel y cambia al reiniciarlo.

Pruebas desde el destinatario autorizado: `mis pagos`, `70000001`, `UNCP-EXP-004821` y foto de `web/admin/public/demo/recibo-sintetico.png`. El adaptador de pagos y el OCR son los mismos de la plataforma local. No existe integración ADESA ni consulta de pagos reales. Una coincidencia de OCR indica correspondencia con datos ficticios, no autenticidad bancaria.

Las imágenes se descargan de Meta con token, HTTPS, host permitido, sin redirects, límite 5 MiB y timeout; OCR se ejecuta localmente sin guardar imagen ni texto. Se persiste solamente hash del identificador del mensaje y estado de envío en `.local-data/whatsapp-staging`, con permisos privados. No hay mensajes, fotos, números ni credenciales en el ledger. Dedupe por creación exclusiva antes del envío. En fallos ambiguos o caída del proceso no se reintenta automáticamente: revisar `uncertain_or_failed` y claims vacíos; el usuario puede volver a escribir un mensaje nuevo. Esto evita duplicados pero no garantiza entrega. El servicio procesa secuencialmente; es una demo de un destinatario, no un worker de producción.

Detener tanto lanzador/servidor como cloudflared para terminar la demo. No publicar `.env` ni `.local-data`. Conectividad pública, suscripción Meta y recepción real deben comprobarse por separado de tests locales.

## Notas de voz y formato

GROQ_API_KEY es opcional y se lee únicamente en el lanzador staging. Cuando se configura, las notas de voz del remitente autorizado se descargan con el mismo transporte Meta acotado a 5 MiB y se envían a Groq (endpoint fijo de transcripción, modelo whisper-large-v3-turbo, idioma es, timeout 20 s). El audio y la transcripción no se persisten ni se imprimen. La transcripción alimenta exclusivamente los casos de consulta de pagos; no ejecuta SQL ni capacidades nuevas. Sin clave se responde indicando que la voz no está configurada. Requiere reiniciar el proceso al cambiar la clave. Esto puede consumir cuota del proveedor. El workspace local sigue sin llamadas externas.

Las respuestas usan negritas de WhatsApp, separación por campos y estados en español. Fotos: enviar el recibo sintético de web/admin/public/demo/recibo-sintetico.png como imagen. La foto real del usuario puede producir sin coincidencia porque los datos consultados son ficticios. No se evalúa autenticidad documental.

Referencia de demostración autorizada: `V364-00011432` se asocia al registro sintético `UNCP-EVAL-006173`, S/ 3,00, pago confirmado. Es una coincidencia programada para la hackatón, sin comprobación institucional. No se incorpora la foto ni identidad del titular al repositorio.

Presentación para pitch: respuestas sin emojis y con una única etiqueta discreta «Datos de prueba». Se mantiene la distinción entre pago confirmado en el fixture y verificación institucional.

Uso del comprobante: el DTO incluye voucher_use (available, used, review_required), estado explícito independiente del pago. El fixture de S/ 3,00 está usado en su trámite de evaluación y se presenta como no válido para iniciar otro trámite. El de S/ 198,00 requiere revisión administrativa por interrupción. No hay endpoint de consumo/reserva de vouchers ni bloqueo transaccional de trámites externos; esta entrega muestra el estado configurado de prueba. No acredita reglas institucionales reales.

Pitch: se retira la etiqueta «Datos de prueba» del texto de WhatsApp a solicitud del presentador. El origen sigue siendo el fixture sintético documentado; no implica una integración con Tesorería.

Códigos dictados: extracción determinista de una única referencia en frases, espacios y dígitos individuales en español; conserva ceros y no aproxima códigos. Ambigüedad solicita aclaración. Web con arrastre, preview y límites de 5 MiB.

Privacidad visual: la web no presenta códigos de pago/referencias en historial ni resultado, no rellena códigos al seleccionar registro/foto y retira ejemplos descargables. El input acepta exclusivamente el valor introducido por el usuario. Esto no implementa autorización de datos reales: el API demo sigue sirviendo fixtures sintéticos en loopback.

Consulta web compacta: un método visible (código/foto), resultado e historial por DNI en diálogos Radix con gestión de foco y Escape. El historial se presenta después de consultar el DNI de prueba. No se muestran códigos ni referencias en resultados. Aceptación de escritorio/móvil comprueba que la página cabe en viewport y que Escape cierra el diálogo.

Prueba de DNI en web y WhatsApp: 70000001 con pagos; 70000002 estudiante sintético sin pagos; 70000003 documento desconocido. Historial vacío no se presenta como error ni como documento inexistente.

Publicación temporal autorizada: scripts/serve_public_payment_demo.py expone solo /tramites, assets Next y operaciones /v1/demo/payments en 127.0.0.1:18196. El túnel apunta a este gateway; no expone administración, login ni sesiones. El gateway exige Origin del host público para POST y reenvía al upstream loopback fijo, sin cookies ni claves administrativas. Los fixtures son públicos para la presentación, sin datos de estudiantes reales. META_TEST_RECIPIENT admite lista de números separados por coma; cada respuesta se dirige al remitente autorizado del evento firmado, nunca al conjunto de destinatarios. La verificación de destinatarios del sandbox Meta se completa por separado en su panel.
