CREATE TABLE admin_identities (
    issuer TEXT NOT NULL CHECK (char_length(issuer) BETWEEN 1 AND 2048),
    subject TEXT NOT NULL CHECK (char_length(subject) BETWEEN 1 AND 255),
    disabled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (issuer, subject)
);

CREATE TABLE admin_memberships (
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'approver', 'viewer')),
    disabled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (issuer, subject, organization_id),
    FOREIGN KEY (issuer, subject) REFERENCES admin_identities(issuer, subject) ON DELETE CASCADE
);

CREATE INDEX admin_memberships_organization_idx
    ON admin_memberships (organization_id, role)
    WHERE disabled_at IS NULL;

CREATE TABLE admin_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash CHAR(64) NOT NULL UNIQUE,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    FOREIGN KEY (issuer, subject) REFERENCES admin_identities(issuer, subject) ON DELETE CASCADE
);

CREATE INDEX admin_sessions_identity_idx
    ON admin_sessions (issuer, subject, expires_at)
    WHERE revoked_at IS NULL;

CREATE TABLE admin_oidc_attempts (
    state_hash CHAR(64) PRIMARY KEY,
    browser_hash CHAR(64) NOT NULL,
    nonce TEXT NOT NULL,
    code_verifier TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);

CREATE INDEX admin_oidc_attempts_expiry_idx
    ON admin_oidc_attempts (expires_at)
    WHERE consumed_at IS NULL;
