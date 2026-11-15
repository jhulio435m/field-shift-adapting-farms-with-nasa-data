# Adaptive assistant v1

Ejecutable aislado: configuración privada indicada por operador; stdin JSON
`{"message":"texto natural"}`. La configuración, no stdin, fija actor, conexión,
credenciales, proveedor y límites. stdout entrega JSON con estado, texto, evidencia
normalizada. Los identificadores de suscripción se comunican en el texto. No imprime secretos ni filas.

Estados: completed, clarification, blocked, unavailable, deferred, unknown, notification, ignored. Errores no se presentan como
resultados válidos. Las propuestas del modelo (`query`, `clarify`, `subscribe`, `cancel`, `reset`, `unsupported`) no tienen campos de organización,
credenciales, URL, SQL ni comandos. Catálogo/versiones y argumentos se validan en aplicación.

Transporte de eventos: origen registrado, firma HMAC sobre cuerpo original y
payload genérico resource/event_id/kind/values. Rechazar origen no registrado,
esquema inválido, suscripción cancelada, destinatario desvinculado o permisos revocados.
Webhook y consulta periódica son contratos distintos. Local no activa conexiones externas.

Contrato operativo: [contracts/adaptive/README.md](../../../contracts/adaptive/README.md).
