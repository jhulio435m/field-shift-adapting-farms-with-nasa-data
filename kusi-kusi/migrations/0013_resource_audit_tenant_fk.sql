ALTER TABLE local_resource_access_audit
 DROP CONSTRAINT local_resource_access_audit_actor_id_fkey;

ALTER TABLE local_resource_access_audit
 ADD CONSTRAINT local_resource_access_audit_tenant_actor_fk
 FOREIGN KEY (organization_id,actor_id)
 REFERENCES local_users(organization_id,id) ON DELETE CASCADE;
