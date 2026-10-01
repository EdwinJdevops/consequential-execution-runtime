#![forbid(unsafe_code)]

use cer_core::{EffectContract, RecoveryCapability, RecoverySemantics};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectOutcome {
    ProvenNoEffect,
    ProvenEffect,
    UnknownEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedResource {
    pub resource_id: String,
    pub state_token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedEffectError {
    pub declared: RecoverySemantics,
    pub prepared: RecoverySemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEffect<T> {
    prepared: T,
    contract: EffectContract,
    recovery: RecoveryCapability,
}

impl<T> PreparedEffect<T> {
    pub fn new(
        prepared: T,
        contract: EffectContract,
        recovery: RecoveryCapability,
    ) -> Result<Self, PreparedEffectError> {
        let prepared_semantics = recovery.semantics();

        if prepared_semantics != contract.recovery {
            return Err(PreparedEffectError {
                declared: contract.recovery,
                prepared: prepared_semantics,
            });
        }

        Ok(Self {
            prepared,
            contract,
            recovery,
        })
    }

    pub fn prepared(&self) -> &T {
        &self.prepared
    }

    pub fn contract(&self) -> EffectContract {
        self.contract
    }

    pub fn recovery(&self) -> &RecoveryCapability {
        &self.recovery
    }
}

pub trait ResourceAdapter {
    type Error;
    type Prepared;

    fn resolve(&self, resource_id: &str) -> Result<ResolvedResource, Self::Error>;

    fn prepare(
        &self,
        resource: &ResolvedResource,
    ) -> Result<PreparedEffect<Self::Prepared>, Self::Error>;

    fn execute(&self, prepared: &Self::Prepared) -> Result<EffectOutcome, Self::Error>;

    fn reconcile(&self, prepared: &Self::Prepared) -> Result<EffectOutcome, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cer_core::{MutationKind, OutcomeResolution, RetrySemantics};

    fn contract(recovery: RecoverySemantics) -> EffectContract {
        EffectContract {
            mutation: MutationKind::Mutating,
            retry: RetrySemantics::ReconcileBeforeRetry,
            outcome_resolution: OutcomeResolution::ProviderLookup,
            recovery,
        }
    }

    #[test]
    fn prepared_effect_accepts_matching_recovery_semantics() {
        let prepared = PreparedEffect::new(
            "request",
            contract(RecoverySemantics::CompensatingAction),
            RecoveryCapability::CompensatingAction,
        )
        .unwrap();

        assert_eq!(prepared.prepared(), &"request");
    }

    #[test]
    fn prepared_effect_rejects_recovery_semantics_mismatch() {
        let error = PreparedEffect::new(
            "request",
            contract(RecoverySemantics::Irreversible),
            RecoveryCapability::CompensatingAction,
        )
        .unwrap_err();

        assert_eq!(
            error,
            PreparedEffectError {
                declared: RecoverySemantics::Irreversible,
                prepared: RecoverySemantics::CompensatingAction,
            }
        );
    }
}
