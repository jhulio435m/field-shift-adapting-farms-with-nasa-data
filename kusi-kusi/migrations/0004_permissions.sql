CREATE TABLE capability_grants (
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 principal_hash CHAR(64) NOT NULL REFERENCES organization_api_keys(key_hash) ON DELETE CASCADE,
 permission TEXT NOT NULL,
 PRIMARY KEY (organization_id, principal_hash, permission)
);
