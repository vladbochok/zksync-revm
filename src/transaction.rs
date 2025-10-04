//! Contains the `[ZKsyncTx]` type and its implementation.
pub mod abstraction;
pub mod error;
pub mod priority_tx;

pub use abstraction::{OpTxTr, ZKsyncTx};
pub use error::ZKsyncTxError;
