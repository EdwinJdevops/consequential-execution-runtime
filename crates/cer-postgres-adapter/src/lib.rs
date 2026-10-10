#![forbid(unsafe_code)]

pub mod journal;

use cer_core::{ConditionalOutcome, ConditionalTarget, ProposedAction};
use postgres::Client;

/// A database-owned versioned resource. The caller must provision the schema:
/// CREATE TABLE cer_resources (resource_id TEXT PRIMARY KEY,
/// version BIGINT NOT NULL, payload JSONB NOT NULL);
pub struct PostgresTarget {
    client: Client,
}

impl PostgresTarget {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl ConditionalTarget for PostgresTarget {
    type Error = postgres::Error;

    fn compare_and_execute(
        &mut self,
        action: &ProposedAction,
        expected_state: &str,
    ) -> Result<ConditionalOutcome, Self::Error> {
        // Version tokens are decimal BIGINT strings; invalid tokens cannot
        // match a database row. No mutation is attempted.
        let Ok(expected_version) = expected_state.parse::<i64>() else {
            return Ok(ConditionalOutcome::StateChanged);
        };
        // The statement itself is atomic under PostgreSQL row locking.
        // Query errors are NOT proof of no effect; the coordinator must treat
        // transport failures as ambiguous until reconciled.
        let row = self.client.query_opt(
            "UPDATE cer_resources SET payload = $3::text::jsonb, version = version + 1 \
             WHERE resource_id = $1 AND version = $2 RETURNING version",
            &[
                &action.resource_id,
                &expected_version,
                &action.arguments.as_value().to_string(),
            ],
        )?;
        Ok(if row.is_some() {
            ConditionalOutcome::Applied
        } else {
            ConditionalOutcome::StateChanged
        })
    }
}
