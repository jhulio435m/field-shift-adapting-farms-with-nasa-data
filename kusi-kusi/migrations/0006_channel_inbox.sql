CREATE TABLE channel_inbox (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 channel_id UUID NOT NULL,
 event_id TEXT NOT NULL,
 sender TEXT NOT NULL,
 kind TEXT NOT NULL CHECK (kind IN ('text','audio','status')),
 content TEXT NOT NULL,
 received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 processed_at TIMESTAMPTZ,
 UNIQUE (channel_id, event_id)
);
CREATE INDEX channel_inbox_pending ON channel_inbox (organization_id, received_at) WHERE processed_at IS NULL;
