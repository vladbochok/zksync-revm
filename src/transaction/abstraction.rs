//! Optimism transaction abstraction containing the `[OpTxTr]` trait and corresponding `[OpTransaction]` type.
use super::priority_tx::{L1ToL2TransactionParts, UPGRADE_TRANSACTION_TYPE, L1_PRIORITY_TRANSACTION_TYPE};
use auto_impl::auto_impl;
use revm::{
    context::{
        tx::{TxEnvBuildError, TxEnvBuilder},
        TxEnv,
    },
    context_interface::transaction::Transaction,
    handler::SystemCallTx,
    primitives::{Address, Bytes, TxKind, B256, U256},
};
use std::vec;

/// Optimism Transaction trait.
#[auto_impl(&, &mut, Box, Arc)]
pub trait OpTxTr: Transaction {
    /// Mint of the deposit transaction
    fn mint(&self) -> Option<U256>;

    fn is_l1_to_l2_tx(&self) -> bool {
        self.tx_type() == UPGRADE_TRANSACTION_TYPE || self.tx_type() == L1_PRIORITY_TRANSACTION_TYPE
    }
}

/// Optimism transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpTransaction<T: Transaction> {
    /// Base transaction fields.
    pub base: T,
    /// Deposit transaction parts.
    pub deposit: L1ToL2TransactionParts,
}

impl<T: Transaction> AsRef<T> for OpTransaction<T> {
    fn as_ref(&self) -> &T {
        &self.base
    }
}

impl<T: Transaction> OpTransaction<T> {
    /// Create a new Optimism transaction.
    pub fn new(base: T) -> Self {
        Self {
            base,
            deposit: L1ToL2TransactionParts::default(),
        }
    }
}

impl OpTransaction<TxEnv> {
    /// Create a new Optimism transaction.
    pub fn builder() -> OpTransactionBuilder {
        OpTransactionBuilder::new()
    }
}

impl Default for OpTransaction<TxEnv> {
    fn default() -> Self {
        Self {
            base: TxEnv::default(),
            deposit: L1ToL2TransactionParts::default(),
        }
    }
}

impl<TX: Transaction + SystemCallTx> SystemCallTx for OpTransaction<TX> {
    fn new_system_tx_with_caller(
        caller: Address,
        system_contract_address: Address,
        data: Bytes,
    ) -> Self {
        let mut tx = OpTransaction::new(TX::new_system_tx_with_caller(
            caller,
            system_contract_address,
            data,
        ));


        tx
    }
}

impl<T: Transaction> Transaction for OpTransaction<T> {
    type AccessListItem<'a>
        = T::AccessListItem<'a>
    where
        T: 'a;
    type Authorization<'a>
        = T::Authorization<'a>
    where
        T: 'a;

    fn tx_type(&self) -> u8 {
        self.base.tx_type()
    }

    fn caller(&self) -> Address {
        self.base.caller()
    }

    fn gas_limit(&self) -> u64 {
        self.base.gas_limit()
    }

    fn value(&self) -> U256 {
        self.base.value()
    }

    fn input(&self) -> &Bytes {
        self.base.input()
    }

    fn nonce(&self) -> u64 {
        self.base.nonce()
    }

    fn kind(&self) -> TxKind {
        self.base.kind()
    }

    fn chain_id(&self) -> Option<u64> {
        self.base.chain_id()
    }

    fn access_list(&self) -> Option<impl Iterator<Item = Self::AccessListItem<'_>>> {
        self.base.access_list()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.base.max_priority_fee_per_gas()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.base.max_fee_per_gas()
    }

    fn gas_price(&self) -> u128 {
        self.base.gas_price()
    }

    fn blob_versioned_hashes(&self) -> &[B256] {
        self.base.blob_versioned_hashes()
    }

    fn max_fee_per_blob_gas(&self) -> u128 {
        self.base.max_fee_per_blob_gas()
    }

    fn effective_gas_price(&self, base_fee: u128) -> u128 {
        // L1 to L2 transactions use gas_price directly
        if self.is_l1_to_l2_tx() {
            return self.gas_price();
        }
        self.base.effective_gas_price(base_fee)
    }

    fn authorization_list_len(&self) -> usize {
        self.base.authorization_list_len()
    }

    fn authorization_list(&self) -> impl Iterator<Item = Self::Authorization<'_>> {
        self.base.authorization_list()
    }
}

impl<T: Transaction> OpTxTr for OpTransaction<T> {
    fn mint(&self) -> Option<U256> {
        self.deposit.mint
    }
}

/// Builder for constructing [`OpTransaction`] instances
#[derive(Default, Debug)]
pub struct OpTransactionBuilder {
    base: TxEnvBuilder,
    deposit: L1ToL2TransactionParts,
}

impl OpTransactionBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            base: TxEnvBuilder::new(),
            deposit: L1ToL2TransactionParts::default(),
        }
    }

    /// Set the base transaction builder based for TxEnvBuilder.
    pub fn base(mut self, base: TxEnvBuilder) -> Self {
        self.base = base;
        self
    }

    /// Set the mint of the deposit transaction.
    pub fn mint(mut self, mint: U256) -> Self {
        self.deposit.mint = Some(mint);
        self
    }

    /// Build the [`OpTransaction`] with default values for missing fields.
    ///
    /// This is useful for testing and debugging where it is not necessary to
    /// have full [`OpTransaction`] instance.
    ///
    /// If the source hash is not [`B256::ZERO`], set the transaction type to deposit and remove the enveloped transaction.
    pub fn build_fill(mut self) -> OpTransaction<TxEnv> {
        let base = self.base.build_fill();

        OpTransaction {
            base,
            deposit: self.deposit,
        }
    }

    /// Build the [`OpTransaction`] instance, return error if the transaction is not valid.
    ///
    pub fn build(mut self) -> Result<OpTransaction<TxEnv>, OpBuildError> {
        let base = self.base.build()?;

        Ok(OpTransaction {
            base,
            deposit: self.deposit,
        })
    }
}

/// Error type for building [`TxEnv`]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpBuildError {
    /// Base transaction build error
    Base(TxEnvBuildError),
    /// Missing enveloped transaction bytes
    MissingEnvelopedTxBytes,
    /// Missing source hash for deposit transaction
    MissingSourceHashForDeposit,
}

impl From<TxEnvBuildError> for OpBuildError {
    fn from(error: TxEnvBuildError) -> Self {
        OpBuildError::Base(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revm::{
        context_interface::Transaction,
        primitives::{Address, B256},
    };

    #[test]
    fn test_deposit_transaction_fields() {
        let base_tx = TxEnv::builder()
            .gas_limit(10)
            .gas_price(100)
            .gas_priority_fee(Some(5));

        let op_tx = OpTransaction::builder()
            .base(base_tx)
            .mint(0u128)
            .build()
            .unwrap();
        // Verify transaction type (deposit transactions should have tx_type based on OpSpecId)
        // The tx_type is derived from the transaction structure, not set manually
        // Verify common fields access
        assert_eq!(op_tx.gas_limit(), 10);
        assert_eq!(op_tx.kind(), revm::primitives::TxKind::Call(Address::ZERO));
        // Verify gas related calculations - deposit transactions use gas_price for effective gas price
        assert_eq!(op_tx.effective_gas_price(90), 100);
        assert_eq!(op_tx.max_fee_per_gas(), 100);
    }
}
