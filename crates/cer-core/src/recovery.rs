use crate::effect::RecoverySemantics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryCapability {
    DirectReversal,
    CompensatingAction,
    SnapshotRestore { evidence_id: String },
    Reconstruction { evidence_id: String },
    ExternalCompensation,
    NoRecovery,
}

impl RecoveryCapability {
    pub fn requires_prepared_evidence(&self) -> bool {
        matches!(
            self,
            Self::SnapshotRestore { .. } | Self::Reconstruction { .. }
        )
    }

    pub fn semantics(&self) -> RecoverySemantics {
        match self {
            Self::DirectReversal => RecoverySemantics::DirectReversal,
            Self::CompensatingAction => RecoverySemantics::CompensatingAction,
            Self::SnapshotRestore { .. } => RecoverySemantics::SnapshotRestore,
            Self::Reconstruction { .. } => RecoverySemantics::Reconstruction,
            Self::ExternalCompensation => RecoverySemantics::ExternalCompensation,
            Self::NoRecovery => RecoverySemantics::Irreversible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_recovery_is_explicit() {
        assert!(!RecoveryCapability::NoRecovery.requires_prepared_evidence());
        assert_eq!(
            RecoveryCapability::NoRecovery.semantics(),
            RecoverySemantics::Irreversible
        );
    }

    #[test]
    fn snapshot_has_evidence() {
        assert!(RecoveryCapability::SnapshotRestore {
            evidence_id: "snapshot-123".into()
        }
        .requires_prepared_evidence());
    }
}
