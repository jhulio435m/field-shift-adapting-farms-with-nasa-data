# ADR 0005: social authorization isolated from administrative identity

Status: accepted for local backend delivery.

Social OAuth links external accounts; it does not authenticate administrators to the panel. Initiation requires existing admin plus tenant credentials. A high-entropy state and browser cookie bind callbacks to a persisted organization and initiating principal. State is consumed transactionally before external exchange, including denial paths. PKCE support is tested but native profiles remain confidential web flows where its parameters have not been verified.

Provider transports use fixed endpoints and configured scopes. The server validates granted permissions and never exposes access/refresh tokens in responses. Credentials are encrypted with RustCrypto XChaCha20-Poly1305, random nonces and associated tenant/account context. Envelopes carry a key id to permit staged key rotation.

One provider/app/account belongs to a single tenant. Refresh/revocation use leases; a stale or uncertain refresh requires reauthorization rather than reuse of a potentially rotated refresh token. Local disconnection is distinct from confirmed provider revocation. Instagram revocation is explicitly pending/manual until its remote contract is verified.

Live app approval, HTTPS callback registration, linking UI, Facebook Page token selection, publishing and account deletion callbacks are outside this backend delivery.
