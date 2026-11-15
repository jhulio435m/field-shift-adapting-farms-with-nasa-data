ALTER TABLE capabilities ADD COLUMN input_schema TEXT NOT NULL DEFAULT '{}', ADD COLUMN output_schema TEXT NOT NULL DEFAULT '{}';
