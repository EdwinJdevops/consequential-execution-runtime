use crate::arguments::ActionArguments;
use crate::identity::{fingerprint, ActionFingerprint, FingerprintError};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProposedAction {
    pub action_id: String,
    pub resource_id: String,
    pub operation: String,
    pub arguments: ActionArguments,
    pub observed_state: String,
    pub policy_version: String,
}

impl ProposedAction {
    pub fn fingerprint(&self) -> Result<ActionFingerprint, FingerprintError> {
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

    fn arguments(value: serde_json::Value) -> ActionArguments {
        ActionArguments::from_serializable(&value).unwrap()
    }

    fn action() -> ProposedAction {
        ProposedAction {
            action_id: "a-1".into(),
            resource_id: "db/customer".into(),
            operation: "schema-migrate".into(),
            arguments: arguments(json!({
                "migration": 42,
                "options": {"lock_timeout": "5s"}
            })),
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
        changed.arguments = arguments(json!({
            "migration": 43,
            "options": {"lock_timeout": "5s"}
        }));

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
    fn json_object_property_order_does_not_change_fingerprint() {
        let mut first = action();
        first.arguments =
            ActionArguments::from_json_str(r#"{"region":"us-east-1","count":2}"#).unwrap();

        let mut second = action();
        second.arguments =
            ActionArguments::from_json_str(r#"{"count":2,"region":"us-east-1"}"#).unwrap();

        assert_eq!(first.fingerprint().unwrap(), second.fingerprint().unwrap());
    }

    #[test]
    fn unicode_is_not_normalized_before_fingerprinting() {
        let mut composed = action();
        composed.arguments = arguments(json!({"name": "\u{00E9}"}));

        let mut decomposed = action();
        decomposed.arguments = arguments(json!({"name": "e\u{0301}"}));

        assert_ne!(
            composed.fingerprint().unwrap(),
            decomposed.fingerprint().unwrap()
        );
    }
}
