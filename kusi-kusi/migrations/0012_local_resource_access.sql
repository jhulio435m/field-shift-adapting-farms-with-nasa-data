CREATE TABLE local_resource_grants (
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 user_id UUID NOT NULL,
 capability TEXT NOT NULL CHECK (capability = 'demo.resource.read'),
 resource_ref TEXT NOT NULL CHECK (resource_ref IN ('demo-resource-001','demo-resource-002')),
 created_by UUID NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 PRIMARY KEY (organization_id,user_id,capability,resource_ref),
 FOREIGN KEY (organization_id,user_id) REFERENCES local_users(organization_id,id) ON DELETE CASCADE,
 FOREIGN KEY (organization_id,created_by) REFERENCES local_users(organization_id,id) ON DELETE RESTRICT
);
CREATE INDEX local_resource_grants_user ON local_resource_grants(organization_id,user_id);

CREATE TABLE local_resource_access_audit (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 actor_id UUID NOT NULL REFERENCES local_users(id) ON DELETE CASCADE,
 capability TEXT NOT NULL CHECK (capability = 'demo.resource.read'),
 resource_digest CHAR(64) NOT NULL,
 allowed BOOLEAN NOT NULL,
 adapter_invoked BOOLEAN NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX local_resource_access_audit_tenant ON local_resource_access_audit(organization_id,created_at DESC);
