ALTER TABLE local_outbox ADD COLUMN cancel_requested BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE local_outbox ADD COLUMN managed_profile_id UUID;
ALTER TABLE local_outbox ADD COLUMN managed_profile_revision BIGINT CHECK(managed_profile_revision>0);
ALTER TABLE local_outbox ADD COLUMN managed_adapter_version TEXT;
ALTER TABLE local_outbox ADD COLUMN approved_digest TEXT;
ALTER TABLE local_outbox ADD CONSTRAINT managed_outbox_profile FOREIGN KEY(organization_id,managed_profile_id) REFERENCES managed_connection_settings(organization_id,id);
ALTER TABLE local_outbox ADD CONSTRAINT managed_outbox_profile_snapshot CHECK((managed_profile_id IS NULL AND managed_profile_revision IS NULL AND managed_adapter_version IS NULL) OR (managed_profile_id IS NOT NULL AND managed_profile_revision IS NOT NULL AND managed_adapter_version IS NOT NULL));
ALTER TABLE local_outbox DROP CONSTRAINT local_outbox_status_check;
ALTER TABLE local_outbox ADD CONSTRAINT local_outbox_status_check CHECK(status IN ('queued','sending','simulated','accepted','delivered','read','failed','unknown','cancelled','awaiting_review'));
CREATE OR REPLACE FUNCTION version_managed_outbox() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF (OLD.status,OLD.cancel_requested,OLD.conversation_id,OLD.content,OLD.recipient,OLD.channel_id,OLD.managed_profile_id,OLD.managed_profile_revision,OLD.managed_adapter_version) IS DISTINCT FROM (NEW.status,NEW.cancel_requested,NEW.conversation_id,NEW.content,NEW.recipient,NEW.channel_id,NEW.managed_profile_id,NEW.managed_profile_revision,NEW.managed_adapter_version) THEN
  NEW.version=OLD.version+1;
 END IF;
 IF (OLD.conversation_id,OLD.content,OLD.recipient,OLD.channel_id,OLD.managed_profile_id,OLD.managed_profile_revision,OLD.managed_adapter_version) IS DISTINCT FROM (NEW.conversation_id,NEW.content,NEW.recipient,NEW.channel_id,NEW.managed_profile_id,NEW.managed_profile_revision,NEW.managed_adapter_version) THEN
  NEW.approved_digest=NULL;
  IF OLD.approved_digest IS NOT NULL OR OLD.status='awaiting_review' THEN NEW.status='awaiting_review'; END IF;
 END IF;
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION record_managed_outbox_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO managed_execution_events(organization_id,execution_id,version,status) VALUES(NEW.organization_id,NEW.id,NEW.version,CASE WHEN NEW.status='sending' AND NEW.cancel_requested THEN 'cancel_requested' ELSE NEW.status END) ON CONFLICT DO NOTHING;
 RETURN NEW;
END $$;
