use revm::{
    context::Cfg,
    context_interface::ContextTr,
    handler::{EthPrecompiles, PrecompileProvider},
    interpreter::{Gas, InputsImpl, InstructionResult, InterpreterResult},
    precompile::{
        self, bn254, secp256r1, Precompile, PrecompileError, PrecompileId, PrecompileResult,
        Precompiles,
    },
    primitives::{address, hardfork::SpecId, ruint::aliases::B160, Address, OnceLock, U256},
};
use std::string::String;

// setDeployedCodeEVM(address,bytes) - 1223adc7
const SET_DEPLOYED_CODE_EVM_SELECTOR: &[u8] = &[0x12, 0x23, 0xad, 0xc7];
// Contract Deployer system hook (contract) needed for all envs (force deploy)
pub const CONTRACT_DEPLOYER_ADDRESS: Address = address!("0000000000000000000000000000000000008006");

pub const L2_GENESIS_UPGRADE_ADDRESS: Address = address!("0000000000000000000000000000000000010001");

pub const MAX_CODE_SIZE: usize = 0x6000;

/// Run the deployer precompile.
pub fn deployer_precompile_call(caller: Address, is_static: bool, gas_limit: u64, mut calldata: &[u8]) -> InterpreterResult {
    if calldata.len() < 4 {
        return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
    }
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&calldata[..4]);

    match selector {
        s if s == SET_DEPLOYED_CODE_EVM_SELECTOR => {
            if is_static {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
            }
            // in future we need to handle regular(not genesis) protocol upgrades
            if caller != L2_GENESIS_UPGRADE_ADDRESS {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
            }

            // decoding according to setDeployedCodeEVM(address,bytes)
            calldata = &calldata[4..];
            if calldata.len() < 64 {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
            }

            // check that first 12 bytes in address encoding are zero
            if calldata[0..12].iter().any(|byte| *byte != 0) {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
            }
            let address = match B160::try_from_be_slice(&calldata[12..32]) {
                Some(address) => address,
                None => return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit)),
            };


            let bytecode_offset: usize = match U256::from_be_slice(&calldata[32..64]).try_into() {
                Ok(offset) => offset,
                Err(_) => return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit)),
            };

            let bytecode_length_encoding_end = match bytecode_offset.checked_add(32) {
                Some(deployments_encoding_end) => deployments_encoding_end,
                None => return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit)),
            };
            let bytecode_length: usize = match U256::from_be_slice(
                &calldata[bytecode_length_encoding_end - 32..bytecode_length_encoding_end],
            )
            .try_into()
            {
                Ok(length) => length,
                Err(_) => return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit)),
            };

            if calldata.len() < bytecode_length_encoding_end + bytecode_length {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
            }

            let bytecode = &calldata
                [bytecode_length_encoding_end..bytecode_length_encoding_end + bytecode_length];

            // Although this can be called as a part of protocol upgrade,
            // we are checking the next invariants, just in case
            // EIP-3541: reject code starting with 0xEF.
            // EIP-158: reject code of length > 24576.
            if !bytecode.is_empty() && bytecode[0] == 0xEF || bytecode.len() > MAX_CODE_SIZE {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit));
            }

            // system.io.deploy_code(
            //     ExecutionEnvironmentType::EVM,
            //     resources,
            //     &address,
            //     bytecode,
            //     bytecode.len() as u32,
            //     0,
            // )?;
            InterpreterResult::new(InstructionResult::Return, [].into(), Gas::new(gas_limit))
        }
        _ => InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(gas_limit)),
    }
}
