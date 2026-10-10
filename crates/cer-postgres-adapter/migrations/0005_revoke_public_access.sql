-- Apply as the schema owner after 0004.
-- PostgreSQL grants table privileges to PUBLIC only when explicitly configured;
-- this migration removes any such broad grants, including inherited legacy grants.
-- This is NOT sufficient to isolate the application if it connects as table owner.
REVOKE ALL ON TABLE cer_operation_journal FROM PUBLIC;
REVOKE ALL ON TABLE cer_postgres_effects FROM PUBLIC;
REVOKE ALL ON TABLE cer_resources FROM PUBLIC;
