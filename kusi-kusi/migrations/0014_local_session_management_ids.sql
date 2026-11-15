ALTER TABLE local_sessions
 ADD COLUMN id UUID NOT NULL DEFAULT gen_random_uuid();

CREATE UNIQUE INDEX local_sessions_management_id ON local_sessions(id);
