CREATE TABLE creative_jobs (
 id UUID PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 idempotency_key UUID NOT NULL,
 principal_hash CHAR(64) NOT NULL,
 proposal TEXT NOT NULL,
 approval_digest CHAR(64) NOT NULL,
 approval_expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + interval '30 minutes',
 state TEXT NOT NULL DEFAULT 'awaiting_approval' CHECK (state IN ('awaiting_approval','approved','submitting','queued','processing','succeeded','failed','unknown','canceled')),
 approved_by_hash CHAR(64),
 approved_at TIMESTAMPTZ,
 remote_request TEXT,
 result TEXT NOT NULL DEFAULT 'null',
 error_code TEXT,
 poll_failures INTEGER NOT NULL DEFAULT 0,
 lease_token UUID,
 lease_until TIMESTAMPTZ,
 next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE (organization_id, idempotency_key)
);
CREATE INDEX creative_jobs_pending ON creative_jobs (next_attempt_at) WHERE state IN ('approved','queued','processing');
CREATE INDEX creative_jobs_tenant ON creative_jobs (organization_id, created_at DESC);
CREATE TABLE creative_events (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 job_id UUID NOT NULL REFERENCES creative_jobs(id) ON DELETE CASCADE,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 event TEXT NOT NULL,
 principal_hash CHAR(64),
 occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX creative_events_tenant ON creative_events (organization_id, job_id);
