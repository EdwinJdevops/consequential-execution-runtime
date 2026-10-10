//! Durable operation identity and replay admission.
//!
//! This journal prevents duplicate admission, not yet duplicate external effects.
//! A production executor must reconcile an ambiguous effect before retry.
use postgres::{Client, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    New,
    Existing,
    Conflict,
}

/// The caller owns the connection and must provision the journal schema.
/// The operation ID is unique; a reused ID with different intent is rejected.
pub fn admit(
    client: &mut Client,
    operation_id: &str,
    fingerprint: &str,
) -> Result<Admission, Error> {
    let inserted = client.execute(
        "INSERT INTO cer_operation_journal (operation_id, fingerprint, status) \
         VALUES ($1, $2, 'PENDING') ON CONFLICT (operation_id) DO NOTHING",
        &[&operation_id, &fingerprint],
    )?;
    if inserted == 1 {
        return Ok(Admission::New);
    }
    let row = client.query_one(
        "SELECT fingerprint FROM cer_operation_journal WHERE operation_id = $1",
        &[&operation_id],
    )?;
    let existing: String = row.get(0);
    Ok(if existing == fingerprint {
        Admission::Existing
    } else {
        Admission::Conflict
    })
}

/// Provision once via a controlled migration, not on each operation.
pub const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS cer_operation_journal (\
    operation_id TEXT PRIMARY KEY, \
    fingerprint TEXT NOT NULL, \
    status TEXT NOT NULL CHECK (status IN ('PENDING', 'EXECUTING', 'UNKNOWN', 'SUCCEEDED', 'FAILED'))\
)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    Claimed,
    NotClaimable,
}

/// Atomically moves a newly admitted operation to EXECUTING.
/// An existing, unknown, or terminal operation is never claimed again.
pub fn claim(client: &mut Client, operation_id: &str, fingerprint: &str) -> Result<Claim, Error> {
    let changed = client.execute(
        "UPDATE cer_operation_journal SET status = 'EXECUTING' \
         WHERE operation_id = $1 AND fingerprint = $2 AND status = 'PENDING'",
        &[&operation_id, &fingerprint],
    )?;
    Ok(if changed == 1 {
        Claim::Claimed
    } else {
        Claim::NotClaimable
    })
}

/// Fail closed after an execution attempt whose effect cannot be proven.
/// Only the owner of the execution path should invoke this transition.
pub fn mark_unknown(client: &mut Client, operation_id: &str) -> Result<bool, Error> {
    let changed = client.execute(
        "UPDATE cer_operation_journal SET status = 'UNKNOWN' \
         WHERE operation_id = $1 AND status = 'EXECUTING'",
        &[&operation_id],
    )?;
    Ok(changed == 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconciliation {
    Recorded,
    NotEligible,
}

/// Records an externally verified outcome. The caller must establish evidence
/// from the actual target system; this function does not verify that evidence.
pub fn reconcile(
    client: &mut Client,
    operation_id: &str,
    verified_status: &str,
    evidence_reference: &str,
) -> Result<Reconciliation, Error> {
    if !matches!(verified_status, "SUCCEEDED" | "FAILED") || evidence_reference.is_empty() {
        return Ok(Reconciliation::NotEligible);
    }
    let changed = client.execute(
        "UPDATE cer_operation_journal SET status = $2 \
         WHERE operation_id = $1 AND status IN ('EXECUTING', 'UNKNOWN') \
         AND length($3::text) > 0",
        &[&operation_id, &verified_status, &evidence_reference],
    )?;
    Ok(if changed == 1 {
        Reconciliation::Recorded
    } else {
        Reconciliation::NotEligible
    })
}
