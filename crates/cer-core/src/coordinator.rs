use crate::{revalidate, AdmissionError, ApprovalBinding, ExecutionState, ProposedAction};

/// The target owns the authoritative state and must atomically compare the
/// observed version with the version used for approval before mutation.
/// An ordinary read followed by a write does not satisfy this contract.
pub trait ConditionalTarget {
    type Error;
    fn compare_and_execute(
        &mut self,
        action: &ProposedAction,
        expected_state: &str,
    ) -> Result<ConditionalOutcome, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalOutcome {
    Applied,
    StateChanged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatedOutcome {
    Completed,
    Rejected(AdmissionError),
    StateChanged,
    ExecutionUnknown,
}

pub fn execute_conditionally<T: ConditionalTarget>(
    target: &mut T,
    state: ExecutionState,
    approval: &ApprovalBinding,
    action: &ProposedAction,
    fresh_state: &str,
) -> CoordinatedOutcome {
    let permit = match revalidate(state, approval, action, fresh_state) {
        Ok(permit) => permit,
        Err(error) => return CoordinatedOutcome::Rejected(error),
    };
    if !permit.matches_action(action) {
        return CoordinatedOutcome::Rejected(AdmissionError::ApprovalMismatch);
    }
    match target.compare_and_execute(action, fresh_state) {
        Ok(ConditionalOutcome::Applied) => CoordinatedOutcome::Completed,
        Ok(ConditionalOutcome::StateChanged) => CoordinatedOutcome::StateChanged,
        Ok(ConditionalOutcome::Unknown) => CoordinatedOutcome::ExecutionUnknown,
        Err(_) => CoordinatedOutcome::ExecutionUnknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionArguments;
    use serde_json::json;

    struct FakeTarget {
        state: String,
        effects: usize,
        ambiguous: bool,
    }

    impl ConditionalTarget for FakeTarget {
        type Error = ();

        fn compare_and_execute(
            &mut self,
            _action: &ProposedAction,
            expected_state: &str,
        ) -> Result<ConditionalOutcome, Self::Error> {
            if self.state != expected_state {
                return Ok(ConditionalOutcome::StateChanged);
            }
            self.effects += 1;
            if self.ambiguous {
                Ok(ConditionalOutcome::Unknown)
            } else {
                Ok(ConditionalOutcome::Applied)
            }
        }
    }

    fn action() -> ProposedAction {
        ProposedAction {
            action_id: "a1".into(),
            resource_id: "db/customer".into(),
            operation: "update".into(),
            arguments: ActionArguments::from_serializable(&json!({"value": 3})).unwrap(),
            observed_state: "v1".into(),
            policy_version: "p1".into(),
        }
    }

    struct FailedTarget;

    impl ConditionalTarget for FailedTarget {
        type Error = ();

        fn compare_and_execute(
            &mut self,
            _action: &ProposedAction,
            _expected_state: &str,
        ) -> Result<ConditionalOutcome, Self::Error> {
            Err(())
        }
    }

    #[test]
    fn adapter_error_is_ambiguous_not_proven_failure() {
        let action = action();
        let approval = ApprovalBinding::bind(&action).unwrap();
        assert_eq!(
            execute_conditionally(
                &mut FailedTarget,
                ExecutionState::Revalidating,
                &approval,
                &action,
                "v1"
            ),
            CoordinatedOutcome::ExecutionUnknown
        );
    }

    #[test]
    fn changed_after_read_blocks_effect() {
        let action = action();
        let approval = ApprovalBinding::bind(&action).unwrap();
        let mut target = FakeTarget {
            state: "v2".into(),
            effects: 0,
            ambiguous: false,
        };
        assert_eq!(
            execute_conditionally(
                &mut target,
                ExecutionState::Revalidating,
                &approval,
                &action,
                "v1"
            ),
            CoordinatedOutcome::StateChanged
        );
        assert_eq!(target.effects, 0);
    }

    #[test]
    fn stale_approval_never_calls_target() {
        let action = action();
        let approval = ApprovalBinding::bind(&action).unwrap();
        let mut target = FakeTarget {
            state: "v1".into(),
            effects: 0,
            ambiguous: false,
        };
        assert_eq!(
            execute_conditionally(
                &mut target,
                ExecutionState::Revalidating,
                &approval,
                &action,
                "v2"
            ),
            CoordinatedOutcome::Rejected(AdmissionError::StateChanged)
        );
        assert_eq!(target.effects, 0);
    }

    #[test]
    fn unknown_effect_is_not_called_failure() {
        let action = action();
        let approval = ApprovalBinding::bind(&action).unwrap();
        let mut target = FakeTarget {
            state: "v1".into(),
            effects: 0,
            ambiguous: true,
        };
        assert_eq!(
            execute_conditionally(
                &mut target,
                ExecutionState::Revalidating,
                &approval,
                &action,
                "v1"
            ),
            CoordinatedOutcome::ExecutionUnknown
        );
        assert_eq!(target.effects, 1);
    }
}
