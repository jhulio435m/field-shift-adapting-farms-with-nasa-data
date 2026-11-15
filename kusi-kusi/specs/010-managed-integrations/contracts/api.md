# Contrato local implementado y límites

Contrato autoritativo: `contracts/openapi/openapi.yaml`. Prefijo `/v1/local`; proxy Next fijo existente. Respuestas `Cache-Control: no-store`; cookie HttpOnly, Origin, CSRF en mutaciones y RBAC servidor. Ningún body admite organization_id/tenant_id, credenciales en query ni URL de probe arbitraria. Errores solo `{code, correlation_id}` sin cuerpo remoto.

| Método/ruta | Permiso | Entrada | Salida/efecto |
| --- | --- | --- | --- |
| GET /connector-manifests | propietario | ninguna | fichas saneadas y versiones |
| POST /connection-settings/{kind}/verify | propietario | expected_revision, mode fixture/provider | verificación saneada ligada a revisión; provider bloqueado local |
| POST /connection-settings/{kind}/enable | propietario | expected_revision, verification_id | habilitación exacta; bloqueada local |
| POST /connection-settings/{kind}/disable | propietario | expected_revision | deshabilita nuevas ejecuciones, idempotente |
| GET /connection-settings/{kind}/status | propietario | ninguna | revisión, vigencia, modo, estado; sin identidad privada completa |
| GET /channel-operations/telegram | propietario | cursor | salud y bindings saneados paginados |
| POST /channel-operations/telegram/bindings/{id}/revoke | propietario | expected_version | revocación idempotente del contexto registrado |
| GET /managed-executions | actor autorizado | cursor | solo ejecuciones accesibles; página máxima 50 |
| GET /managed-executions/{id} | actor autorizado | ninguna | timeline y evidencia reautorizada |
| POST /managed-executions/{id}/cancel | actor autorizado | expected_version | cancel_requested/cancelled/already_completed/uncertain |
| POST /managed-executions/{id}/resume | actor autorizado | expected_version, proposal_digest | reautorización y recuperación sin reenvío ambiguo |
| POST /managed-executions/{id}/review | aprobador según operación | expected_version, proposal_digest, decision | usa política de aprobación existente; no concede nuevas capacidades |
| POST /integration-evaluations | propietario | suite_version | evaluación fixture sin proveedores pagos |
| GET /integration-evaluations/{id} | propietario | ninguna | métricas y evidencia sintética |
| GET /authorized-memories | actor | cursor | metadatos de memoria propia vigente |
| DELETE /authorized-memories/{id} | actor | ninguna | borrado inmediato de recuperación |
| POST /policy-simulations | propietario | operation, resource_reference | decisión explicada sin mutación de permisos |

Las rutas existentes PUT/GET/DELETE de connection-settings conservan compatibilidad, incorporando estado adicional sin readback de secretos. Todo nuevo campo de escritura rechaza desconocidos. expected_revision/expected_version: entero positivo; falta/incompatibilidad devuelve 409 revision_conflict sin exponer revisión ajena. 401 sin sesión, 403 por rol/CSRF/Origin o local_external_blocked; 404 uniforme para recurso inexistente/inaccesible; 422 entrada inválida; 429 límite; 503 provider_unavailable saneado. Probes inválidos devuelven resultado failed con categoría invalid_credentials, insufficient_scope, timeout o unavailable, nunca excepciones crudas.

No se añaden operaciones empresariales de escritura. Las revisiones de efectos de canal y creativos conservan sus permisos existentes; aprobar no es una vía para saltar el catálogo de capacidades. El transporte real, sus allowlists y activación pertenecen a un entorno separado, no a este prefijo local.


Adiciones de implementación: GET/PUT `/authorized-memories/preferences` para consentimiento y TTL propio (1–604800 segundos), y POST `/managed-queries` para lectura sintética con principal de sesión, manifiesto y ACL 007. La memoria inicial está integrada con los dos recursos de demostración existentes, no con datos remotos. Las evaluaciones son deterministas con preguntas y evidencia ficticia versionadas; `quality_evaluated=false`. Las rutas de revisión manejan únicamente propuestas explícitas `awaiting_review` de outbox; propuestas creativas/publicación conservan sus contratos separados. La lista de vínculos incluye contextos registrados de la organización sin remitentes, aunque el primer endpoint conserve el nombre Telegram. Enable/provider siempre bloqueados en local; no hay implementación de su administración OIDC real en 010.


GET `/managed-executions/{id}/proposal` devuelve el snapshot privado de contenido/destino para revisión necesaria, solo al actor propietario con rol aprobador/owner, en estado awaiting_review y con vínculo vigente si es canal. No devuelve propuestas ligadas a proveedores mediante este contrato local. El digest y revisión deben coincidir con el detalle antes de habilitar aprobación. Las listas/timeline siguen sin cuerpos.
