//! Contains the `[ZKsyncTx]` type and its implementation.
pub mod abstraction;
pub mod priority_tx;
pub mod error;

pub use abstraction::{ZKsyncTx, OpTxTr};
pub use error::ZKsyncTxError;
