use crate::{ApprovalBinding, ExecutionState, ProposedAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidState,
    ApprovalMismatch,
    StateChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revalidated {
    fingerprint: crate::ActionFingerprint,
}

pub fn revalidate(
    state: ExecutionState,
    approved: &ApprovalBinding,
    proposed: &ProposedAction,
    fresh_state: &str,
) -> Result<Revalidated, AdmissionError> {
    if state != ExecutionState::Revalidating {
        return Err(AdmissionError::InvalidState);
    }
    if !approved.matches(proposed).map_err(|_| AdmissionError::ApprovalMismatch)? {
        return Err(AdmissionError::ApprovalMismatch);
    }
    if proposed.observed_state != fresh_state {
        return Err(AdmissionError::StateChanged);
    }
    Ok(Revalidated { fingerprint: proposed.fingerprint().map_err(|_| AdmissionError::ApprovalMismatch)? })
}

impl Revalidated {
    pub fn matches_action(&self, action: &ProposedAction) -> bool {
        action.fingerprint().is_ok_and(|fingerprint| fingerprint == self.fingerprint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionArguments;
    use serde_json::json;

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

    #[test]
    fn exact_approval_and_fresh_state_admit() {
        let a = action();
        let approval = ApprovalBinding::bind(&a).unwrap();
        let permit = revalidate(ExecutionState::Revalidating, &approval, &a, "v1").unwrap();
        assert!(permit.matches_action(&a));
    }

    #[test]
    fn stale_state_blocks() {
        let a = action();
        let approval = ApprovalBinding::bind(&a).unwrap();
        assert_eq!(revalidate(ExecutionState::Revalidating, &approval, &a, "v2"), Err(AdmissionError::StateChanged));
    }

    #[test]
    fn substituted_target_blocks() {
        let a = action();
        let approval = ApprovalBinding::bind(&a).unwrap();
        let mut changed = a;
        changed.resource_id = "db/other".into();
        assert_eq!(revalidate(ExecutionState::Revalidating, &approval, &changed, "v1"), Err(AdmissionError::ApprovalMismatch));
    }

    #[test]
    fn unauthorized_state_blocks() {
        let a = action();
        let approval = ApprovalBinding::bind(&a).unwrap();
        assert_eq!(revalidate(ExecutionState::Proposed, &approval, &a, "v1"), Err(AdmissionError::InvalidState));
    }
}
