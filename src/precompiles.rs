//! Contains ZKsync OS specific precompiles.
use crate::OpSpecId;
use std::vec;
use revm::{
    context::{Cfg, LocalContextTr},
    context_interface::ContextTr,
    handler::{EthPrecompiles, PrecompileProvider},
    interpreter::{Gas, InputsImpl, InstructionResult, InterpreterResult},
    precompile::{
        self, bn254, secp256r1, Precompile, PrecompileError, PrecompileId, PrecompileResult,
        Precompiles,
    },
    primitives::{address, hardfork::SpecId, Address, OnceLock},
};
use std::boxed::Box;
use std::string::String;
pub mod deployer;

use deployer::{CONTRACT_DEPLOYER_ADDRESS, deployer_precompile_call};

pub const L1_MESSENGER_ADDRESS: Address = address!("0000000000000000000000000000000000008008");
pub const L2_BASE_TOKEN_ADDRESS: Address =  address!("000000000000000000000000000000000000800a");

/// Optimism precompile provider
#[derive(Debug, Clone)]
pub struct ZKsyncPrecompiles {
    /// Inner precompile provider is same as Ethereums.
    inner: EthPrecompiles,
    /// Spec id of the precompile provider.
    spec: OpSpecId,
}

impl ZKsyncPrecompiles {
    /// Create a new precompile provider with the given OpSpec.
    #[inline]
    pub fn new_with_spec(spec: OpSpecId) -> Self {
        // TODO: remove unneded precompiles
        let precompiles = Precompiles::new(spec.into_eth_spec().into());
        Self {
            inner: EthPrecompiles {
                precompiles,
                spec: SpecId::default(),
            },
            spec,
        }
    }

    /// Precompiles getter.
    #[inline]
    pub fn precompiles(&self) -> &'static Precompiles {
        self.inner.precompiles
    }
}

impl<CTX> PrecompileProvider<CTX> for ZKsyncPrecompiles
where
    CTX: ContextTr<Cfg: Cfg<Spec = OpSpecId>>,
{
    type Output = InterpreterResult;

    #[inline]
    fn set_spec(&mut self, spec: <CTX::Cfg as Cfg>::Spec) -> bool {
        if spec == self.spec {
            return false;
        }
        *self = Self::new_with_spec(spec);
        true
    }

    #[inline]
    fn run(
        &mut self,
        context: &mut CTX,
        address: &Address,
        inputs: &InputsImpl,
        is_static: bool,
        gas_limit: u64,
    ) -> Result<Option<Self::Output>, String> {
        if *address == CONTRACT_DEPLOYER_ADDRESS {   
            let input_bytes = match &inputs.input {
                revm::interpreter::CallInput::SharedBuffer(range) => {
                    if let Some(slice) = context.local().shared_memory_buffer_slice(range.clone()) {
                        slice.to_vec()
                    } else {
                        vec![]
                    }
                }
                revm::interpreter::CallInput::Bytes(bytes) => bytes.0.to_vec(),
            };
            return Ok(Some(deployer_precompile_call(inputs.caller_address, is_static, gas_limit, &input_bytes)));
        } else if *address == L1_MESSENGER_ADDRESS {
            // TODO: write the precompile 
            return Ok(Some(InterpreterResult::new(InstructionResult::Return, [].into(), Gas::new(gas_limit))));
        } else if *address == L2_BASE_TOKEN_ADDRESS {
            return Ok(Some(InterpreterResult::new(InstructionResult::Return, [].into(), Gas::new(gas_limit))));
        }

        self.inner
            .run(context, address, inputs, is_static, gas_limit)
    }

    #[inline]
    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        self.inner.warm_addresses()
    }

    #[inline]
    fn contains(&self, address: &Address) -> bool {
        self.inner.contains(address)
    }
}

impl Default for ZKsyncPrecompiles {
    fn default() -> Self {
        Self::new_with_spec(OpSpecId::Initial)
    }
}
