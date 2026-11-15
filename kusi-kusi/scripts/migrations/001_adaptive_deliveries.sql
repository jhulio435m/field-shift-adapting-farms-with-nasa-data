-- Additive staging migration; preserve existing pairing and update deduplication.
CREATE TABLE IF NOT EXISTS adaptive_schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS adaptive_deliveries (
    digest TEXT PRIMARY KEY,
    status TEXT NOT NULL CHECK(status IN ('accepted', 'unknown'))
);
INSERT OR IGNORE INTO adaptive_schema_migrations(version) VALUES(1);
