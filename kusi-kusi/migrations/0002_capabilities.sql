CREATE TABLE capabilities (
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 identifier TEXT NOT NULL,
 version TEXT NOT NULL,
 description TEXT NOT NULL,
 adapter TEXT NOT NULL,
 required_permission TEXT NOT NULL,
 enabled BOOLEAN NOT NULL DEFAULT false,
 timeout_ms INTEGER NOT NULL CHECK (timeout_ms BETWEEN 100 AND 30000),
 PRIMARY KEY (organization_id, identifier, version)
);
