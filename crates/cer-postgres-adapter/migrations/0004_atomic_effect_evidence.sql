-- Immutable attribution for PostgreSQL-owned effects. Apply after 0003.
-- The same transaction that mutates the resource must insert this evidence.
CREATE TABLE IF NOT EXISTS cer_postgres_effects (
    operation_id TEXT PRIMARY KEY REFERENCES cer_operation_journal(operation_id) ON DELETE CASCADE,
    fingerprint TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    version_before BIGINT NOT NULL,
    version_after BIGINT NOT NULL,
    CONSTRAINT cer_postgres_effects_version_step
        CHECK (version_after = version_before + 1)
);
