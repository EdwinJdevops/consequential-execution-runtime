//! Read-only observation of PostgreSQL-owned effects.
//! Matching resource state alone cannot prove which operation caused it.
use postgres::{Client, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    ConsistentWithEffect,
    NotObserved,
    Inconclusive,
}

/// Inspect the authoritative resource row. This never writes journal state.
/// A matching payload/version is *not* sufficient evidence of causation.
pub fn observe(
    client: &mut Client,
    resource_id: &str,
    expected_version: i64,
    expected_payload: &str,
) -> Result<Observation, Error> {
    let row = client.query_opt(
        "SELECT version, payload::text FROM cer_resources WHERE resource_id = $1",
        &[&resource_id],
    )?;
    let Some(row) = row else {
        return Ok(Observation::Inconclusive);
    };
    let version: i64 = row.get(0);
    let payload: String = row.get(1);
    let expected: serde_json::Value = match serde_json::from_str(expected_payload) {
        Ok(value) => value,
        Err(_) => return Ok(Observation::Inconclusive),
    };
    let actual: serde_json::Value = match serde_json::from_str(&payload) {
        Ok(value) => value,
        Err(_) => return Ok(Observation::Inconclusive),
    };
    if expected_version.checked_add(1) == Some(version) && actual == expected {
        Ok(Observation::ConsistentWithEffect)
    } else if version == expected_version {
        Ok(Observation::NotObserved)
    } else {
        Ok(Observation::Inconclusive)
    }
}
