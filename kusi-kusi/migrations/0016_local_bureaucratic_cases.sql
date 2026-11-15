CREATE TABLE local_bureaucratic_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    source_key TEXT NOT NULL,
    title TEXT NOT NULL,
    official_url TEXT NOT NULL,
    resolution TEXT NOT NULL,
    version TEXT NOT NULL,
    validity_status TEXT NOT NULL CHECK (validity_status IN ('unverified','current','needs_confirmation','superseded')),
    page_or_section TEXT NOT NULL,
    citation_excerpt TEXT NOT NULL,
    document_sha256 CHAR(64) NOT NULL,
    verified_by UUID,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, source_key),
    UNIQUE (organization_id, id),
    FOREIGN KEY (organization_id, verified_by)
        REFERENCES local_users(organization_id, id) ON DELETE SET NULL (verified_by)
);

CREATE TABLE local_bureaucratic_cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    requester_user_id UUID NOT NULL,
    assigned_staff_user_id UUID NOT NULL,
    workflow_key TEXT NOT NULL,
    tracking_code TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'needs_information'
        CHECK (status IN ('draft','needs_information','ready_for_review','awaiting_institution','answered','closed')),
    payment_amount_cents BIGINT NOT NULL CHECK (payment_amount_cents > 0),
    payment_currency CHAR(3) NOT NULL CHECK (payment_currency = 'PEN'),
    payment_concept TEXT NOT NULL,
    payment_reference_masked TEXT NOT NULL CHECK (
        char_length(payment_reference_masked) = 8
        AND left(payment_reference_masked, 4) = '****'
        AND right(payment_reference_masked, 4) ~ '^[0-9]{4}$'
    ),
    health_interruption_summary TEXT NOT NULL
        CHECK (health_interruption_summary = 'Interrumpido por motivos de salud'),
    receipt_fixture_label TEXT NOT NULL,
    is_synthetic BOOLEAN NOT NULL DEFAULT true CHECK (is_synthetic),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at TIMESTAMPTZ,
    UNIQUE (organization_id, id),
    UNIQUE (organization_id, tracking_code),
    FOREIGN KEY (organization_id, requester_user_id)
        REFERENCES local_users(organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, assigned_staff_user_id)
        REFERENCES local_users(organization_id, id) ON DELETE CASCADE
);

CREATE INDEX local_bureaucratic_cases_requester_idx
    ON local_bureaucratic_cases (organization_id, requester_user_id, updated_at DESC);
CREATE INDEX local_bureaucratic_cases_staff_idx
    ON local_bureaucratic_cases (organization_id, assigned_staff_user_id, updated_at DESC);

CREATE TABLE local_bureaucratic_case_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL,
    case_id UUID NOT NULL,
    actor_user_id UUID,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'case_opened','source_added','guidance_viewed','request_drafted','request_edited',
        'request_approved','request_simulated_sent','information_requested','response_recorded','case_closed'
    )),
    actor_type TEXT NOT NULL CHECK (actor_type IN ('requester','staff','system')),
    summary TEXT NOT NULL,
    source_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (organization_id, case_id)
        REFERENCES local_bureaucratic_cases(organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, actor_user_id)
        REFERENCES local_users(organization_id, id) ON DELETE SET NULL (actor_user_id),
    FOREIGN KEY (organization_id, source_key)
        REFERENCES local_bureaucratic_sources(organization_id, source_key) ON DELETE SET NULL (source_key)
);

CREATE INDEX local_bureaucratic_case_events_case_idx
    ON local_bureaucratic_case_events (organization_id, case_id, created_at);

CREATE TABLE local_bureaucratic_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL,
    case_id UUID NOT NULL,
    requester_user_id UUID NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    recipient_unit TEXT NOT NULL CHECK (char_length(recipient_unit) BETWEEN 1 AND 120),
    body TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4000),
    include_receipt_fixture BOOLEAN NOT NULL DEFAULT false,
    digest CHAR(64) NOT NULL,
    state TEXT NOT NULL DEFAULT 'draft' CHECK (state IN ('draft','approved','simulated_sent')),
    approved_by UUID,
    approved_at TIMESTAMPTZ,
    shared_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (organization_id, case_id)
        REFERENCES local_bureaucratic_cases(organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, requester_user_id)
        REFERENCES local_users(organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, approved_by)
        REFERENCES local_users(organization_id, id) ON DELETE SET NULL (approved_by),
    UNIQUE (organization_id, case_id),
    UNIQUE (organization_id, id)
);

CREATE TABLE local_bureaucratic_sharing_consents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL,
    request_id UUID NOT NULL,
    request_version INTEGER NOT NULL,
    recipient_unit TEXT NOT NULL,
    scope TEXT NOT NULL CHECK (scope = 'request_body_and_selected_receipt_fixture'),
    consented_by UUID NOT NULL,
    consented_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    FOREIGN KEY (organization_id, request_id)
        REFERENCES local_bureaucratic_requests(organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, consented_by)
        REFERENCES local_users(organization_id, id) ON DELETE CASCADE,
    UNIQUE (organization_id, request_id, request_version)
);

CREATE TABLE local_bureaucratic_responses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL,
    case_id UUID NOT NULL,
    responder_user_id UUID NOT NULL,
    response_type TEXT NOT NULL CHECK (response_type IN (
        'request_information','guidance','referral','institutional_decision'
    )),
    body TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4000),
    citation_keys TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (organization_id, case_id)
        REFERENCES local_bureaucratic_cases(organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, responder_user_id)
        REFERENCES local_users(organization_id, id) ON DELETE CASCADE
);

CREATE INDEX local_bureaucratic_responses_case_idx
    ON local_bureaucratic_responses (organization_id, case_id, created_at);
