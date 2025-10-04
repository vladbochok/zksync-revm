use revm::{
    context::{Cfg, JournalTr}, context_interface::ContextTr, handler::{EthPrecompiles, PrecompileProvider}, interpreter::{Gas, InputsImpl, InstructionResult, InterpreterResult}, precompile::{
        self, bn254, secp256r1, Precompile, PrecompileError, PrecompileId, PrecompileResult, Precompiles
    }, primitives::{address, hardfork::SpecId, ruint::aliases::B160, Address, OnceLock, B256, U256}, Database
};
use core::ops::Add;
use std::string::String;

use crate::OpSpecId;

// setBytecodeDetailsEVM(address,bytes32,uint32,bytes32) - f6eca0b0
pub const SET_EVM_BYTECODE_DETAILS: &[u8] = &[0xf6, 0xec, 0xa0, 0xb0];
// Contract Deployer system hook (contract) needed for all envs (force deploy)
pub const CONTRACT_DEPLOYER_ADDRESS: Address = address!("0000000000000000000000000000000000008006");

pub const L2_GENESIS_UPGRADE_ADDRESS: Address =
    address!("0000000000000000000000000000000000010001");

pub const MAX_CODE_SIZE: usize = 0x6000;

/// Run the deployer precompile.
pub fn deployer_precompile_call<CTX>(
    ctx: &mut CTX,
    caller: Address,
    is_static: bool,
    gas_limit: u64,
    mut calldata: &[u8],
) -> InterpreterResult
where
    CTX: ContextTr<Cfg: Cfg<Spec = OpSpecId>>,
{
    if calldata.len() < 4 {
        return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(10));
    }
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&calldata[..4]);
    panic!("entry");
    match selector {
        s if s == SET_EVM_BYTECODE_DETAILS => {
            if is_static {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(10));
            }
            // in future we need to handle regular(not genesis) protocol upgrades
            if caller != L2_GENESIS_UPGRADE_ADDRESS {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(10));
            }

            // decoding according to setDeployedCodeEVM(address,bytes)
            calldata = &calldata[4..];
            if calldata.len() < 128 {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(10));
            }

            // check that first 12 bytes in address encoding are zero
            if calldata[0..12].iter().any(|byte| *byte != 0) {
                return InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(10));
            }
            let address = Address::from_slice(&calldata[12..32]);

            let _bytecode_hash =
                B256::from_slice(calldata[32..64].try_into().expect("Always valid"));

            let bytecode_length: u32 = match U256::from_be_slice(&calldata[64..96]).try_into() {
                Ok(length) => length,
                Err(_) => {
                    return InterpreterResult::new(
                        InstructionResult::Revert,
                        [].into(),
                        Gas::new(10),
                    );
                }
            };

            let observable_bytecode_hash =
                B256::from_slice(calldata[96..128].try_into().expect("Always valid"));

            // Although this can be called as a part of protocol upgrade,
            // we are checking the next invariants, just in case
            // EIP-158: reject code of length > 24576.
            if bytecode_length as usize > MAX_CODE_SIZE {
                return InterpreterResult::new(
                    InstructionResult::Revert,
                    [].into(),
                    Gas::new(gas_limit),
                );
            }

            let bytecode = ctx.db_mut().code_by_hash(observable_bytecode_hash).expect(
                "The bytecode is expected to be pre-loaded for any deployer precompile call",
            );
            panic!("bytecode is small {:?}", address);
            ctx.journal_mut().set_code(address, bytecode);
            InterpreterResult::new(InstructionResult::Return, [].into(), Gas::new(10))
        }
        _ => InterpreterResult::new(InstructionResult::Revert, [].into(), Gas::new(10)),
    }
}
