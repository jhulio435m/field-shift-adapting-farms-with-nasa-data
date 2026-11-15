CREATE TABLE execution_audit (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 event_id UUID NOT NULL,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 capability TEXT NOT NULL CHECK (length(capability) <= 120),
 version TEXT NOT NULL CHECK (length(version) <= 32),
 outcome TEXT NOT NULL CHECK (outcome IN ('started','succeeded','forbidden','invalid_data','unavailable')),
 occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX execution_audit_tenant_time ON execution_audit (organization_id, occurred_at DESC);
