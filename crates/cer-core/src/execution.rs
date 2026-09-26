#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionState {
    Proposed,
    ResolvingState,
    Resolved,
    Preparing,
    Prepared,
    AwaitingApproval,
    Authorized,
    Revalidating,
    Executing,
    Completed,
    FailedBeforeEffect,
    ExecutionUnknown,
    Reconciling,
    RecoveryRequired,
    ManualIntervention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionError {
    pub from: ExecutionState,
    pub to: ExecutionState,
}

impl ExecutionState {
    pub fn transition(self, to: Self) -> Result<Self, TransitionError> {
        use ExecutionState::*;

        let allowed = matches!(
            (self, to),
            (Proposed, ResolvingState)
                | (ResolvingState, Resolved)
                | (Resolved, Preparing)
                | (Preparing, Prepared)
                | (Prepared, AwaitingApproval)
                | (Prepared, Authorized)
                | (AwaitingApproval, Authorized)
                | (Authorized, Revalidating)
                | (Revalidating, Executing)
                | (Revalidating, AwaitingApproval)
                | (Executing, Completed)
                | (Executing, FailedBeforeEffect)
                | (Executing, ExecutionUnknown)
                | (ExecutionUnknown, Reconciling)
                | (Reconciling, Completed)
                | (Reconciling, RecoveryRequired)
                | (Reconciling, ManualIntervention)
                | (RecoveryRequired, Proposed)
                | (RecoveryRequired, ManualIntervention)
        );

        if allowed {
            Ok(to)
        } else {
            Err(TransitionError { from: self, to })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_execute_directly_from_proposed() {
        assert!(ExecutionState::Proposed
            .transition(ExecutionState::Executing)
            .is_err());
    }

    #[test]
    fn unknown_requires_reconciliation() {
        let state = ExecutionState::Executing
            .transition(ExecutionState::ExecutionUnknown)
            .unwrap();

        assert!(state.transition(ExecutionState::Completed).is_err());
        assert_eq!(
            state.transition(ExecutionState::Reconciling).unwrap(),
            ExecutionState::Reconciling
        );
    }

    #[test]
    fn approval_cannot_skip_revalidation() {
        assert!(ExecutionState::Authorized
            .transition(ExecutionState::Executing)
            .is_err());
    }

    #[test]
    fn awaiting_approval_cannot_execute() {
        assert!(ExecutionState::AwaitingApproval
            .transition(ExecutionState::Executing)
            .is_err());
    }
}
