CREATE TABLE creative_budgets (
 organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
 daily_limit_microusd BIGINT NOT NULL CHECK (daily_limit_microusd BETWEEN 0 AND 1000000000),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE creative_budget_changes (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 principal_hash CHAR(64) NOT NULL,
 daily_limit_microusd BIGINT NOT NULL,
 changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
