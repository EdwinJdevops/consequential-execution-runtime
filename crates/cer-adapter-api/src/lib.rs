#![forbid(unsafe_code)]

use cer_core::RecoveryCapability;

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

pub trait ResourceAdapter {
    type Error;
    type Prepared;

    fn resolve(&self, resource_id: &str) -> Result<ResolvedResource, Self::Error>;

    fn prepare(
        &self,
        resource: &ResolvedResource,
    ) -> Result<(Self::Prepared, RecoveryCapability), Self::Error>;

    fn execute(&self, prepared: &Self::Prepared) -> Result<EffectOutcome, Self::Error>;

    fn reconcile(&self, prepared: &Self::Prepared) -> Result<EffectOutcome, Self::Error>;
}
