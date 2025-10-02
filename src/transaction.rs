//! Contains the `[OpTransaction]` type and its implementation.
pub mod abstraction;
pub mod priority_tx;
pub mod error;

pub use abstraction::{OpTransaction, OpTxTr};
pub use error::OpTransactionError;
