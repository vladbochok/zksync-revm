//! Contains the `[ZkHaltReason]` type.
use revm::context_interface::result::HaltReason;

/// ZKsync OS halt reason.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ZkHaltReason {
    /// Base halt reason.
    Base(HaltReason),
    /// Failed deposit halt reason.
    FailedDeposit,
}

impl From<HaltReason> for ZkHaltReason {
    fn from(value: HaltReason) -> Self {
        Self::Base(value)
    }
}

impl TryFrom<ZkHaltReason> for HaltReason {
    type Error = ZkHaltReason;

    fn try_from(value: ZkHaltReason) -> Result<HaltReason, ZkHaltReason> {
        match value {
            ZkHaltReason::Base(reason) => Ok(reason),
            ZkHaltReason::FailedDeposit => Err(value),
        }
    }
}
