//! Contains the `[OpSpecId]` type and its implementation.
use core::str::FromStr;
use revm::primitives::hardfork::{SpecId, UnknownHardfork};

/// Optimism spec id.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(non_camel_case_types)]
pub enum OpSpecId {
    #[default]
    Initial,
}

impl OpSpecId {
    /// Converts the [`OpSpecId`] into a [`SpecId`].
    pub const fn into_eth_spec(self) -> SpecId {
        match self {
            Self::Initial => SpecId::PRAGUE, // TODO: Should it be CANCUN?
        }
    }

    /// Checks if the [`OpSpecId`] is enabled in the other [`OpSpecId`].
    pub const fn is_enabled_in(self, other: OpSpecId) -> bool {
        other as u8 <= self as u8
    }
}

impl From<OpSpecId> for SpecId {
    fn from(spec: OpSpecId) -> Self {
        spec.into_eth_spec()
    }
}

impl FromStr for OpSpecId {
    type Err = UnknownHardfork;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            name::Initial => Ok(OpSpecId::Initial),
            _ => Err(UnknownHardfork),
        }
    }
}

impl From<OpSpecId> for &'static str {
    fn from(spec_id: OpSpecId) -> Self {
        match spec_id {
            OpSpecId::Initial => name::Initial,
        }
    }
}

/// String identifiers for ZKsync OS hardforks
pub mod name {
    /// Initial spec name.
    pub const Initial: &str = "Initial";
}
