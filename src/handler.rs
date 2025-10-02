//!Handler related to Optimism chain
use crate::{
    api::exec::OpContextTr,
    constants::{BASE_FEE_RECIPIENT, L1_FEE_RECIPIENT, OPERATOR_FEE_RECIPIENT},
    transaction::{priority_tx::{UPGRADE_TRANSACTION_TYPE, L1_PRIORITY_TRANSACTION_TYPE}, ZKsyncTxError, OpTxTr},
    OpHaltReason, OpSpecId,
};
use revm::{
    context::{result::InvalidTransaction, LocalContextTr},
    context_interface::{
        context::ContextError,
        result::{EVMError, ExecutionResult, FromStringError},
        Block, Cfg, ContextTr, JournalTr, Transaction,
    },
    handler::{
        evm::FrameTr,
        handler::EvmTrError,
        post_execution::{self, reimburse_caller},
        pre_execution::validate_account_nonce_and_code,
        EthFrame, EvmTr, FrameResult, Handler, MainnetHandler,
    },
    inspector::{Inspector, InspectorEvmTr, InspectorHandler},
    interpreter::{interpreter::EthInterpreter, interpreter_action::FrameInit, Gas},
    primitives::{hardfork::SpecId, U256},
};
use std::boxed::Box;

/// Optimism handler extends the [`Handler`] with Optimism specific logic.
#[derive(Debug, Clone)]
pub struct ZKsyncHandler<EVM, ERROR, FRAME> {
    /// Mainnet handler allows us to use functions from the mainnet handler inside optimism handler.
    /// So we dont duplicate the logic
    pub mainnet: MainnetHandler<EVM, ERROR, FRAME>,
    /// Phantom data to avoid type inference issues.
    pub _phantom: core::marker::PhantomData<(EVM, ERROR, FRAME)>,
}

impl<EVM, ERROR, FRAME> ZKsyncHandler<EVM, ERROR, FRAME> {
    /// Create a new Optimism handler.
    pub fn new() -> Self {
        Self {
            mainnet: MainnetHandler::default(),
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<EVM, ERROR, FRAME> Default for ZKsyncHandler<EVM, ERROR, FRAME> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait to check if the error is a transaction error.
///
/// Used in cache_error handler to catch deposit transaction that was halted.
pub trait IsTxError {
    /// Check if the error is a transaction error.
    fn is_tx_error(&self) -> bool;
}

impl<DB, TX> IsTxError for EVMError<DB, TX> {
    fn is_tx_error(&self) -> bool {
        matches!(self, EVMError::Transaction(_))
    }
}

impl<EVM, ERROR, FRAME> Handler for ZKsyncHandler<EVM, ERROR, FRAME>
where
    EVM: EvmTr<Context: OpContextTr, Frame = FRAME>,
    ERROR: EvmTrError<EVM> + From<ZKsyncTxError> + FromStringError + IsTxError,
    // TODO `FrameResult` should be a generic trait.
    // TODO `FrameInit` should be a generic.
    FRAME: FrameTr<FrameResult = FrameResult, FrameInit = FrameInit>,
{
    type Evm = EVM;
    type Error = ERROR;
    type HaltReason = OpHaltReason;

    fn validate_env(&self, evm: &mut Self::Evm) -> Result<(), Self::Error> {
        // Do not perform any extra validation for deposit transactions, they are pre-verified on L1.
        let ctx = evm.ctx();
        let tx = ctx.tx();
        let tx_type = tx.tx_type();
        self.mainnet.validate_env(evm)
    }

    fn validate_against_state_and_deduct_caller(
        &self,
        evm: &mut Self::Evm,
    ) -> Result<(), Self::Error> {
        let ctx = evm.ctx();

        let basefee = ctx.block().basefee() as u128;
        let blob_price = ctx.block().blob_gasprice().unwrap_or_default();
        let is_l1_to_l2_tx = ctx.tx().is_l1_to_l2_tx();
        let spec = ctx.cfg().spec();
        let block_number = ctx.block().number();
        let is_eip3607_disabled = ctx.cfg().is_eip3607_disabled();
        let is_nonce_check_disabled = ctx.cfg().is_nonce_check_disabled();

        let mint = ctx.tx().mint().unwrap_or_default();

        let (tx, journal) = ctx.tx_journal_mut();

        let caller_account = journal.load_account_code(tx.caller())?.data;

        if !is_l1_to_l2_tx {
            // validates account nonce and code
            validate_account_nonce_and_code(
                &mut caller_account.info,
                tx.nonce(),
                is_eip3607_disabled,
                is_nonce_check_disabled,
            )?;
        }

        // old balance is journaled before mint is incremented.
        let old_balance = caller_account.info.balance;

        let mut new_balance = caller_account.info.balance.saturating_add(U256::from(mint)).max(tx.value());

        // Touch account so we know it is changed.
        caller_account.mark_touch();
        caller_account.info.balance = new_balance;

        // Bump the nonce for calls. Nonce for CREATE will be bumped in `handle_create`.
        if !is_l1_to_l2_tx && tx.kind().is_call() {
            caller_account.info.nonce = caller_account.info.nonce.saturating_add(1);
        }

        // NOTE: all changes to the caller account should journaled so in case of error
        // we can revert the changes.
        journal.caller_accounting_journal_entry(tx.caller(), old_balance, tx.kind().is_call());

        Ok(())
    }

    fn last_frame_result(
        &mut self,
        evm: &mut Self::Evm,
        frame_result: &mut <<Self::Evm as EvmTr>::Frame as FrameTr>::FrameResult,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reimburse_caller(
        &self,
        evm: &mut Self::Evm,
        frame_result: &mut <<Self::Evm as EvmTr>::Frame as FrameTr>::FrameResult,
    ) -> Result<(), Self::Error> {
        reimburse_caller(evm.ctx(), frame_result.gas(), U256::ZERO).map_err(From::from)
    }

    fn refund(
        &self,
        evm: &mut Self::Evm,
        frame_result: &mut <<Self::Evm as EvmTr>::Frame as FrameTr>::FrameResult,
        eip7702_refund: i64,
    ) {
        frame_result.gas_mut().record_refund(eip7702_refund);

        let is_l1_to_l2_tx = evm.ctx().tx().is_l1_to_l2_tx();

        // Prior to Regolith, deposit transactions did not receive gas refunds.
        let is_gas_refund_disabled = is_l1_to_l2_tx;
        if !is_gas_refund_disabled {
            frame_result.gas_mut().set_final_refund(
                evm.ctx()
                    .cfg()
                    .spec()
                    .into_eth_spec()
                    .is_enabled_in(SpecId::LONDON),
            );
        }
    }

    fn reward_beneficiary(
        &self,
        evm: &mut Self::Evm,
        frame_result: &mut <<Self::Evm as EvmTr>::Frame as FrameTr>::FrameResult,
    ) -> Result<(), Self::Error> {
        let is_deposit = evm.ctx().tx().is_l1_to_l2_tx();

        // Transfer fee to coinbase/beneficiary.
        if is_deposit {
            return Ok(());
        }

        self.mainnet.reward_beneficiary(evm, frame_result)?;
        let basefee = evm.ctx().block().basefee() as u128;

        // If the transaction is not a deposit transaction, fees are paid out
        // to both the Base Fee Vault as well as the L1 Fee Vault.
        let ctx = evm.ctx();
        let spec = ctx.cfg().spec();

        Ok(())
    }

    fn execution_result(
        &mut self,
        evm: &mut Self::Evm,
        frame_result: <<Self::Evm as EvmTr>::Frame as FrameTr>::FrameResult,
    ) -> Result<ExecutionResult<Self::HaltReason>, Self::Error> {
        match core::mem::replace(evm.ctx().error(), Ok(())) {
            Err(ContextError::Db(e)) => return Err(e.into()),
            Err(ContextError::Custom(e)) => return Err(Self::Error::from_string(e)),
            Ok(_) => (),
        }

        let exec_result =
            post_execution::output(evm.ctx(), frame_result).map_haltreason(OpHaltReason::Base);

        evm.ctx().journal_mut().commit_tx();
        evm.ctx().local_mut().clear();
        evm.frame_stack().clear();

        Ok(exec_result)
    }

    fn catch_error(
        &self,
        evm: &mut Self::Evm,
        error: Self::Error,
    ) -> Result<ExecutionResult<Self::HaltReason>, Self::Error> {
        let is_deposit = evm.ctx().tx().is_l1_to_l2_tx();
        let output = if error.is_tx_error() && is_deposit {
            let ctx = evm.ctx();
            // let spec = ctx.cfg().spec();
            let tx = ctx.tx();
            let caller = tx.caller();
            let mint = tx.mint();
            let gas_limit = tx.gas_limit();

            // discard all changes of this transaction
            evm.ctx().journal_mut().discard_tx();

            // If the transaction is a deposit transaction and it failed
            // for any reason, the caller nonce must be bumped, and the
            // gas reported must be altered depending on the Hardfork. This is
            // also returned as a special Halt variant so that consumers can more
            // easily distinguish between a failed deposit and a failed
            // normal transaction.

            // Increment sender nonce and account balance for the mint amount. Deposits
            // always persist the mint amount, even if the transaction fails.
            let acc: &mut revm::state::Account = evm.ctx().journal_mut().load_account(caller)?.data;

            let old_balance = acc.info.balance;

            // decrement transaction id as it was incremented when we discarded the tx.
            acc.transaction_id -= 1;
            acc.info.nonce = acc.info.nonce.saturating_add(1);
            acc.info.balance = acc
                .info
                .balance
                .saturating_add(U256::from(mint.unwrap_or_default()));
            acc.mark_touch();

            // add journal entry for accounts
            evm.ctx()
                .journal_mut()
                .caller_accounting_journal_entry(caller, old_balance, true);

            // The gas used of a failed deposit post-regolith is the gas
            // limit of the transaction. pre-regolith, it is the gas limit
            // of the transaction for non system transactions and 0 for system
            // transactions.
            let gas_used = gas_limit;
            // clear the journal
            Ok(ExecutionResult::Halt {
                reason: OpHaltReason::FailedDeposit,
                gas_used,
            })
        } else {
            Err(error)
        };
        // do the cleanup
        evm.ctx().local_mut().clear();
        evm.frame_stack().clear();

        output
    }
}

impl<EVM, ERROR> InspectorHandler for ZKsyncHandler<EVM, ERROR, EthFrame<EthInterpreter>>
where
    EVM: InspectorEvmTr<
        Context: OpContextTr,
        Frame = EthFrame<EthInterpreter>,
        Inspector: Inspector<<<Self as Handler>::Evm as EvmTr>::Context, EthInterpreter>,
    >,
    ERROR: EvmTrError<EVM> + From<ZKsyncTxError> + FromStringError + IsTxError,
{
    type IT = EthInterpreter;
}
