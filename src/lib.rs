//! ZKsync OS specific constants, types, and helpers.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc as std;

pub mod api;
pub mod constants;
pub mod evm;
pub mod handler;
pub mod precompiles;
pub mod result;
pub mod spec;
pub mod transaction;

pub use api::{
    builder::ZkBuilder,
    default_ctx::{DefaultZk, ZkContext},
};
pub use evm::ZKsyncEvm;
pub use result::ZkHaltReason;
pub use spec::*;
pub use transaction::{ZKsyncTx, error::ZKsyncTxError};
