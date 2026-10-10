//! Journal-gated execution. An existing or unclaimable operation is never retried.
use crate::journal::{admit, claim, mark_unknown, Admission, Claim};
use cer_core::{
    execute_conditionally, ApprovalBinding, CoordinatedOutcome, ExecutionState, ProposedAction,
};
use postgres::{Client, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournaledOutcome {
    Completed,
    StateChanged,
    Rejected,
    Unknown,
    AlreadyExists,
    IdentityConflict,
    NotClaimable,
}

/// Executes at most one attempt for an operation identity. Database errors
/// propagate; callers must not treat those errors as evidence of no effect.
pub fn execute_once(
    journal: &mut Client,
    target: &mut crate::PostgresTarget,
    operation_id: &str,
    fingerprint: &str,
    approval: &ApprovalBinding,
    action: &ProposedAction,
    fresh_state: &str,
) -> Result<JournaledOutcome, Error> {
    match admit(journal, operation_id, fingerprint)? {
        Admission::Existing => return Ok(JournaledOutcome::AlreadyExists),
        Admission::Conflict => return Ok(JournaledOutcome::IdentityConflict),
        Admission::New => {}
    }
    if claim(journal, operation_id, fingerprint)? != Claim::Claimed {
        return Ok(JournaledOutcome::NotClaimable);
    }
    let result = execute_conditionally(
        target,
        ExecutionState::Revalidating,
        approval,
        action,
        fresh_state,
    );
    let (status, outcome) = match result {
        CoordinatedOutcome::Completed => ("SUCCEEDED", JournaledOutcome::Completed),
        CoordinatedOutcome::StateChanged => ("FAILED", JournaledOutcome::StateChanged),
        CoordinatedOutcome::Rejected(_) => ("FAILED", JournaledOutcome::Rejected),
        CoordinatedOutcome::ExecutionUnknown => ("UNKNOWN", JournaledOutcome::Unknown),
    };
    // A journal write error after an external effect leaves the outcome
    // ambiguous. The operation remains non-claimable until reconciliation.
    if status == "UNKNOWN" {
        mark_unknown(journal, operation_id)?;
    } else {
        journal.execute(
            "UPDATE cer_operation_journal SET status = $2 \
             WHERE operation_id = $1 AND status = 'EXECUTING'",
            &[&operation_id, &status],
        )?;
    }
    Ok(outcome)
}
