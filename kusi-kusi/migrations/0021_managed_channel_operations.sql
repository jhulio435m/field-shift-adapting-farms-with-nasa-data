ALTER TABLE local_channel_bindings ADD COLUMN id UUID NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE local_channel_bindings ADD CONSTRAINT local_binding_id_unique UNIQUE(id);
ALTER TABLE local_channel_bindings ADD COLUMN version BIGINT NOT NULL DEFAULT 1 CHECK(version>0);
ALTER TABLE local_channel_bindings ADD COLUMN expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '30 days';
ALTER TABLE local_channel_bindings ADD COLUMN revoked_at TIMESTAMPTZ;
ALTER TABLE local_outbox ADD COLUMN version BIGINT NOT NULL DEFAULT 1 CHECK(version>0);
ALTER TABLE local_outbox DROP CONSTRAINT local_outbox_status_check;
ALTER TABLE local_outbox ADD CONSTRAINT local_outbox_status_check CHECK(status IN ('queued','sending','simulated','accepted','delivered','read','failed','unknown','cancelled'));
ALTER TABLE local_outbox ADD CONSTRAINT local_outbox_tenant_id UNIQUE(organization_id,id);
CREATE TABLE managed_execution_events (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 organization_id UUID NOT NULL, execution_id UUID NOT NULL,
 version BIGINT NOT NULL, status TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 FOREIGN KEY(organization_id,execution_id) REFERENCES local_outbox(organization_id,id) ON DELETE CASCADE,
 UNIQUE(execution_id,version)
);
CREATE FUNCTION version_managed_outbox() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.status IS DISTINCT FROM NEW.status THEN NEW.version=OLD.version+1; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER managed_outbox_version BEFORE UPDATE ON local_outbox FOR EACH ROW EXECUTE FUNCTION version_managed_outbox();
CREATE FUNCTION record_managed_outbox_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO managed_execution_events(organization_id,execution_id,version,status) VALUES(NEW.organization_id,NEW.id,NEW.version,NEW.status) ON CONFLICT DO NOTHING;
 RETURN NEW;
END $$;
CREATE TRIGGER managed_outbox_event AFTER INSERT OR UPDATE ON local_outbox FOR EACH ROW EXECUTE FUNCTION record_managed_outbox_event();
INSERT INTO managed_execution_events(organization_id,execution_id,version,status) SELECT organization_id,id,version,status FROM local_outbox;
