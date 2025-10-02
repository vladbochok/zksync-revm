//! Contains the `[L1BlockInfo]` type and its implementation.
use crate::{
    constants::{
        BASE_FEE_SCALAR_OFFSET, BLOB_BASE_FEE_SCALAR_OFFSET, ECOTONE_L1_BLOB_BASE_FEE_SLOT,
        ECOTONE_L1_FEE_SCALARS_SLOT, EMPTY_SCALARS, L1_BASE_FEE_SLOT, L1_BLOCK_CONTRACT,
        L1_OVERHEAD_SLOT, L1_SCALAR_SLOT, NON_ZERO_BYTE_COST, OPERATOR_FEE_CONSTANT_OFFSET,
        OPERATOR_FEE_SCALARS_SLOT, OPERATOR_FEE_SCALAR_DECIMAL, OPERATOR_FEE_SCALAR_OFFSET,
    },
    OpSpecId,
};
use revm::{
    database_interface::Database,
    interpreter::{
        gas::{get_tokens_in_calldata, NON_ZERO_BYTE_MULTIPLIER_ISTANBUL, STANDARD_TOKEN_COST},
        Gas,
    },
    primitives::{hardfork::SpecId, U256},
};

/// L1 block info
///
/// We can extract L1 epoch data from each L2 block, by looking at the `setL1BlockValues`
/// transaction data. This data is then used to calculate the L1 cost of a transaction.
///
/// Here is the format of the `setL1BlockValues` transaction data:
///
/// setL1BlockValues(uint64 _number, uint64 _timestamp, uint256 _basefee, bytes32 _hash,
/// uint64 _sequenceNumber, bytes32 _batcherHash, uint256 _l1FeeOverhead, uint256 _l1FeeScalar)
///
/// For now, we only care about the fields necessary for L1 cost calculation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct L1BlockInfo {
    /// The L2 block number. If not same as the one in the context,
    /// L1BlockInfo is not valid and will be reloaded from the database.
    pub l2_block: U256,
    /// The base fee of the L1 origin block.
    pub l1_base_fee: U256,
    /// The current L1 fee overhead. None if Ecotone is activated.
    pub l1_fee_overhead: Option<U256>,
    /// The current L1 fee scalar.
    pub l1_base_fee_scalar: U256,
    /// The current L1 blob base fee. None if Ecotone is not activated, except if `empty_ecotone_scalars` is `true`.
    pub l1_blob_base_fee: Option<U256>,
    /// The current L1 blob base fee scalar. None if Ecotone is not activated.
    pub l1_blob_base_fee_scalar: Option<U256>,
    /// The current L1 blob base fee. None if Isthmus is not activated, except if `empty_ecotone_scalars` is `true`.
    pub operator_fee_scalar: Option<U256>,
    /// The current L1 blob base fee scalar. None if Isthmus is not activated.
    pub operator_fee_constant: Option<U256>,
    /// True if Ecotone is activated, but the L1 fee scalars have not yet been set.
    pub(crate) empty_ecotone_scalars: bool,
    /// Last calculated l1 fee cost. Uses as a cache between validation and pre execution stages.
    pub tx_l1_cost: Option<U256>,
}

impl L1BlockInfo {
    /// Try to fetch the L1 block info from the database.
    pub fn try_fetch<DB: Database>(
        db: &mut DB,
        l2_block: U256,
        spec_id: OpSpecId,
    ) -> Result<L1BlockInfo, DB::Error> {
        // Ensure the L1 Block account is loaded into the cache after Ecotone. With EIP-4788, it is no longer the case
        // that the L1 block account is loaded into the cache prior to the first inquiry for the L1 block info.
        if spec_id.into_eth_spec().is_enabled_in(SpecId::CANCUN) {
            let _ = db.basic(L1_BLOCK_CONTRACT)?;
        }

        let l1_base_fee = db.storage(L1_BLOCK_CONTRACT, L1_BASE_FEE_SLOT)?;
        // TODO
        if true {
            let l1_fee_overhead = db.storage(L1_BLOCK_CONTRACT, L1_OVERHEAD_SLOT)?;
            let l1_fee_scalar = db.storage(L1_BLOCK_CONTRACT, L1_SCALAR_SLOT)?;

            Ok(L1BlockInfo {
                l1_base_fee,
                l1_fee_overhead: Some(l1_fee_overhead),
                l1_base_fee_scalar: l1_fee_scalar,
                ..Default::default()
            })
        } else {
            let l1_blob_base_fee = db.storage(L1_BLOCK_CONTRACT, ECOTONE_L1_BLOB_BASE_FEE_SLOT)?;
            let l1_fee_scalars = db
                .storage(L1_BLOCK_CONTRACT, ECOTONE_L1_FEE_SCALARS_SLOT)?
                .to_be_bytes::<32>();

            let l1_base_fee_scalar = U256::from_be_slice(
                l1_fee_scalars[BASE_FEE_SCALAR_OFFSET..BASE_FEE_SCALAR_OFFSET + 4].as_ref(),
            );
            let l1_blob_base_fee_scalar = U256::from_be_slice(
                l1_fee_scalars[BLOB_BASE_FEE_SCALAR_OFFSET..BLOB_BASE_FEE_SCALAR_OFFSET + 4]
                    .as_ref(),
            );

            // Check if the L1 fee scalars are empty. If so, we use the Bedrock cost function.
            // The L1 fee overhead is only necessary if `empty_ecotone_scalars` is true, as it was deprecated in Ecotone.
            let empty_ecotone_scalars = l1_blob_base_fee.is_zero()
                && l1_fee_scalars[BASE_FEE_SCALAR_OFFSET..BLOB_BASE_FEE_SCALAR_OFFSET + 4]
                    == EMPTY_SCALARS;
            let l1_fee_overhead = empty_ecotone_scalars
                .then(|| db.storage(L1_BLOCK_CONTRACT, L1_OVERHEAD_SLOT))
                .transpose()?;

            if true {
                let operator_fee_scalars = db
                    .storage(L1_BLOCK_CONTRACT, OPERATOR_FEE_SCALARS_SLOT)?
                    .to_be_bytes::<32>();

                // Post-isthmus L1 block info
                // The `operator_fee_scalar` is stored as a big endian u32 at
                // OPERATOR_FEE_SCALAR_OFFSET.
                let operator_fee_scalar = U256::from_be_slice(
                    operator_fee_scalars
                        [OPERATOR_FEE_SCALAR_OFFSET..OPERATOR_FEE_SCALAR_OFFSET + 4]
                        .as_ref(),
                );
                // The `operator_fee_constant` is stored as a big endian u64 at
                // OPERATOR_FEE_CONSTANT_OFFSET.
                let operator_fee_constant = U256::from_be_slice(
                    operator_fee_scalars
                        [OPERATOR_FEE_CONSTANT_OFFSET..OPERATOR_FEE_CONSTANT_OFFSET + 8]
                        .as_ref(),
                );
                Ok(L1BlockInfo {
                    l2_block,
                    l1_base_fee,
                    l1_base_fee_scalar,
                    l1_blob_base_fee: Some(l1_blob_base_fee),
                    l1_blob_base_fee_scalar: Some(l1_blob_base_fee_scalar),
                    empty_ecotone_scalars,
                    l1_fee_overhead,
                    operator_fee_scalar: Some(operator_fee_scalar),
                    operator_fee_constant: Some(operator_fee_constant),
                    tx_l1_cost: None,
                })
            } else {
                // Pre-isthmus L1 block info
                Ok(L1BlockInfo {
                    l1_base_fee,
                    l1_base_fee_scalar,
                    l1_blob_base_fee: Some(l1_blob_base_fee),
                    l1_blob_base_fee_scalar: Some(l1_blob_base_fee_scalar),
                    empty_ecotone_scalars,
                    l1_fee_overhead,
                    ..Default::default()
                })
            }
        }
    }

    /// Calculate the operator fee for executing this transaction.
    ///
    /// Introduced in isthmus. Prior to isthmus, the operator fee is always zero.
    pub fn operator_fee_charge(&self, input: &[u8], gas_limit: U256) -> U256 {
        // If the input is a deposit transaction or empty, the default value is zero.
        if input.first() == Some(&0x7E) {
            return U256::ZERO;
        }

        self.operator_fee_charge_inner(gas_limit)
    }

    /// Calculate the operator fee for the given `gas`.
    fn operator_fee_charge_inner(&self, gas: U256) -> U256 {
        U256::ZERO
    }

    /// Calculate the operator fee for executing this transaction.
    ///
    /// Introduced in isthmus. Prior to isthmus, the operator fee is always zero.
    pub fn operator_fee_refund(&self, gas: &Gas, spec_id: OpSpecId) -> U256 {
        // if !spec_id.is_enabled_in(OpSpecId::ISTHMUS) {
        //     return U256::ZERO;
        // }

        let operator_cost_gas_limit = self.operator_fee_charge_inner(U256::from(gas.limit()));
        let operator_cost_gas_used = self.operator_fee_charge_inner(U256::from(
            gas.limit() - (gas.remaining() + gas.refunded() as u64),
        ));

        operator_cost_gas_limit.saturating_sub(operator_cost_gas_used)
    }

    /// Calculate the data gas for posting the transaction on L1. Calldata costs 16 gas per byte
    /// after compression.
    ///
    /// Prior to fjord, calldata costs 16 gas per non-zero byte and 4 gas per zero byte.
    ///
    /// Prior to regolith, an extra 68 non-zero bytes were included in the rollup data costs to
    /// account for the empty signature.
    pub fn data_gas(&self, input: &[u8], spec_id: OpSpecId) -> U256 {
        // if spec_id.is_enabled_in(OpSpecId::FJORD) {
        //     let estimated_size = U256::ZERO;

        //     return estimated_size
        //         .saturating_mul(U256::from(NON_ZERO_BYTE_COST))
        //         .wrapping_div(U256::from(1_000_000));
        // };

        // tokens in calldata where non-zero bytes are priced 4 times higher than zero bytes (Same as in Istanbul).
        let mut tokens_in_transaction_data = get_tokens_in_calldata(input, true);

        // Prior to regolith, an extra 68 non zero bytes were included in the rollup data costs.
        // if !spec_id.is_enabled_in(OpSpecId::REGOLITH) {
        //     tokens_in_transaction_data += 68 * NON_ZERO_BYTE_MULTIPLIER_ISTANBUL;
        // }

        U256::from(tokens_in_transaction_data.saturating_mul(STANDARD_TOKEN_COST))
    }

    /// Clears the cached L1 cost of the transaction.
    pub fn clear_tx_l1_cost(&mut self) {
        self.tx_l1_cost = None;
    }

    /// Calculate the gas cost of a transaction based on L1 block data posted on L2, depending on the [OpSpecId] passed.
    pub fn calculate_tx_l1_cost(&mut self, input: &[u8], spec_id: OpSpecId) -> U256 {
        if let Some(tx_l1_cost) = self.tx_l1_cost {
            return tx_l1_cost;
        }
        U256::ZERO
    }

    /// Calculate the gas cost of a transaction based on L1 block data posted on L2, pre-Ecotone.
    fn calculate_tx_l1_cost_bedrock(&self, input: &[u8], spec_id: OpSpecId) -> U256 {
        let rollup_data_gas_cost = self.data_gas(input, spec_id);
        rollup_data_gas_cost
            .saturating_add(self.l1_fee_overhead.unwrap_or_default())
            .saturating_mul(self.l1_base_fee)
            .saturating_mul(self.l1_base_fee_scalar)
            .wrapping_div(U256::from(1_000_000))
    }

    /// Calculate the gas cost of a transaction based on L1 block data posted on L2, post-Ecotone.
    ///
    /// [OpSpecId::ECOTONE] L1 cost function:
    /// `(calldataGas/16)*(l1BaseFee*16*l1BaseFeeScalar + l1BlobBaseFee*l1BlobBaseFeeScalar)/1e6`
    ///
    /// We divide "calldataGas" by 16 to change from units of calldata gas to "estimated # of bytes when compressed".
    /// Known as "compressedTxSize" in the spec.
    ///
    /// Function is actually computed as follows for better precision under integer arithmetic:
    /// `calldataGas*(l1BaseFee*16*l1BaseFeeScalar + l1BlobBaseFee*l1BlobBaseFeeScalar)/16e6`
    fn calculate_tx_l1_cost_ecotone(&self, input: &[u8], spec_id: OpSpecId) -> U256 {
        // There is an edgecase where, for the very first Ecotone block (unless it is activated at Genesis), we must
        // use the Bedrock cost function. To determine if this is the case, we can check if the Ecotone parameters are
        // unset.
        if self.empty_ecotone_scalars {
            return self.calculate_tx_l1_cost_bedrock(input, spec_id);
        }

        let rollup_data_gas_cost = self.data_gas(input, spec_id);
        let l1_fee_scaled = self.calculate_l1_fee_scaled_ecotone();

        l1_fee_scaled
            .saturating_mul(rollup_data_gas_cost)
            .wrapping_div(U256::from(1_000_000 * NON_ZERO_BYTE_COST))
    }

    // l1BaseFee*16*l1BaseFeeScalar + l1BlobBaseFee*l1BlobBaseFeeScalar
    fn calculate_l1_fee_scaled_ecotone(&self) -> U256 {
        let calldata_cost_per_byte = self
            .l1_base_fee
            .saturating_mul(U256::from(NON_ZERO_BYTE_COST))
            .saturating_mul(self.l1_base_fee_scalar);
        let blob_cost_per_byte = self
            .l1_blob_base_fee
            .unwrap_or_default()
            .saturating_mul(self.l1_blob_base_fee_scalar.unwrap_or_default());

        calldata_cost_per_byte.saturating_add(blob_cost_per_byte)
    }
}
