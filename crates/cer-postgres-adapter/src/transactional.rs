//! Atomic execution for PostgreSQL-owned resources only.
//! The operation journal and resource mutation share one database transaction.
use cer_core::{revalidate, ApprovalBinding, ExecutionState, ProposedAction};
use postgres::Client;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicOutcome {
    Applied,
    StateChanged,
    Rejected,
    AlreadyApplied,
    ExistingFailed,
    ExistingUnresolved,
    IdentityConflict,
    UnsupportedOperation,
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
    // Approval identity must match before any durable lookup or mutation.
    if !approval.matches(action)? {
        return Ok(AtomicOutcome::Rejected);
    }
    // The PostgreSQL adapter supports exactly one mutation semantic.
    // Never interpret arbitrary action operation labels as a resource update.
    if action.operation != "update" {
        return Ok(AtomicOutcome::UnsupportedOperation);
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
        } else if status == "FAILED" {
            AtomicOutcome::ExistingFailed
        } else {
            AtomicOutcome::ExistingUnresolved
        });
    }
    // Existing operation identities are resolved above, even when the original
    // observation is now stale after a successful commit.
    if revalidate(ExecutionState::Revalidating, approval, action, fresh_state).is_err() {
        return Ok(AtomicOutcome::Rejected);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableOutcome {
    Absent,
    Succeeded,
    Failed,
    Unresolved,
    IdentityConflict,
}

/// Authoritative recovery read after a disconnected or uncertain COMMIT.
/// Never interprets a missing row as proof that no external effect occurred.
pub fn inspect_durable_outcome(
    client: &mut Client,
    action: &ProposedAction,
) -> Result<DurableOutcome, Box<dyn std::error::Error>> {
    let fingerprint = action
        .fingerprint()?
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let row = client.query_opt(
        "SELECT fingerprint, status FROM cer_operation_journal WHERE operation_id = $1",
        &[&action.action_id],
    )?;
    let Some(row) = row else {
        return Ok(DurableOutcome::Absent);
    };
    let stored: String = row.get(0);
    let status: String = row.get(1);
    Ok(if stored != fingerprint {
        DurableOutcome::IdentityConflict
    } else {
        match status.as_str() {
            "SUCCEEDED" => DurableOutcome::Succeeded,
            "FAILED" => DurableOutcome::Failed,
            _ => DurableOutcome::Unresolved,
        }
    })
}
