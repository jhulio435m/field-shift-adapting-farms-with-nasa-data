# Referencias de GitHub para Kusi Kusi

Investigación: 2026-11-10. Se revisaron repositorios, documentación y licencias primarias. No se instalaron ni ejecutaron estos proyectos; las funciones descritas proceden de sus fuentes, no de una prueba de integración.

## Contexto y prioridad

El piloto es Plane + ERP CONEIMERA/Frappe + Telegram, con configuración desde la web. Actualmente los perfiles cifrados se guardan, pero `connection_settings.rs` devuelve `runtime_active: false` y `verification: "not_verified"`. La primera entrega útil es resolver esas conexiones desde el runtime autorizado y verificar su alcance, conservando las restricciones del entorno local.

Kusi Kusi ya dispone de puertos, capacidades, ACL por recurso, auditoría, deduplicación, recuperación persistente y aprobaciones versionadas. Las propuestas siguientes amplían esos mecanismos.

## Proyectos y aportes

| Proyecto | Qué estudiar o incorporar | Aplicación concreta |
| --- | --- | --- |
| [Activepieces](https://github.com/activepieces/activepieces) | Contrato de conectores, autenticación declarativa, validación de conexiones y referencias estables | Manifiesto por adaptador con campos de autenticación, capacidades, versiones y validación. Gestión web del estado de cada conexión. |
| [Nango](https://github.com/NangoHQ/nango) | Ciclo de credenciales, conexiones por usuario/organización, renovación y límites de proveedor | Resolver credenciales exclusivamente en servidor; mostrar errores saneados, expiración y necesidad de reconexión. |
| [OpenClaw](https://github.com/openclaw/openclaw) | Gateway de mensajería, políticas por chat, pairing y transportes Telegram | Administrar vinculaciones existentes desde la web; estado, cancelación y progreso de tareas; evaluar polling opcional para pruebas locales. |
| [Hermes Agent](https://github.com/NousResearch/hermes-agent) | Continuidad conversacional y experiencia de asistentes en mensajería | Memoria acotada por actor/organización y retención; progreso de trabajos y continuidad entre sesiones autorizadas. |
| [LangGraph](https://github.com/langchain-ai/langgraph) | Checkpoints, interrupciones y reanudación con intervención humana | Línea temporal de ejecución y revisión de operaciones futuras, sobre el estado persistente y las aprobaciones existentes. |
| [Dify](https://github.com/langgenius/dify) | Playground, evaluación, recuperación documental y observabilidad de respuestas | Preguntas de prueba por conector; evidencia y fuentes visibles; evaluación de respuestas con ACL. |
| [Rig](https://github.com/0xPlaygrounds/rig) | Contratos Rust para modelos y herramientas, streaming y record/replay | Evaluar una biblioteca compatible detrás de los puertos de IA, con autorización y validación en aplicación. |
| [Cedar](https://github.com/cedar-policy/cedar) | Políticas RBAC/ABAC, validación de esquemas y pruebas de autorización | Evaluar cuando crezcan las reglas: simulador de políticas y explicación controlada de denegaciones. No sustituye identidad verificada ni filtrado de recursos. |
| [n8n](https://github.com/n8n-io/n8n) | Prueba de pasos y revisión humana de herramientas | Interfaz para inspeccionar una propuesta y aprobar la revisión exacta antes de operaciones con efectos. |

No se confirmó en esta revisión un conector reutilizable de Activepieces para Plane o Frappe. Su conector Telegram sí se inspeccionó; no debe interpretarse la tabla como disponibilidad de los tres conectores del piloto.

## Límites de reutilización

- **Activepieces:** el [LICENSE](https://github.com/activepieces/activepieces/blob/main/LICENSE) distingue comunidad MIT de directorios empresariales comerciales. Revisar el archivo y sus dependencias antes de copiar código.
- **Nango:** [Elastic License 2.0](https://github.com/NangoHQ/nango/blob/master/LICENSE), con restricciones sobre servicios alojados que expongan una parte sustancial de su funcionalidad. Evaluar el alcance antes de incorporarlo a una oferta multiempresa.
- **Dify:** [licencia con condiciones adicionales](https://github.com/langgenius/dify/blob/main/LICENSE), incluida autorización para entorno multiinquilino y condiciones de marca de frontend; no tratarla como Apache 2.0 sin modificaciones.
- **n8n:** [Sustainable Use License y condiciones empresariales](https://github.com/n8n-io/n8n/blob/master/LICENSE.md); no asumir permiso comercial irrestricto para embeberlo o distribuirlo.
- **OpenClaw, Hermes Agent, LangGraph y Rig:** licencias MIT en los repositorios revisados; **Cedar:** Apache 2.0. Las dependencias y archivos particulares requieren su propia revisión.
- **Rig:** el `main` revisado declara Rust **1.97.1**; Kusi usa **1.95.0**. Evaluar una versión compatible antes de introducir la dependencia o proponer una actualización de toolchain.

## Secuencia recomendada

1. **Conexiones utilizables y verificables.** Añadir un resolver servidor que vincule organización, adaptador, perfil y revisión aprobada. Desencriptar solo al ejecutar el adaptador. Invalidar verificaciones anteriores al cambiar credenciales o destino. En modo local, usar fixtures y mantener bloqueadas llamadas externas; activar proveedores reales requiere un entorno explícito que lo permita.
2. **Validación de alcance.** Diseñar probes de lectura: identidad de Telegram mediante `getMe`, identidad autenticada y permisos necesarios en Plane/Frappe. Confirmar contratos de cada instalación. Mostrar fecha, revisión validada y categorías de error sin tokens, cuerpos privados ni credenciales. Una autenticación correcta no prueba todas las ACL de recursos.
3. **Manifiesto de adaptadores.** Declarar autenticación, capacidades, esquemas de argumentos/resultados, versión y prueba de conexión. Mantener contratos independientes del lenguaje y del proveedor. Las credenciales no entran al prompt y la aplicación autoriza cada capacidad.
4. **Operación de Telegram desde la web.** Exponer estado del canal, vinculaciones y revocación; políticas por chat, progreso y cancelación. Reutilizar deduplicación y contexto registrado. Separar configuración, verificación y activación; no registrar webhooks ni enviar mensajes por guardar un formulario.
5. **Ejecuciones inspeccionables.** Mostrar estados, pausas, revisión aprobada, resultados saneados y recuperación. Los checkpoints no garantizan efectos externos exactamente una vez: conservar idempotencia y conciliación, sin reenviar automáticamente operaciones ambiguas.
6. **Evaluación y conocimiento autorizado.** Dataset sintético de consultas Plane/Frappe, casos de denegación/revocación y evidencia por fuente. Introducir búsqueda documental solo con autorización por recurso durante recuperación y respuesta. Evaluar Cedar si la complejidad de políticas lo justifica.

## Criterios de aceptación para el primer bloque

- Guardar un perfil no activa tráfico externo ni webhooks.
- El runtime obtiene la conexión desde identidad verificada y contexto registrado; nunca desde un tenant enviado por el cliente.
- La verificación queda asociada a una revisión exacta; rotación y borrado invalidan ese estado.
- Una conexión válida sin concesión de recurso sigue produciendo denegación.
- UI, errores, auditoría y prompts no revelan secretos ni cuerpos privados.
- Hay pruebas de aislamiento, revocación, conflictos de revisión y bloqueo de llamadas externas en modo local.
- Los cambios de API actualizan OpenAPI y documentación; los cambios de esquema incluyen migración.

## Fuentes técnicas específicas

- [Activepieces: modelo y ciclo de conexiones](https://github.com/activepieces/activepieces/blob/main/brain/knowledge/connections-auth/app-connections.md).
- [Activepieces: contrato del conector Telegram](https://github.com/activepieces/activepieces/blob/main/packages/pieces/community/telegram-bot/src/index.ts).
- [Nango: autenticación, conexiones y sincronización](https://github.com/NangoHQ/nango/blob/master/README.md).
- [OpenClaw: Telegram](https://github.com/openclaw/openclaw/blob/main/docs/channels/telegram.md).
- [Hermes Agent: capacidades y gateway](https://github.com/NousResearch/hermes-agent/blob/main/README.md).
- [LangGraph: interrupciones y reanudación](https://docs.langchain.com/oss/python/langgraph/interrupts).
- [Dify: capacidades](https://github.com/langgenius/dify/blob/main/README.md).
- [Rig: workspace y requisito Rust](https://github.com/0xPlaygrounds/rig/blob/main/Cargo.toml).
- [Cedar: biblioteca y modelo de autorización](https://github.com/cedar-policy/cedar/blob/main/README.md).
- [n8n: revisión humana de herramientas](https://github.com/n8n-io/n8n-docs/blob/main/docs/build/integrate-ai/ai-examples/human-in-the-loop-for-tools.md).
