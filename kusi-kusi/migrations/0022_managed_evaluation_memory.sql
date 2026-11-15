CREATE TABLE managed_evaluation_runs(
 id UUID PRIMARY KEY, organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 actor_id UUID NOT NULL, suite_version TEXT NOT NULL, result JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 FOREIGN KEY(organization_id,actor_id) REFERENCES local_users(organization_id,id) ON DELETE CASCADE
);
CREATE TABLE authorized_memory_preferences(
 organization_id UUID NOT NULL, actor_id UUID NOT NULL,
 consent BOOLEAN NOT NULL DEFAULT false, ttl_seconds BIGINT NOT NULL DEFAULT 604800 CHECK(ttl_seconds BETWEEN 1 AND 604800),
 PRIMARY KEY(organization_id,actor_id),
 FOREIGN KEY(organization_id,actor_id) REFERENCES local_users(organization_id,id) ON DELETE CASCADE
);
CREATE TABLE authorized_memories(
 id UUID PRIMARY KEY, organization_id UUID NOT NULL, actor_id UUID NOT NULL,
 source_ref TEXT NOT NULL, source_version TEXT NOT NULL, ciphertext TEXT,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ NOT NULL,
 deleted_at TIMESTAMPTZ, CHECK(expires_at<=created_at+interval '7 days'),
 FOREIGN KEY(organization_id,actor_id) REFERENCES local_users(organization_id,id) ON DELETE CASCADE,
 UNIQUE(organization_id,actor_id,source_ref)
);

CREATE FUNCTION invalidate_resource_memory() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 UPDATE authorized_memories SET deleted_at=COALESCE(deleted_at,now()),ciphertext=NULL WHERE organization_id=OLD.organization_id AND actor_id=OLD.user_id AND source_ref=OLD.resource_ref;
 RETURN OLD;
END $$;
CREATE TRIGGER managed_memory_grant_revocation BEFORE DELETE ON local_resource_grants FOR EACH ROW EXECUTE FUNCTION invalidate_resource_memory();
