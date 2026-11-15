ALTER TABLE managed_connection_settings ADD CONSTRAINT managed_connection_tenant_id UNIQUE(organization_id,id);
ALTER TABLE managed_connection_settings ADD COLUMN adapter_version TEXT NOT NULL DEFAULT '1';
ALTER TABLE managed_connection_settings ADD COLUMN enabled_revision BIGINT;
ALTER TABLE managed_connection_settings ADD COLUMN disabled_at TIMESTAMPTZ;
CREATE TABLE managed_connection_verifications (
 id UUID PRIMARY KEY, organization_id UUID NOT NULL, profile_id UUID NOT NULL,
 profile_revision BIGINT NOT NULL CHECK(profile_revision>0), adapter_version TEXT NOT NULL,
 mode TEXT NOT NULL CHECK(mode IN ('fixture','provider')),
 status TEXT NOT NULL CHECK(status IN ('pending','verified','failed','stale')),
 checked_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '15 minutes',
 scope_summary TEXT NOT NULL DEFAULT 'not_checked', error_code TEXT,
 FOREIGN KEY(organization_id,profile_id) REFERENCES managed_connection_settings(organization_id,id) ON DELETE CASCADE,
 CHECK(expires_at<=checked_at+interval '15 minutes')
);
CREATE UNIQUE INDEX managed_probe_pending ON managed_connection_verifications(profile_id,profile_revision) WHERE status='pending';
CREATE INDEX managed_verification_latest ON managed_connection_verifications(organization_id,profile_id,checked_at DESC);
CREATE FUNCTION invalidate_managed_connection() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.revision <> OLD.revision THEN
  NEW.enabled_revision=NULL;
  NEW.disabled_at=now();
  UPDATE managed_connection_verifications SET status='stale' WHERE profile_id=OLD.id AND status IN ('pending','verified');
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER managed_connection_revision BEFORE UPDATE ON managed_connection_settings FOR EACH ROW EXECUTE FUNCTION invalidate_managed_connection();
