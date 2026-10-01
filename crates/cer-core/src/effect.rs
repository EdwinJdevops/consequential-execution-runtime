#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationKind {
    ReadOnly,
    Mutating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrySemantics {
    RepeatableAfterUnknown,
    StableIdempotencyKeyRequired,
    ReconcileBeforeRetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeResolution {
    ProviderLookup,
    PostconditionVerification,
    ProviderLookupOrPostcondition,
    Unavailable,
}

impl OutcomeResolution {
    pub fn is_available(self) -> bool {
        !matches!(self, Self::Unavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverySemantics {
    DirectReversal,
    CompensatingAction,
    SnapshotRestore,
    Reconstruction,
    ExternalCompensation,
    Irreversible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectContract {
    pub mutation: MutationKind,
    pub retry: RetrySemantics,
    pub outcome_resolution: OutcomeResolution,
    pub recovery: RecoverySemantics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryContext {
    pub same_canonical_request: bool,
    pub same_idempotency_key: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    Allowed,
    RequireReconciliation,
    Denied,
}

impl EffectContract {
    pub fn retry_after_unknown(self, context: RetryContext) -> RetryDecision {
        if !context.same_canonical_request {
            return RetryDecision::Denied;
        }

        match self.retry {
            RetrySemantics::RepeatableAfterUnknown => RetryDecision::Allowed,
            RetrySemantics::StableIdempotencyKeyRequired => {
                if context.same_idempotency_key {
                    RetryDecision::Allowed
                } else {
                    RetryDecision::Denied
                }
            }
            RetrySemantics::ReconcileBeforeRetry => {
                if self.outcome_resolution.is_available() {
                    RetryDecision::RequireReconciliation
                } else {
                    RetryDecision::Denied
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(retry: RetrySemantics, outcome_resolution: OutcomeResolution) -> EffectContract {
        EffectContract {
            mutation: MutationKind::Mutating,
            retry,
            outcome_resolution,
            recovery: RecoverySemantics::CompensatingAction,
        }
    }

    #[test]
    fn repeatable_unknown_requires_identical_request() {
        let contract = contract(
            RetrySemantics::RepeatableAfterUnknown,
            OutcomeResolution::Unavailable,
        );

        assert_eq!(
            contract.retry_after_unknown(RetryContext {
                same_canonical_request: false,
                same_idempotency_key: false,
            }),
            RetryDecision::Denied
        );
    }

    #[test]
    fn repeatable_unknown_allows_identical_request() {
        let contract = contract(
            RetrySemantics::RepeatableAfterUnknown,
            OutcomeResolution::Unavailable,
        );

        assert_eq!(
            contract.retry_after_unknown(RetryContext {
                same_canonical_request: true,
                same_idempotency_key: false,
            }),
            RetryDecision::Allowed
        );
    }

    #[test]
    fn idempotency_key_must_be_reused() {
        let contract = contract(
            RetrySemantics::StableIdempotencyKeyRequired,
            OutcomeResolution::ProviderLookup,
        );

        assert_eq!(
            contract.retry_after_unknown(RetryContext {
                same_canonical_request: true,
                same_idempotency_key: false,
            }),
            RetryDecision::Denied
        );

        assert_eq!(
            contract.retry_after_unknown(RetryContext {
                same_canonical_request: true,
                same_idempotency_key: true,
            }),
            RetryDecision::Allowed
        );
    }

    #[test]
    fn unsafe_unknown_requires_reconciliation_when_available() {
        let contract = contract(
            RetrySemantics::ReconcileBeforeRetry,
            OutcomeResolution::PostconditionVerification,
        );

        assert_eq!(
            contract.retry_after_unknown(RetryContext {
                same_canonical_request: true,
                same_idempotency_key: false,
            }),
            RetryDecision::RequireReconciliation
        );
    }

    #[test]
    fn unreconcilable_unknown_denies_retry() {
        let contract = contract(
            RetrySemantics::ReconcileBeforeRetry,
            OutcomeResolution::Unavailable,
        );

        assert_eq!(
            contract.retry_after_unknown(RetryContext {
                same_canonical_request: true,
                same_idempotency_key: false,
            }),
            RetryDecision::Denied
        );
    }
}
