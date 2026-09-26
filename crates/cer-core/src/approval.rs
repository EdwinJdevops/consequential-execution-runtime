#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ProposedAction { pub action_id:String, pub resource_id:String, pub operation:String, pub canonical_arguments:String, pub observed_state:String, pub policy_version:String }
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ApprovalBinding { action_id:String, resource_id:String, operation:String, canonical_arguments:String, observed_state:String, policy_version:String }
impl ApprovalBinding {
pub fn bind(a:&ProposedAction)->Self{Self{action_id:a.action_id.clone(),resource_id:a.resource_id.clone(),operation:a.operation.clone(),canonical_arguments:a.canonical_arguments.clone(),observed_state:a.observed_state.clone(),policy_version:a.policy_version.clone()}}
pub fn matches(&self,a:&ProposedAction)->bool{self.action_id==a.action_id&&self.resource_id==a.resource_id&&self.operation==a.operation&&self.canonical_arguments==a.canonical_arguments&&self.observed_state==a.observed_state&&self.policy_version==a.policy_version}
}
#[cfg(test)] mod tests {use super::*;fn a()->ProposedAction{ProposedAction{action_id:"a-1".into(),resource_id:"db/customer".into(),operation:"schema-migrate".into(),canonical_arguments:"migration=42".into(),observed_state:"version=10".into(),policy_version:"p1".into()}}
#[test]fn exact(){let x=a();assert!(ApprovalBinding::bind(&x).matches(&x));}
#[test]fn stale(){let x=a();let b=ApprovalBinding::bind(&x);let mut y=x;y.observed_state="version=11".into();assert!(!b.matches(&y));}
#[test]fn target_substitution(){let x=a();let b=ApprovalBinding::bind(&x);let mut y=x;y.resource_id="db/production".into();assert!(!b.matches(&y));}
#[test]fn argument_substitution(){let x=a();let b=ApprovalBinding::bind(&x);let mut y=x;y.canonical_arguments="migration=43".into();assert!(!b.matches(&y));}
#[test]fn policy_change(){let x=a();let b=ApprovalBinding::bind(&x);let mut y=x;y.policy_version="p2".into();assert!(!b.matches(&y));}}
