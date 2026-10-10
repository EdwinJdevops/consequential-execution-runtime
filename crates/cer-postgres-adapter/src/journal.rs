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
