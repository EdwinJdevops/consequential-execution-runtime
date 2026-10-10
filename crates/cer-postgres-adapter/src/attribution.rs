//! PostgreSQL-owned effect attribution; not applicable to external AWS effects.
use postgres::{Client, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributedObservation {
    Proven,
    DifferentOperation,
    Missing,
}

/// Reads the operation marker written in the same transaction as the resource.
pub fn observe_attribution(
    client: &mut Client,
    resource_id: &str,
    operation_id: &str,
) -> Result<AttributedObservation, Error> {
    let row = client.query_opt(
        "SELECT last_operation_id FROM cer_resources WHERE resource_id = $1",
        &[&resource_id],
    )?;
    let Some(row) = row else {
        return Ok(AttributedObservation::Missing);
    };
    let marker: Option<String> = row.get(0);
    Ok(match marker.as_deref() {
        Some(value) if value == operation_id => AttributedObservation::Proven,
        Some(_) => AttributedObservation::DifferentOperation,
        None => AttributedObservation::Missing,
    })
}

/// Atomically updates a PostgreSQL-owned resource and records its operation ID.
/// This does not claim journal atomicity; the journal is a separate connection.
pub fn apply_attributed(
    client: &mut Client,
    resource_id: &str,
    expected_version: i64,
    payload: &str,
    operation_id: &str,
) -> Result<bool, Error> {
    let changed = client.execute(
        "UPDATE cer_resources SET payload = $3::text::jsonb, \
         version = version + 1, last_operation_id = $4 \
         WHERE resource_id = $1 AND version = $2",
        &[&resource_id, &expected_version, &payload, &operation_id],
    )?;
    Ok(changed == 1)
}
