CREATE TABLE managed_connection_settings (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('plane','frappe','telegram')),
    label TEXT NOT NULL CHECK (length(label) BETWEEN 2 AND 120),
    base_url TEXT,
    workspace TEXT,
    bot_username TEXT,
    webhook_url TEXT,
    credential_ciphertext TEXT,
    credential_fields TEXT[] NOT NULL DEFAULT '{}',
    revision BIGINT NOT NULL CHECK (revision > 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id,kind),
    CHECK ((credential_ciphertext IS NULL) = (cardinality(credential_fields) = 0))
);
