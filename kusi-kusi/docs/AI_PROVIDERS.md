# Proveedores de IA y transcripción

La capa de aplicación define `LlmProvider` y `Transcriber`; el dominio y los conectores empresariales no dependen de proveedores. Los perfiles se configuran por organización. Un perfil de conversación y otro de audio pueden usar proveedores distintos, por ejemplo Groq Whisper para audio y Gemini, GPT o Claude para interpretar la solicitud.

## Implementación

| Proveedor configurado | Protocolo usado | Autenticación |
|---|---|---|
| `openai` | Responses `/v1/responses`, `store:false` | Bearer |
| `gemini` | `v1beta/models/{model}:generateContent` | `x-goog-api-key` |
| `claude` | `/v1/messages`, versión explícita | `x-api-key` y `anthropic-version` |
| `openrouter` | `/api/v1/chat/completions` | Bearer; backends autorizados explícitos |
| `nvidia` | `/v1/chat/completions` en integrate.api.nvidia.com | Bearer |
| `groq` | `/openai/v1/chat/completions` | Bearer |
| `ollama` | `/api/chat`, sin streaming | Opcional según despliegue |
| `vllm`, `lmstudio`, `openai_compatible` | Chat Completions en endpoint del administrador | Opcional según despliegue |

Los modelos se seleccionan explícitamente: no hay aliases de “último modelo” ni selección automática. La compatibilidad de herramientas depende del modelo; `tools_supported` debe habilitarse después de verificarlo. Los nombres de herramientas usan letras, números, `_` o `-` (máximo 64 caracteres); el orquestador debe mapearlos a capacidades empresariales, cuyos ids pueden contener puntos.

Los adaptadores normalizan texto, propuestas de herramienta, motivo de finalización y metadatos de uso. Rechazan resultados incompletos o bloqueados y propuestas de herramientas desconocidas o con argumentos que no satisfacen su esquema. Los esquemas no admiten referencias remotas. Una propuesta nunca autoriza ni ejecuta una operación. No se implementa todavía un bucle multiturno de resultados de herramientas: la orquestación conversacional permanece separada y pendiente.

## Configuración

`KUSI_LLM_PROFILES` contiene un objeto JSON `{"organization-uuid":{"profile-name":{...}}}`. [Ejemplo completo](../examples/llm-profiles.json). `KUSI_AUDIO_PROFILES` tiene la misma separación por organización; [ejemplos de audio](../examples/audio-profiles.json). Reemplazar el UUID, los modelos y referencias de credenciales antes de habilitar. Los ejemplos son plantillas, no credenciales ni modelos verificados en una cuenta real.

Cada perfil de conversación declara `provider`, `model`, `api_key_env`, `endpoint`, `max_output_tokens`, `timeout_ms`, `tools_supported`, `anthropic_version` y `routing_providers`. Las claves se resuelven desde variables de entorno protegidas. Endpoints cloud son fijos; no se permite sustituirlos por una URL arbitraria. Los endpoints locales son configuración exclusiva del servidor: HTTP solo en loopback o HTTPS para servidores administrados. No exponer APIs locales sin autenticación y restricciones de red.

OpenRouter requiere una lista explícita `routing_providers` de slugs autorizados; envía `only`, `allow_fallbacks:false`, `require_parameters:true`, `data_collection:deny`. Esto restringe el enrutamiento de la solicitud, pero no reemplaza evaluación de contratos, retención ni privacidad del servicio. No hay fallback automático de Kusi Kusi ni reintentos que puedan repetir cargos.

La configuración y presencia de claves se validan al iniciar. No se validan créditos, acceso al modelo ni disponibilidad remota hasta realizar una llamada. Solo habilitar proveedores autorizados por el cliente y evaluar los datos enviados. Las claves, prompts y respuestas no se registran en logs.

## API de conversación

`POST /v1/ai/generate` requiere `x-admin-key` y bearer de tenant. Ejemplo de cuerpo:

```json
{"profile":"local","request":{"instructions":"Interpreta solicitudes. No inventes datos empresariales.","input":"Consulta ficticia","tools":[]}}
```

Devuelve `text`, `tools`, `model`, `finish_reason`, `usage`. El endpoint es una herramienta de operador, no un chatbot público. Los datos devueltos por el modelo no son fuente autorizada de información empresarial. El flujo de permisos y ejecución de conectores debe seguir pasando por `ExecuteCapability`.

## Transcripción: Groq, OpenAI y servidores locales compatibles

Puerto `Transcriber`, perfiles separados y carga multipart hacia el proveedor. Groq usa `/openai/v1/audio/transcriptions`; el perfil de ejemplo usa `whisper-large-v3-turbo`. La selección entre ese modelo y `whisper-large-v3` depende de calidad, precio y disponibilidad; consultar la documentación y precios oficiales antes de activar.

`POST /v1/ai/transcribe?profile=groq_whisper&filename=voice.ogg&language=es`, cuerpo binario del archivo, `Content-Type: application/octet-stream`, más ambas credenciales de operador/tenant. El backend envía `file`, `model`, `response_format=json` y, si se indica, idioma ISO de dos letras.

Límite propio: 10 MiB por archivo. Formatos admitidos: flac, mp3, mp4, mpeg, mpga, m4a, ogg, wav, webm. Es una validación de nombre/metadatos; el proveedor debe validar la codificación real. No se descargan URLs de audio aportadas por usuarios. Los bytes se procesan en memoria, sin guardarse en disco. No hay traducción, streaming, troceado, conversión FFmpeg ni descarga automática desde WhatsApp/Telegram en esta entrega. OGG/Opus de mensajería debe probarse con el proveedor elegido; no se instala ni descarga ningún modelo local automáticamente.

## Límites y estado verificado

Inferencia no streaming; input+instrucciones hasta 64 KiB, máximo 32 herramientas, salida HTTP del proveedor hasta 2 MiB, timeout hasta 120 s. No hay control de presupuesto acumulado, rate limit de usuarios, facturación, streaming, embeddings, visión ni síntesis de voz. El token de salida es un límite por solicitud, no una garantía de precio.

Se verificaron contratos mediante fixtures de Responses, Gemini, Messages, Chat Completions y Ollama; rechazo de truncamiento, estructura de llamadas a herramientas y un servidor HTTP simulado local. Se comprobaron validadores de audio y multipart. No se realizaron llamadas cloud ni se verificó calidad de un modelo real. Se necesitan claves autorizadas y modelos accesibles para pruebas reales. Los perfiles no están conectados todavía al worker de conversaciones.

## Documentación oficial consultada

- OpenAI: [texto/Responses](https://developers.openai.com/api/docs/guides/text), [function calling](https://developers.openai.com/api/docs/guides/function-calling), [audio](https://developers.openai.com/api/docs/guides/speech-to-text).
- Gemini: [generateContent](https://ai.google.dev/api/generate-content), [herramientas](https://ai.google.dev/gemini-api/docs/function-calling), [salidas estructuradas](https://ai.google.dev/gemini-api/docs/structured-output).
- Anthropic: [Messages](https://platform.claude.com/docs/en/api/messages/create), [tool use](https://platform.claude.com/docs/en/agents-and-tools/tool-use/how-tool-use-works).
- OpenRouter: [Chat Completions](https://openrouter.ai/docs/api/api-reference/chat/create-a-chat-completion), [routing](https://openrouter.ai/docs/guides/routing/provider-selection), [privacidad](https://openrouter.ai/docs/guides/privacy/data-collection).
- NVIDIA: [LLM APIs](https://docs.api.nvidia.com/nim/reference/llm-apis). NIM local puede usar el perfil compatible con endpoint explícito.
- Groq: [compatibilidad](https://console.groq.com/docs/openai), [transcripción y precios publicados](https://console.groq.com/docs/speech-to-text), [API](https://console.groq.com/docs/api-reference).
- Ollama: [chat nativo](https://docs.ollama.com/api/chat), [compatibilidad OpenAI](https://docs.ollama.com/api/openai-compatibility).
- vLLM: [servidor compatible](https://docs.vllm.ai/en/latest/serving/openai_compatible_server/); el sitio devolvió redirección durante esta revisión, por lo que la integración se limita al contrato compatible y no presume particularidades de vLLM.
- LM Studio: [endpoints compatibles](https://lmstudio.ai/docs/developer/openai-compat).
- llama.cpp: [servidor oficial](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md); utilizar perfil `openai_compatible`.

Este índice cubre las APIs utilizadas y sus límites, no reproduce el contenido completo de cada proveedor. Modelos, precios y permisos cambian; verificar en el proveedor antes de habilitar cada perfil.

Prueba de API local repetible: `python3 scripts/verify_ai.py` (requiere API local y tenant demo como las pruebas de canales). Comprueba inferencia con proveedor simulado, aislamiento de perfiles y subida multipart real hacia un proveedor simulado; no cobra ni descarga modelos.
