-- Upgrade databases provisioned before evidence_reference was added.
-- Apply with the deployment migration mechanism before enabling reconciliation.
ALTER TABLE cer_operation_journal
    ADD COLUMN IF NOT EXISTS evidence_reference TEXT;
