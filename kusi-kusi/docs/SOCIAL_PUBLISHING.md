# Publicaciones sociales: diseño propuesto

Estado: backend OAuth social implementado (ver OAUTH.md); la publicación real sigue en diseño. WhatsApp y las APIs de publicación son productos distintos: configurar WhatsApp no concede permisos para administrar Instagram o Facebook. El registro actual solo acepta operaciones read; no habilitar publish como lectura para eludir ese control.

## Encaje en Kusi Kusi

Un módulo Publishing utiliza puertos de cuentas vinculadas, borradores, activos multimedia, autorizaciones, confirmaciones y cola persistente. Adaptadores oficiales por plataforma implementan creación/subida, consulta de estado y reconciliación. El LLM puede sugerir texto y variantes; no obtiene tokens OAuth ni publica por decisión propia.

Capacidades futuras: `social.drafts.create`, `social.drafts.read`, `instagram.publish`, `facebook.publish`, `tiktok.publish`, `social.publications.status`. Todo recurso pertenece a una organización; una cuenta social no se elige libremente mediante parámetros de un prompt.

## Flujo

1. Administrador vincula cada cuenta por OAuth, con state/PKCE cuando el flujo oficial lo admita, permisos mínimos y tokens cifrados/rotables fuera de logs.
2. Usuario autorizado prepara un borrador con variantes por plataforma y medios propios o licenciados.
3. Kusi Kusi valida formatos, destino, permisos vigentes y requisitos de privacidad/comercialización de la red.
4. Se muestra vista previa con cuenta concreta, texto, archivos, privacidad y fecha/hora.
5. La aprobación queda vinculada al usuario y al hash de esa revisión del borrador. Cambiar texto, medios, cuenta o privacidad exige nueva aprobación.
6. El scheduler persiste un trabajo independiente por destino. Se registra el id remoto y se consulta el estado hasta confirmar publicación o error.
7. Ante timeout ambiguo, reconciliar antes de repetir: una clave de idempotencia interna no garantiza que la red externa no haya aceptado ya el primer intento.

Estados: draft, awaiting_approval, approved, scheduled, uploading, processing, published, failed, unknown. Reportar cada red por separado; una aceptación de carga no prueba publicación.

## Adaptadores y requisitos externos

- Instagram: usar API oficial de publicación para las cuentas y formatos habilitados por Meta. Elegir explícitamente el flujo de Instagram Login o Facebook Login, y verificar permisos, elegibilidad y revisión aplicables. No asumir que todos los tipos de cuenta o contenido tienen el mismo soporte.
- Facebook: publicación mediante Pages API para páginas vinculadas y autorizadas. No prometer publicación en perfiles personales. Los permisos exactos y acceso para clientes externos requieren comprobar la app y la documentación vigente.
- TikTok: Content Posting API; consulta previa de creator info, selección de privacidad e interacciones, consentimiento y estados posteriores. La app necesita scope autorizado y auditoría para levantar las restricciones de visibilidad privada. Las directrices rechazan herramientas limitadas al uso interno del equipo; validar el encaje del producto multiempresa y la experiencia de aprobación antes de solicitar acceso.

## Orden de implementación

Primero OAuth/secretos y ciclo de confirmación genérico; después cola y auditoría; luego un adaptador por vez con cuentas de prueba, y por último programación y publicación a múltiples destinos. No habilitar acciones irreversibles hasta disponer de vista previa, aprobación e idempotencia/reconciliación.

## Fuentes y límites de esta revisión

- Meta Instagram: https://developers.facebook.com/docs/instagram-platform/instagram-api-with-instagram-login/content-publishing
- Meta Pages: https://developers.facebook.com/docs/pages-api/posts/
- TikTok Direct Post: https://developers.tiktok.com/docs/en/content-posting-api-get-started
- TikTok UX/revisión: https://developers.tiktok.com/doc/content-sharing-guidelines/
- TikTok estados: https://developers.tiktok.com/docs/en/content-posting-api-reference-get-video-status

Las páginas de Meta devolvieron bloqueo/429 durante esta revisión; los scopes y detalles de su flujo quedan pendientes de verificación oficial al configurar las apps. Las restricciones de TikTok se contrastaron con su documentación oficial. No se crearon apps sociales, no se solicitaron scopes y no se publicaron contenidos.

## Implementado ahora: simulación local

Panel de publicaciones/calendario, archivo privado, caption, privacidad, programación, snapshot versionado, idempotencia, expiración/digest de aprobación y comprobante `simulated:<uuid>`. Editar invalida aprobación; un trabajador verifica el aprobador antes de completar. No hay publicación nativa ni llamadas a redes sociales desde este flujo. El OAuth existente conserva su alcance separado. Ver LOCAL_WORKSPACE.md y ROADMAP.md para los requisitos del piloto real.
