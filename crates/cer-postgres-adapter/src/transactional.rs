//! Atomic execution for PostgreSQL-owned resources only.
//! The operation journal and resource mutation share one database transaction.
use cer_core::{revalidate, ApprovalBinding, ExecutionState, ProposedAction};
use postgres::{Client, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicOutcome {
    Applied,
    StateChanged,
    Rejected,
    AlreadyApplied,
    ExistingFailed,
    IdentityConflict,
}

/// Performs one version-checked PostgreSQL mutation with a durable journal
/// outcome in the SAME transaction. A failed COMMIT is ambiguous to its caller:
/// reconnect and inspect the journal before deciding what happened.
pub fn execute_atomic(
    client: &mut Client,
    approval: &ApprovalBinding,
    action: &ProposedAction,
    fresh_state: &str,
) -> Result<AtomicOutcome, Box<dyn std::error::Error>> {
    if revalidate(ExecutionState::Revalidating, approval, action, fresh_state).is_err() {
        return Ok(AtomicOutcome::Rejected);
    }
    let fingerprint = action
        .fingerprint()?
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut tx = client.transaction()?;
    let inserted = tx.execute(
        "INSERT INTO cer_operation_journal (operation_id, fingerprint, status)
         VALUES ($1, $2, 'PENDING') ON CONFLICT (operation_id) DO NOTHING",
        &[&action.action_id, &fingerprint],
    )?;
    if inserted == 0 {
        let row = tx.query_one(
            "SELECT fingerprint, status FROM cer_operation_journal WHERE operation_id = $1",
            &[&action.action_id],
        )?;
        let existing: String = row.get(0);
        let status: String = row.get(1);
        return Ok(if existing != fingerprint {
            AtomicOutcome::IdentityConflict
        } else if status == "SUCCEEDED" {
            AtomicOutcome::AlreadyApplied
        } else {
            AtomicOutcome::ExistingFailed
        });
    }
    let expected = match fresh_state.parse::<i64>() {
        Ok(version) => version,
        Err(_) => {
            tx.execute(
                "UPDATE cer_operation_journal SET status = 'FAILED'
                 WHERE operation_id = $1",
                &[&action.action_id],
            )?;
            tx.commit()?;
            return Ok(AtomicOutcome::StateChanged);
        }
    };
    let changed = tx.execute(
        "UPDATE cer_resources SET payload = $3::text::jsonb,
         version = version + 1, last_operation_id = $4
         WHERE resource_id = $1 AND version = $2",
        &[
            &action.resource_id,
            &expected,
            &action.arguments.as_value().to_string(),
            &action.action_id,
        ],
    )?;
    let status = if changed == 1 { "SUCCEEDED" } else { "FAILED" };
    tx.execute(
        "UPDATE cer_operation_journal SET status = $2
         WHERE operation_id = $1 AND status = 'PENDING'",
        &[&action.action_id, &status],
    )?;
    tx.commit()?;
    Ok(if changed == 1 {
        AtomicOutcome::Applied
    } else {
        AtomicOutcome::StateChanged
    })
}
