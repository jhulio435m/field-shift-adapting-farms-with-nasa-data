# Official messaging channels

Implemented: dedicated per-channel webhook endpoints, authenticated receipt, normalized text/audio references and Meta delivery status events, transactional PostgreSQL inbox and deduplication. Incoming provider ids identify a messaging sender; they do not authenticate the person to a customer system. No business capability is automatically executed by a webhook.

## Configuration

Set `KUSI_CHANNELS` to a JSON object keyed by stable channel UUIDs. Each value has `organization_id`, `provider` (`telegram` or `whatsapp`), `external_id`, `secret_env`, and optional `verify_token_env`. Values ending in `_env` are environment variable names, not secrets. The organization must already exist. Configuration is validated at startup; no configured channels means all webhook channel ids return 404.

Example structure (replace identifiers; set secrets separately in the local ignored .env or a deployment secret store):

```json
{
  "11111111-1111-4111-8111-111111111111": {
    "organization_id": "22222222-2222-4222-8222-222222222222",
    "provider": "telegram",
    "external_id": "your-bot-id",
    "secret_env": "TELEGRAM_WEBHOOK_SECRET",
    "verify_token_env": null
  },
  "33333333-3333-4333-8333-333333333333": {
    "organization_id": "22222222-2222-4222-8222-222222222222",
    "provider": "whatsapp",
    "external_id": "your-phone-number-id",
    "secret_env": "META_APP_SECRET",
    "verify_token_env": "META_VERIFY_TOKEN"
  }
}
```

Webhook URL: `https://your-host/webhooks/<channel-uuid>`. HTTPS termination/public routing must be deployed separately. UUID knowledge is insufficient to authenticate a webhook.

## Telegram

Create a bot with BotFather. Register the HTTPS callback using official `setWebhook`, supplying the same random `secret_token` referenced by `secret_env`. The application validates `X-Telegram-Bot-Api-Secret-Token` with constant-time comparison. Generate at least 16 characters from Telegram's allowed alphabet. This delivery accepts private text/voice messages and acknowledges unsupported updates without storing them. Group messages, edits, callbacks and attachments other than voice are not processed.

## WhatsApp Cloud API

Configure the callback and verify token in the Meta app. GET verification requires `hub.mode=subscribe` and the configured verification token and returns the original challenge. POST validates HMAC-SHA256 `X-Hub-Signature-256` over raw bytes using the App Secret. The registered Phone Number ID must match the event metadata. This dedicated callback represents one configured number; shared-app routing of multiple numbers through one callback is not implemented. Subscribe the appropriate WABA/app to message events using the official setup procedure.

## Sending text

`POST /v1/channels/<channel-uuid>/send` requires both `x-admin-key` and tenant bearer authentication; tenant must own the configured channel. Body: `{"recipient":"provider-recipient-id","text":"message"}`. For WhatsApp also supply `graph_version` explicitly (for example the supported version selected in your Meta app). Provider access token is read only from server environment `KUSI_CHANNEL_TOKEN_<channel UUID without hyphens>`. Provider URLs are fixed, redirects/proxies disabled, requests timed out and response size bounded. Telegram sends use `sendMessage`; WhatsApp sends use the Cloud API messages endpoint. No send is performed during local webhook tests.

Only text dispatch is implemented, without automatic retry. This avoids duplicating messages after an ambiguous network failure. No outgoing durable queue or template-message workflow exists yet. Operators must respect official WhatsApp customer-service window/template rules; this endpoint does not establish eligibility to send a free-form message. Recipient/phone ids do not prove customer identity.

## Persistence and limits

Webhook acknowledgements occur after inbox commit. DB errors return 503 so providers can retry. Unique `(channel_id,event_id)` constraints deduplicate across process restarts. Batch inserts are transactional. The existing Axum body limit applies; normalized batches are limited to 100 events and content to 16 KiB per event. Audio stores the provider media id only; downloading/transcription is pending. The inbox contains message content, so deployment requires retention/deletion policies and restricted DB access. No conversational worker consumes the inbox yet; messages remain pending. No credentials are logged, and HTTP traces omit query strings to protect verification tokens.

## Verification

`./scripts/verify.sh` checks Rust formatting, Clippy, unit tests, PostgreSQL integration and build. With a local API and demo key available, run `KUSI_API_URL=http://127.0.0.1:8080 python3 scripts/verify_channels.py`. It starts a temporary fixture API, tests both authenticated webhook types, rejects missing signatures/secrets and mismatched Meta phone ids, checks verification challenge, and verifies two persisted rows for four valid duplicate deliveries. It sends nothing to external providers. Fictional test inbox rows remain in the local demo tenant.

Official references: https://core.telegram.org/bots/api#setwebhook and https://developers.facebook.com/docs/whatsapp/cloud-api/ . Live bot registration, Meta account subscription and actual message delivery are unverified until official credentials and a public HTTPS callback are provided.

## Espacio local

El trabajador local procesa inbox firmado y conversaciones del panel con debounce, contexto y códigos de vinculación de cuenta. Las respuestas de canales externos se guardan en `local_outbox` con estado `queued`; no se envían automáticamente. Las conversaciones del panel usan entrega `simulated`. Este modo no descarga/transcribe notas de voz ni ejecuta capacidades empresariales. Conectar el outbox real y la identidad de sistemas externos requiere la siguiente entrega con credenciales oficiales.
