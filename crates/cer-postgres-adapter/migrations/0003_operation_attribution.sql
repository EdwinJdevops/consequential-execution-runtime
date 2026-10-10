-- PostgreSQL-owned resource attribution. Deploy before using attributed writes.
ALTER TABLE cer_resources
    ADD COLUMN IF NOT EXISTS last_operation_id TEXT;
