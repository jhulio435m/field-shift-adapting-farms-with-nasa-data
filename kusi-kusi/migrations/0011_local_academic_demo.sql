ALTER TABLE local_users
 ADD CONSTRAINT local_users_organization_id_id_key UNIQUE (organization_id, id);

CREATE TABLE local_academic_guardian_links (
 organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
 user_id UUID NOT NULL,
 student_ref TEXT NOT NULL CHECK (student_ref IN ('demo-student-001','demo-student-002')),
 created_by UUID NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 PRIMARY KEY (organization_id, user_id, student_ref),
 FOREIGN KEY (organization_id, user_id)
   REFERENCES local_users(organization_id, id) ON DELETE CASCADE,
 FOREIGN KEY (organization_id, created_by)
   REFERENCES local_users(organization_id, id) ON DELETE RESTRICT
);

CREATE INDEX local_academic_guardian_user
 ON local_academic_guardian_links (organization_id, user_id);
