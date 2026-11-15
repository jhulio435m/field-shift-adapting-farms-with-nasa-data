-- Policies are adapter-wide so another capability/version cannot bypass them.
ALTER TABLE organization_api_keys ADD CONSTRAINT organization_api_keys_tenant_hash_unique UNIQUE (organization_id, key_hash);

CREATE TABLE external_resource_policies (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    adapter TEXT NOT NULL CHECK (length(adapter) BETWEEN 1 AND 120),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, adapter)
);

CREATE TABLE external_identity_links (
    id UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL,
    principal_hash CHAR(64) NOT NULL,
    adapter TEXT NOT NULL,
    external_subject TEXT NOT NULL CHECK (length(external_subject) BETWEEN 1 AND 255),
    verification_method TEXT NOT NULL DEFAULT 'operator_manual' CHECK (verification_method='operator_manual'),
    verified_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL CHECK (expires_at > verified_at),
    revoked_at TIMESTAMPTZ,
    PRIMARY KEY (organization_id, principal_hash, adapter),
    FOREIGN KEY (organization_id, adapter) REFERENCES external_resource_policies ON DELETE CASCADE,
    FOREIGN KEY (organization_id, principal_hash) REFERENCES organization_api_keys(organization_id, key_hash) ON DELETE CASCADE
);

CREATE TABLE external_resource_grants (
    link_id UUID NOT NULL REFERENCES external_identity_links(id) ON DELETE CASCADE,
    capability TEXT NOT NULL CHECK (length(capability) BETWEEN 1 AND 120),
    version TEXT NOT NULL CHECK (length(version) BETWEEN 1 AND 32),
    resource_ref TEXT NOT NULL CHECK (length(resource_ref) BETWEEN 1 AND 255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL CHECK (expires_at > created_at),
    revoked_at TIMESTAMPTZ,
    PRIMARY KEY (link_id, capability, version, resource_ref)
);
