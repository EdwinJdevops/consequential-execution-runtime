#![forbid(unsafe_code)]

pub mod approval;
pub mod arguments;
pub mod effect;
pub mod execution;
pub mod identity;
pub mod recovery;

pub use approval::{ApprovalBinding, ProposedAction};
pub use arguments::{ActionArguments, ArgumentError};
pub use effect::{
    EffectContract, MutationKind, OutcomeResolution, RecoverySemantics, RetryContext,
    RetryDecision, RetrySemantics,
};
pub use execution::{ExecutionState, TransitionError};
pub use identity::{ActionFingerprint, FingerprintError};
pub use recovery::RecoveryCapability;
