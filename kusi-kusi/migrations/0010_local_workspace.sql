ALTER TABLE organization_api_keys ADD COLUMN expires_at TIMESTAMPTZ;
CREATE TABLE local_users (
 id UUID PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 username TEXT NOT NULL, display_name TEXT NOT NULL, password_hash TEXT NOT NULL,
 role TEXT NOT NULL CHECK (role IN ('owner','editor','approver','viewer')),
 disabled BOOLEAN NOT NULL DEFAULT false, platform_admin BOOLEAN NOT NULL DEFAULT false,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(organization_id,username)
);
CREATE TABLE local_sessions (
 key_hash CHAR(64) PRIMARY KEY REFERENCES organization_api_keys(key_hash) ON DELETE CASCADE,
 user_id UUID NOT NULL REFERENCES local_users(id) ON DELETE CASCADE,
 expires_at TIMESTAMPTZ NOT NULL, revoked_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE local_rate_limits (
 key_hash CHAR(64) PRIMARY KEY, window_start TIMESTAMPTZ NOT NULL DEFAULT now(), attempts INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE local_assets (
 id UUID PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 uploaded_by UUID NOT NULL REFERENCES local_users(id), label TEXT NOT NULL, filename TEXT NOT NULL,
 media_type TEXT NOT NULL, size_bytes BIGINT NOT NULL CHECK (size_bytes>0), sha256 CHAR(64) NOT NULL,
 storage_key TEXT NOT NULL UNIQUE, simulated BOOLEAN NOT NULL DEFAULT false,
 creative_job_id UUID REFERENCES creative_jobs(id), created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(organization_id,id), UNIQUE(creative_job_id)
);
CREATE TABLE local_conversations (
 id UUID PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 user_id UUID REFERENCES local_users(id), channel_id UUID NOT NULL, sender TEXT NOT NULL,
 title TEXT NOT NULL, context TEXT NOT NULL DEFAULT '{}', context_expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '24 hours',
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(organization_id,channel_id,sender,user_id)
);
CREATE TABLE local_messages (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 conversation_id UUID NOT NULL REFERENCES local_conversations(id) ON DELETE CASCADE,
 event_key TEXT NOT NULL, direction TEXT NOT NULL CHECK(direction IN ('in','out')),
 content TEXT NOT NULL, metadata TEXT NOT NULL DEFAULT '{}', created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(conversation_id,event_key,direction)
);
CREATE TABLE local_link_codes (
 code_hash CHAR(64) PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 user_id UUID NOT NULL REFERENCES local_users(id), channel_id UUID NOT NULL,
 expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '10 minutes', used_at TIMESTAMPTZ
);
CREATE TABLE local_channel_bindings (
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 channel_id UUID NOT NULL, sender TEXT NOT NULL, user_id UUID NOT NULL REFERENCES local_users(id),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY(organization_id,channel_id,sender)
);
ALTER TABLE channel_inbox ADD COLUMN asset_id UUID REFERENCES local_assets(id);
ALTER TABLE channel_inbox ADD COLUMN group_id UUID;
ALTER TABLE channel_inbox ADD COLUMN lease_token UUID;
ALTER TABLE channel_inbox ADD COLUMN lease_until TIMESTAMPTZ;
ALTER TABLE channel_inbox ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE channel_inbox ADD COLUMN error_code TEXT;
CREATE TABLE local_outbox (
 id UUID PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 conversation_id UUID NOT NULL REFERENCES local_conversations(id) ON DELETE CASCADE,
 channel_id UUID NOT NULL, recipient TEXT NOT NULL, content TEXT NOT NULL,
 provider_message_id TEXT, status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN ('queued','sending','simulated','accepted','delivered','read','failed','unknown')),
 lease_until TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(conversation_id,id)
);
CREATE TABLE local_publications (
 id UUID PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 created_by UUID NOT NULL REFERENCES local_users(id), asset_id UUID NOT NULL,
 platform TEXT NOT NULL CHECK(platform IN ('instagram','facebook','tiktok')),
 snapshot TEXT NOT NULL, caption TEXT NOT NULL, privacy TEXT NOT NULL CHECK(privacy IN ('private','friends','public')),
 scheduled_at TIMESTAMPTZ, revision INTEGER NOT NULL DEFAULT 1,
 approval_digest CHAR(64) NOT NULL, approval_expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '30 minutes',
 state TEXT NOT NULL DEFAULT 'draft' CHECK(state IN ('draft','approved','scheduled','publishing','published_simulated','failed','canceled')),
 approved_by UUID REFERENCES local_users(id), approved_at TIMESTAMPTZ,
 receipt TEXT, lease_until TIMESTAMPTZ, idempotency_key UUID NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(organization_id,idempotency_key), FOREIGN KEY(organization_id,asset_id) REFERENCES local_assets(organization_id,id)
);
CREATE INDEX local_publications_due ON local_publications(scheduled_at) WHERE state IN ('approved','scheduled');
CREATE TABLE local_audit (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 user_id UUID REFERENCES local_users(id), action TEXT NOT NULL, target TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX local_audit_tenant ON local_audit(organization_id,created_at DESC);
