CREATE TABLE oauth_sessions (
 state_hash CHAR(64) PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 provider TEXT NOT NULL,
 principal_hash CHAR(64) NOT NULL,
 browser_hash CHAR(64) NOT NULL,
 profile TEXT NOT NULL,
 verifier_ciphertext TEXT,
 expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '10 minutes',
 consumed_at TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX oauth_sessions_expiry ON oauth_sessions(expires_at);
CREATE TABLE oauth_connections (
 id UUID PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 provider TEXT NOT NULL,
 client_id TEXT NOT NULL,
 external_id TEXT NOT NULL,
 scopes TEXT NOT NULL,
 profile TEXT NOT NULL,
 tokens_ciphertext TEXT,
 status TEXT NOT NULL CHECK (status IN ('active','refreshing','needs_reauth','revoking','revoke_pending','disconnected')),
 expires_at TIMESTAMPTZ NOT NULL,
 refresh_expires_at TIMESTAMPTZ,
 issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 operation_id UUID,
 lease_until TIMESTAMPTZ,
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(provider,client_id,external_id)
);
CREATE INDEX oauth_connections_tenant ON oauth_connections(organization_id,updated_at DESC);
CREATE TABLE oauth_events (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 connection_id UUID REFERENCES oauth_connections(id) ON DELETE SET NULL,
 provider TEXT NOT NULL,
 event TEXT NOT NULL,
 principal_hash CHAR(64),
 occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
