use crate::identity::{fingerprint, validate_jcs_value, ActionFingerprint, FingerprintError};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProposedAction {
    pub action_id: String,
    pub resource_id: String,
    pub operation: String,
    pub arguments: Value,
    pub observed_state: String,
    pub policy_version: String,
}

impl ProposedAction {
    pub fn fingerprint(&self) -> Result<ActionFingerprint, FingerprintError> {
        validate_jcs_value(&self.arguments)?;
        fingerprint(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBinding {
    fingerprint: ActionFingerprint,
}

impl ApprovalBinding {
    pub fn bind(action: &ProposedAction) -> Result<Self, FingerprintError> {
        Ok(Self {
            fingerprint: action.fingerprint()?,
        })
    }

    pub fn matches(&self, action: &ProposedAction) -> Result<bool, FingerprintError> {
        Ok(self.fingerprint == action.fingerprint()?)
    }

    pub fn fingerprint(&self) -> ActionFingerprint {
        self.fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn action() -> ProposedAction {
        ProposedAction {
            action_id: "a-1".into(),
            resource_id: "db/customer".into(),
            operation: "schema-migrate".into(),
            arguments: json!({"migration": 42, "options": {"lock_timeout": "5s"}}),
            observed_state: "version=10".into(),
            policy_version: "p1".into(),
        }
    }

    #[test]
    fn exact_binding_matches() {
        let action = action();
        let binding = ApprovalBinding::bind(&action).unwrap();

        assert!(binding.matches(&action).unwrap());
    }

    #[test]
    fn stale_state_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original).unwrap();
        let mut changed = original;
        changed.observed_state = "version=11".into();

        assert!(!binding.matches(&changed).unwrap());
    }

    #[test]
    fn target_substitution_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original).unwrap();
        let mut changed = original;
        changed.resource_id = "db/production".into();

        assert!(!binding.matches(&changed).unwrap());
    }

    #[test]
    fn operation_substitution_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original).unwrap();
        let mut changed = original;
        changed.operation = "schema-drop".into();

        assert!(!binding.matches(&changed).unwrap());
    }

    #[test]
    fn argument_substitution_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original).unwrap();
        let mut changed = original;
        changed.arguments = json!({"migration": 43, "options": {"lock_timeout": "5s"}});

        assert!(!binding.matches(&changed).unwrap());
    }

    #[test]
    fn policy_change_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original).unwrap();
        let mut changed = original;
        changed.policy_version = "p2".into();

        assert!(!binding.matches(&changed).unwrap());
    }

    #[test]
    fn action_instance_change_invalidates_binding() {
        let original = action();
        let binding = ApprovalBinding::bind(&original).unwrap();
        let mut changed = original;
        changed.action_id = "a-2".into();

        assert!(!binding.matches(&changed).unwrap());
    }

    #[test]
    fn unicode_is_not_normalized_before_fingerprinting() {
        let mut composed = action();
        composed.arguments = json!({"name": "\u{00E9}"});

        let mut decomposed = action();
        decomposed.arguments = json!({"name": "e\u{0301}"});

        assert_ne!(
            composed.fingerprint().unwrap(),
            decomposed.fingerprint().unwrap()
        );
    }

    #[test]
    fn invalid_jcs_profile_input_cannot_be_bound() {
        let mut invalid = action();
        invalid.arguments = json!({"id": 9_007_199_254_740_992_u64});

        assert!(matches!(
            ApprovalBinding::bind(&invalid),
            Err(FingerprintError::NonInteroperableInteger(_))
        ));
    }
}
