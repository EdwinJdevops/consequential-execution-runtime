#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedAction {
    pub action_id: String,
    pub resource_id: String,
    pub operation: String,
    pub canonical_arguments: String,
    pub observed_state: String,
    pub policy_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBinding {
    action_id: String,
    resource_id: String,
    operation: String,
    canonical_arguments: String,
    observed_state: String,
    policy_version: String,
}

impl ApprovalBinding {
    pub fn bind(action: &ProposedAction) -> Self {
        Self {
            action_id: action.action_id.clone(),
            resource_id: action.resource_id.clone(),
            operation: action.operation.clone(),
            canonical_arguments: action.canonical_arguments.clone(),
            observed_state: action.observed_state.clone(),
            policy_version: action.policy_version.clone(),
        }
    }

    pub fn matches(&self, action: &ProposedAction) -> bool {
        self.action_id == action.action_id
            && self.resource_id == action.resource_id
            && self.operation == action.operation
            && self.canonical_arguments == action.canonical_arguments
            && self.observed_state == action.observed_state
            && self.policy_version == action.policy_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action() -> ProposedAction {
        ProposedAction {
            action_id: "a-1".into(),
            resource_id: "db/customer".into(),
            operation: "schema-migrate".into(),
            canonical_arguments: "migration=42".into(),
            observed_state: "version=10".into(),
            policy_version: "p1".into(),
        }
    }

    #[test]
    fn exact_binding_matches() {
        let action = action();
        assert!(ApprovalBinding::bind(&action).matches(&action));
    }

    #[test]
    fn stale_state_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original);
        let mut changed = original;
        changed.observed_state = "version=11".into();
        assert!(!binding.matches(&changed));
    }

    #[test]
    fn target_substitution_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original);
        let mut changed = original;
        changed.resource_id = "db/production".into();
        assert!(!binding.matches(&changed));
    }

    #[test]
    fn argument_substitution_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original);
        let mut changed = original;
        changed.canonical_arguments = "migration=43".into();
        assert!(!binding.matches(&changed));
    }

    #[test]
    fn policy_change_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original);
        let mut changed = original;
        changed.policy_version = "p2".into();
        assert!(!binding.matches(&changed));
    }
}
